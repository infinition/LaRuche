use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

pub struct Todo;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TodoItem {
    id: u64,
    text: String,
    done: bool,
}

#[derive(Debug, Default)]
struct TodoStore {
    next_id: u64,
    items: Vec<TodoItem>,
}

static TODO_STORE: OnceLock<Mutex<TodoStore>> = OnceLock::new();

fn store() -> &'static Mutex<TodoStore> {
    TODO_STORE.get_or_init(|| {
        Mutex::new(TodoStore {
            next_id: 1,
            items: Vec::new(),
        })
    })
}

/// Ce que le modele voulait dire, a partir de ce qu'il a ecrit.
///
/// Renvoie le verbe, et le texte a utiliser quand il a ete trouve ailleurs que dans
/// `text`. Deux glissements observes, tous deux fatals pour une passe:
///
/// - un synonyme du verbe: `create`, `complete`, `show`;
/// - le TEXTE de la tache pose dans `action`, sans `text` du tout. C'est le plus
///   frequent, il arrive des que le modele redige plusieurs taches d'affilee. Une
///   eclaireuse a ainsi perdu quatre passes sur `{"action": "Chercher Tama-Palace /
///   TamaTalk et forums"}`. L'intention ne fait aucun doute: c'est un ajout.
fn normaliser_action<'a>(brut: &'a str, args: &'a serde_json::Value) -> (&'a str, Option<&'a str>) {
    let verbe = brut.trim().to_lowercase();
    match verbe.as_str() {
        "add" | "create" | "new" | "ajouter" | "ajoute" => ("add", None),
        "done" | "complete" | "completed" | "finish" | "check" | "terminer" | "fait" => {
            ("done", None)
        }
        "list" | "ls" | "show" | "all" | "lister" => ("list", None),
        _ => {
            // Ni verbe connu ni synonyme. Si aucun `text` n'accompagne, c'est que le
            // texte EST la; sinon on laisse remonter l'erreur, qui reste informative.
            if args["text"].as_str().map(str::trim).unwrap_or("").is_empty()
                && brut.trim().chars().count() > 2
            {
                ("add", Some(brut.trim()))
            } else {
                (brut, None)
            }
        }
    }
}

#[async_trait]
impl Abeille for Todo {
    fn nom(&self) -> &str {
        "todo"
    }

    fn description(&self) -> &str {
        "Manage an in-memory task list for the current session. Actions: add a task, mark one done, list all."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["add", "done", "list"], "description": "Action to perform" },
                "text": { "type": "string", "description": "Task text (required for action=add)" },
                "id": { "type": "integer", "description": "Task ID (required for action=done)" }
            },
            "required": ["action"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let brut = args["action"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' argument"))?;
        let (action, texte_implicite) = normaliser_action(brut, &args);
        match action {
            "add" => {
                let text = texte_implicite
                    .or_else(|| args["text"].as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'text' for todo add"))?
                    .trim();
                if text.is_empty() {
                    return Ok(ResultatAbeille::err("Task text is empty"));
                }
                let mut guard = store().lock().unwrap();
                let id = guard.next_id;
                guard.next_id += 1;
                guard.items.push(TodoItem {
                    id,
                    text: text.to_string(),
                    done: false,
                });
                Ok(ResultatAbeille::ok(format!("Task added #{id}: {text}")))
            }
            "done" => {
                let id = args["id"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'id' for todo done"))?;
                let mut guard = store().lock().unwrap();
                if let Some(item) = guard.items.iter_mut().find(|item| item.id == id) {
                    item.done = true;
                    Ok(ResultatAbeille::ok(format!(
                        "Task done #{id}: {}",
                        item.text
                    )))
                } else {
                    Ok(ResultatAbeille::err(format!("Unknown task: #{id}")))
                }
            }
            "list" => {
                let guard = store().lock().unwrap();
                if guard.items.is_empty() {
                    return Ok(ResultatAbeille::ok("No tasks."));
                }
                let lines = guard
                    .items
                    .iter()
                    .map(|item| {
                        format!(
                            "- [{}] #{} {}",
                            if item.done { "x" } else { " " },
                            item.id,
                            item.text
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ResultatAbeille::ok(lines))
            }
            other => Ok(ResultatAbeille::err(format!(
                "Unknown todo action: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::normaliser_action;
    use serde_json::json;

    #[test]
    fn les_verbes_et_leurs_synonymes() {
        let vide = json!({});
        for (ecrit, attendu) in [
            ("add", "add"), ("create", "add"), ("Ajouter", "add"),
            ("done", "done"), ("complete", "done"), ("CHECK", "done"),
            ("list", "list"), ("ls", "list"), ("show", "list"),
        ] {
            let (a, t) = normaliser_action(ecrit, &vide);
            assert_eq!(a, attendu, "verbe {ecrit}");
            assert!(t.is_none(), "un verbe n'apporte pas de texte");
        }
    }

    /// La regression exacte, vue quatre fois de suite sur une meme eclaireuse.
    #[test]
    fn le_texte_pose_dans_action_devient_un_ajout() {
        let sans_texte = json!({});
        let (a, t) = normaliser_action("Chercher Tama-Palace / TamaTalk et forums", &sans_texte);
        assert_eq!(a, "add");
        assert_eq!(t, Some("Chercher Tama-Palace / TamaTalk et forums"));
    }

    /// Mais si un `text` accompagne, on ne devine rien: l'erreur reste informative.
    #[test]
    fn avec_un_texte_l_action_inconnue_reste_une_erreur() {
        let avec_texte = json!({ "text": "la vraie tache" });
        let (a, t) = normaliser_action("supprimer", &avec_texte);
        assert_eq!(a, "supprimer");
        assert!(t.is_none());
    }
}
