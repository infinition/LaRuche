//! Multi-provider LLM streaming abstraction.
//!
//! Supports:
//! - **ollama** (default): local Ollama instance
//! - **openai**: OpenAI-compatible APIs (Deepseek, Together, Groq, etc.)
//! - **anthropic**: Anthropic Claude API
//!
//! All providers support **native tool calling** (OpenAI `tools:` format)
//! when a tools array is provided. The parser accumulates `tool_calls` from
//! streaming chunks and delivers them on the final chunk.

use crate::brain::ToolCall;
use crate::streaming::{ollama_chat_stream, OllamaChunk};
use anyhow::Result;
use futures_util::Stream;
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

/// Structured provider error (HTTP code + body) returned on non-2xx responses.
#[derive(Debug, Clone)]
pub struct ProviderError {
    pub status: u16,
    pub body: String,
    pub retry_after: Option<String>,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider API error {}: {}", self.status, self.body)
    }
}

impl std::error::Error for ProviderError {}

/// Converts the LaRuche tool format (name, description, parameters)
/// to the OpenAI `tools` format (type: function, function: {name, description, parameters}).
pub fn convertir_tools_openai(tools: &[serde_json::Value]) -> Vec<serde_json::Value> {
    tools.iter().filter_map(|t| {
        let name = t["name"].as_str()?;
        let description = t["description"].as_str().unwrap_or("");
        let parameters = t.get("parameters").cloned().unwrap_or(serde_json::json!({}));
        Some(serde_json::json!({
            "type": "function",
            "function": {
                "name": name,
                "description": description,
                "parameters": parameters,
            }
        }))
    }).collect()
}

/// Last gate before the wire: refuse a `tools` entry a provider will reject outright.
///
/// A single malformed entry kills the whole turn, and the message comes back from the
/// far end in a shape that says nothing about which tool produced it:
/// `tools[3].function: missing field name` (deepseek), 400, no fallback, turn lost.
/// Two defects have that effect and both are cheap to see from here:
///
/// 1. a `function` object with no usable `name`. Nothing in `convertir_tools_openai`
///    can emit one, yet a 400 naming exactly that reached a user, so the payload is
///    checked rather than assumed. The entry is dropped and its keys are logged, which
///    is what a next occurrence needs in order to be traced back to its source.
/// 2. two entries sharing a name ("Tool names must be unique"), which a registry
///    holding a plugin or MCP tool that shadows a builtin can produce.
///
/// Dropping one capability costs the model one tool it probably was not about to
/// call. Losing the turn costs everything it had done so far.
fn assainir_tools_openai(tools: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let mut vus: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut propres = Vec::with_capacity(tools.len());
    for (i, t) in tools.into_iter().enumerate() {
        let nom = t["function"]["name"].as_str().unwrap_or("").trim().to_string();
        if nom.is_empty() {
            let cles: Vec<&str> = t["function"]
                .as_object()
                .map(|o| o.keys().map(String::as_str).collect())
                .unwrap_or_default();
            tracing::error!(
                target: "provider",
                index = i,
                cles_function = ?cles,
                entree = %t.to_string().chars().take(200).collect::<String>(),
                "tool definition without a usable name: dropped before sending"
            );
            continue;
        }
        if !vus.insert(nom.clone()) {
            tracing::warn!(target: "provider", tool = %nom, "duplicate tool definition: dropped before sending");
            continue;
        }
        propres.push(t);
    }
    propres
}

/// Shared HTTP client for OpenAI-compatible backends, pinned to HTTP/1.1.
///
/// Large request bodies were being cut in flight. The dumps settle what we emit:
/// valid JSON, pure ASCII, surrogate pairs balanced, correctly terminated. Yet the
/// provider's parser kept stopping at, or a few kilobytes short of, the end of the
/// body, at a point that MOVED between attempts (111677 of 111677, then 81733 of
/// 86010). A payload defect is deterministic; a moving cut is transport. Truncated
/// large POSTs under HTTP/2 flow control is a known failure mode, and reqwest
/// negotiates h2 by default over ALPN.
///
/// Also reused instead of rebuilt per call: the previous code created a fresh
/// client for every request, paying a TLS handshake each time.
///
/// Set `LARUCHE_HTTP2=1` to negotiate h2 again, to compare.
fn client_openai() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        let h2 = std::env::var("LARUCHE_HTTP2").map(|v| v == "1").unwrap_or(false);
        let b = reqwest::Client::builder().timeout(std::time::Duration::from_secs(600));
        if h2 { b } else { b.http1_only() }
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Re-encode a JSON document so every character is ASCII, escaping the rest as `\uXXXX`.
///
/// serde_json emits real UTF-8, which is valid JSON and normally fine. It stopped
/// being fine against one gateway that counts our BYTES as characters: it read a
/// body of 83250 bytes / 83145 chars and reported "EOF while parsing a string at
/// column 83250", the byte figure. Reading multi-byte UTF-8 one byte at a time
/// walks off the end of the document, inside a string, every time. Deterministic,
/// which is why three retries failed identically.
///
/// With `\uXXXX` escapes there is no multi-byte character left on the wire: bytes
/// and characters coincide and no consumer can mis-frame them. The document is
/// strictly equivalent (JSON escapes are part of the spec) and only grows by the
/// few accented characters a prompt carries. Non-ASCII only ever appears INSIDE
/// string literals in serde_json output, so escaping it can never touch structure.
fn json_ascii(brut: &str) -> String {
    if brut.is_ascii() {
        return brut.to_string();
    }
    let mut out = String::with_capacity(brut.len() + 16);
    let mut tampon = [0u16; 2];
    for c in brut.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            // Chars beyond the BMP need their surrogate pair, as JSON requires.
            for unite in c.encode_utf16(&mut tampon) {
                out.push_str(&format!("\\u{unite:04x}"));
            }
        }
    }
    out
}

/// Bring a request body under a BYTE budget, which the token gauge cannot enforce.
///
/// The gauge reasons in tokens against the model's window. The wall we keep hitting
/// is counted in bytes and sits far earlier: a refused body of 114497 bytes read as
/// 15% of a 128k window, so no compaction ever triggered. Two levers, in order of
/// how much they actually save:
///
/// 1. the fattest tool results, largest first, keeping head and tail. One web page or
///    file read routinely lands 16 to 24 KB, and a handful of them IS the body.
///    Losing the middle of one page beats losing the turn, and the marker tells the
///    model what happened so it can re-read a narrower range.
/// 2. the tool list, trimmed from the tail, and only when step 1 was not enough.
///
/// 3. the arguments of past tool CALLS, which lever 1 cannot see: it reads
///    `content`, and an assistant message carrying a call has `content: null`.
/// 4. the tool list, trimmed from the tail.
/// 5. the oldest exchanges, dropped outright, and only when nothing else is left.
///
/// The order used to be the reverse, and it was wrong on measurement. The 33 native
/// tools serialize to 17911 bytes, 542 on average. Cutting them to the old floor of
/// 4 saves 15888 bytes, a fifth of the guard, while removing seven eighths of what
/// the agent can DO. On the bodies that actually trip the guard, 100 KB and up, that
/// trim cannot get under the limit on its own: it just ships a crippled agent and
/// fails anyway. Tools are the last-but-one lever, and the floor is 12.
///
/// Lever 5 replaces a promise this function could not keep. It used to say messages
/// are never dropped, and it shipped whatever it had left when the levers ran out.
/// The refused body of 2026-08-27 is what that costs: 81812 bytes for a 76800 limit,
/// 196 messages already cut to 100 characters of head and tail, 12 tools which is
/// the floor, and 31.5 KB of system prompts and tool schemas that no lever touches.
/// It went out, was refused three times and killed the turn. Losing the oldest
/// exchanges beats losing the whole turn, so they go, the newest ones and the
/// mission always kept.
fn reduire_sous_budget(body: &serde_json::Value, limite: usize) -> Result<serde_json::Value> {
    let mut reduit = body.clone();
    let taille = |v: &serde_json::Value| -> usize {
        serde_json::to_string(v).map(|s| json_ascii(&s).len()).unwrap_or(0)
    };

    if taille(&reduit) > limite {
        // EVERY message is a candidate except the system prompt and the first user
        // turn, which carries the mission itself. Restricting this to `role == "tool"`
        // left the real offenders untouched: an observation that failed native
        // correlation comes back as a `user` message, and the curator sends the whole
        // mission transcript as ONE user message (measured: 109 KB, the largest body
        // of the run). Trimming what we can see beats refusing to look.
        let premier_user = reduit["messages"]
            .as_array()
            .and_then(|ms| ms.iter().position(|m| m["role"] == "user"));
        let mut par_taille: Vec<(usize, usize)> = reduit["messages"]
            .as_array()
            .map(|ms| {
                ms.iter()
                    .enumerate()
                    .filter(|(i, m)| m["role"] != "system" && Some(*i) != premier_user)
                    .map(|(i, m)| (i, m["content"].as_str().map(str::len).unwrap_or(0)))
                    .collect()
            })
            .unwrap_or_default();
        par_taille.sort_by_key(|p| std::cmp::Reverse(p.1));

        // If the mission turn is itself the whole payload (curator review, scout
        // briefing), it has to give ground too: it is that or the call fails.
        if par_taille.is_empty() {
            if let Some(i) = premier_user {
                let n = reduit["messages"][i]["content"].as_str().map(str::len).unwrap_or(0);
                par_taille.push((i, n));
            }
        }

        // ADAPTIVE, in successive passes. A fixed 1500-char keep only bites on very
        // large observations, and the shape that actually breaks a research run is
        // the opposite: no whale, a shoal. A refused body carried 23 tool results of
        // roughly 3000 chars each, 73 KB in total, every single one just under the
        // threshold, so nothing was ever cut. Each pass keeps less, until it fits.
        for garde in [1_500usize, 800, 400, 200, 100] {
            if taille(&reduit) <= limite {
                break;
            }
            for (idx, _) in &par_taille {
                if taille(&reduit) <= limite {
                    break;
                }
                let Some(texte) = reduit["messages"][*idx]["content"].as_str().map(str::to_string)
                else {
                    continue;
                };
                let total = texte.chars().count();
                if total <= garde * 2 + 200 {
                    continue; // already at or below what this pass would keep
                }
                let chars: Vec<char> = texte.chars().collect();
                let tete: String = chars[..garde].iter().collect();
                let queue: String = chars[chars.len() - garde..].iter().collect();
                reduit["messages"][*idx]["content"] = serde_json::json!(format!(
                    "{tete}\n\n[... {} chars cut to fit the request budget - re-read a narrower range if you need the middle ...]\n\n{queue}",
                    total - garde * 2
                ));
            }
        }
    }

    // Les arguments des appels d'outils passes, angle mort du levier precedent.
    //
    // Il ne regarde que `content`, et un message assistant qui porte un appel a
    // `content: null`: toute sa masse est dans `tool_calls[].function.arguments`.
    // Sur le corps refuse du 2026-08-27, 93 messages assistant sur 196, cela
    // faisait 10100 caracteres parfaitement invisibles au rognage.
    //
    // Le remplacement reste un objet JSON valide, pas une chaine coupee au
    // milieu: une gateway qui reparse les arguments refuserait alors le corps
    // pour une raison bien pire que sa taille, et la reparation serait la panne.
    if taille(&reduit) > limite {
        let porteurs: Vec<usize> = reduit["messages"]
            .as_array()
            .map(|ms| {
                ms.iter()
                    .enumerate()
                    .filter(|(_, m)| m["tool_calls"].is_array())
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default();
        for i in porteurs {
            if taille(&reduit) <= limite {
                break;
            }
            let Some(appels) = reduit["messages"][i]["tool_calls"].as_array().cloned() else {
                continue;
            };
            for (j, appel) in appels.iter().enumerate() {
                let Some(args) = appel["function"]["arguments"].as_str() else {
                    continue;
                };
                let total = args.chars().count();
                if total <= ARGUMENTS_GARDE * 2 + 200 {
                    continue;
                }
                let chars: Vec<char> = args.chars().collect();
                let tete: String = chars[..ARGUMENTS_GARDE].iter().collect();
                let queue: String = chars[chars.len() - ARGUMENTS_GARDE..].iter().collect();
                let coupe = serde_json::json!({
                    "_tronque": format!(
                        "{tete} [... {} caracteres coupes pour tenir dans le budget ...] {queue}",
                        total - ARGUMENTS_GARDE * 2
                    )
                });
                reduit["messages"][i]["tool_calls"][j]["function"]["arguments"] =
                    serde_json::json!(coupe.to_string());
            }
        }
    }

    // Avant-dernier levier. Worth little in bytes (see the header), so it runs
    // after everything else and stops at 12 tools rather than 4: below that the
    // agent loses the web, the files and the shell at once, which costs far more
    // than the kilobytes it buys.
    if taille(&reduit) > limite {
        if let Some(liste) = body.get("tools").and_then(|t| t.as_array()).cloned() {
            let mut gardes = liste.len();
            while gardes > PLANCHER_OUTILS && taille(&reduit) > limite {
                gardes = gardes.saturating_sub(2).max(PLANCHER_OUTILS);
                reduit["tools"] = serde_json::json!(liste[..gardes]);
            }
        }
    }

    // Dernier recours, quand il ne reste plus rien a rendre.
    if taille(&reduit) > limite {
        elaguer_les_plus_vieux(&mut reduit, limite);
    }

    // Le corps part quand meme: la limite est un garde place SOUS un mur observe,
    // pas le mur lui-meme, et un corps un peu au-dessus passe souvent. Mais si la
    // requete est refusee ensuite, ce journal dit que ce n'etait pas faute d'avoir
    // essaye, et donne le chiffre irreductible a regarder.
    let reste = taille(&reduit);
    if reste > limite {
        tracing::warn!(
            target: "provider",
            taille = reste,
            limite,
            messages = reduit["messages"].as_array().map(Vec::len).unwrap_or(0),
            outils = reduit["tools"].as_array().map(Vec::len).unwrap_or(0),
            "budget de corps: tous les leviers epuises, le corps reste au-dessus"
        );
    }
    Ok(reduit)
}

/// Messages recents jamais elagues: ils portent le fil en cours, et les perdre
/// coute plus cher que de perdre un vieil echange.
const QUEUE_INTOUCHABLE: usize = 16;

/// Tete et queue gardees d'arguments d'appel d'outil.
const ARGUMENTS_GARDE: usize = 300;

/// Retire les plus vieux echanges jusqu'a tenir dans le budget.
///
/// Trois choses sont protegees: les messages systeme, le premier tour utilisateur
/// qui porte la mission, et les [`QUEUE_INTOUCHABLE`] derniers messages.
///
/// Un appel d'outil part avec ses reponses, et une deuxieme passe ramasse les
/// `tool` devenus orphelins. Un `tool` sans son appel, ou un `tool_calls` sans
/// reponse, fait refuser le corps par les gateways strictes: elaguer de travers
/// remplacerait un refus pour taille par un refus pour structure.
fn elaguer_les_plus_vieux(reduit: &mut serde_json::Value, limite: usize) {
    let taille = |v: &serde_json::Value| -> usize {
        serde_json::to_string(v)
            .map(|s| json_ascii(&s).len())
            .unwrap_or(0)
    };
    let Some(messages) = reduit["messages"].as_array().cloned() else {
        return;
    };
    let premier_user = messages.iter().position(|m| m["role"] == "user");
    let debut_queue = messages.len().saturating_sub(QUEUE_INTOUCHABLE);
    let protege = |i: usize| -> bool {
        i >= debut_queue || messages[i]["role"] == "system" || Some(i) == premier_user
    };

    // Le poids d'un message, plus la virgule qui le separe du suivant. Estimer
    // ainsi evite de reserialiser le corps entier a chaque retrait, ce qui serait
    // quadratique sur deux cents messages.
    let poids: Vec<usize> = messages.iter().map(|m| taille(m) + 1).collect();
    let mut total = taille(reduit);
    let mut retire = vec![false; messages.len()];

    for i in 0..messages.len() {
        if total <= limite {
            break;
        }
        if protege(i) || retire[i] {
            continue;
        }
        let mut groupe = vec![i];
        if let Some(appels) = messages[i]["tool_calls"].as_array() {
            let ids: Vec<&str> = appels.iter().filter_map(|a| a["id"].as_str()).collect();
            for (j, m) in messages.iter().enumerate().skip(i + 1) {
                if m["role"] == "tool"
                    && m["tool_call_id"].as_str().is_some_and(|id| ids.contains(&id))
                {
                    groupe.push(j);
                }
            }
        }
        // Un groupe dont une part est protegee reste entier: mieux vaut garder un
        // vieil echange de trop qu'une moitie d'echange.
        if groupe.iter().any(|j| protege(*j)) {
            continue;
        }
        for j in groupe {
            if !retire[j] {
                retire[j] = true;
                total = total.saturating_sub(poids[j]);
            }
        }
    }

    if !retire.iter().any(|r| *r) {
        return;
    }

    let ids_vivants: std::collections::HashSet<&str> = messages
        .iter()
        .enumerate()
        .filter(|(i, _)| !retire[*i])
        .filter_map(|(_, m)| m["tool_calls"].as_array())
        .flatten()
        .filter_map(|a| a["id"].as_str())
        .collect();
    for i in 0..messages.len() {
        if retire[i] || messages[i]["role"] != "tool" {
            continue;
        }
        if let Some(id) = messages[i]["tool_call_id"].as_str() {
            if !ids_vivants.contains(id) {
                retire[i] = true;
            }
        }
    }

    let combien = retire.iter().filter(|r| **r).count();
    let mut gardes: Vec<serde_json::Value> = messages
        .iter()
        .enumerate()
        .filter(|(i, _)| !retire[*i])
        .map(|(_, m)| m.clone())
        .collect();

    // Le dire au modele, dans le message de mission qui survit toujours: sinon il
    // refait le travail dont il ne voit plus la trace.
    if let Some(i) = gardes.iter().position(|m| m["role"] == "user") {
        if let Some(texte) = gardes[i]["content"].as_str() {
            gardes[i]["content"] = serde_json::json!(format!(
                "{texte}\n\n[... {combien} anciens echanges retires pour tenir dans le budget de la requete: si un detail ancien te manque, relis ta memoire ou refais l'observation ...]"
            ));
        }
    }
    let restants = gardes.len();
    reduit["messages"] = serde_json::json!(gardes);
    tracing::warn!(
        target: "provider",
        retires = combien,
        restants,
        "budget de corps: les plus vieux echanges ont ete retires"
    );
}

/// Fewest tools the trim may leave. The old value was 4, which took the agent
/// down to a fraction of its capability for a fifth of the guard in bytes.
const PLANCHER_OUTILS: usize = 12;

/// Byte budget for a request body, per provider.
///
/// THE WALL IS REAL, and this was established the hard way. It was briefly raised
/// to 256 KB on the theory that the clustered rejection columns (81733, 81736,
/// 81813, 82233) were complaints ABOUT the tool schemas rather than truncation,
/// since they all fall inside the `tools` block, the body's last key in
/// alphabetical order. That theory is dead: on 2026-08-26 a body of 84436 bytes
/// was refused four times running, with a garbled path
/// (`tools[15].function.parameters.properties.?: key must be a string`) pointing
/// at column 84061, near the end. Our body was pure ASCII, every tool schema
/// structurally sound, and it re-parsed locally as valid JSON. The gateway cuts
/// the upload around 80 KB and its parser then reports nonsense at the cut.
///
/// The evidence, across every refused body ever dumped to `%TEMP%`: the smallest
/// is 83451 bytes. Nothing under 80 KB has EVER been refused. So 76800 is not a
/// misdiagnosis, it is a well-placed guard just under an observed wall.
///
/// What the misadventure did leave behind, and what is kept: the trim ORDER, now
/// observations first and tools last (see [`reduire_sous_budget`]). Being under
/// the wall is non-negotiable; arriving there with 12 tools instead of 4 is free.
fn limite_corps(base: &str) -> usize {
    // No gateway in the middle to cut the upload: the only limit is what the
    // local runtime accepts. This part of the per-provider split holds.
    if is_local_base_url(base) {
        return 4 * 1024 * 1024;
    }
    // Every remote gateway gets the measured-safe value. No provider has been
    // shown to accept more, and a summit is no place to find out.
    76_800
}

/// Append what we actually sent to a provider error, so a parse complaint can be read.
///
/// Turns "EOF while parsing a string at line 1 column 106091" into something
/// decidable: if that column equals the end of our body, they received a truncated
/// upload; if it falls inside, the payload itself is the problem. When the provider
/// names the offending message (`messages[14].content`), its size is reported too.
fn diagnostiquer_corps(
    erreur: &str,
    corps_brut: &str,
    messages: &[serde_json::Value],
) -> String {
    let mut notes = vec![format!(
        "we sent {} bytes / {} chars, {} messages",
        corps_brut.len(),
        corps_brut.chars().count(),
        messages.len()
    )];

    // THE decisive check. serde_json cannot emit invalid JSON, so if our own body
    // parses here while the provider refuses it, the fault is not in what we built.
    // Cheap (sub-millisecond on ~80 KB) and it ends an entire class of speculation.
    match serde_json::from_str::<serde_json::Value>(corps_brut) {
        Ok(_) => notes.push("OUR body re-parses locally as valid JSON".into()),
        Err(e) => notes.push(format!("OUR OWN body is invalid JSON: {e} <-- our bug")),
    }

    // Keep the exact bytes whenever the far end says our PAYLOAD is at fault, so the
    // next occurrence is decidable instead of being re-derived from a column number.
    //
    // The condition used to be the single phrase "parse the request body", which is one
    // gateway's wording. A refusal reading `Failed to deserialize the JSON body into the
    // target type: tools[3].function: missing field name` matched none of it, so nothing
    // was written and the only evidence left was a byte count. Any complaint that names a
    // field, a path or the deserializer now dumps.
    let motif = erreur.to_lowercase();
    let corps_en_cause = ["parse the request body", "deserialize", "invalid schema", "missing field", "invalid type"]
        .iter()
        .any(|m| motif.contains(m));
    if corps_en_cause {
        let chemin = std::env::temp_dir().join(format!(
            "laruche-corps-refuse-{}.json",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));
        if std::fs::write(&chemin, corps_brut).is_ok() {
            notes.push(format!("body dumped to {}", chemin.display()));
        }
    }

    // "... at line 1 column 106091" -> compare with where our body ends.
    if let Some(reste) = erreur.rsplit("column ").next() {
        let chiffres: String = reste.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(colonne) = chiffres.parse::<usize>() {
            let fin = corps_brut.chars().count();
            let verdict = if colonne >= fin.saturating_sub(2) {
                "their parser stopped AT THE END of our body: truncated upload"
            } else {
                "their parser stopped INSIDE our body: payload issue, not transport"
            };
            notes.push(format!("column {colonne} vs our end {fin}: {verdict}"));
        }
    }

    // "messages[14].content" -> how big is ours?
    if let Some(reste) = erreur.split("messages[").nth(1) {
        let idx: String = reste.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(i) = idx.parse::<usize>() {
            if let Some(m) = messages.get(i) {
                let taille = m["content"].as_str().map(str::len).unwrap_or(0);
                notes.push(format!("our messages[{i}].content is {taille} bytes"));
            }
        }
    }

    format!("{erreur} [{}]", notes.join(" | "))
}

/// Finalize the streaming tool-call accumulator into an ordered list. The accumulator is
/// keyed by the streaming `index`, so we sort by that (NOT by the provider-random `id`),
/// which preserves the model's intended order of parallel tool calls.
///
/// DRAINING on purpose. A stream can reach a finalization point more than once: OpenAI-compatible
/// providers send a chunk carrying `finish_reason`, then the `data: [DONE]` sentinel, and both
/// call sites finalize. Leaving the accumulator populated made the second pass re-emit the SAME
/// ids; the consumer appends them and the next request carries `tool_calls: [X, X]`, which the
/// API rejects with `Duplicate value for 'tool_call_id'` (seen on deepseek-v4-flash).
fn finaliser_tool_calls(
    acc: &mut std::collections::HashMap<u32, (String, String, String)>,
) -> Option<Vec<ToolCall>> {
    if acc.is_empty() {
        return None;
    }
    let mut calls: Vec<(u32, ToolCall)> = acc
        .drain()
        .map(|(idx, (id, name, args_str))| {
            (
                idx,
                ToolCall {
                    id,
                    name,
                    args: serde_json::from_str(&args_str).unwrap_or(serde_json::Value::Null),
                },
            )
        })
        .collect();
    calls.sort_by_key(|(idx, _)| *idx);
    Some(calls.into_iter().map(|(_, c)| c).collect())
}

/// Reasoning effort asked of a thinking-capable model. Provider-neutral here;
/// each backend maps it to its own knob (`reasoning_effort` for OpenAI/Codex,
/// a `thinking` budget for Anthropic). Free-form so a new level (`max`,
/// `ultra`...) works without a code change.
///
/// The point is **granularity**: a MoA advisor should think hard while the
/// synthesizer answers fast, and a background task (curateur, judge) should
/// never burn a deep-reasoning budget.
pub type Effort<'a> = Option<&'a str>;

/// Token budget granted to Anthropic "extended thinking" per effort level.
/// `None` = no thinking block (the model answers directly).
fn budget_pensee(effort: &str) -> Option<u32> {
    match effort.trim().to_lowercase().as_str() {
        "none" | "off" | "minimal" => None,
        "low" => Some(4_000),
        "medium" => Some(10_000),
        "high" => Some(24_000),
        "max" | "ultra" => Some(48_000),
        _ => None,
    }
}

/// Unified streaming entry point: dispatches to the correct provider.
#[allow(clippy::too_many_arguments)]
pub async fn provider_chat_stream(
    provider: &str,
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
    ollama_url: &str,
    tools: Option<&[serde_json::Value]>,  // new parameter
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    provider_chat_stream_effort(
        provider, model, messages, temperature, max_tokens, api_key, api_base, ollama_url, tools,
        None,
    )
    .await
}

/// Same, with an explicit **reasoning effort**. Callers that care (the main
/// butinage loop, MoA roles) use this one; everything else keeps the plain
/// entry point, which passes `None`.
#[allow(clippy::too_many_arguments)]
pub async fn provider_chat_stream_effort(
    provider: &str,
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
    ollama_url: &str,
    tools: Option<&[serde_json::Value]>,
    effort: Effort<'_>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    // Vault reference (`@@NAME` / `${NAME}`) resolved HERE, at the single door every
    // provider call goes through. It used to be each caller's job, and callers
    // forget: seven paths substituted, several others passed `profile.api_key`
    // straight down. A key that resolves on one route and not another is worse
    // than no vault at all, because it silently pushes the user back to storing
    // the raw value in `provider-profiles.json`, which is plain text on disk.
    //
    // A literal key is untouched: `substituer` only rewrites what it recognises.
    let cle = crate::secrets::substituer(api_key);
    let api_key = cle.as_str();

    // A model held by another node of the swarm, written `peer:<host>:<port>`. Every
    // LaRuche exposes /v1/chat/completions, so a peer is reachable as an OpenAI-compatible
    // endpoint and needs no protocol of its own.
    //
    // If it does not answer, the call falls back to the LOCAL provider rather than
    // failing: a peer is a machine that can be asleep, unplugged or rebooting, and a
    // scheduled task must not die because the box next door went off.
    //
    // Mais le repli est ANNONCE. Il etait muet: un `warn!` dans le journal, et la
    // reponse arrivait comme si le pair avait repondu. Substituer un modele en silence
    // est pire qu'un echec, parce que la reponse est alors attribuee au mauvais modele -
    // « je selectionne le modele du voisin, il ne se passe rien » venait de la.
    if let Some(reste) = provider.strip_prefix("peer:") {
        let base = format!("http://{reste}/v1");
        match openai_chat_stream(
            model, messages, temperature, max_tokens, "", Some(&base), tools, effort,
        )
        .await
        {
            Ok(flux) => return Ok(flux),
            Err(e) => {
                tracing::warn!(peer = reste, error = %e, "swarm peer unreachable, falling back to local");
                let repli =
                    ollama_chat_stream(ollama_url, model, messages, temperature, max_tokens, tools)
                        .await?;
                // La cause la plus courante, de loin: la ruche visee s'annonce sur le
                // reseau mais n'ecoute que sur 127.0.0.1. On nomme le remede tout de
                // suite plutot que de laisser chercher.
                let avis = OllamaChunk {
                    text: format!(
                        "> La ruche `{reste}` n'a pas repondu ({e}). Reponse produite en local.\n\
                         > Si elle est allumee, elle doit etre demarree avec `LARUCHE_BIND_LAN=1` \
                         pour accepter les connexions du reseau.\n\n"
                    ),
                    done: false,
                    finish_reason: None,
                    eval_count: None,
                    eval_duration: None,
                    prompt_eval_count: None,
                    tool_calls: None,
                };
                // Forme qualifiee: `StreamExt` n'est pas importe ici, et l'importer
                // entrerait en conflit avec celui de tokio_stream deja utilise.
                return Ok(Box::pin(futures_util::StreamExt::chain(
                    futures_util::stream::once(async move { avis }),
                    repli,
                )));
            }
        }
    }
    match provider {
        "openai" | "miel" => {
            openai_chat_stream(model, messages, temperature, max_tokens, api_key, api_base, tools, effort).await
        }
        // llama.cpp `llama-server` (OpenAI-compatible, local, no key). Default base
        // matches the local launch scripts (port 8001); override via api_base.
        "llamacpp" | "llama.cpp" | "llama-server" => {
            let base = api_base.or(Some("http://127.0.0.1:8001"));
            openai_chat_stream(model, messages, temperature, max_tokens, api_key, base, tools, effort).await
        }
        // LM Studio and vLLM both serve the OpenAI wire format, so they need no client of
        // their own: only their usual port. Named here rather than left to the catch-all,
        // which routes to Ollama and would have sent every request to the wrong server.
        "lmstudio" | "lm-studio" => {
            let base = api_base.or(Some("http://127.0.0.1:1234"));
            openai_chat_stream(model, messages, temperature, max_tokens, api_key, base, tools, effort).await
        }
        "vllm" => {
            let base = api_base.or(Some("http://127.0.0.1:8000"));
            openai_chat_stream(model, messages, temperature, max_tokens, api_key, base, tools, effort).await
        }
        "anthropic" => {
            anthropic_chat_stream(model, messages, temperature, max_tokens, api_key, api_base, tools, effort).await
        }
        "codex" => codex_chat_stream(model, messages, temperature, max_tokens, api_base, effort).await,
        _ => ollama_chat_stream(ollama_url, model, messages, temperature, max_tokens, tools).await,
    }
}

// ─── Signer mesh ────────────────────────────────────────────────────────────
pub type MeshSigner = std::sync::Arc<dyn Fn(&str) -> Vec<(String, String)> + Send + Sync>;
static MESH_SIGNER: std::sync::OnceLock<MeshSigner> = std::sync::OnceLock::new();
pub fn set_mesh_signer(s: MeshSigner) { let _ = MESH_SIGNER.set(s); }
fn mesh_headers(path: &str) -> Vec<(String, String)> {
    MESH_SIGNER.get().map(|s| s(path)).unwrap_or_default()
}

// ─── OpenAI-compatible streaming (Deepseek, Together, Groq, etc.) ────────────

#[allow(clippy::too_many_arguments)]
async fn openai_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
    tools: Option<&[serde_json::Value]>,
    effort: Effort<'_>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let api_key = api_key.trim();
    let base = normalize_base_url(api_base.unwrap_or("https://api.openai.com"));
    let base = base.as_str();
    if api_key.is_empty() && !is_local_base_url(base) {
        anyhow::bail!("API key is required for OpenAI-compatible provider. Configure in Settings > Providers.");
    }
    let bearer = if api_key.is_empty() { "local-no-key" } else { api_key };
    // Build the chat-completions URL. Most OpenAI-compatible APIs (OpenAI, Groq,
    // Deepseek, Together) take a bare host and expect `/v1/chat/completions`. Some
    // (z.ai GLM at `/api/paas/v4`, OpenRouter at `/api/v1`) already carry a version
    // segment in the base path, where forcing another `/v1` would 404. So only add
    // `/v1` when the base does not already end in a version segment or the full path.
    let trimmed = base.trim_end_matches('/');
    let has_version = trimmed
        .rsplit('/')
        .next()
        .map(|s| s.len() >= 2 && s.starts_with('v') && s[1..].chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(false);
    let url = if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if has_version {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    };

    let openai_messages: Vec<serde_json::Value> = messages.iter().map(|m| {
        // Native tool transcript (OpenAI wire format). The generic layer guarantees
        // pairing (every tool_calls entry is followed by its role:"tool" results).
        if m["role"].as_str() == Some("tool") {
            return serde_json::json!({
                "role": "tool",
                "tool_call_id": m["tool_call_id"].as_str().unwrap_or(""),
                "content": m["content"].as_str().unwrap_or(""),
            });
        }
        if let Some(tcs) = m.get("tool_calls").and_then(|v| v.as_array()) {
            let tool_calls: Vec<serde_json::Value> = tcs.iter().map(|t| {
                // OpenAI wants `arguments` as a JSON *string*; the generic layer
                // carries an object.
                let args = t["function"]["arguments"].clone();
                serde_json::json!({
                    "id": t["id"].as_str().unwrap_or(""),
                    "type": "function",
                    "function": {
                        "name": t["function"]["name"].as_str().unwrap_or(""),
                        "arguments": serde_json::to_string(&args).unwrap_or_else(|_| "{}".into()),
                    }
                })
            }).collect();
            return serde_json::json!({
                "role": "assistant",
                "content": m["content"].as_str().unwrap_or(""),
                "tool_calls": tool_calls,
            });
        }
        let attachments_val = m.get("attachments").and_then(|a| a.as_array());
        let has_attachments = attachments_val.map(|a| !a.is_empty()).unwrap_or(false);
        if has_attachments {
            let mut parts = vec![serde_json::json!({"type": "text", "text": m["content"].as_str().unwrap_or("")})];
            for att in attachments_val.unwrap() {
                let kind = att["kind"].as_str().unwrap_or("");
                let mime_type = att["mime_type"].as_str().unwrap_or("");
                let data = att["data"].as_str().unwrap_or("");
                if kind == "image" {
                    parts.push(serde_json::json!({"type": "image_url", "image_url": {
                        "url": format!("data:{};base64,{}", mime_type, data)
                    }}));
                } else if kind == "audio" {
                    let format = match mime_type {
                        "audio/mpeg" | "audio/mp3" => "mp3",
                        _ => "wav",
                    };
                    parts.push(serde_json::json!( {
                        "type": "input_audio",
                        "input_audio": {"data": data, "format": format}
                    }));
                }
            }
            serde_json::json!({"role": m["role"], "content": parts})
        } else {
            serde_json::json!({"role": m["role"], "content": m["content"].as_str().unwrap_or("")})
        }
    }).collect();

    let mut body = serde_json::json!({
        "model": model,
        "messages": openai_messages,
        "stream": true,
        "temperature": temperature,
    });
    if max_tokens > 0 {
        body["max_tokens"] = serde_json::json!(max_tokens);
    }
    // Ask for real token usage on the final stream chunk (OpenAI, Groq, Deepseek, ...).
    // Without this, streaming responses carry no usage and the gauge falls back to estimates.
    body["stream_options"] = serde_json::json!({ "include_usage": true });
    // Reasoning effort (thinking models). Unknown values are forwarded as-is: a new
    // level ships without a code change; backends that ignore the field are unaffected.
    if let Some(e) = effort.map(str::trim).filter(|e| !e.is_empty()) {
        body["reasoning_effort"] = serde_json::json!(e);
    }
    // Send native tool definitions (OpenAI format)
    if let Some(tools_list) = tools {
        let openai_tools = assainir_tools_openai(convertir_tools_openai(tools_list));
        if !openai_tools.is_empty() {
            body["tools"] = serde_json::json!(openai_tools);
        }
    }

    let client = client_openai();
    let mut req = client.post(&url)
        .header("Authorization", format!("Bearer {}", bearer))
        .header("Content-Type", "application/json");
    if is_local_base_url(base) {
        for (k, v) in mesh_headers("/v1/chat/completions") { req = req.header(k, v); }
    }
    // Serialize ONCE, so the bytes we measure are the bytes we send, and ship it
    // pure ASCII so no consumer can confuse our bytes with our characters.
    let mut corps_brut = json_ascii(&serde_json::to_string(&body)?);

    // BYTE guard, which the token gauge cannot provide. Some gateways refuse a
    // request body past a size of their own: observed rejections clustered at
    // columns 81733, 81736, 81813 and 82233, all just under 80 KiB, on bodies our
    // own parser accepts. The gauge showed 9% of a 128k window at the time, because
    // it counts tokens while the wall is counted in bytes.
    //
    // Order and cost of each lever live on `reduire_sous_budget`. In short: the
    // fattest observations first, then past tool-call arguments, then the tool
    // list down to a floor, and only then the oldest exchanges. If even that is
    // not enough the request goes out as is, with a warning naming what is left.
    let limite = limite_corps(base);
    if corps_brut.len() > limite {
        let avant = corps_brut.len();
        body = reduire_sous_budget(&body, limite)?;
        corps_brut = json_ascii(&serde_json::to_string(&body)?);
        tracing::warn!(
            target: "provider",
            avant,
            apres = corps_brut.len(),
            limite,
            "request body over the byte guard: trimmed"
        );
    }
    debug_assert!(
        serde_json::from_str::<serde_json::Value>(&corps_brut).is_ok(),
        "the escaped body must stay valid JSON"
    );
    tracing::debug!(
        target: "provider",
        octets = corps_brut.len(),
        caracteres = corps_brut.chars().count(),
        messages = openai_messages.len(),
        "openai-compatible request body"
    );
    let mut response = req.body(corps_brut.clone()).send().await?;

    // Thinking mode on some OpenAI-compatible backends (deepseek) demands that the
    // `reasoning_content` it streamed be handed BACK on the next call. We accumulate
    // it but never replay it, because the engine's transcript has no slot for it, so
    // the second turn of any thinking conversation died on
    // "The reasoning_content in the thinking mode must be passed back to the API".
    // Rather than lose the feature everywhere it works statelessly (OpenAI o-series),
    // drop the flag and retry once: the turn completes without thinking instead of
    // failing. Replaying the reasoning is the real fix and needs a transcript change.
    if response.status().as_u16() == 400 && body.get("reasoning_effort").is_some() {
        let apercu = response.text().await.unwrap_or_default();
        if apercu.contains("reasoning_content") {
            tracing::warn!(
                target: "provider",
                "thinking mode requires replaying reasoning_content, which we cannot do yet: retrying without reasoning_effort"
            );
            let mut sans_pensee = body.clone();
            sans_pensee.as_object_mut().map(|o| o.remove("reasoning_effort"));
            let corps_sans = json_ascii(&serde_json::to_string(&sans_pensee)?);
            let mut rejeu = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", bearer))
                .header("Content-Type", "application/json");
            if is_local_base_url(base) {
                for (k, v) in mesh_headers("/v1/chat/completions") {
                    rejeu = rejeu.header(k, v);
                }
            }
            // The diagnostic below must describe the bytes that ACTUALLY went out. Left
            // pointing at the first body, it reported a size and a message count for a
            // request the provider never saw, which is worse than no diagnostic at all.
            response = rejeu.body(corps_sans.clone()).send().await?;
            corps_brut = corps_sans;
        } else {
            return Err(ProviderError {
                status: 400,
                body: diagnostiquer_corps(&apercu, &corps_brut, &openai_messages),
                retry_after: None,
            }
            .into());
        }
    }

    tracing::info!(target: "provider", url = %url, model = %model, status = %response.status(), "openai-compatible request sent");
    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        // Carry the SIZE of what we sent into the error. When a provider answers
        // "failed to parse the request body ... at column N", the only question that
        // matters is whether N is where our body ENDS (they received a truncated
        // upload) or somewhere in the middle (we really did send something they
        // dislike). Without this number the two are indistinguishable, and guessing
        // between them wasted a full debugging round.
        let diagnostic = diagnostiquer_corps(&body_text, &corps_brut, &openai_messages);
        tracing::warn!(
            target: "provider",
            status = status.as_u16(),
            octets_envoyes = corps_brut.len(),
            body = %body_text.chars().take(300).collect::<String>(),
            "openai-compatible request failed"
        );
        return Err(ProviderError { status: status.as_u16(), body: diagnostic, retry_after: None }.into());
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);

    tokio::spawn(async move {
        let mut buffer: Vec<u8> = Vec::new();
        // Opt-in (RUCHE_DEBUG_SSE=1): log the first few raw SSE lines to diagnose an
        // unfamiliar provider's response shape. Off by default to avoid noise.
        let dbg_sse = std::env::var("RUCHE_DEBUG_SSE").as_deref() == Ok("1");
        let mut dbg_lines = 0u8;
        // tool_calls accumulator keyed by index (streaming delta)
        // Each entry: (id, name, partial_args_string)
        let mut tool_call_acc: std::collections::HashMap<u32, (String, String, String)> = std::collections::HashMap::new();
        // Actual usage (if the server includes it: OpenAI with stream_options, llama.cpp by default).
        let mut in_tok: Option<u64> = None;
        let mut out_tok: Option<u64> = None;
        // Reasoning models stream chain-of-thought in `reasoning_content`. We accumulate it but
        // never stream it as the answer. Only if the model produced NO `content` at all (e.g. a
        // broken "flash" proxy) do we surface the reasoning as a last resort, so the turn is not
        // silently empty. `reasoning_emitted` guards against emitting it twice.
        let mut content_streamed = false;
        let mut reasoning_acc = String::new();
        let mut reasoning_emitted = false;

        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.extend_from_slice(&bytes);
                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                        let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
                        if dbg_sse && dbg_lines < 5 && !line.is_empty() {
                            dbg_lines += 1;
                            tracing::info!(target: "provider", line = %line.chars().take(280).collect::<String>(), "raw SSE line");
                        }
                        if line.is_empty() || line == "data: [DONE]" {
                            if line == "data: [DONE]" {
                                // Last resort: model produced only reasoning and no content, and the
                                // stream ends via [DONE] without an in-chunk finish_reason.
                                if !content_streamed && !reasoning_emitted {
                                    let r = reasoning_acc.trim();
                                    if !r.is_empty() {
                                        // Defensive: a malformed stream could send [DONE] twice;
                                        // the flag prevents re-emitting the reasoning block.
                                        #[allow(unused_assignments)]
                                        {
                                            reasoning_emitted = true;
                                        }
                                        let _ = tx.send(OllamaChunk {
                                            text: r.to_string(), done: false,
                                            finish_reason: None, eval_count: None,
                                            eval_duration: None, prompt_eval_count: None,
                                            tool_calls: None,
                                        }).await;
                                    }
                                }
                                // Finalize the accumulated tool_calls (ordered by index).
                                let tool_calls = finaliser_tool_calls(&mut tool_call_acc);
                                let _ = tx.send(OllamaChunk {
                                    text: String::new(), done: true,
                                    finish_reason: Some("stop".to_string()),
                                    eval_count: None, eval_duration: None,
                                    prompt_eval_count: None,
                                    tool_calls,
                                }).await;
                                return;
                            }
                            continue;
                        }
                        let json_str = if let Some(stripped) = line.strip_prefix("data: ") { stripped } else { &line };
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                            // Actual usage (top-level, present on the final chunk or a dedicated chunk).
                            if let Some(u) = parsed["usage"]["prompt_tokens"].as_u64() { in_tok = Some(u); }
                            if let Some(u) = parsed["usage"]["completion_tokens"].as_u64() { out_tok = Some(u); }
                            // With `stream_options.include_usage`, usage arrives in a DEDICATED
                            // final chunk (empty `choices`), AFTER the finish_reason chunk. That
                            // chunk has no content and no finish_reason, so the emission below
                            // would drop it and the gauge/budget would see 0 tokens. Emit a
                            // trailing done-chunk carrying the usage so it is never lost.
                            let usage_only = parsed["usage"].is_object()
                                && parsed["choices"].as_array().map(|a| a.is_empty()).unwrap_or(true);
                            if usage_only && (in_tok.is_some() || out_tok.is_some()) {
                                let _ = tx.send(OllamaChunk {
                                    text: String::new(),
                                    done: true,
                                    finish_reason: Some("stop".to_string()),
                                    eval_count: out_tok,
                                    eval_duration: None,
                                    prompt_eval_count: in_tok,
                                    tool_calls: None,
                                }).await;
                                continue;
                            }
                            let mut text = parsed["choices"][0]["delta"]["content"].as_str().unwrap_or("").to_string();
                            if !text.is_empty() {
                                content_streamed = true;
                            }
                            // Accumulate reasoning (chain-of-thought) without streaming it as the answer.
                            if let Some(rc) = parsed["choices"][0]["delta"]["reasoning_content"].as_str() {
                                reasoning_acc.push_str(rc);
                            }
                            let finish_reason = parsed["choices"][0]["finish_reason"].as_str().map(str::to_string);
                            let done = finish_reason.is_some();

                            // Last resort: if the model produced NO content at all, surface the
                            // accumulated reasoning on the final chunk so the turn is not silently empty.
                            if done && !content_streamed && !reasoning_emitted && text.is_empty() {
                                let r = reasoning_acc.trim();
                                if !r.is_empty() {
                                    text = r.to_string();
                                    reasoning_emitted = true;
                                }
                            }

                            // Parse the tool_calls delta (OpenAI streaming format)
                            if let Some(tc_deltas) = parsed["choices"][0]["delta"]["tool_calls"].as_array() {
                                for tc_delta in tc_deltas {
                                    let idx = tc_delta["index"].as_u64().unwrap_or(0) as u32;
                                    let entry = tool_call_acc.entry(idx).or_insert_with(|| {
                                        (String::new(), String::new(), String::new())
                                    });
                                    // id: present only on the first chunk of the tool call
                                    if let Some(id_val) = tc_delta["id"].as_str() {
                                        entry.0 = id_val.to_string();
                                    }
                                    if entry.0.is_empty() {
                                        entry.0 = format!("call_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
                                    }
                                    // function.name: present on the first chunk
                                    if let Some(name_val) = tc_delta["function"]["name"].as_str() {
                                        entry.1 = name_val.to_string();
                                    }
                                    // function.arguments: concatenated across multiple chunks
                                    if let Some(args_val) = tc_delta["function"]["arguments"].as_str() {
                                        entry.2.push_str(args_val);
                                    }
                                }
                            }

                            if !text.is_empty() || done {
                                // Send the accumulated tool_calls only on the final chunk
                                let tool_calls = if done { finaliser_tool_calls(&mut tool_call_acc) } else { None };

                                let chunk = OllamaChunk {
                                    text, done, finish_reason,
                                    eval_count: if done { out_tok } else { None },
                                    eval_duration: None,
                                    prompt_eval_count: if done { in_tok } else { None },
                                    tool_calls,
                                };
                                if tx.send(chunk).await.is_err() { return; }
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => { tracing::error!(error = %e, "Error reading OpenAI stream"); return; }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

// ─── Anthropic (Claude) streaming ──────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn anthropic_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_key: &str,
    api_base: Option<&str>,
    tools: Option<&[serde_json::Value]>,
    effort: Effort<'_>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let api_key = api_key.trim();
    let base = normalize_base_url(api_base.unwrap_or("https://api.anthropic.com"));
    let url = format!("{}/v1/messages", base.trim_end_matches('/'));

    let anthropic_max: u32 = if max_tokens > 0 { max_tokens } else { 4096 };

    // Anthropic wants the system prompt as a top-level `system` field, NOT a message
    // (a system role inside `messages` is rejected). Pull any system messages out, and
    // mark the last system block with `cache_control: ephemeral` so the large, stable
    // prefix (system prompt) is served from the prompt cache on repeated calls, cutting
    // input cost and latency.
    //
    // Native tool transcript: assistant `tool_calls` become `tool_use` content blocks;
    // `role:"tool"` results become `tool_result` blocks grouped - with any adjacent
    // user text and images - into a SINGLE user message (strict alternation, and
    // parallel results must share one user turn). The generic layer guarantees
    // call/result pairing.
    let mut system_blocks: Vec<serde_json::Value> = Vec::new();
    let mut convo: Vec<serde_json::Value> = Vec::new();
    let mut user_blocks: Vec<serde_json::Value> = Vec::new();
    fn flush_user(convo: &mut Vec<serde_json::Value>, user_blocks: &mut Vec<serde_json::Value>) {
        if !user_blocks.is_empty() {
            convo.push(serde_json::json!({"role": "user", "content": std::mem::take(user_blocks)}));
        }
    }
    for m in messages {
        match m["role"].as_str().unwrap_or("user") {
            "system" => {
                if let Some(text) = m["content"].as_str() {
                    if !text.trim().is_empty() {
                        system_blocks.push(serde_json::json!({"type": "text", "text": text}));
                    }
                }
            }
            "tool" => {
                user_blocks.push(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": m["tool_call_id"].as_str().unwrap_or(""),
                    "content": m["content"].as_str().unwrap_or(""),
                }));
            }
            "assistant" => {
                flush_user(&mut convo, &mut user_blocks);
                let mut blocks: Vec<serde_json::Value> = Vec::new();
                let text = m["content"].as_str().unwrap_or("");
                if !text.trim().is_empty() {
                    blocks.push(serde_json::json!({"type": "text", "text": text}));
                }
                if let Some(tcs) = m.get("tool_calls").and_then(|v| v.as_array()) {
                    for t in tcs {
                        blocks.push(serde_json::json!({
                            "type": "tool_use",
                            "id": t["id"].as_str().unwrap_or(""),
                            "name": t["function"]["name"].as_str().unwrap_or(""),
                            // `input` is an object: the generic layer carries one.
                            "input": t["function"]["arguments"].clone(),
                        }));
                    }
                }
                if !blocks.is_empty() {
                    convo.push(serde_json::json!({"role": "assistant", "content": blocks}));
                }
            }
            _ => {
                // user: text + native image blocks (Anthropic vision).
                let text = m["content"].as_str().unwrap_or("");
                if !text.trim().is_empty() {
                    user_blocks.push(serde_json::json!({"type": "text", "text": text}));
                }
                if let Some(atts) = m.get("attachments").and_then(|a| a.as_array()) {
                    for att in atts {
                        if att["kind"].as_str() == Some("image") {
                            user_blocks.push(serde_json::json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": att["mime_type"].as_str().unwrap_or("image/png"),
                                    "data": att["data"].as_str().unwrap_or(""),
                                }
                            }));
                        }
                    }
                }
            }
        }
    }
    flush_user(&mut convo, &mut user_blocks);
    // Breakpoint on the FIRST block, not the last. Anthropic caches the prefix up to
    // and INCLUDING the marked block, and the first block is the big stable system
    // prompt. Marking the last one used to be equivalent, but the engine now appends
    // a volatile system message (clock, recalled memory) at the tail of the context:
    // it arrives here as the final block, so marking "last" would pin the cache to
    // content that changes every minute and defeat caching entirely.
    if let Some(first) = system_blocks.first_mut() {
        first["cache_control"] = serde_json::json!({"type": "ephemeral"});
    }

    let mut body = serde_json::json!({
        "model": model,
        "messages": convo,
        "stream": true,
        "max_tokens": anthropic_max,
        "temperature": temperature,
    });
    if !system_blocks.is_empty() {
        body["system"] = serde_json::json!(system_blocks);
    }
    // Extended thinking: Anthropic takes a token BUDGET, not a level. It must stay
    // below max_tokens, and the API requires temperature=1 when thinking is on.
    if let Some(budget) = effort.and_then(budget_pensee) {
        let budget = budget.min(anthropic_max.saturating_sub(1024).max(1024));
        body["thinking"] = serde_json::json!({ "type": "enabled", "budget_tokens": budget });
        body["temperature"] = serde_json::json!(1);
    }

    // Anthropic also supports native tool calling, with a slightly different format
    if let Some(tools_list) = tools {
        let mut anthropic_tools: Vec<serde_json::Value> = tools_list.iter().filter_map(|t| {
            Some(serde_json::json!({
                "name": t["name"].as_str()?,
                "description": t["description"].as_str().unwrap_or(""),
                "input_schema": t.get("parameters").cloned().unwrap_or(serde_json::json!({})),
            }))
        }).collect();
        if !anthropic_tools.is_empty() {
            // Cache the tool definitions too (stable across a conversation): mark the
            // last tool so the whole tools block is cached up to that point.
            if let Some(last) = anthropic_tools.last_mut() {
                last["cache_control"] = serde_json::json!({"type": "ephemeral"});
            }
            body["tools"] = serde_json::json!(anthropic_tools);
        }
    }

    // ... the rest of the Anthropic code stays identical
    _anthropic_send_request(&url, api_key, body).await
}

async fn _anthropic_send_request(
    url: &str,
    api_key: &str,
    body: serde_json::Value,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    let client = reqwest::Client::new();
    let mut response = client.post(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        return Err(ProviderError { status: status.as_u16(), body: body_text, retry_after: None }.into());
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);

    tokio::spawn(async move {
        let mut buffer: Vec<u8> = Vec::new();
        // Actual usage provided by Anthropic in the stream: input at `message_start`,
        // output at `message_delta`. Emitted on the final chunk for an accurate gauge.
        let mut in_tok: Option<u64> = None;
        let mut out_tok: Option<u64> = None;
        // Native tool_use blocks, keyed by content-block index: (id, name, partial_json).
        let mut tool_acc: std::collections::HashMap<u64, (String, String, String)> =
            std::collections::HashMap::new();
        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.extend_from_slice(&bytes);
                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                        let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
                        if line.is_empty() { continue; }

                        // Anthropic SSE: event: ..., data: {...}
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                let chunk_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                match chunk_type {
                                    "message_start" => {
                                        if let Some(u) = parsed["message"]["usage"]["input_tokens"].as_u64() {
                                            in_tok = Some(u);
                                        }
                                    }
                                    "message_delta" => {
                                        if let Some(u) = parsed["usage"]["output_tokens"].as_u64() {
                                            out_tok = Some(u);
                                        }
                                    }
                                    "content_block_start"
                                        // A tool_use block opens with its id + name.
                                        if parsed["content_block"]["type"].as_str() == Some("tool_use") => {
                                            let idx = parsed["index"].as_u64().unwrap_or(0);
                                            let id = parsed["content_block"]["id"].as_str().unwrap_or("").to_string();
                                            let name = parsed["content_block"]["name"].as_str().unwrap_or("").to_string();
                                            tool_acc.insert(idx, (id, name, String::new()));
                                        }
                                    _ => {}
                                }
                                // tool_use arguments stream as input_json_delta on the block.
                                if chunk_type == "content_block_delta"
                                    && parsed["delta"]["type"].as_str() == Some("input_json_delta")
                                {
                                    if let Some(pj) = parsed["delta"]["partial_json"].as_str() {
                                        let idx = parsed["index"].as_u64().unwrap_or(0);
                                        tool_acc
                                            .entry(idx)
                                            .or_insert_with(|| (String::new(), String::new(), String::new()))
                                            .2
                                            .push_str(pj);
                                    }
                                }
                                let text = match chunk_type {
                                    "content_block_delta" => parsed["delta"]["text"].as_str().unwrap_or("").to_string(),
                                    _ => String::new(),
                                };
                                let done = chunk_type == "message_stop";
                                let finish_reason = if done { Some("stop".to_string()) } else { None };
                                // Emit the accumulated tool_use blocks (ordered by index) on stop.
                                let tool_calls = if done && !tool_acc.is_empty() {
                                    let mut calls: Vec<(u64, ToolCall)> = tool_acc
                                        .iter()
                                        .map(|(idx, (id, name, args_str))| {
                                            let args = if args_str.trim().is_empty() {
                                                serde_json::json!({})
                                            } else {
                                                serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null)
                                            };
                                            (*idx, ToolCall { id: id.clone(), name: name.clone(), args })
                                        })
                                        .collect();
                                    calls.sort_by_key(|(idx, _)| *idx);
                                    Some(calls.into_iter().map(|(_, c)| c).collect())
                                } else {
                                    None
                                };

                                if !text.is_empty() || done {
                                    let _ = tx.send(OllamaChunk {
                                        text, done, finish_reason,
                                        eval_count: if done { out_tok } else { None },
                                        eval_duration: None,
                                        prompt_eval_count: if done { in_tok } else { None },
                                        tool_calls,
                                    }).await;
                                }
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => { tracing::error!(error = %e, "Error reading Anthropic stream"); return; }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

// ─── Codex (ChatGPT) ────────────────────────────────────────────────────────

fn codex_request_body(model: &str, input: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "input": input,
        "stream": true,
        // The ChatGPT Codex backend rejects stored responses for subscription
        // OAuth calls (`400: Store must be set to false`).
        "store": false,
    })
}

async fn codex_chat_stream(
    model: &str,
    messages: &[serde_json::Value],
    temperature: f32,
    max_tokens: u32,
    api_base: Option<&str>,
    effort: Effort<'_>,
) -> Result<Pin<Box<dyn Stream<Item = OllamaChunk> + Send>>> {
    use crate::codex_auth;
    let _ = (temperature, max_tokens);
    let access_token = codex_auth::resolve_codex_access_token()
        .await.map_err(|e| anyhow::anyhow!("Auth Codex: {e}"))?;
    let base = match api_base.map(|b| b.trim_end_matches('/').to_string()) {
        Some(b) if b.contains("backend-api/codex") => b,
        _ => codex_auth::DEFAULT_CODEX_BASE_URL.to_string(),
    };
    let url = format!("{}/responses", base);

    let mut instructions = String::new();
    let mut input: Vec<serde_json::Value> = Vec::new();
    for m in messages {
        match m["role"].as_str().unwrap_or("user") {
            "system" => instructions.push_str(&format!("{}\n", m["content"].as_str().unwrap_or(""))),
            // Text-only Responses API: native tool structures are re-rendered as text
            // so the transcript stays coherent.
            "tool" => {
                input.push(serde_json::json!({
                    "role": "user",
                    "content": format!(
                        "[Tool Result: {}]\n{}",
                        m["name"].as_str().unwrap_or("tool"),
                        m["content"].as_str().unwrap_or("")
                    ),
                }));
            }
            role => {
                let mut content = m["content"].as_str().unwrap_or("").to_string();
                if let Some(tcs) = m.get("tool_calls").and_then(|v| v.as_array()) {
                    for t in tcs {
                        content.push_str(&format!(
                            "\n<tool_call>{}</tool_call>",
                            serde_json::json!({
                                "name": t["function"]["name"],
                                "arguments": t["function"]["arguments"],
                            })
                        ));
                    }
                }
                let entry = serde_json::json!({
                    "role": if role == "assistant" { "assistant" } else { "user" },
                    "content": content,
                });
                input.push(entry);
            }
        }
    }

    let mut body = codex_request_body(model, input);
    if !instructions.trim().is_empty() {
        body["instructions"] = serde_json::json!(instructions.trim());
    }
    // Codex Responses API: reasoning effort travels in its own object.
    if let Some(e) = effort.map(str::trim).filter(|e| !e.is_empty()) {
        body["reasoning"] = serde_json::json!({ "effort": e });
    }

    let client = reqwest::Client::new();
    let mut req = client.post(&url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream");
    // Anti-Cloudflare headers (User-Agent, originator, account id) required by the Codex
    // backend; without them requests are likely rejected with a 403.
    for (k, v) in codex_auth::codex_headers(&access_token) { req = req.header(k, v); }
    let mut response = req.json(&body).send().await?;

    let status = response.status();
    tracing::info!(target: "provider", url = %url, model = %model, status = %status, "Codex request sent");
    if !status.is_success() {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let body = response
            .text()
            .await
            .unwrap_or_else(|e| format!("unable to read provider error: {e}"));
        return Err(ProviderError {
            status: status.as_u16(),
            body,
            retry_after,
        }
        .into());
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<OllamaChunk>(64);
    tokio::spawn(async move {
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => {
                    buffer.extend_from_slice(&bytes);
                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                        let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
                        if line.is_empty() { continue; }
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                let ctype = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                let text = match ctype {
                                    "response.output_text.delta" => parsed["delta"].as_str().unwrap_or("").to_string(),
                                    _ => String::new(),
                                };
                                let done = ctype == "response.completed" || ctype == "response.incomplete";
                                if !text.is_empty() || done {
                                    let _ = tx.send(OllamaChunk {
                                        text, done,
                                        finish_reason: if done { Some("stop".to_string()) } else { None },
                                        eval_count: None, eval_duration: None,
                                        prompt_eval_count: None,
                                        tool_calls: None,
                                    }).await;
                                }
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => { tracing::error!(error = %e, "Error reading Codex stream"); return; }
            }
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn normalize_base_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() { return "https://api.openai.com".to_string(); }
    let lower = url.to_lowercase();
    // Handle "localhost" addresses by keeping http:// if explicitly set
    if lower.starts_with("http://") || lower.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

pub fn is_local_base_url(url: &str) -> bool {
    let u = url.to_lowercase();
    if u.contains("localhost") || u.contains("127.0.0.1") || u.contains("::1") || u.contains(".local")
    {
        return true;
    }
    // Private ranges too: a llama.cpp served from another machine on the LAN has the
    // same strict chat template and the same absence of gateway quirks as one served
    // from this one. Only the address distinguishes a self-hosted model from a cloud
    // API, since both are declared `provider: "openai"`.
    let hote = u
        .split("//")
        .nth(1)
        .unwrap_or(&u)
        .split('/')
        .next()
        .unwrap_or_default();
    hote.starts_with("192.168.")
        || hote.starts_with("10.")
        || hote.starts_with("0.0.0.0")
        || (hote.starts_with("172.")
            && hote
                .split('.')
                .nth(1)
                .and_then(|o| o.parse::<u8>().ok())
                .is_some_and(|o| (16..=31).contains(&o)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_subscription_requests_disable_storage() {
        let body = codex_request_body("gpt-5.6-luna", vec![]);
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);
        assert_eq!(body["model"], "gpt-5.6-luna");
    }

    /// Regression: a stream reaches finalization twice (a chunk carrying
    /// `finish_reason`, then the `data: [DONE]` sentinel). The second pass must
    /// yield nothing, otherwise the same ids are emitted again, the consumer
    /// appends them, and the next request carries `tool_calls: [X, X]` which the
    /// API rejects with `Duplicate value for 'tool_call_id'`.
    /// The curator sends the whole mission transcript as ONE user message.
    ///
    /// The reordering, locked down: trimming observations is enough, so the agent
    /// keeps every tool it had. Before, tools were cut FIRST and an agent that had
    /// only fat observations lost most of its capability for nothing.
    #[test]
    fn le_budget_garde_tous_les_outils_quand_les_observations_suffisent() {
        let outils: Vec<serde_json::Value> = (0..33)
            .map(|i| serde_json::json!({ "type": "function", "function": { "name": format!("t{i}") } }))
            .collect();
        let body = serde_json::json!({
            "model": "m",
            "tools": outils,
            "messages": [
                { "role": "system", "content": "s" },
                { "role": "user", "content": "mission" },
                { "role": "tool", "content": "o".repeat(60_000) },
                { "role": "tool", "content": "p".repeat(60_000) }
            ]
        });
        let apres = reduire_sous_budget(&body, 76_800).unwrap();
        let corps = json_ascii(&serde_json::to_string(&apres).unwrap()).len();
        assert!(corps <= 76_800, "must fit, got {corps}");
        assert_eq!(
            apres["tools"].as_array().unwrap().len(),
            33,
            "tools must survive when trimming observations was enough"
        );
    }

    /// Les arguments d'appel d'outil comptent, et le rognage ne les voyait pas.
    ///
    /// Un message assistant qui porte un appel a `content: null`: toute sa masse
    /// est ailleurs. Le corps refuse du 2026-08-27 en portait 10100 caracteres
    /// intouchables. Ce qui est ecrit en remplacement doit rester du JSON, sinon
    /// une gateway qui reparse les arguments refuse le corps pour une raison bien
    /// pire que celle qu'on essayait de corriger.
    #[test]
    fn le_budget_rabote_les_arguments_dappels_doutils() {
        let messages: Vec<serde_json::Value> = std::iter::once(
            serde_json::json!({ "role": "system", "content": "s" }),
        )
        .chain(std::iter::once(
            serde_json::json!({ "role": "user", "content": "mission" }),
        ))
        .chain((0..20).map(|i| {
            serde_json::json!({
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": format!("c{i}"),
                    "type": "function",
                    "function": { "name": "f", "arguments": "z".repeat(5_000) }
                }]
            })
        }))
        .collect();
        let body = serde_json::json!({ "model": "m", "messages": messages });

        let apres = reduire_sous_budget(&body, 20_000).unwrap();
        let corps = json_ascii(&serde_json::to_string(&apres).unwrap()).len();
        assert!(corps <= 20_000, "must fit, got {corps}");

        let args = apres["messages"][2]["tool_calls"][0]["function"]["arguments"]
            .as_str()
            .expect("arguments stay a string");
        assert!(args.len() < 5_000, "arguments were not trimmed");
        serde_json::from_str::<serde_json::Value>(args)
            .expect("trimmed arguments must still parse as JSON");
    }

    /// Le dernier recours, sur la forme exacte du corps qui a tue un tour.
    ///
    /// 2026-08-27: 81812 octets pour une limite de 76800, deja rognes partout,
    /// 12 outils soit le plancher, et l'essentiel du poids dans 31,5 Ko de prompts
    /// systeme et de schemas plus deux cents petits messages qu'aucun levier ne
    /// mordait. La fonction rendait ce corps tel quel, il partait, se faisait
    /// refuser trois fois, et le tour mourait.
    #[test]
    fn le_budget_elague_les_plus_vieux_echanges_en_dernier_recours() {
        let outils: Vec<serde_json::Value> = (0..12)
            .map(|i| {
                serde_json::json!({
                    "type": "function",
                    "function": { "name": format!("t{i}"), "description": "d".repeat(1_200) }
                })
            })
            .collect();
        // Des messages courts, tous SOUS le seuil de rognage du dernier passage:
        // c'est le banc qui ne se laisse pas mordre, et c'est le cas reel.
        let mut messages = vec![
            serde_json::json!({ "role": "system", "content": "S".repeat(14_000) }),
            serde_json::json!({ "role": "user", "content": "la mission" }),
        ];
        for i in 0..190 {
            messages.push(serde_json::json!({
                "role": if i % 2 == 0 { "assistant" } else { "user" },
                "content": format!("m{i}-{}", "x".repeat(250))
            }));
        }
        messages.push(serde_json::json!({ "role": "system", "content": "R".repeat(2_700) }));
        let body = serde_json::json!({ "model": "m", "tools": outils, "messages": messages });

        let avant = json_ascii(&serde_json::to_string(&body).unwrap()).len();
        assert!(avant > 76_800, "le cas de test doit deborder, il fait {avant}");

        let apres = reduire_sous_budget(&body, 76_800).unwrap();
        let corps = json_ascii(&serde_json::to_string(&apres).unwrap()).len();
        assert!(corps <= 76_800, "toujours au-dessus du budget: {corps}");

        let restants = apres["messages"].as_array().unwrap();
        assert!(
            restants.len() < 193,
            "aucun message n'a ete retire, le levier n'a pas joue"
        );
        // Les deux systemes et la mission survivent toujours.
        assert_eq!(
            restants.iter().filter(|m| m["role"] == "system").count(),
            2,
            "un message systeme a ete retire"
        );
        let mission = restants
            .iter()
            .find(|m| m["content"].as_str().is_some_and(|c| c.starts_with("la mission")))
            .expect("la mission a ete retiree");
        assert!(
            mission["content"]
                .as_str()
                .unwrap()
                .contains("anciens echanges retires"),
            "le modele n'est pas prevenu de ce qui a disparu"
        );
        // Et la queue recente est intacte: c'est elle qui porte le fil en cours.
        assert!(
            restants
                .iter()
                .any(|m| m["content"].as_str().is_some_and(|c| c.starts_with("m189-"))),
            "le message le plus recent a ete retire"
        );
    }

    /// Elaguer de travers remplacerait un refus pour taille par un refus pour
    /// structure: un `tool` sans son appel fait rejeter le corps entier.
    #[test]
    fn lelagage_ne_laisse_jamais_un_tool_orphelin() {
        let mut messages = vec![
            serde_json::json!({ "role": "system", "content": "s" }),
            serde_json::json!({ "role": "user", "content": "la mission" }),
        ];
        for i in 0..90 {
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": format!("c{i}"),
                    "type": "function",
                    "function": { "name": "f", "arguments": "{}" }
                }]
            }));
            messages.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": format!("c{i}"),
                "content": "r".repeat(300)
            }));
        }
        let body = serde_json::json!({ "model": "m", "messages": messages });
        let apres = reduire_sous_budget(&body, 12_000).unwrap();
        let restants = apres["messages"].as_array().unwrap();
        assert!(restants.len() < 182, "le levier n'a pas joue");

        let vivants: std::collections::HashSet<&str> = restants
            .iter()
            .filter_map(|m| m["tool_calls"].as_array())
            .flatten()
            .filter_map(|a| a["id"].as_str())
            .collect();
        for m in restants {
            if m["role"] == "tool" {
                let id = m["tool_call_id"].as_str().unwrap();
                assert!(vivants.contains(id), "reponse orpheline pour {id}");
            }
        }
    }

    /// When observations cannot get there alone, tools give ground, but never
    /// below the floor: under it the agent loses web, files and shell at once.
    #[test]
    fn le_budget_ne_descend_jamais_sous_le_plancher_doutils() {
        let outils: Vec<serde_json::Value> = (0..33)
            .map(|i| {
                serde_json::json!({
                    "type": "function",
                    "function": { "name": format!("t{i}"), "description": "d".repeat(600) }
                })
            })
            .collect();
        // Messages already minimal: the only remaining lever is the tool list.
        let body = serde_json::json!({
            "model": "m",
            "tools": outils,
            "messages": [
                { "role": "system", "content": "s" },
                { "role": "user", "content": "mission" }
            ]
        });
        let apres = reduire_sous_budget(&body, 8_000).unwrap();
        let restants = apres["tools"].as_array().unwrap().len();
        assert!(
            restants >= PLANCHER_OUTILS,
            "trimmed below the floor: {restants} < {PLANCHER_OUTILS}"
        );
    }

    /// Regression guard, paid for in a broken run: on 2026-08-26 this limit was
    /// briefly 256 KB and an 84436-byte body was refused four times. The smallest
    /// body ever refused is 83451 bytes; nothing under 80 KB ever was. No remote
    /// provider may sit above the measured-safe value.
    #[test]
    fn aucun_fournisseur_distant_ne_depasse_le_mur_mesure() {
        /// Smallest body a gateway has ever refused.
        const PLUS_PETIT_REFUS: usize = 83_451;
        for base in [
            "https://api.deepseek.com/v1",
            "https://api.openai.com/v1",
            "https://openrouter.ai/api/v1",
            "https://passerelle.inconnue.example/v1",
        ] {
            let limite = limite_corps(base);
            assert!(
                limite < PLUS_PETIT_REFUS,
                "{base} would be allowed {limite} bytes, at or past the observed wall"
            );
        }
        // Local runtimes have no gateway to cut the upload.
        assert!(limite_corps("http://127.0.0.1:8080/v1") > PLUS_PETIT_REFUS);
    }

    /// Measured at 109301 chars, in a body of two messages. A trimmer that only
    /// looked at `role == "tool"` never even saw it, and the request went out at
    /// 114 KB. When the mission turn IS the payload, it has to give ground too.
    #[test]
    fn le_budget_rabote_meme_un_unique_message_utilisateur_geant() {
        let body = serde_json::json!({
            "model": "m",
            "messages": [
                { "role": "system", "content": "x".repeat(4_951) },
                { "role": "user", "content": "y".repeat(109_301) }
            ]
        });
        let avant = json_ascii(&serde_json::to_string(&body).unwrap()).len();
        assert!(avant > 76_800, "fixture must exceed the guard: {avant}");

        let apres = reduire_sous_budget(&body, 76_800).unwrap();
        let corps = json_ascii(&serde_json::to_string(&apres).unwrap()).len();
        assert!(corps <= 76_800, "must fit, got {corps}");
        assert_eq!(apres["messages"].as_array().unwrap().len(), 2, "no message dropped");
        assert!(
            apres["messages"][1]["content"].as_str().unwrap().contains("cut to fit"),
            "the cut must be announced"
        );
        // The system prompt is never touched.
        assert_eq!(apres["messages"][0]["content"].as_str().unwrap().len(), 4_951);
    }

    /// The shape that actually breaks a research run: a SHOAL, not a whale.
    ///
    /// A tool definition without a name must never reach the wire.
    ///
    /// The turn that motivated this came back as `tools[3].function: missing field
    /// name`, 400, no fallback, 177 messages of work lost. Probing the provider showed
    /// that wording is emitted for exactly one shape: a `function` object whose `name`
    /// key is absent (an empty name, a null one or a dotted one each say something
    /// else). Dropping the entry costs one capability; sending it costs the turn.
    #[test]
    fn un_outil_sans_nom_ne_part_jamais_sur_le_fil() {
        let outils = vec![
            serde_json::json!({"type": "function", "function": {"name": "web_fetch", "description": "d", "parameters": {}}}),
            serde_json::json!({"type": "function", "function": {"description": "d", "parameters": {}}}),
            serde_json::json!({"type": "function", "function": {"name": "   ", "description": "d"}}),
            serde_json::json!({"type": "function", "function": {}}),
        ];
        let propres = assainir_tools_openai(outils);
        assert_eq!(propres.len(), 1, "only the named tool survives");
        assert_eq!(propres[0]["function"]["name"], "web_fetch");
    }

    /// Two tools sharing a name are refused as a block ("Tool names must be unique"),
    /// so the second one goes rather than the turn.
    #[test]
    fn un_nom_en_double_ne_part_pas_deux_fois() {
        let outils = vec![
            serde_json::json!({"type": "function", "function": {"name": "shell_exec", "description": "builtin"}}),
            serde_json::json!({"type": "function", "function": {"name": "shell_exec", "description": "plugin qui masque le builtin"}}),
        ];
        let propres = assainir_tools_openai(outils);
        assert_eq!(propres.len(), 1);
        assert_eq!(propres[0]["function"]["description"], "builtin", "the first one wins");
    }

    /// The registry schema goes through the converter untouched: every entry named.
    #[test]
    fn le_registre_converti_garde_tous_ses_noms() {
        let schema = vec![
            serde_json::json!({"name": "a", "description": "d", "parameters": {"type": "object"}, "origin": "builtin"}),
            serde_json::json!({"description": "sans nom", "parameters": {}}),
        ];
        let convertis = convertir_tools_openai(&schema);
        assert_eq!(convertis.len(), 1, "a nameless schema entry is already dropped here");
        assert!(assainir_tools_openai(convertis)
            .iter()
            .all(|t| t["function"]["name"].as_str().is_some_and(|n| !n.is_empty())));
    }

    /// The refused body carried 38 messages, 23 of them tool results of roughly 3000
    /// chars, 73 KB in total. Not one exceeded the fixed 3500-char threshold, so the
    /// first version of the guard cut nothing at all and the request went out over
    /// budget anyway.
    #[test]
    fn le_budget_rabote_aussi_un_banc_de_petites_observations() {
        let mut messages = vec![
            serde_json::json!({ "role": "system", "content": "x".repeat(20_889) }),
            serde_json::json!({ "role": "user", "content": "jepa 2026" }),
        ];
        for _ in 0..23 {
            messages.push(serde_json::json!({ "role": "tool", "content": "y".repeat(3_100) }));
        }
        let body = serde_json::json!({ "model": "m", "messages": messages, "tools": [] });

        let avant = json_ascii(&serde_json::to_string(&body).unwrap()).len();
        assert!(avant > 76_800, "the fixture must exceed the guard: {avant}");

        let apres = reduire_sous_budget(&body, 76_800).unwrap();
        let corps = json_ascii(&serde_json::to_string(&apres).unwrap()).len();
        assert!(corps <= 76_800, "must fit, got {corps}");
        assert_eq!(apres["messages"].as_array().unwrap().len(), 25, "no message dropped");
        assert_eq!(apres["messages"][1]["content"], "jepa 2026", "the question is untouched");
    }

    /// A conversation of fat tool results must still fit on the wire.
    ///
    /// The real body that kept being refused was 114497 bytes for 16 messages: one
    /// observation of 23898 chars, two of 16099, one of 14101. The token gauge read
    /// 15% of a 128k window and never triggered a compaction, because the wall is
    /// counted in bytes and it sits far earlier.
    #[test]
    fn le_budget_en_octets_rabote_les_grosses_observations() {
        // Mirrors the shape of the refused body.
        let gros = |n: usize| "x".repeat(n);
        let messages = serde_json::json!([
            { "role": "system", "content": gros(21_000) },
            { "role": "user", "content": "explain the architecture" },
            { "role": "tool", "content": gros(23_898) },
            { "role": "tool", "content": gros(16_099) },
            { "role": "tool", "content": gros(16_099) },
            { "role": "tool", "content": gros(14_101) },
        ]);
        let mut body = serde_json::json!({ "model": "m", "messages": messages });
        let outils: Vec<serde_json::Value> = (0..20)
            .map(|i| serde_json::json!({ "name": format!("t{i}"), "description": gros(500) }))
            .collect();
        body["tools"] = serde_json::json!(outils);

        let brut = json_ascii(&serde_json::to_string(&body).unwrap());
        assert!(brut.len() > 76_800, "the fixture must exceed the guard: {}", brut.len());

        let apres = reduire_sous_budget(&body, 76_800).unwrap();
        let corps = json_ascii(&serde_json::to_string(&apres).unwrap());
        assert!(
            corps.len() <= 76_800,
            "the guard must bring the body under budget, got {}",
            corps.len()
        );
        // The exchange survives: every message is still there, in order.
        assert_eq!(apres["messages"].as_array().unwrap().len(), 6);
        assert_eq!(apres["messages"][1]["content"], "explain the architecture");
        // And a truncated observation says so.
        let recolte = apres["messages"][2]["content"].as_str().unwrap();
        assert!(recolte.contains("cut to fit the request budget"), "the cut must be announced");
    }

    /// A self-hosted model is recognised by its ADDRESS, never by its provider name.
    ///
    /// llama.cpp, Ollama and LM Studio all speak the OpenAI API, so a local profile is
    /// routinely declared `provider: "openai"`. The real configuration that broke:
    /// name "llama.cpp Local (:8001)", provider "openai", base_url
    /// "http://127.0.0.1:8001". Treated as a cloud backend, it received a trailing
    /// system message and refused the request outright.
    #[test]
    fn une_adresse_locale_est_reconnue_quel_que_soit_le_nom_du_provider() {
        for local in [
            "http://127.0.0.1:8001",
            "http://localhost:11434",
            "http://192.168.1.40:8080",
            "http://10.0.0.5:8001",
            "http://172.16.4.2:8001",
            "http://172.31.255.1:8001",
            "http://0.0.0.0:8001",
            "http://mon-pc.local:8001",
        ] {
            assert!(is_local_base_url(local), "must be local: {local}");
        }
        for distant in [
            "https://api.deepseek.com",
            "https://api.z.ai/api/paas/v4/chat/completions",
            "https://api.openai.com",
            "http://172.32.0.1:8001",
            "http://11.0.0.1:8001",
        ] {
            assert!(!is_local_base_url(distant), "must be remote: {distant}");
        }
    }

    /// The wire must carry no multi-byte character.
    ///
    /// A gateway counted our bytes as characters: 83250 bytes / 83145 chars came
    /// back as "EOF while parsing a string at column 83250", the byte figure. Once
    /// every non-ASCII character travels escaped, bytes and characters coincide and
    /// the mis-framing cannot happen.
    #[test]
    fn le_corps_part_en_ascii_pur_et_reste_du_json_equivalent() {
        let doc = serde_json::json!({
            "messages": [
                { "role": "user", "content": "Résumé d'une clé été à Cannes 🐝" },
                { "role": "assistant", "content": "Voilà, c'est prêt." }
            ]
        });
        let brut = serde_json::to_string(&doc).unwrap();
        assert!(!brut.is_ascii(), "the fixture must actually contain accents");

        let ascii = json_ascii(&brut);
        assert!(ascii.is_ascii(), "no multi-byte character may remain");
        assert_eq!(
            ascii.len(),
            ascii.chars().count(),
            "bytes and chars must coincide"
        );

        // Escaping changes the encoding, never the meaning.
        let relu: serde_json::Value = serde_json::from_str(&ascii).unwrap();
        assert_eq!(relu, doc);
        assert_eq!(relu["messages"][0]["content"], "Résumé d'une clé été à Cannes 🐝");

        // An already-ASCII document is returned untouched.
        let simple = "{\"a\":\"b\"}";
        assert_eq!(json_ascii(simple), simple);
    }

    #[test]
    fn finaliser_tool_calls_ne_reemet_pas_les_memes_ids() {
        let mut acc: std::collections::HashMap<u32, (String, String, String)> =
            std::collections::HashMap::new();
        acc.insert(
            1,
            ("call_01_b".into(), "file_read".into(), r#"{"path":"a"}"#.into()),
        );
        acc.insert(
            0,
            ("call_00_a".into(), "web_deep_search".into(), r#"{"q":"x"}"#.into()),
        );

        let premier = finaliser_tool_calls(&mut acc).expect("first pass yields the calls");
        assert_eq!(premier.len(), 2);
        // Ordered by streaming index, not by id.
        assert_eq!(premier[0].id, "call_00_a");
        assert_eq!(premier[0].name, "web_deep_search");
        assert_eq!(premier[0].args["q"], "x");
        assert_eq!(premier[1].id, "call_01_b");

        assert!(
            finaliser_tool_calls(&mut acc).is_none(),
            "the accumulator must be drained, a second finalization emits nothing"
        );
    }
}
