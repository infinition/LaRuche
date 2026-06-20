use crate::abeille::AbeilleRegistry;
use crate::brain::{boucle_react, ChatEvent, EssaimConfig};
use crate::session::Session;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentResult {
    pub task: String,
    pub summary: String,
    pub iterations_limit: usize,
}

pub fn config_sous_agent(parent: &EssaimConfig) -> EssaimConfig {
    let mut config = parent.clone();
    config.max_iterations = config.max_iterations.min(8).max(1);
    for tool in ["delegate", "run_script", "mixture_of_agents"] {
        if !config.disabled_tools.iter().any(|name| name == tool) {
            config.disabled_tools.push(tool.to_string());
        }
    }
    config
}

pub async fn lancer_sous_agent(
    task: &str,
    context: Option<&str>,
    registry: Arc<AbeilleRegistry>,
    config: &EssaimConfig,
) -> Result<SubagentResult> {
    let full_prompt = match context.map(str::trim).filter(|ctx| !ctx.is_empty()) {
        Some(ctx) => format!("{task}\n\nContexte parent:\n{ctx}"),
        None => task.to_string(),
    };
    let sub_config = config_sous_agent(config);
    let mut session = Session::new(&sub_config.model);
    let (tx, _rx) = broadcast::channel::<ChatEvent>(64);
    let response = boucle_react(&full_prompt, &mut session, &registry, &sub_config, &tx).await?;

    Ok(SubagentResult {
        task: task.to_string(),
        summary: response,
        iterations_limit: sub_config.max_iterations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_sous_agent_limite_iterations_et_recursion() {
        let mut parent = EssaimConfig::default();
        parent.max_iterations = 20;

        let child = config_sous_agent(&parent);

        assert_eq!(child.max_iterations, 8);
        assert!(child.disabled_tools.contains(&"delegate".to_string()));
        assert!(child.disabled_tools.contains(&"run_script".to_string()));
        assert!(child
            .disabled_tools
            .contains(&"mixture_of_agents".to_string()));
    }
}
