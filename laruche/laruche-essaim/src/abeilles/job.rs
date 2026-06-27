//! Outils pour la gestion de jobs longs en arrière-plan.
//!
//! - `submit_job` : lance un script shell en background, retourne un job_id.
//! - `check_job_status` : vérifie le statut d'un job par son job_id.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::job_queue::{JobQueue, JobStatus};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

// ─── SubmitJob ────────────────────────────────────────────────────────────────

/// Lance un script en arrière-plan et retourne un `job_id` pour le suivre.
pub struct SubmitJob {
    pub queue: Arc<JobQueue>,
}

#[async_trait]
impl Abeille for SubmitJob {
    fn nom(&self) -> &str {
        "submit_job"
    }

    fn description(&self) -> &str {
        "Soumet un script shell à exécuter en arrière-plan. \
         Retourne un `job_id` que tu peux utiliser avec `check_job_status`. \
         Utilise pour les scripts longs (> 30s) qui ne nécessitent pas de \
         réponse immédiate."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "script": {
                    "type": "string",
                    "description": "The shell script to execute"
                },
                "label": {
                    "type": "string",
                    "description": "Optional label to identify the job"
                }
            },
            "required": ["script"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let script = args["script"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Argument 'script' manquant"))?;
        let label = args["label"].as_str();

        let job_id = self.queue.submit(script, label).await;

        tracing::info!(job_id = %job_id, script_len = script.len(), "Job soumis en arrière-plan");

        Ok(ResultatAbeille::ok(format!(
            "Job soumis ! job_id: {job_id}\nUtilise `check_job_status` avec ce job_id pour vérifier l'avancement.",
        )))
    }
}

// ─── CheckJobStatus ───────────────────────────────────────────────────────────

/// Vérifie le statut d'un job soumis via `submit_job`.
pub struct CheckJobStatus {
    pub queue: Arc<JobQueue>,
}

#[async_trait]
impl Abeille for CheckJobStatus {
    fn nom(&self) -> &str {
        "check_job_status"
    }

    fn description(&self) -> &str {
        "Vérifie le statut d'un job soumis avec `submit_job`. \
         Retourne 'En cours', 'Terminé' ou 'Échoué' avec les détails."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "job_id": {
                    "type": "string",
                    "description": "The job_id returned by submit_job"
                }
            },
            "required": ["job_id"]
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
        let job_id = args["job_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Argument 'job_id' manquant"))?;

        match self.queue.check(job_id).await {
            Some(JobStatus::Running { started, progress }) => {
                let elapsed = started.elapsed().as_secs();
                let pct = progress.map(|p| format!(" ({:.0}%)", p * 100.0)).unwrap_or_default();
                Ok(ResultatAbeille::ok(format!(
                    "Job {job_id} : EN COURS depuis {elapsed}s{pct}. Vérifie à nouveau dans 30s."
                )))
            }
            Some(JobStatus::Completed { output, elapsed }) => {
                let truncated = if output.len() > 2000 {
                    format!("{}... [tronqué, {} chars]", &output[..2000], output.len())
                } else {
                    output.clone()
                };
                Ok(ResultatAbeille::ok(format!(
                    "Job {job_id} : TERMINÉ en {elapsed:.0?}\n{}",
                    truncated
                )))
            }
            Some(JobStatus::Failed { error, elapsed }) => {
                Ok(ResultatAbeille::ok(format!(
                    "Job {job_id} : ÉCHOUÉ après {elapsed:.0?}\nErreur: {error}"
                )))
            }
            None => Ok(ResultatAbeille::ok(format!(
                "Job {job_id} : INCONNU (pas encore soumis ou déjà nettoyé)."
            ))),
        }
    }
}
