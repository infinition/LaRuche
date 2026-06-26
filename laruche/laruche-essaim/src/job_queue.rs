//! File d'attente de jobs longs pour l'agent.
//!
//! Permet à l'agent de lancer des scripts longs en background
//! et de revenir vérifier leur statut plus tard (pattern polling).
//!
//! # Usage
//! 1. Agent appelle `submit_job` avec un script → reçoit un `job_id`
//! 2. Agent continue son raisonnement (autres outils, réflexion...)
//! 3. Agent appelle `check_job_status(job_id)` pour voir l'avancement
//! 4. Quand le job est terminé, agent récupère le résultat

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Statut d'un job en arrière-plan.
#[derive(Debug, Clone)]
pub enum JobStatus {
    /// En cours d'exécution
    Running {
        started: Instant,
        progress: Option<f32>,
    },
    /// Terminé avec succès
    Completed {
        output: String,
        elapsed: std::time::Duration,
    },
    /// Échec
    Failed {
        error: String,
        elapsed: std::time::Duration,
    },
}

/// Gestionnaire de jobs longs (thread-safe, partagé entre outils).
#[derive(Debug, Clone)]
pub struct JobQueue {
    jobs: Arc<RwLock<HashMap<String, JobStatus>>>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Soumet un script shell en arrière-plan.
    /// Retourne un `job_id` que l'agent pourra utiliser pour checker le statut.
    pub async fn submit(&self, script: &str, label: Option<&str>) -> String {
        let job_id = format!(
            "job_{}_{}",
            label.unwrap_or("script"),
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("x")
        );
        let jobs = self.jobs.clone();
        let id = job_id.clone();
        let script = script.to_string();

        // Écrire "Running" dans la HashMap
        {
            let mut w = jobs.write().await;
            w.insert(
                id.clone(),
                JobStatus::Running {
                    started: Instant::now(),
                    progress: None,
                },
            );
        }

        // Lancer en background
        tokio::spawn(async move {
            let start = Instant::now();

            // Exécution via tokio::process::Command
            let mut command = if cfg!(windows) {
                let mut cmd = tokio::process::Command::new("cmd");
                cmd.arg("/C").arg(&script);
                cmd
            } else {
                let mut cmd = tokio::process::Command::new("sh");
                cmd.arg("-c").arg(&script);
                cmd
            };
            let output = command.output().await;

            let mut w = jobs.write().await;
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let combined = if stderr.is_empty() {
                        stdout
                    } else {
                        format!("[stdout]\n{stdout}\n[stderr]\n{stderr}")
                    };
                    if out.status.success() {
                        w.insert(
                            id.clone(),
                            JobStatus::Completed {
                                output: combined,
                                elapsed: start.elapsed(),
                            },
                        );
                    } else {
                        w.insert(
                            id.clone(),
                            JobStatus::Failed {
                                error: format!(
                                    "Exit {}: {}",
                                    out.status.code().unwrap_or(-1),
                                    combined.chars().take(500).collect::<String>()
                                ),
                                elapsed: start.elapsed(),
                            },
                        );
                    }
                }
                Err(e) => {
                    w.insert(
                        id.clone(),
                        JobStatus::Failed {
                            error: e.to_string(),
                            elapsed: start.elapsed(),
                        },
                    );
                }
            }
        });

        job_id
    }

    /// Vérifie le statut d'un job.
    pub async fn check(&self, job_id: &str) -> Option<JobStatus> {
        let r = self.jobs.read().await;
        r.get(job_id).cloned()
    }

    /// Nombre de jobs en cours.
    #[allow(dead_code)]
    pub async fn running_count(&self) -> usize {
        let r = self.jobs.read().await;
        r.values()
            .filter(|s| matches!(s, JobStatus::Running { .. }))
            .count()
    }

    /// Nettoie les jobs terminés de plus de 1h.
    #[allow(dead_code)]
    pub async fn nettoyer(&self) -> usize {
        let mut w = self.jobs.write().await;
        let _now = Instant::now();
        let stale: Vec<String> =
            w.iter()
                .filter_map(|(id, status)| {
                    let elapsed = match status {
                        JobStatus::Completed { elapsed, .. }
                        | JobStatus::Failed { elapsed, .. } => *elapsed,
                        _ => return None,
                    };
                    if elapsed > std::time::Duration::from_secs(3600) {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();
        let count = stale.len();
        for id in stale {
            w.remove(&id);
        }
        count
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, timeout, Duration};

    #[tokio::test]
    async fn submit_execute_une_commande_shell_portable() {
        let queue = JobQueue::new();
        let job_id = queue.submit("echo laruche_job_ok", Some("test")).await;

        let status = timeout(Duration::from_secs(5), async {
            loop {
                match queue.check(&job_id).await {
                    Some(JobStatus::Completed { output, .. }) => break output,
                    Some(JobStatus::Failed { error, .. }) => panic!("{error}"),
                    Some(JobStatus::Running { .. }) | None => {
                        sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        })
        .await
        .expect("job shell timeout");

        assert!(status.contains("laruche_job_ok"));
    }
}
