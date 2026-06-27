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
        let action = args["action"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' argument"))?;
        match action {
            "add" => {
                let text = args["text"]
                    .as_str()
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
