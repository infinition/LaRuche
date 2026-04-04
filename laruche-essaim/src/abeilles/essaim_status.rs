use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

/// Get basic system information.
pub struct SystemInfo;

#[async_trait]
impl Abeille for SystemInfo {
    fn nom(&self) -> &str {
        "system_info"
    }

    fn description(&self) -> &str {
        "Get basic system information: OS, hostname, current directory, and current time."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
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
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let info = format!(
            "OS: {}\nHostname: {}\nCurrent directory: {}\nTime: {}",
            std::env::consts::OS,
            hostname,
            cwd,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        );

        Ok(ResultatAbeille::ok(info))
    }
}
