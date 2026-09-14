//! Durable background jobs using the same guarded shell executor as foreground tools.
use crate::abeille::{Abeille, ContextExecution};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum JobStatus {
    Running {
        started: Instant,
        progress: Option<f32>,
    },
    Completed {
        output: String,
        elapsed: Duration,
    },
    Failed {
        error: String,
        elapsed: Duration,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    id: String,
    owner: Option<String>,
    working_dir: PathBuf,
    state: String,
    output: String,
    started: i64,
    finished: Option<i64>,
    elapsed_ms: u64,
}

type Cancellations = Mutex<HashMap<String, (Option<String>, tokio::sync::watch::Sender<bool>)>>;
fn cancellations() -> &'static Cancellations {
    static ALL: OnceLock<Cancellations> = OnceLock::new();
    ALL.get_or_init(Default::default)
}
pub fn cancel_owner(owner: &str) {
    if let Ok(all) = cancellations().lock() {
        for (run, sender) in all.values() {
            if run.as_deref() == Some(owner) {
                let _ = sender.send(true);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct JobQueue {
    jobs: Arc<RwLock<HashMap<String, Record>>>,
    root: Arc<PathBuf>,
}
impl JobQueue {
    pub fn new() -> Self {
        Self::with_root(PathBuf::from("sessions/jobs"))
    }
    pub fn with_root(root: PathBuf) -> Self {
        let mut jobs = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                if entry.path().extension().is_none_or(|e| e != "json") {
                    continue;
                }
                if let Ok(raw) = std::fs::read_to_string(entry.path()) {
                    if let Ok(mut record) = serde_json::from_str::<Record>(&raw) {
                        if record.state == "running" {
                            record.state = "failed".into();
                            record.output = "Process interrupted or owner restarted; external outcome unknown. Do not replay blindly.".into();
                        }
                        jobs.insert(record.id.clone(), record);
                    }
                }
            }
        }
        Self {
            jobs: Arc::new(RwLock::new(jobs)),
            root: Arc::new(root),
        }
    }
    fn persist(&self, record: &Record) -> anyhow::Result<()> {
        use std::io::Write;
        std::fs::create_dir_all(self.root.as_path())?;
        let path = self.root.join(format!("{}.json", record.id));
        let tmp = path.with_extension("tmp");
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(serde_json::to_string(record)?.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
    pub async fn submit_in_context(
        &self,
        script: &str,
        _label: Option<&str>,
        ctx: &ContextExecution,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let record = Record {
            id: id.clone(),
            owner: ctx.run_id.clone(),
            working_dir: ctx.working_dir.clone(),
            state: "running".into(),
            output: String::new(),
            started: chrono::Utc::now().timestamp(),
            finished: None,
            elapsed_ms: 0,
        };
        let mut jobs = self.jobs.write().await;
        anyhow::ensure!(
            jobs.values().filter(|j| j.state == "running").count() < 4,
            "Background job capacity reached (4)"
        );
        self.persist(&record)?;
        jobs.insert(id.clone(), record.clone());
        drop(jobs);
        let (cancel, mut rx) = tokio::sync::watch::channel(false);
        cancellations()
            .lock()
            .map_err(|_| anyhow::anyhow!("Cancellation registry unavailable"))?
            .insert(id.clone(), (ctx.run_id.clone(), cancel));
        let queue = self.clone();
        let context = ctx.clone();
        let script = script.to_string();
        tokio::spawn(async move {
            let started = Instant::now();
            let future = crate::abeilles::shell::ShellExec.executer(
                serde_json::json!({"command":script, "timeout_secs":3600}),
                &context,
            );
            tokio::pin!(future);
            let mut record = record;
            match tokio::select! {
                result = &mut future => Some(result),
                _ = rx.changed() => None,
            } {
                Some(Ok(result)) if result.success => {
                    record.state = "completed".into();
                    record.output = result.output;
                }
                Some(Ok(result)) => {
                    record.state = "failed".into();
                    record.output = result.error.unwrap_or(result.output);
                }
                Some(Err(e)) => {
                    record.state = "failed".into();
                    record.output = e.to_string();
                }
                None => {
                    record.state = "failed".into();
                    record.output =
                        "Cancelled; reconcile any external effects before retrying.".into();
                }
            }
            record.finished = Some(chrono::Utc::now().timestamp());
            record.elapsed_ms = started.elapsed().as_millis() as u64;
            if let Err(e) = queue.persist(&record) {
                record.state = "failed".into();
                record.output = format!("Result could not be persisted, outcome unknown: {e}");
            }
            if let Ok(mut all) = cancellations().lock() {
                all.remove(&record.id);
            }
            queue.jobs.write().await.insert(record.id.clone(), record);
        });
        Ok(id)
    }
    pub async fn cancel(&self, id: &str, owner: Option<&str>) -> bool {
        let jobs = self.jobs.read().await;
        if !jobs.get(id).is_some_and(|r| r.owner.as_deref() == owner) {
            return false;
        }
        cancellations()
            .lock()
            .ok()
            .and_then(|all| all.get(id).map(|(_, sender)| sender.send(true).is_ok()))
            .unwrap_or(false)
    }
    pub async fn check(&self, id: &str) -> Option<JobStatus> {
        let jobs = self.jobs.read().await;
        let r = jobs.get(id)?;
        let elapsed = Duration::from_millis(r.elapsed_ms);
        Some(match r.state.as_str() {
            "running" => JobStatus::Running {
                started: Instant::now()
                    .checked_sub(Duration::from_secs(
                        (chrono::Utc::now().timestamp() - r.started).max(0) as u64,
                    ))
                    .unwrap_or_else(Instant::now),
                progress: None,
            },
            "completed" => JobStatus::Completed {
                output: r.output.clone(),
                elapsed,
            },
            _ => JobStatus::Failed {
                error: r.output.clone(),
                elapsed,
            },
        })
    }
    pub async fn running_count(&self) -> usize {
        self.jobs
            .read()
            .await
            .values()
            .filter(|r| r.state == "running")
            .count()
    }
    pub async fn nettoyer(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let mut jobs = self.jobs.write().await;
        let stale: Vec<_> = jobs
            .values()
            .filter(|r| r.finished.is_some_and(|t| now - t > 3600))
            .map(|r| r.id.clone())
            .collect();
        for id in &stale {
            jobs.remove(id);
            let _ = std::fs::remove_file(self.root.join(format!("{id}.json")));
        }
        stale.len()
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
    #[tokio::test]
    async fn job_uses_working_directory_and_persists_result() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&root).unwrap();
        let queue = JobQueue::with_root(root.join("jobs"));
        let ctx = ContextExecution {
            working_dir: root.clone(),
            ..Default::default()
        };
        let id = queue
            .submit_in_context("echo laruche_job_ok", Some("test"), &ctx)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            while matches!(queue.check(&id).await, Some(JobStatus::Running { .. })) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert!(
            matches!(JobQueue::with_root(root.join("jobs")).check(&id).await, Some(JobStatus::Completed { output, .. }) if output.contains("laruche_job_ok"))
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
