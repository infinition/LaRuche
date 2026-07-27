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
    /// What the user sees and clicks in the web chat.
    pub emoji: &'static str,
    /// The emoji Telegram accepts for THIS reaction, which is not always ours.
    ///
    /// The Bot API only takes reactions from a closed list of 72, and three of our six
    /// are absent from it: no U+1F602, no U+1F62E, no U+1F615. `love` is the subtle one,
    /// the list carries U+2764 WITHOUT the variation selector, so the emoji we display
    /// is not byte-identical to the one Telegram wants.
    pub emoji_telegram: &'static str,
    /// What the model is told to do. Behavioural, never emotional.
    pub consigne: &'static str,
}

/// The full palette. Deliberately SIX: past that, a picker becomes a decision, and a
/// reaction has to stay cheaper than typing the sentence it replaces.
pub const REACTIONS: &[Reaction] = &[
    Reaction {
        cle: "up",
        emoji: "👍",
        emoji_telegram: "👍",
        consigne: "The user APPROVED it. Keep the same format, depth and tone for what \
                   follows. Do not restate it, do not re-explain what was already accepted, \
                   and do not thank them for the reaction.",
    },
    Reaction {
        cle: "down",
        emoji: "👎",
        emoji_telegram: "👎",
        consigne: "The user REJECTED it. Do NOT apologise and do NOT rephrase the same \
                   answer: the approach itself was wrong, not the wording. Question the \
                   assumption you built it on, try a different method, and verify with a \
                   tool instead of asserting. If you genuinely cannot tell what was wrong, \
                   ask ONE precise question rather than guessing twice.",
    },
    Reaction {
        cle: "love",
        emoji: "❤️",
        // U+2764 alone: with the variation selector Telegram rejects it.
        emoji_telegram: "❤",
        consigne: "The user found it excellent. That level of depth and that format are the \
                   target for the rest of this conversation.",
    },
    Reaction {
        cle: "haha",
        emoji: "😂",
        // U+1F602 is not on the list; U+1F923 is its closest sibling.
        emoji_telegram: "🤣",
        consigne: "The tone landed. Stay light, but do not force a joke into the next \
                   answer: the content still comes first.",
    },
    Reaction {
        cle: "wow",
        emoji: "😮",
        // U+1F62E is not on the list; U+1F92F carries the same surprise.
        emoji_telegram: "🤯",
        consigne: "Something in it was unexpected for the user. Make sure that point was \
                   actually established rather than plausible, and expand on it instead of \
                   moving on.",
    },
    Reaction {
        cle: "confused",
        emoji: "😕",
        // U+1F615 is not on the list; U+1F914 is the readable stand-in.
        emoji_telegram: "🤔",
        consigne: "The user did NOT understand it. Re-explain the SAME content differently: \
                   shorter, concrete, one worked example. Do not add new material, and do \
                   not simply repeat it in the same words.",
    },
];

/// Look a reaction up by key.
pub fn trouver(cle: &str) -> Option<&'static Reaction> {
    REACTIONS.iter().find(|r| r.cle == cle)
}

/// Which of our six intents an INBOUND emoji expresses.
///
/// Telegram lets the user pick from 72 emoji, so anything can arrive. Writing 72
/// separate instructions would be the wrong shape: most are near-synonyms, only ONE is
/// ever injected per turn, and each extra text is another thing to keep true. What
/// actually matters to the next answer is the INTENT, and there are six of those.
///
/// So this is a fan-in: many emoji, few behaviours. Anything unmapped returns None and
/// steers nothing, which is the honest outcome for 🍌 rather than a made-up reading.
pub fn intention_pour_emoji(emoji: &str) -> Option<&'static Reaction> {
    // Compare on the base codepoints: Telegram sends U+2764 bare where a client may
    // send U+2764 U+FE0F, and the two must not be different reactions.
    let nu: String = emoji.chars().filter(|c| *c != '\u{FE0F}').collect();
    let cle = match nu.as_str() {
        "👍" | "👌" | "🤝" | "💯" | "🆒" | "✍" | "🏆" => "up",
        "👎" | "🤬" | "🤮" | "💩" | "🖕" | "😡" | "💔" | "😢" | "😭" | "💊" => "down",
        "❤" | "🥰" | "😍" | "😘" | "💘" | "💋" | "❤‍🔥" | "🤗" | "🙏" | "🌚" | "😇" => "love",
        "😁" | "🤣" | "🤡" | "😈" | "🤪" | "🙈" | "🙉" | "🙊" | "💅" | "🗿" | "🥴" | "🍌"
        | "🍓" | "🍾" | "🌭" | "🐳" | "🦄" | "🎃" | "👻" | "👾" | "🎅" | "🎄" | "☃" | "😎"
        | "🤓" | "👨‍💻" | "🕊" => "haha",
        "🔥" | "👏" | "🤯" | "😱" | "🎉" | "🤩" | "⚡" | "👀" | "😨" | "🫡" => "wow",
        "🤔" | "🤨" | "😐" | "🤷" | "🤷‍♂" | "🤷‍♀" | "🥱" | "😴" => "confused",
        _ => return None,
    };
    trouver(cle)
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
    fn chaque_reaction_porte_un_emoji_que_telegram_accepte() {
        // The Bot API takes reactions only from a closed list of 72, and rejects the
        // rest with a silent 400 on a fire-and-forget call. Three of our six are absent
        // from it, so the mapping is not decoration: without it, half the palette fails
        // quietly on Telegram.
        for r in REACTIONS {
            assert!(!r.emoji_telegram.is_empty(), "no Telegram emoji for {}", r.cle);
            // U+FE0F is the trap. Telegram lists U+2764 bare, and the variation selector
            // makes the string a different one that the API refuses.
            assert!(
                !r.emoji_telegram.contains('\u{FE0F}'),
                "{} carries a variation selector, Telegram will refuse it",
                r.cle
            );
        }
        let tg = |cle: &str| trouver(cle).unwrap().emoji_telegram;
        // Allowed as-is, and identical to what the web chat shows.
        assert_eq!(tg("up"), "\u{1F44D}");
        assert_eq!(tg("down"), "\u{1F44E}");
        // Absent from the list, so they MUST differ from our display emoji.
        for cle in ["love", "haha", "wow", "confused"] {
            let r = trouver(cle).unwrap();
            assert_ne!(
                r.emoji, r.emoji_telegram,
                "{cle} is not on the Telegram list, it needs a substitute"
            );
        }
        assert_eq!(tg("love"), "\u{2764}");
        assert_eq!(tg("haha"), "\u{1F923}");
        assert_eq!(tg("wow"), "\u{1F92F}");
        assert_eq!(tg("confused"), "\u{1F914}");
    }

    #[test]
    fn les_emojis_entrants_retombent_sur_une_intention() {
        let cle = |e: &str| intention_pour_emoji(e).map(|r| r.cle);
        // Our own six, whatever their exact codepoints on the wire.
        assert_eq!(cle("👍"), Some("up"));
        assert_eq!(cle("👎"), Some("down"));
        assert_eq!(cle("🤔"), Some("confused"));
        // The variation selector must not create a second, unknown reaction: Telegram
        // sends U+2764 bare where a web client sends U+2764 U+FE0F.
        assert_eq!(cle("❤"), Some("love"));
        assert_eq!(cle("❤️"), Some("love"));
        // Emoji we never show but the user can still pick in Telegram.
        assert_eq!(cle("🔥"), Some("wow"));
        assert_eq!(cle("🤣"), Some("haha"));
        assert_eq!(cle("💩"), Some("down"));
        assert_eq!(cle("🙏"), Some("love"));
        // Something with no defensible reading steers nothing rather than being
        // assigned an invented meaning.
        assert_eq!(cle("🧱"), None);
        // And every intent it returns is one we can actually act on.
        for e in ["👍", "👎", "❤", "🤣", "🔥", "🤔"] {
            let r = intention_pour_emoji(e).unwrap();
            assert!(bloc_volatil(r.cle).is_some(), "{e} maps to an unusable key");
        }
    }

    #[test]
    fn une_cle_inconnue_ne_produit_rien() {
        assert!(bloc_volatil("shrug").is_none());
        assert!(!est_connue("shrug"));
        assert!(!est_connue(""));
        assert!(est_connue("up"));
    }
}

/// Prefix the agent uses to leave a reaction on the user's message: the game and chat
/// emote convention, `/haha` on its own line.
///
/// NOT `>>`, which was the first choice and is Markdown: `>` opens a blockquote, so the
/// moment a marker survived stripping for any reason it rendered as a quoted word in the
/// chat. Seen live with deepseek, which had emitted the marker perfectly.
///
/// A marker has to be inert in Markdown, and this one also happens to be a convention
/// every model has seen a million times, which is worth more than any amount of
/// explaining. A bare `/word` line is never prose, so nothing legitimate collides.
pub const MARQUEUR: &str = "/";

/// The instruction added to the prompt when `reactions_agent` is on. Nothing is added
/// when it is off, which is the default: this costs budget on every single turn.
pub fn consigne_prompt() -> String {
    let cles: Vec<String> = REACTIONS
        .iter()
        .map(|r| format!("{} = {}", r.cle, r.emoji))
        .collect();
    // Every line here exists because the first version failed in a specific way. It
    // described the format with a `<key>` placeholder and no example, said "optional"
    // and "no reaction is the normal case", and never covered the one case that matters
    // in practice. Asked point blank to react, the agent answered "tu veux que je te
    // fasse une réaction emoji ?" and never emitted a marker: it was obeying, and
    // talking ABOUT the reaction is not reacting.
    format!(
        "## Reacting to the user\n\
         You may add ONE emoji reaction to the user's message, the way an emote works in \
         a game or a chat: a line holding nothing but a slash and a key, as the FIRST or \
         the LAST line of your reply.\n\
         \n\
         Emotes: {}\n\
         \n\
         A complete reply, exactly like this:\n\
         {MARQUEUR}haha\n\
         Bien vu, je n'y avais pas pense.\n\
         \n\
         That line is stripped before display and becomes the emoji under the user's \
         message, so it IS the reaction: never mention it, never explain it, never \
         announce that you are about to react, and never put it inside a sentence, a code \
         block or a tool call. Writing \"I react with a thumbs up\" is NOT reacting.\n\
         \n\
         Use it sparingly, only when it adds something. The exception: when the user asks \
         you to react, emit the emote, do not reply that you will.",
        cles.join(", ")
    )
}

/// Strip the agent's reaction marker from an answer.
///
/// Returns the cleaned text and the key, when a VALID one was found. Deliberately
/// strict, because everything here is a way for a marker to reach the user's screen:
/// the line must be the first or the last, must hold nothing else, and must carry a
/// key we defined. A `/thumbup` in the middle of a paragraph is prose, not a
/// reaction, and is left exactly where the model put it.
pub fn extraire_reaction(texte: &str) -> (String, Option<String>) {
    let lignes: Vec<&str> = texte.lines().collect();
    if lignes.is_empty() {
        return (texte.to_string(), None);
    }
    let cle_de = |l: &str| -> Option<String> {
        let t = l.trim();
        let reste = t.strip_prefix(MARQUEUR)?.trim();
        // One token only: `/up` is a reaction, `/up and here is why` is a sentence, and
        // a sentence stays on screen where the model put it.
        if reste.is_empty() || reste.split_whitespace().count() != 1 {
            return None;
        }
        let cle = reste.trim_end_matches(['.', ',', '!', ':', ';']).to_lowercase();
        est_connue(&cle).then_some(cle)
    };

    let mut restantes = lignes.clone();
    let mut trouvee = None;
    // First line, then last. Only ONE is honoured: a model that stamped both is
    // guessing rather than reacting.
    if let Some(cle) = cle_de(restantes[0]) {
        trouvee = Some(cle);
        restantes.remove(0);
    } else if restantes.len() > 1 {
        if let Some(cle) = cle_de(restantes[restantes.len() - 1]) {
            trouvee = Some(cle);
            restantes.pop();
        }
    }
    if trouvee.is_none() {
        return (texte.to_string(), None);
    }
    (restantes.join("\n").trim().to_string(), trouvee)
}

#[cfg(test)]
mod tests_agent {
    use super::*;

    #[test]
    fn un_marqueur_en_tete_ou_en_queue_est_retire_du_texte_affiche() {
        let (t, c) = extraire_reaction("/haha\nBien vu, c'est drole.");
        assert_eq!(c.as_deref(), Some("haha"));
        assert_eq!(t, "Bien vu, c'est drole.");

        let (t, c) = extraire_reaction("Voila le resultat.\n/up");
        assert_eq!(c.as_deref(), Some("up"));
        assert_eq!(t, "Voila le resultat.");

        // Trailing punctuation is a model being a model, not a different key.
        let (_, c) = extraire_reaction("/wow.\ntexte");
        assert_eq!(c.as_deref(), Some("wow"));
    }

    #[test]
    fn un_marqueur_au_milieu_dune_phrase_reste_du_texte() {
        // The whole point of the strictness: this must reach the user untouched
        // rather than being silently eaten as a reaction.
        let brut = "Le pouce /up sert a valider.";
        let (t, c) = extraire_reaction(brut);
        assert_eq!(c, None);
        assert_eq!(t, brut);

        // A line that starts with the marker but says more is prose too.
        let brut2 = "/up and here is why it works";
        let (t2, c2) = extraire_reaction(brut2);
        assert_eq!(c2, None);
        assert_eq!(t2, brut2);
    }

    #[test]
    fn une_cle_inventee_nest_jamais_retiree() {
        // A hallucinated key must stay visible: silently swallowing it would hide the
        // fact that the model is emitting markers we never defined.
        let brut = "/thumbsup\ntexte";
        let (t, c) = extraire_reaction(brut);
        assert_eq!(c, None);
        assert_eq!(t, brut);
    }

    #[test]
    fn un_seul_marqueur_est_honore() {
        let (t, c) = extraire_reaction("/up\nmilieu\n/down");
        assert_eq!(c.as_deref(), Some("up"));
        // The second one stays in the text: it is evidence of a model guessing, and
        // hiding it would make that invisible.
        assert!(t.contains("/down"), "{t}");
    }

    #[test]
    fn la_consigne_montre_un_exemple_et_couvre_la_demande_explicite() {
        let c = consigne_prompt();
        // A `<key>` placeholder with no example got copied literally or ignored. The
        // marker must appear ready to paste.
        assert!(c.contains("/haha"), "no usable example:
{c}");
        assert!(!c.contains("<key>"), "placeholder syntax invites a literal copy:
{c}");
        // The keys carry their emoji, so the model knows what it is choosing.
        assert!(c.contains("down = "));
        // The failure observed live: asked to react, it answered that it would react.
        assert!(c.contains("when the user asks you to react"), "{c}");
        assert!(c.contains("is NOT reacting"), "{c}");
    }

    #[test]
    fn un_texte_ordinaire_traverse_intact() {
        let brut = "Reponse normale sans aucun marqueur.";
        let (t, c) = extraire_reaction(brut);
        assert_eq!(c, None);
        assert_eq!(t, brut);
        assert!(consigne_prompt().contains("confused"));
    }
}
