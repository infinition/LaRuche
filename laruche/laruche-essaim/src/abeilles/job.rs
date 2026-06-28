//! Tools for managing long-running background jobs.
//!
//! - `submit_job`: launches a shell script in the background, returns a job_id.
//! - `check_job_status`: checks the status of a job by its job_id.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::job_queue::{JobQueue, JobStatus};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

// ─── SubmitJob ────────────────────────────────────────────────────────────────

/// Launches a script in the background and returns a `job_id` to track it.
pub struct SubmitJob {
    pub queue: Arc<JobQueue>,
}

#[async_trait]
impl Abeille for SubmitJob {
    fn nom(&self) -> &str {
        "submit_job"
    }

    fn description(&self) -> &str {
        "Submits a shell script to run in the background. \
         Returns a `job_id` you can use with `check_job_status`. \
         Use for long-running scripts (> 30s) that do not require an \
         immediate response."
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
            .ok_or_else(|| anyhow::anyhow!("Missing 'script' argument"))?;
        let label = args["label"].as_str();

        let job_id = self.queue.submit(script, label).await;

        tracing::info!(job_id = %job_id, script_len = script.len(), "Job submitted in background");

        Ok(ResultatAbeille::ok(format!(
            "Job submitted! job_id: {job_id}\nUse `check_job_status` with this job_id to check progress.",
        )))
    }
}

// ─── CheckJobStatus ───────────────────────────────────────────────────────────

/// Checks the status of a job submitted via `submit_job`.
pub struct CheckJobStatus {
    pub queue: Arc<JobQueue>,
}

#[async_trait]
impl Abeille for CheckJobStatus {
    fn nom(&self) -> &str {
        "check_job_status"
    }

    fn description(&self) -> &str {
        "Checks the status of a job submitted with `submit_job`. \
         Returns 'Running', 'Completed' or 'Failed' with details."
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
            .ok_or_else(|| anyhow::anyhow!("Missing 'job_id' argument"))?;

        match self.queue.check(job_id).await {
            Some(JobStatus::Running { started, progress }) => {
                let elapsed = started.elapsed().as_secs();
                let pct = progress.map(|p| format!(" ({:.0}%)", p * 100.0)).unwrap_or_default();
                Ok(ResultatAbeille::ok(format!(
                    "Job {job_id}: RUNNING for {elapsed}s{pct}. Check again in 30s."
                )))
            }
            Some(JobStatus::Completed { output, elapsed }) => {
                let truncated = if output.len() > 2000 {
                    format!("{}... [truncated, {} chars]", &output[..2000], output.len())
                } else {
                    output.clone()
                };
                Ok(ResultatAbeille::ok(format!(
                    "Job {job_id}: COMPLETED in {elapsed:.0?}\n{}",
                    truncated
                )))
            }
            Some(JobStatus::Failed { error, elapsed }) => {
                Ok(ResultatAbeille::ok(format!(
                    "Job {job_id}: FAILED after {elapsed:.0?}\nError: {error}"
                )))
            }
            None => Ok(ResultatAbeille::ok(format!(
                "Job {job_id}: UNKNOWN (not yet submitted or already cleaned up)."
            ))),
        }
    }
}
