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

/// JSON shape the judge LLM must return. Kept here so the prompt and the parser
/// can never drift apart.
const FORMAT_REPONSE: &str = r#"{
  "pertinence": <0-100>,
  "methodologie": <0-100>,
  "objectif": <0-100>,
  "conformite_marque": <0-100>,
  "confiance": <0-100>,
  "avis": "approuver" | "reviser" | "escalader",
  "instruction": "<corrective instruction for the worker, required when avis is reviser>",
  "raison": "<one short line shown in the chat trace>"
}"#;

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
         Assess the draft against the charter. Approve readily when it is good; a \
         revision that does not measurably improve the draft is worse than shipping \
         the original. When you revise, the instruction must be specific and \
         executable, naming what is wrong and what to do.\n\n\
         Output a SINGLE JSON object and nothing else. No prose, no markdown, no code \
         fence. Your reply MUST start with the character {{ and end with }}.\n\
         Schema:\n{format}\n\n\
         Example of a valid reply:\n\
         {{\"pertinence\":85,\"methodologie\":80,\"objectif\":82,\"conformite_marque\":90,\
\"confiance\":88,\"avis\":\"approuver\",\"instruction\":\"\",\"raison\":\"Clear, on-scope, grounded.\"}}",
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
    match s.trim().to_lowercase().as_str() {
        "approuver" | "approve" | "approved" => Avis::Approuver,
        "escalader" | "escalate" => Avis::Escalader,
        // Default to revise: the conservative choice is to look again, not to ship.
        _ => Avis::Reviser,
    }
}

/// Parse the judge LLM reply into a [`Scorecard`]. Tolerant of surrounding prose
/// or a code fence. Missing scores default to 0, unknown verdicts default to
/// "reviser" (the safe choice). Pure.
pub fn parser_scorecard(reponse: &str) -> Result<Scorecard, String> {
    let brut = extraire_json(reponse).ok_or("no JSON object found in judge reply")?;
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
        assert!(p.contains("\"avis\""));
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
