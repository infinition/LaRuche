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
    format!(
        "{charte}\n\n\
         ---\n\
         You are judging {cible}.\n\n\
         User objective (north star):\n{objectif}\n\n\
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

fn score(v: &serde_json::Value, cle: &str) -> u8 {
    v.get(cle)
        .and_then(|x| x.as_u64())
        .unwrap_or(0)
        .min(100) as u8
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
            let key = k.trim().trim_start_matches(['-', '*', '#', '`', ' ']).to_lowercase();
            // Keep only plausibly-alphabetic keys (avoids matching JSON like {"x":1}).
            if !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphabetic() || c == '_' || c == ' ')
            {
                champs.entry(key).or_insert_with(|| v.trim().to_string());
            }
        }
    }
    let get = |keys: &[&str]| -> Option<String> {
        keys.iter().find_map(|k| champs.get(*k).cloned())
    };
    let num = |keys: &[&str]| -> u8 { get(keys).map(|v| nombre_borne(&v)).unwrap_or(0) };

    let verdict = get(&["verdict", "avis"]).unwrap_or_default();
    let has_signal =
        !verdict.is_empty() || champs.contains_key("relevance") || champs.contains_key("pertinence");
    if !has_signal {
        return None;
    }
    Some(Scorecard {
        pertinence: num(&["relevance", "pertinence"]),
        methodologie: num(&["methodology", "methodologie", "method"]),
        objectif: num(&["objective", "objectif"]),
        conformite_marque: num(&["brand", "conformite_marque", "brand_compliance"]),
        confiance: num(&["confidence", "confiance"]),
        avis: avis_depuis(&verdict),
        instruction: get(&["instruction"]).unwrap_or_default(),
        raison: get(&["reason", "raison"]).unwrap_or_default(),
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
    Ok(Scorecard {
        pertinence: score(&v, "pertinence"),
        methodologie: score(&v, "methodologie"),
        objectif: score(&v, "objectif"),
        conformite_marque: score(&v, "conformite_marque"),
        confiance: score(&v, "confiance"),
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
        }
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
        // Missing scores default to 0.
        assert_eq!(c.methodologie, 0);
    }

    #[test]
    fn scores_are_clamped_to_100() {
        let r = r#"{"pertinence":250,"avis":"approuver"}"#;
        assert_eq!(parser_scorecard(r).unwrap().pertinence, 100);
    }

    #[test]
    fn unknown_verdict_defaults_to_revise() {
        let r = r#"{"avis":"maybe"}"#;
        assert_eq!(parser_scorecard(r).unwrap().avis, Avis::Reviser);
    }

    #[test]
    fn braces_inside_strings_do_not_break_extraction() {
        let r = r#"{"raison":"use {placeholder} syntax","avis":"approuver"}"#;
        let c = parser_scorecard(r).unwrap();
        assert_eq!(c.raison, "use {placeholder} syntax");
        assert_eq!(c.avis, Avis::Approuver);
    }

    #[test]
    fn no_json_is_an_error() {
        assert!(parser_scorecard("I could not assess this.").is_err());
    }
}
