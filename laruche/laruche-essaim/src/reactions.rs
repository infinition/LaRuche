//! User reactions on an answer, and what the agent must DO about them.
//!
//! A reaction is the cheapest correction signal there is: one click, no sentence to
//! write. It costs nothing in the prompt when there is none, and it rides in the
//! VOLATILE tail tier when there is one, so the cached prefix never moves.
//!
//! The whole design sits in [`Reaction::consigne`]. A reaction is not an emotion to
//! acknowledge, it is an instruction to follow. Told "the user is unhappy", a model
//! apologises and reformulates the same answer, which is the single most useless
//! thing it can do. Told "the approach was wrong, change method", it changes method.
//! Every line below is written as an imperative about the NEXT answer.

/// One reaction the user can leave, and what it means for what happens next.
pub struct Reaction {
    /// Stable key, what the API and the session file store.
    pub cle: &'static str,
    /// What the user sees and clicks.
    pub emoji: &'static str,
    /// What the model is told to do. Behavioural, never emotional.
    pub consigne: &'static str,
}

/// The full palette. Deliberately SIX: past that, a picker becomes a decision, and a
/// reaction has to stay cheaper than typing the sentence it replaces.
pub const REACTIONS: &[Reaction] = &[
    Reaction {
        cle: "up",
        emoji: "👍",
        consigne: "The user APPROVED it. Keep the same format, depth and tone for what \
                   follows. Do not restate it, do not re-explain what was already accepted, \
                   and do not thank them for the reaction.",
    },
    Reaction {
        cle: "down",
        emoji: "👎",
        consigne: "The user REJECTED it. Do NOT apologise and do NOT rephrase the same \
                   answer: the approach itself was wrong, not the wording. Question the \
                   assumption you built it on, try a different method, and verify with a \
                   tool instead of asserting. If you genuinely cannot tell what was wrong, \
                   ask ONE precise question rather than guessing twice.",
    },
    Reaction {
        cle: "love",
        emoji: "❤️",
        consigne: "The user found it excellent. That level of depth and that format are the \
                   target for the rest of this conversation.",
    },
    Reaction {
        cle: "haha",
        emoji: "😂",
        consigne: "The tone landed. Stay light, but do not force a joke into the next \
                   answer: the content still comes first.",
    },
    Reaction {
        cle: "wow",
        emoji: "😮",
        consigne: "Something in it was unexpected for the user. Make sure that point was \
                   actually established rather than plausible, and expand on it instead of \
                   moving on.",
    },
    Reaction {
        cle: "confused",
        emoji: "😕",
        consigne: "The user did NOT understand it. Re-explain the SAME content differently: \
                   shorter, concrete, one worked example. Do not add new material, and do \
                   not simply repeat it in the same words.",
    },
];

/// Look a reaction up by key.
pub fn trouver(cle: &str) -> Option<&'static Reaction> {
    REACTIONS.iter().find(|r| r.cle == cle)
}

/// Is this key one we defined? Guards the API against a client sending anything.
pub fn est_connue(cle: &str) -> bool {
    trouver(cle).is_some()
}

/// The block injected into the volatile tail tier, or `None` when the key is unknown.
///
/// Kept to a few lines on purpose: it sits at the very end of the context, where the
/// model reads last, and it competes with the findings ledger for that attention.
pub fn bloc_volatil(cle: &str) -> Option<String> {
    let r = trouver(cle)?;
    Some(format!(
        "## The user reacted {} to your last answer\n{}\nThis is a live steering signal \
         for THIS turn. Do not store it, do not mention the reaction itself.",
        r.emoji, r.consigne
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_reaction_est_une_consigne_daction() {
        assert_eq!(REACTIONS.len(), 6);
        for r in REACTIONS {
            assert!(!r.cle.is_empty() && !r.emoji.is_empty());
            // An instruction, not a feeling report. If a consigne ever shrinks to
            // "the user is unhappy", the whole feature degrades into apologising.
            assert!(r.consigne.len() > 60, "consigne too thin for {}", r.cle);
        }
        // Keys are unique: a duplicate would silently shadow one in `trouver`.
        let mut cles: Vec<&str> = REACTIONS.iter().map(|r| r.cle).collect();
        cles.sort_unstable();
        let avant = cles.len();
        cles.dedup();
        assert_eq!(cles.len(), avant);
    }

    #[test]
    fn le_rejet_interdit_explicitement_les_excuses_et_la_reformulation() {
        // The one that matters. A thumbs-down must change METHOD.
        let bloc = bloc_volatil("down").unwrap();
        assert!(bloc.contains("👎"));
        assert!(bloc.contains("Do NOT apologise"));
        assert!(bloc.contains("do NOT rephrase"));
        assert!(bloc.contains("different method"));
    }

    #[test]
    fn une_cle_inconnue_ne_produit_rien() {
        assert!(bloc_volatil("shrug").is_none());
        assert!(!est_connue("shrug"));
        assert!(!est_connue(""));
        assert!(est_connue("up"));
    }
}
