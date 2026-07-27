//! The Reine's judge: the bridge between a draft and the pure decision core
//! ([`laruche_butinage::cap::reine`]). This module builds the prompt that asks
//! an LLM to assess a draft against LaReine's charter, and parses the reply into
//! a [`Scorecard`]. Both halves are pure and unit-tested; the actual provider
//! call is injected by the integration layer (like [`crate::background_review`]),
//! so nothing here reaches the network or the live chat loop.

use laruche_butinage::cap::reine::{Avis, Scorecard, Tier};

/// Everything the judge needs to assess a single draft.
pub struct DemandeJugement<'a> {
    /// Which scope is being judged (response, artifact, supervision).
    pub tier: Tier,
    /// The user's underlying objective (the north star), if known.
    pub objectif: &'a str,
    /// The original request being answered.
    pub requete: &'a str,
    /// The draft to judge.
    pub brouillon: &'a str,
    /// The LaReine charter body (the rubric). Loaded from the charter skill.
    pub charte: &'a str,
    /// Recent conversation transcript for context (the last N turns before the
    /// draft), so the Reine judges with awareness of what came before. Empty when
    /// the context window is 0 or there is no prior history.
    pub contexte: &'a str,
    /// Live workshop introspection: which tools the worker HAD available and which
    /// it actually called for this draft (with failures). This is what makes the
    /// METHODOLOGY score real: a draft claiming verification without a single tool
    /// call reads very differently from one backed by fetches. Empty = unknown.
    pub atelier: &'a str,
}

/// Line-based shape the judge must return (one `KEY: value` per line). Far more
/// reliable for small local models than nested JSON. Kept here so the prompt and
/// the parser never drift apart.
const FORMAT_REPONSE: &str = "ANALYSIS: <your reasoning in 1-2 sentences, before scoring>\n\
RELEVANCE: <0-100>\n\
METHODOLOGY: <0-100>\n\
OBJECTIVE: <0-100>\n\
BRAND: <0-100>\n\
CONFIDENCE: <0-100>\n\
VERDICT: approve | revise | escalate\n\
INSTRUCTION: <corrective instruction, only when VERDICT is revise>\n\
REASON: <one short line>";

fn tier_libelle(tier: Tier) -> &'static str {
    match tier {
        Tier::Reponse => "a chat answer about to be sent to the user",
        Tier::Artefact => "a self-created artifact (skill, tool, memory edit, or mission)",
        Tier::Supervision => "the current state of a swarm task you are supervising",
    }
}

/// Build the full judge prompt. Pure: deterministic for a given input.
///
/// The charter is the rubric; the rest frames the specific draft. The model is
/// told to answer with the strict JSON of [`FORMAT_REPONSE`] and nothing else.
pub fn construire_prompt(d: &DemandeJugement) -> String {
    let objectif = if d.objectif.trim().is_empty() {
        "(not explicitly stated; infer it from the request)"
    } else {
        d.objectif
    };
    let contexte_bloc = if d.contexte.trim().is_empty() {
        String::new()
    } else {
        format!(
            "Recent conversation (for context, oldest first):\n{}\n\n",
            d.contexte.trim()
        )
    };
    let atelier_bloc = if d.atelier.trim().is_empty() {
        String::new()
    } else {
        format!(
            "Workshop introspection (tools available to the worker, and what it \
             actually did to produce this draft - weigh METHODOLOGY on this, not on \
             what the draft claims):\n{}\n\n",
            d.atelier.trim()
        )
    };
    format!(
        "{charte}\n\n\
         ---\n\
         You are judging {cible}.\n\n\
         User objective (north star):\n{objectif}\n\n\
         {contexte_bloc}\
         {atelier_bloc}\
         Original request:\n{requete}\n\n\
         Draft to judge:\n{brouillon}\n\n\
         ---\n\
         Assess the draft against the charter. Reason briefly in the ANALYSIS line, then \
         score. Approve readily when it is good; a revision that does not measurably improve \
         the draft is worse than shipping the original. When you revise, the instruction must \
         be specific and executable, naming what is wrong and what to do.\n\n\
         Reply with EXACTLY these lines, one \"KEY: value\" per line, and nothing else (no \
         extra prose, no JSON, no markdown):\n{format}\n\n\
         Example:\n\
         ANALYSIS: Clear and on-scope; tone is warm which is fine; claims are grounded.\n\
         RELEVANCE: 85\nMETHODOLOGY: 80\nOBJECTIVE: 82\nBRAND: 90\nCONFIDENCE: 88\n\
         VERDICT: approve\nINSTRUCTION: \nREASON: Clear, on-scope, grounded.",
        charte = d.charte.trim(),
        cible = tier_libelle(d.tier),
        objectif = objectif.trim(),
        contexte_bloc = contexte_bloc,
        atelier_bloc = atelier_bloc,
        requete = d.requete.trim(),
        brouillon = d.brouillon.trim(),
        format = FORMAT_REPONSE,
    )
}

/// Extract the first balanced JSON object from a string that may carry prose or a
/// code fence around it. Returns the raw object slice.
fn extraire_json(s: &str) -> Option<&str> {
    let debut = s.find('{')?;
    let mut profondeur = 0i32;
    let mut dans_chaine = false;
    let mut echappe = false;
    for (i, c) in s[debut..].char_indices() {
        if dans_chaine {
            if echappe {
                echappe = false;
            } else if c == '\\' {
                echappe = true;
            } else if c == '"' {
                dans_chaine = false;
            }
            continue;
        }
        match c {
            '"' => dans_chaine = true,
            '{' => profondeur += 1,
            '}' => {
                profondeur -= 1;
                if profondeur == 0 {
                    return Some(&s[debut..=debut + i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Absent is NOT zero.
///
/// A judge reply whose VERDICT line parsed but whose score lines did not used to
/// produce 0 relevance, 0 methodology, 0 objective, with 95 confidence. "Your answer
/// is worth nothing" then drove an escalation and five rework rounds against a draft
/// nobody had actually faulted. It happened twice in nine recorded scorecards.
///
/// So a score we could not read is a score we do not HAVE. The three substantive axes
/// must carry at least one real number, otherwise the reply is unusable and the caller
/// stops rather than inventing a verdict. The axes we are missing take the mean of the
/// ones we read: a neutral stand-in, never the worst possible value.
fn completer(
    pertinence: Option<u8>,
    methodologie: Option<u8>,
    objectif: Option<u8>,
    marque: Option<u8>,
    confiance: Option<u8>,
) -> Option<(u8, u8, u8, u8, u8)> {
    let substantiels = [pertinence, methodologie, objectif];
    if substantiels.iter().all(Option::is_none) {
        return None;
    }
    let lus: Vec<u8> = substantiels.iter().flatten().copied().collect();
    let moyenne = (lus.iter().map(|v| *v as u32).sum::<u32>() / lus.len() as u32) as u8;
    Some((
        pertinence.unwrap_or(moyenne),
        methodologie.unwrap_or(moyenne),
        objectif.unwrap_or(moyenne),
        marque.unwrap_or(moyenne),
        // Confidence steers the Hybride escalation, so an unread one must not read as
        // certainty about a judgement we could barely parse.
        confiance.unwrap_or(moyenne.min(60)),
    ))
}

fn score_opt(v: &serde_json::Value, cle: &str) -> Option<u8> {
    v.get(cle)
        .and_then(|x| x.as_u64())
        .map(|n| n.min(100) as u8)
}

fn avis_depuis(s: &str) -> Avis {
    let t = s.trim().to_lowercase();
    if t.starts_with("approuver") || t.starts_with("approve") {
        Avis::Approuver
    } else if t.starts_with("escalader") || t.starts_with("escalate") {
        Avis::Escalader
    } else {
        // Default to revise: the conservative choice is to look again, not to ship.
        Avis::Reviser
    }
}

/// Drop a model's chain-of-thought: keep only what follows the last `</think>`.
fn sans_reasoning(s: &str) -> String {
    match s.rfind("</think>") {
        Some(pos) => s[pos + "</think>".len()..].trim().to_string(),
        None => s.trim().to_string(),
    }
}

/// Extract the first run of digits as a 0..=100 score.
fn nombre_borne(v: &str) -> u8 {
    let n: String = v
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    n.parse::<u64>().unwrap_or(0).min(100) as u8
}

/// Parse a line-based judge reply (`KEY: value` per line). Robust for small models.
/// Returns None when no recognizable verdict or score line is present.
fn parser_lignes(s: &str) -> Option<Scorecard> {
    use std::collections::HashMap;
    let mut champs: HashMap<String, String> = HashMap::new();
    for ligne in s.lines() {
        if let Some((k, v)) = ligne.split_once(':') {
            let raw = k.trim().trim_start_matches(['-', '*', '#', '`', ' ']).to_lowercase();
            // Take the leading alphabetic run as the key, dropping trailing decorations
            // like "(0-100)". A key starting with a non-letter (e.g. JSON `{"x"`) yields
            // an empty key and is skipped, so JSON lines still do not pollute the map.
            let key: String = raw
                .chars()
                .take_while(|c| c.is_ascii_alphabetic() || *c == '_' || *c == ' ')
                .collect();
            let key = key.trim().to_string();
            if !key.is_empty() {
                champs.entry(key).or_insert_with(|| v.trim().to_string());
            }
        }
    }
    // Match a key exactly, or by prefix so decorated keys still resolve (a small model
    // writing "RELEVANCE SCORE: 85" or "confidence (0-100): 70" still maps correctly).
    let get = |keys: &[&str]| -> Option<String> {
        keys.iter().find_map(|k| {
            champs.get(*k).cloned().or_else(|| {
                champs
                    .iter()
                    .find(|(kk, _)| kk.as_str().starts_with(*k))
                    .map(|(_, v)| v.clone())
            })
        })
    };
    let num = |keys: &[&str]| -> Option<u8> { get(keys).map(|v| nombre_borne(&v)) };

    let verdict = get(&["verdict", "avis"]).unwrap_or_default();
    // A verdict ALONE is not a scorecard. Accepting one filled the three axes with
    // zeros and shipped that as a judgement; see `completer`.
    let (pertinence, methodologie, objectif, conformite_marque, confiance) = completer(
        num(&["relevance", "pertinence"]),
        num(&["methodology", "methodologie", "method"]),
        num(&["objective", "objectif"]),
        num(&["brand", "conformite_marque", "brand_compliance"]),
        num(&["confidence", "confiance"]),
    )?;
    // Verdict: explicit when the model gave one; inferred from the scores when it
    // omitted the line. Inference avoids forcing a rework on a clearly strong draft
    // just because a small model forgot the VERDICT line (the conservative default of
    // "revise" otherwise burns rework rounds on a good answer).
    let avis = if verdict.trim().is_empty() {
        let solide = pertinence >= 75 && methodologie >= 75 && objectif >= 75 && confiance >= 75;
        if solide {
            Avis::Approuver
        } else {
            Avis::Reviser
        }
    } else {
        avis_depuis(&verdict)
    };
    Some(Scorecard {
        pertinence,
        methodologie,
        objectif,
        conformite_marque,
        confiance,
        avis,
        instruction: get(&["instruction"]).unwrap_or_default(),
        raison: get(&["reason", "raison"]).unwrap_or_default(),
        analyse: get(&["analysis", "analyse"]).unwrap_or_default(),
    })
}

/// Parse the judge LLM reply into a [`Scorecard`]. Tolerant of surrounding prose
/// or a code fence. Missing scores default to 0, unknown verdicts default to
/// "reviser" (the safe choice). Pure.
pub fn parser_scorecard(reponse: &str) -> Result<Scorecard, String> {
    // Strip any chain-of-thought first, then try the line format (robust for small
    // models), then fall back to JSON (for models that emit it).
    let propre = sans_reasoning(reponse);
    if let Some(card) = parser_lignes(&propre) {
        return Ok(card);
    }
    let brut = extraire_json(&propre)
        .ok_or("judge reply has neither key:value lines nor a JSON object")?;
    let v: serde_json::Value =
        serde_json::from_str(brut).map_err(|e| format!("invalid judge JSON: {e}"))?;

    let avis = avis_depuis(v.get("avis").and_then(|x| x.as_str()).unwrap_or(""));
    let (pertinence, methodologie, objectif, conformite_marque, confiance) = completer(
        score_opt(&v, "pertinence"),
        score_opt(&v, "methodologie"),
        score_opt(&v, "objectif"),
        score_opt(&v, "conformite_marque"),
        score_opt(&v, "confiance"),
    )
    .ok_or("judge JSON carries no readable score on relevance, methodology or objective")?;
    Ok(Scorecard {
        pertinence,
        methodologie,
        objectif,
        conformite_marque,
        confiance,
        avis,
        instruction: v
            .get("instruction")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        raison: v
            .get("raison")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        analyse: v
            .get("analyse")
            .or_else(|| v.get("analysis"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demande<'a>(brouillon: &'a str) -> DemandeJugement<'a> {
        DemandeJugement {
            tier: Tier::Reponse,
            objectif: "ship a correct answer",
            requete: "what is 2 + 2?",
            brouillon,
            charte: "CHARTER: judge relevance and methodology.",
            contexte: "",
            atelier: "",
        }
    }

    #[test]
    fn prompt_carries_workshop_introspection_when_present() {
        let mut d = demande("4");
        d.atelier = "Tools available: calculator. Trace: 1 call, calculator OK.";
        let p = construire_prompt(&d);
        assert!(p.contains("Workshop introspection"));
        assert!(p.contains("calculator OK"));
        // Absent when empty (no hollow header).
        let p2 = construire_prompt(&demande("4"));
        assert!(!p2.contains("Workshop introspection"));
    }

    #[test]
    fn prompt_contains_charter_request_and_draft() {
        let p = construire_prompt(&demande("4"));
        assert!(p.contains("CHARTER"));
        assert!(p.contains("what is 2 + 2?"));
        assert!(p.contains("Draft to judge"));
        assert!(p.contains("VERDICT"));
        assert!(p.contains("ANALYSIS"));
    }

    #[test]
    fn parses_line_based_reply() {
        let r = "RELEVANCE: 90\nMETHODOLOGY: 80\nOBJECTIVE: 85\nBRAND: 100\nCONFIDENCE: 95\nVERDICT: approve\nINSTRUCTION:\nREASON: solid";
        let c = parser_scorecard(r).unwrap();
        assert_eq!(c.pertinence, 90);
        assert_eq!(c.conformite_marque, 100);
        assert_eq!(c.avis, Avis::Approuver);
        assert_eq!(c.raison, "solid");
    }

    #[test]
    fn decorated_keys_still_resolve() {
        // Small models often decorate the key. Prefix matching must still map them.
        let r = "RELEVANCE SCORE: 88\nMETHODOLOGY (0-100): 82\nOBJECTIVE: 80\nBRAND: 90\nCONFIDENCE level: 77\nVERDICT: approve\nREASON: ok";
        let c = parser_scorecard(r).unwrap();
        assert_eq!(c.pertinence, 88);
        assert_eq!(c.methodologie, 82);
        assert_eq!(c.confiance, 77);
        assert_eq!(c.avis, Avis::Approuver);
    }

    #[test]
    fn missing_verdict_is_inferred_from_scores() {
        // Strong scores but the model forgot the VERDICT line: approve, do not rework.
        let strong = "RELEVANCE: 85\nMETHODOLOGY: 80\nOBJECTIVE: 82\nBRAND: 90\nCONFIDENCE: 88\nREASON: good";
        assert_eq!(parser_scorecard(strong).unwrap().avis, Avis::Approuver);
        // Weak scores and no verdict: still revise (the safe choice).
        let weak = "RELEVANCE: 40\nMETHODOLOGY: 50\nOBJECTIVE: 45\nBRAND: 90\nCONFIDENCE: 60\nREASON: thin";
        assert_eq!(parser_scorecard(weak).unwrap().avis, Avis::Reviser);
    }

    #[test]
    fn strips_reasoning_before_line_parse() {
        let r = "<think>Let me weigh the tone and scope carefully...</think>\nRELEVANCE: 40\nVERDICT: revise\nINSTRUCTION: lead with the answer";
        let c = parser_scorecard(r).unwrap();
        assert_eq!(c.pertinence, 40);
        assert_eq!(c.avis, Avis::Reviser);
        assert_eq!(c.instruction, "lead with the answer");
    }

    #[test]
    fn empty_objective_is_replaced_by_a_hint() {
        let mut d = demande("4");
        d.objectif = "   ";
        let p = construire_prompt(&d);
        assert!(p.contains("infer it from the request"));
    }

    #[test]
    fn parses_clean_json() {
        let r = r#"{"pertinence":90,"methodologie":80,"objectif":85,"conformite_marque":100,"confiance":95,"avis":"approuver","instruction":"","raison":"solid"}"#;
        let c = parser_scorecard(r).unwrap();
        assert_eq!(c.pertinence, 90);
        assert_eq!(c.avis, Avis::Approuver);
        assert_eq!(c.score_global(), 88); // (90+80+85+100)/4 = 88 (integer)
        assert_eq!(c.raison, "solid");
    }

    #[test]
    fn parses_json_wrapped_in_prose_and_fence() {
        let r = "Here is my assessment:\n```json\n{\"pertinence\":40,\"avis\":\"reviser\",\"instruction\":\"lead with the answer\"}\n```\nDone.";
        let c = parser_scorecard(r).unwrap();
        assert_eq!(c.pertinence, 40);
        assert_eq!(c.avis, Avis::Reviser);
        assert_eq!(c.instruction, "lead with the answer");
        // A missing axis takes the mean of the ones we DID read, never zero: reporting
        // 0 methodology for an axis the judge never mentioned is a fabricated verdict,
        // and it cost five rework rounds twice in the recorded scorecards.
        assert_eq!(c.methodologie, 40);
    }

    #[test]
    fn scores_are_clamped_to_100() {
        let r = r#"{"pertinence":250,"avis":"approuver"}"#;
        assert_eq!(parser_scorecard(r).unwrap().pertinence, 100);
    }

    #[test]
    fn unknown_verdict_defaults_to_revise() {
        let r = r#"{"pertinence":70,"avis":"maybe"}"#;
        assert_eq!(parser_scorecard(r).unwrap().avis, Avis::Reviser);
    }

    #[test]
    fn braces_inside_strings_do_not_break_extraction() {
        let r = r#"{"pertinence":90,"raison":"use {placeholder} syntax","avis":"approuver"}"#;
        let c = parser_scorecard(r).unwrap();
        assert_eq!(c.raison, "use {placeholder} syntax");
        assert_eq!(c.avis, Avis::Approuver);
    }

    #[test]
    fn un_verdict_seul_nest_pas_une_scorecard() {
        // The exact shape recorded twice in evals/reine-scorecards.jsonl: relevance 0,
        // methodology 0, objective 0, high confidence, verdict escalate. No judge ever
        // said the answer was worth nothing; the scores simply had not parsed, and the
        // zeros then drove five rework rounds.
        assert!(parser_scorecard(r#"{"avis":"escalader"}"#).is_err());
        assert!(parser_scorecard("VERDICT: escalate").is_err());
        assert!(parser_scorecard("VERDICT: escalate
REASON: unclear").is_err());
    }

    #[test]
    fn un_axe_absent_prend_la_moyenne_des_axes_lus() {
        let c = parser_scorecard("RELEVANCE: 90
OBJECTIVE: 70
VERDICT: approve").unwrap();
        assert_eq!(c.pertinence, 90);
        assert_eq!(c.objectif, 70);
        assert_eq!(c.methodologie, 80, "an unread axis must not read as zero");
        assert_eq!(c.conformite_marque, 80);
        // Confidence we never read must not present itself as certainty: it gates the
        // Hybride escalation.
        assert!(c.confiance <= 60, "confiance = {}", c.confiance);
    }

    #[test]
    fn une_scorecard_complete_nest_pas_touchee() {
        let c = parser_scorecard(
            "RELEVANCE: 95
METHODOLOGY: 90
OBJECTIVE: 95
BRAND: 98
CONFIDENCE: 95
VERDICT: approve",
        )
        .unwrap();
        assert_eq!(
            (c.pertinence, c.methodologie, c.objectif, c.conformite_marque, c.confiance),
            (95, 90, 95, 98, 95)
        );
        assert_eq!(c.avis, Avis::Approuver);
    }

    #[test]
    fn no_json_is_an_error() {
        assert!(parser_scorecard("I could not assess this.").is_err());
    }
}
