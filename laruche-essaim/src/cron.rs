//! Simple task scheduler for the Essaim agent.
//!
//! Supports one-shot (fire once at a specific time) and recurring (cron expression) tasks.
//! Tasks are persisted to disk and survive restarts.

use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// A scheduled task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub name: String,
    pub prompt: String,
    /// Cron expression (5-field) for recurring tasks, or None for one-shot.
    pub cron_expr: Option<String>,
    /// One-shot fire time (ISO 8601). Task auto-disables after firing.
    pub fire_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u32,
}

/// The cron scheduler — manages tasks and dispatches them.
pub struct CronScheduler {
    tasks: HashMap<Uuid, ScheduledTask>,
    file_path: PathBuf,
}

impl CronScheduler {
    /// Create a new scheduler with persistence at the given path.
    pub fn new(file_path: &Path) -> Self {
        let mut scheduler = Self {
            tasks: HashMap::new(),
            file_path: file_path.to_path_buf(),
        };
        // Load existing tasks
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Ok(tasks) = serde_json::from_str::<Vec<ScheduledTask>>(&content) {
                    for task in tasks {
                        scheduler.tasks.insert(task.id, task);
                    }
                    tracing::info!(count = scheduler.tasks.len(), "Loaded scheduled tasks");
                }
            }
        }
        scheduler
    }

    /// Add a new task.
    pub fn add(&mut self, task: ScheduledTask) -> Uuid {
        let id = task.id;
        tracing::info!(id = %id, name = %task.name, "Scheduled task added");
        self.tasks.insert(id, task);
        let _ = self.save();
        id
    }

    /// Remove a task.
    pub fn remove(&mut self, id: &Uuid) -> bool {
        let removed = self.tasks.remove(id).is_some();
        if removed {
            let _ = self.save();
        }
        removed
    }

    /// List all tasks.
    pub fn list(&self) -> Vec<&ScheduledTask> {
        self.tasks.values().collect()
    }

    /// Enable/disable a task.
    pub fn set_enabled(&mut self, id: &Uuid, enabled: bool) -> bool {
        if let Some(task) = self.tasks.get_mut(id) {
            task.enabled = enabled;
            let _ = self.save();
            true
        } else {
            false
        }
    }

    /// Check which tasks are due to fire now. Returns their IDs and prompts.
    pub fn check_due_tasks(&mut self) -> Vec<(Uuid, String)> {
        let now = Utc::now();
        let mut due = Vec::new();

        for task in self.tasks.values_mut() {
            if !task.enabled {
                continue;
            }

            // One-shot tasks
            if let Some(fire_at) = task.fire_at {
                if now >= fire_at {
                    due.push((task.id, task.prompt.clone()));
                    task.last_run = Some(now);
                    task.run_count += 1;
                    task.enabled = false; // Auto-disable after firing
                    tracing::info!(id = %task.id, name = %task.name, "One-shot task fired");
                }
                continue;
            }

            // Recurring tasks with cron expression
            if let Some(ref expr) = task.cron_expr {
                if should_fire_cron(expr, task.last_run, now) {
                    due.push((task.id, task.prompt.clone()));
                    task.last_run = Some(now);
                    task.run_count += 1;
                    tracing::info!(id = %task.id, name = %task.name, run = task.run_count, "Cron task fired");
                }
            }
        }

        if !due.is_empty() {
            let _ = self.save();
        }

        due
    }

    /// Save tasks to disk.
    fn save(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tasks: Vec<&ScheduledTask> = self.tasks.values().collect();
        let json = serde_json::to_string_pretty(&tasks)?;
        std::fs::write(&self.file_path, json)?;
        Ok(())
    }
}

/// Simple cron check: does this cron expression match "now" and hasn't fired this minute?
fn should_fire_cron(expr: &str, last_run: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    let local_now = now.with_timezone(&Local);

    // Parse 5-field cron: minute hour dom month dow
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }

    let minute = local_now.format("%M").to_string();
    let hour = local_now.format("%H").to_string();
    let dom = local_now.format("%d").to_string();
    let month = local_now.format("%m").to_string();
    let dow = local_now.format("%u").to_string(); // 1=Monday, 7=Sunday

    if !cron_field_matches(fields[0], &minute)
        || !cron_field_matches(fields[1], &hour)
        || !cron_field_matches(fields[2], &dom)
        || !cron_field_matches(fields[3], &month)
        || !cron_field_matches(fields[4], &dow)
    {
        return false;
    }

    // Don't fire twice in the same minute
    if let Some(last) = last_run {
        let last_local = last.with_timezone(&Local);
        if last_local.format("%Y%m%d%H%M").to_string()
            == local_now.format("%Y%m%d%H%M").to_string()
        {
            return false;
        }
    }

    true
}

/// Check if a cron field matches a value.
/// Supports: * (any), exact number, comma-separated, ranges (1-5), steps (*/5).
fn cron_field_matches(field: &str, value: &str) -> bool {
    let val: u32 = match value.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };

    if field == "*" {
        return true;
    }

    // Step: */N
    if let Some(step_str) = field.strip_prefix("*/") {
        if let Ok(step) = step_str.parse::<u32>() {
            return step > 0 && val % step == 0;
        }
    }

    // Comma-separated or single values
    for part in field.split(',') {
        let part = part.trim();
        // Range: A-B
        if let Some((start_s, end_s)) = part.split_once('-') {
            if let (Ok(start), Ok(end)) = (start_s.parse::<u32>(), end_s.parse::<u32>()) {
                if val >= start && val <= end {
                    return true;
                }
            }
        } else if let Ok(exact) = part.parse::<u32>() {
            if val == exact {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_field() {
        assert!(cron_field_matches("*", "5"));
        assert!(cron_field_matches("5", "5"));
        assert!(!cron_field_matches("5", "6"));
        assert!(cron_field_matches("*/5", "15"));
        assert!(!cron_field_matches("*/5", "13"));
        assert!(cron_field_matches("1-5", "3"));
        assert!(!cron_field_matches("1-5", "7"));
        assert!(cron_field_matches("1,3,5", "3"));
        assert!(!cron_field_matches("1,3,5", "4"));
    }
}
