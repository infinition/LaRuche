//! Kanban orchestration tools backed by the node's shared board.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use laruche_kanban::KanbanBoard;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct KanbanNext {
    pub kanban_board: Arc<RwLock<KanbanBoard>>,
}

#[async_trait]
impl Abeille for KanbanNext {
    fn nom(&self) -> &str {
        "kanban_next"
    }

    fn description(&self) -> &str {
        "Renvoie la prochaine tache Kanban executable : Ready en priorite, sinon une tache dont toutes les dependances sont terminees."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        _args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let board = self.kanban_board.read().await;
        match board.next_ready() {
            Some(task) => Ok(ResultatAbeille::ok(
                serde_json::to_string(&task).expect("KanbanTask is serializable"),
            )),
            None => Ok(ResultatAbeille::ok("Aucune tache Kanban executable.")),
        }
    }
}

pub struct KanbanComplete {
    pub kanban_board: Arc<RwLock<KanbanBoard>>,
}

#[async_trait]
impl Abeille for KanbanComplete {
    fn nom(&self) -> &str {
        "kanban_complete"
    }

    fn description(&self) -> &str {
        "Marque une tache Kanban terminee et enregistre son resultat. Les enfants bloques sont debloques automatiquement."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "UUID de la tache Kanban" },
                "result": { "type": "string", "description": "Resultat final de la tache" }
            },
            "required": ["id", "result"],
            "additionalProperties": false
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
        let id = args["id"]
            .as_str()
            .ok_or_else(|| anyhow!("'id' manquant"))?
            .parse::<Uuid>()
            .map_err(|_| anyhow!("'id' doit etre un UUID valide"))?;
        let result = args["result"]
            .as_str()
            .ok_or_else(|| anyhow!("'result' manquant"))?;

        let mut board = self.kanban_board.write().await;
        if board.complete(id, result.to_string()) {
            Ok(ResultatAbeille::ok(format!("Tache Kanban {id} terminee.")))
        } else {
            Ok(ResultatAbeille::err(format!(
                "Tache Kanban introuvable: {id}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laruche_kanban::TaskStatus;
    use tempfile::tempdir;

    fn board() -> Arc<RwLock<KanbanBoard>> {
        let path = tempdir().unwrap().keep().join("kanban.json");
        Arc::new(RwLock::new(KanbanBoard::new(&path)))
    }

    #[tokio::test]
    async fn next_returns_the_next_executable_task() {
        let kanban_board = board();
        let task = kanban_board
            .write()
            .await
            .create("Traiter".into(), "Description".into(), None, None, None);
        let tool = KanbanNext { kanban_board };

        let response = tool
            .executer(serde_json::json!({}), &ContextExecution::default())
            .await
            .unwrap();
        let returned: serde_json::Value = serde_json::from_str(&response.output).unwrap();
        assert_eq!(returned["id"], task.id.to_string());
    }

    #[tokio::test]
    async fn complete_marks_done_and_unblocks_children() {
        let kanban_board = board();
        let (parent, child) = {
            let mut board = kanban_board.write().await;
            let parent = board.create("Parent".into(), "".into(), None, None, None);
            let child = board.create("Child".into(), "".into(), None, None, None);
            assert!(board.add_dependency(child.id, parent.id));
            (parent, child)
        };
        let tool = KanbanComplete {
            kanban_board: kanban_board.clone(),
        };

        let response = tool
            .executer(
                serde_json::json!({ "id": parent.id.to_string(), "result": "Fait" }),
                &ContextExecution::default(),
            )
            .await
            .unwrap();

        assert!(response.success);
        let board = kanban_board.read().await;
        assert_eq!(board.get(&parent.id).unwrap().status, TaskStatus::Done);
        assert_eq!(
            board.get(&parent.id).unwrap().result.as_deref(),
            Some("Fait")
        );
        assert_eq!(board.get(&child.id).unwrap().status, TaskStatus::Ready);
    }
}
