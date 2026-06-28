//! Queue of long-running jobs for the agent.
//!
//! Lets the agent launch long scripts in the background
//! and come back later to check their status (polling pattern).
//!
//! # Usage
//! 1. Agent calls `submit_job` with a script, receives a `job_id`
//! 2. Agent continues its reasoning (other tools, thinking...)
//! 3. Agent calls `check_job_status(job_id)` to see progress
//! 4. When the job is done, the agent retrieves the result

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Status of a background job.
#[derive(Debug, Clone)]
pub enum JobStatus {
    /// Running
    Running {
        started: Instant,
        progress: Option<f32>,
    },
    /// Completed successfully
    Completed {
        output: String,
        elapsed: std::time::Duration,
    },
    /// Failed
    Failed {
        error: String,
        elapsed: std::time::Duration,
    },
}

/// Long-running job manager (thread-safe, shared across tools).
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

    /// Submits a shell script in the background.
    /// Returns a `job_id` the agent can use to check the status.
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

        // Write "Running" into the HashMap
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

        // Launch in the background
        tokio::spawn(async move {
            let start = Instant::now();

            // Execution via tokio::process::Command
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

    /// Checks a job's status.
    pub async fn check(&self, job_id: &str) -> Option<JobStatus> {
        let r = self.jobs.read().await;
        r.get(job_id).cloned()
    }

    /// Number of running jobs.
    #[allow(dead_code)]
    pub async fn running_count(&self) -> usize {
        let r = self.jobs.read().await;
        r.values()
            .filter(|s| matches!(s, JobStatus::Running { .. }))
            .count()
    }

    /// Cleans up finished jobs older than 1h.
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
        .expect("shell job timeout");

        assert!(status.contains("laruche_job_ok"));
    }
}
