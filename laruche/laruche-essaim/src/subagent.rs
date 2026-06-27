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

/// Rôle d'un agent spécialisé — chaque rôle a son propre system prompt
/// et son propre jeu d'outils autorisés.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    /// Recherche web approfondie, mémoire, hypothèses falsifiables
    Recherche,
    /// Code Python, calculs, data analysis
    Experimentation,
    /// Validation, fact-check, peer review
    Critique,
    /// Rédaction de rapports structurés
    Synthese,
    /// Classifieur binaire ultra-rapide (filtre watchers/crons)
    Dispatcher,
}

impl AgentRole {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "research" | "recherche" => Self::Recherche,
            "experiment" | "experimentation" | "code" => Self::Experimentation,
            "critique" | "review" | "validation" => Self::Critique,
            "synthesis" | "synthese" | "report" => Self::Synthese,
            "dispatcher" | "filter" | "classifier" => Self::Dispatcher,
            _ => Self::Recherche,
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            AgentRole::Recherche => {
                "Tu es un chercheur rigoureux. \
                 Tu formules une hypothèse falsifiable avant chaque recherche. \
                 Tu ne conclus jamais sans 3 sources minimum. \
                 Tu stockes chaque fait découvert en mémoire immédiatement \
                 via memory_write(node_id='research.<sujet>')."
            }
            AgentRole::Experimentation => {
                "Tu es un data scientist. Tu écris du code Python propre. \
                 Tu analyses chaque erreur et retry intelligemment. \
                 Tu valides tes résultats avant de conclure."
            }
            AgentRole::Critique => {
                "Tu es un pair-reviewer sceptique. \
                 Tu challenges chaque affirmation. \
                 Tu identifies les biais, lacunes et erreurs. \
                 Tu demandes des preuves supplémentaires si insuffisant. \
                 Utilise web_deep_search pour vérifier les faits."
            }
            AgentRole::Synthese => {
                "Tu structures et rédiges des rapports clairs. \
                 Tu cites tes sources. \
                 Tu es concis et précis. \
                 Génère d'abord la structure via <plan>[...]</plan>, \
                 puis remplis chaque section."
            }
            AgentRole::Dispatcher => {
                "Tu es un classifieur. Réponds UNIQUEMENT par un score JSON \
                 {\"score\": 0.0-1.0} indiquant si l'événement mérite \
                 une analyse approfondie. Aucun autre texte."
            }
        }
    }
}

/// Configuration de provider LLM prête à l'emploi pour brancher
/// différents providers sur différents rôles d'agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub api_base: Option<String>,
}

impl ProviderConfig {
    pub fn deepseek_api() -> Self {
        Self {
            provider: "openai".into(),
            model: "deepseek-chat".into(),
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            api_base: Some("https://api.deepseek.com/v1".into()),
        }
    }

    pub fn gemma_local() -> Self {
        Self {
            provider: "ollama".into(),
            model: "gemma4:e4b".into(),
            api_key: String::new(),
            api_base: None,
        }
    }

    pub fn apply_to(self, config: &mut EssaimConfig) {
        config.provider = self.provider;
        config.model = self.model;
        config.api_key = self.api_key;
        config.api_base = self.api_base;
    }
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

/// Génère une config spécialisée par rôle d'agent.
/// Chaque rôle a son propre system prompt, ses limites d'itérations,
/// et ses outils autorisés/restreints.
pub fn config_agent_specialise(
    parent: &EssaimConfig,
    role: AgentRole,
    provider_override: Option<ProviderConfig>,
) -> EssaimConfig {
    let mut cfg = config_sous_agent(parent);

    // Override provider si spécifié
    if let Some(p) = provider_override {
        cfg.provider = p.provider;
        cfg.model = p.model;
        cfg.api_key = p.api_key;
        cfg.api_base = p.api_base;
    }

    // Tools désactivés par rôle (en plus des anti-récursion de base)
    let (max_iter, max_tokens, disabled): (usize, u32, &[&str]) = match role {
        AgentRole::Recherche => (
            25,
            4096,
            &[
                "execute_code",
                "run_script",
                "file_write",
                "shell_exec",
                "delegate",
                "cron_create",
                "task_complete",
            ],
        ),
        AgentRole::Experimentation => (
            15,
            8192,
            &[
                "web_deep_search",
                "web_fetch",
                "delegate",
                "cron_create",
                "watcher_create",
            ],
        ),
        AgentRole::Critique => (
            8,
            2048,
            &[
                "execute_code",
                "run_script",
                "file_write",
                "delegate",
                "cron_create",
                "task_complete",
            ],
        ),
        AgentRole::Synthese => (
            5,
            4096,
            &[
                "execute_code",
                "run_script",
                "shell_exec",
                "delegate",
                "web_deep_search",
            ],
        ),
        AgentRole::Dispatcher => (
            1,
            64,
            &[
                "execute_code",
                "run_script",
                "shell_exec",
                "delegate",
                "web_deep_search",
                "web_fetch",
                "file_read",
                "file_write",
                "file_edit",
                "file_list",
                "cron_create",
                "watcher_create",
            ],
        ),
    };

    cfg.max_iterations = max_iter;
    cfg.max_tokens = max_tokens;
    cfg.system_prompt_override = Some(role.system_prompt().into());

    for tool in disabled {
        if !cfg.disabled_tools.iter().any(|name| name == tool) {
            cfg.disabled_tools.push(tool.to_string());
        }
    }

    cfg
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

/// Dispatcher léger — appel LLM minimal pour une décision binaire.
/// Utilise toujours le modèle le plus petit disponible pour filtrer
/// les événements avant de lancer un agent coûteux.
pub async fn dispatcher_pertinent(
    event_description: &str,
    config: &EssaimConfig,
) -> f32 {
    let dispatcher_cfg = EssaimConfig {
        model: "gemma4:e4b".into(),
        provider: "ollama".into(),
        max_tokens: 64,
        ..config.clone()
    };

    let prompt = format!(
        "{}\n\nScore de pertinence 0.0-1.0 (JSON seul) : {{\"score\": ?}}",
        event_description
    );

    let response = match crate::providers::provider_chat_stream(
        &dispatcher_cfg.provider,
        &dispatcher_cfg.model,
        &[serde_json::json!({"role": "user", "content": prompt})],
        0.0,
        64,
        &dispatcher_cfg.api_key,
        dispatcher_cfg.api_base.as_deref(),
        &dispatcher_cfg.ollama_url,
        None,
    )
    .await
    {
        Ok(mut stream) => {
            let mut out = String::new();
            use futures_util::StreamExt;
            while let Some(chunk) = stream.next().await {
                out.push_str(&chunk.text);
            }
            out
        }
        Err(e) => {
            tracing::warn!(error = %e, "Dispatcher LLM call failed");
            String::new()
        }
    };

    serde_json::from_str::<serde_json::Value>(&response)
        .ok()
        .and_then(|v| v["score"].as_f64())
        .unwrap_or(0.0) as f32
}

/// Cascade de providers : essaie d'abord le plus cheap, escalade si insuffisant.
pub async fn cascade_providers(
    task: &str,
    registry: Arc<AbeilleRegistry>,
    base_config: &EssaimConfig,
    providers: &[ProviderConfig],
) -> Result<String> {
    let mut last_error = String::new();

    for (level, provider) in providers.iter().enumerate() {
        let mut cfg = base_config.clone();
        cfg.provider = provider.provider.clone();
        cfg.model = provider.model.clone();
        cfg.api_key = provider.api_key.clone();
        cfg.api_base = provider.api_base.clone();

        tracing::info!(
            level,
            provider = %cfg.provider,
            model = %cfg.model,
            "Cascade provider — tentative"
        );

        let mut session = Session::new(&cfg.model);
        let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

        match boucle_react(task, &mut session, &registry, &cfg, &tx).await {
            Ok(result) => {
                if level == providers.len().saturating_sub(1) {
                    return Ok(result);
                }
                let quality = evaluer_qualite_reponse(&result).await;
                if quality > 0.75 {
                    return Ok(result);
                }
                tracing::info!(level, quality, "Qualité insuffisante, escalade");
                last_error = format!("Qualité ({quality:.2}) sous le seuil");
            }
            Err(e) => {
                tracing::warn!(level, error = %e, "Provider échoué dans cascade");
                last_error = e.to_string();
            }
        }
    }
    Err(anyhow::anyhow!(
        "Tous les providers de la cascade ont échoué : {last_error}"
    ))
}

/// Évaluation rapide de la qualité d'une réponse (0.0 - 1.0).
async fn evaluer_qualite_reponse(response: &str) -> f32 {
    if response.is_empty() {
        return 0.0;
    }
    let mut score = 0.5f32;
    if response.len() > 100 { score += 0.1; }
    if response.contains('-') || response.contains('*') || response.contains("1.") {
        score += 0.1;
    }
    if response.contains("```") { score += 0.1; }
    if response.contains('\u{2705}') || response.contains("terminé") { score += 0.1; }
    if response.len() < 20 { score -= 0.3; }
    if response.contains("je ne peux pas") || response.contains("désolé") { score -= 0.2; }
    score.clamp(0.0, 1.0)
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

    #[test]
    fn config_specialise_recherche_a_bonnes_contraintes() {
        let parent = EssaimConfig::default();
        let cfg = config_agent_specialise(&parent, AgentRole::Recherche, None);

        assert_eq!(cfg.max_iterations, 25);
        assert!(!cfg.disabled_tools.contains(&"web_deep_search".to_string()));
        assert!(cfg.disabled_tools.contains(&"execute_code".to_string()));
        assert!(cfg.system_prompt_override.is_some());
        assert!(cfg.system_prompt_override.unwrap().contains("chercheur"));
    }

    #[test]
    fn config_specialise_critique_a_peu_diterations() {
        let parent = EssaimConfig::default();
        let cfg = config_agent_specialise(&parent, AgentRole::Critique, None);

        assert_eq!(cfg.max_iterations, 8);
        assert!(cfg.disabled_tools.contains(&"file_write".to_string()));
    }

    #[test]
    fn agent_role_from_str() {
        assert_eq!(AgentRole::from_str("research"), AgentRole::Recherche);
        assert_eq!(AgentRole::from_str("code"), AgentRole::Experimentation);
        assert_eq!(AgentRole::from_str("review"), AgentRole::Critique);
        assert_eq!(AgentRole::from_str("report"), AgentRole::Synthese);
        assert_eq!(AgentRole::from_str("filter"), AgentRole::Dispatcher);
        assert_eq!(AgentRole::from_str("inconnu"), AgentRole::Recherche);
    }
}
