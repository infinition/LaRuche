use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::brain::EssaimConfig;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{future::join_all, StreamExt};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MixtureOfAgents {
    pub config: EssaimConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct MixtureArgs {
    prompt: String,
    #[serde(default)]
    candidates: Vec<CandidateArgs>,
}

#[derive(Debug, Clone, Deserialize)]
struct CandidateArgs {
    model: String,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    api_base: Option<String>,
}

#[async_trait]
impl Abeille for MixtureOfAgents {
    fn nom(&self) -> &str {
        "mixture_of_agents"
    }

    fn description(&self) -> &str {
        "Query multiple models in parallel on a prompt, then synthesize their responses. Use to cross-check reasoning or get a robust answer."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt or task to send to all candidate models"
                },
                "candidates": {
                    "type": "array",
                    "description": "Candidate models as {model, provider?, api_base?}. Empty = use primary model + fallback_models.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "model": { "type": "string" },
                            "provider": { "type": "string" },
                            "api_base": { "type": "string" }
                        },
                        "required": ["model"]
                    }
                }
            },
            "required": ["prompt"]
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
        let args: MixtureArgs = serde_json::from_value(args)?;
        let candidates = candidats_ou_defaut(&self.config, args.candidates);
        if candidates.is_empty() {
            return Ok(ResultatAbeille::err("No candidate models available."));
        }

        let futures = candidates
            .iter()
            .map(|candidate| interroger_candidat(&self.config, candidate, &args.prompt));
        let responses = join_all(futures).await;
        let mut blocs = Vec::new();
        for (candidate, response) in candidates.iter().zip(responses) {
            match response {
                Ok(text) if !text.trim().is_empty() => {
                    blocs.push((candidate.model.clone(), text.trim().to_string()));
                }
                Ok(_) => blocs.push((candidate.model.clone(), "[empty response]".to_string())),
                Err(e) => blocs.push((candidate.model.clone(), format!("[error: {e}]"))),
            }
        }

        let synthese = synthetiser(&self.config, &args.prompt, &blocs)
            .await
            .unwrap_or_else(|| synthese_extractive(&args.prompt, &blocs));
        Ok(ResultatAbeille::ok(synthese))
    }
}

fn candidats_ou_defaut(
    config: &EssaimConfig,
    mut candidates: Vec<CandidateArgs>,
) -> Vec<CandidateArgs> {
    if candidates.is_empty() {
        candidates.push(CandidateArgs {
            model: config.model.clone(),
            provider: None,
            api_base: None,
        });
        candidates.extend(
            config
                .fallback_models
                .iter()
                .cloned()
                .map(|model| CandidateArgs {
                    model,
                    provider: None,
                    api_base: None,
                }),
        );
    }
    candidates
}

async fn interroger_candidat(
    config: &EssaimConfig,
    candidate: &CandidateArgs,
    prompt: &str,
) -> Result<String> {
    let provider = candidate.provider.as_deref().unwrap_or(&config.provider);
    let api_base = candidate.api_base.as_deref().or(config.api_base.as_deref());
    let messages = vec![serde_json::json!({
        "role": "user",
        "content": prompt
    })];
    // MoA role effort: an ADVISOR is where thinking pays off - its job is to explore
    // an angle in depth. It gets the main effort setting.
    let mut stream = crate::providers::provider_chat_stream_effort(
        provider,
        &candidate.model,
        &messages,
        config.temperature,
        config.max_tokens,
        &config.api_key,
        api_base,
        &config.ollama_url,
        None,
        Some(config.reasoning_effort.as_str()).filter(|e| !e.is_empty()),
    )
    .await?;
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        out.push_str(&chunk.text);
    }
    Ok(out)
}

async fn synthetiser(
    config: &EssaimConfig,
    prompt: &str,
    blocs: &[(String, String)],
) -> Option<String> {
    let model = config.aux_model.as_deref().unwrap_or(&config.model);
    let joined = blocs
        .iter()
        .map(|(model, text)| format!("## {model}\n{text}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "You synthesize multiple model responses. Keep reliable points, flag meaningful divergences, and answer clearly."
        }),
        serde_json::json!({
            "role": "user",
            "content": format!("Question:\n{prompt}\n\nCandidate responses:\n{joined}\n\nSynthesis:")
        }),
    ];
    // The SYNTHESIZER answers fast: the thinking already happened in the advisors,
    // its job is to merge them. Auxiliary effort (empty by default = no thinking).
    let stream_result = tokio::time::timeout(
        Duration::from_secs(60),
        crate::providers::provider_chat_stream_effort(
            &config.provider,
            model,
            &messages,
            0.2,
            config.max_tokens.min(2048),
            &config.api_key,
            config.api_base.as_deref(),
            &config.ollama_url,
            None,
            Some(config.reasoning_effort_aux.as_str()).filter(|e| !e.is_empty()),
        ),
    )
    .await
    .ok()?
    .ok()?;
    let mut stream = stream_result;
    let mut out = String::new();
    while let Ok(Some(chunk)) = tokio::time::timeout(Duration::from_secs(60), stream.next()).await {
        out.push_str(&chunk.text);
    }
    let trimmed = out.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(crate) fn synthese_extractive(prompt: &str, blocs: &[(String, String)]) -> String {
    let mut out = format!("Multi-model synthesis for: {prompt}\n\n");
    for (model, text) in blocs {
        out.push_str(&format!("## {model}\n{}\n\n", text.trim()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidats_defaut_inclut_modele_principal_et_fallbacks() {
        let mut config = EssaimConfig::default();
        config.model = "main".into();
        config.fallback_models = vec!["fb1".into(), "fb2".into()];

        let candidates = candidats_ou_defaut(&config, vec![]);

        assert_eq!(
            candidates
                .iter()
                .map(|c| c.model.as_str())
                .collect::<Vec<_>>(),
            vec!["main", "fb1", "fb2"]
        );
    }

    #[test]
    fn synthese_extractive_garde_les_reponses() {
        let text = synthese_extractive(
            "choisir",
            &[
                ("a".into(), "reponse A".into()),
                ("b".into(), "reponse B".into()),
            ],
        );

        assert!(text.contains("choisir"));
        assert!(text.contains("reponse A"));
        assert!(text.contains("reponse B"));
    }
}
