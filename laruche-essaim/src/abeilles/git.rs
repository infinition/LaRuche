//! Git integration abeilles — status, diff, log, commit.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

async fn run_git(args: &[&str], cwd: &std::path::Path) -> Result<String, String> {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        Command::new("git")
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    ).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                Ok(if stdout.is_empty() { stderr } else { stdout })
            } else {
                Err(format!("{}\n{}", stdout, stderr).trim().to_string())
            }
        }
        Ok(Err(e)) => Err(format!("Failed to run git: {}", e)),
        Err(_) => Err("Git command timed out".to_string()),
    }
}

pub struct GitStatus;
#[async_trait]
impl Abeille for GitStatus {
    fn nom(&self) -> &str { "git_status" }
    fn description(&self) -> &str { "Show the git status of the current working directory (modified, staged, untracked files)." }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}, "required": []})
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::Safe }
    async fn executer(&self, _args: serde_json::Value, ctx: &ContextExecution) -> Result<ResultatAbeille> {
        match run_git(&["status", "--short", "--branch"], &ctx.working_dir).await {
            Ok(out) => Ok(ResultatAbeille::ok(out)),
            Err(e) => Ok(ResultatAbeille::err(e)),
        }
    }
}

pub struct GitDiff;
#[async_trait]
impl Abeille for GitDiff {
    fn nom(&self) -> &str { "git_diff" }
    fn description(&self) -> &str { "Show the git diff (changes not yet staged). Optionally specify a file path." }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Optional file path to diff"},
                "staged": {"type": "boolean", "description": "Show staged changes (--cached)"}
            },
            "required": []
        })
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::Safe }
    async fn executer(&self, args: serde_json::Value, ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let mut git_args = vec!["diff", "--stat"];
        if args["staged"].as_bool().unwrap_or(false) {
            git_args.push("--cached");
        }
        // First get summary
        let summary = run_git(&git_args, &ctx.working_dir).await.unwrap_or_default();

        // Then get actual diff (limited)
        let mut diff_args = vec!["diff"];
        if args["staged"].as_bool().unwrap_or(false) {
            diff_args.push("--cached");
        }
        if let Some(path) = args["path"].as_str() {
            diff_args.push("--");
            diff_args.push(path);
        }
        let diff = run_git(&diff_args, &ctx.working_dir).await.unwrap_or_default();
        let truncated: String = diff.chars().take(4000).collect();

        Ok(ResultatAbeille::ok(format!("{}\n\n{}", summary, truncated)))
    }
}

pub struct GitLog;
#[async_trait]
impl Abeille for GitLog {
    fn nom(&self) -> &str { "git_log" }
    fn description(&self) -> &str { "Show recent git commits. Default: last 10 commits." }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer", "description": "Number of commits (default 10)"}
            },
            "required": []
        })
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::Safe }
    async fn executer(&self, args: serde_json::Value, ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let count = args["count"].as_u64().unwrap_or(10).to_string();
        match run_git(&["log", "--oneline", "--graph", &format!("-{}", count)], &ctx.working_dir).await {
            Ok(out) => Ok(ResultatAbeille::ok(out)),
            Err(e) => Ok(ResultatAbeille::err(e)),
        }
    }
}

pub struct GitCommit;
#[async_trait]
impl Abeille for GitCommit {
    fn nom(&self) -> &str { "git_commit" }
    fn description(&self) -> &str { "Stage all changes and create a git commit with the given message." }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {"type": "string", "description": "Commit message"},
                "add_all": {"type": "boolean", "description": "Stage all changes before committing (default true)"}
            },
            "required": ["message"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::NeedsApproval }
    async fn executer(&self, args: serde_json::Value, ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let message = args["message"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'message'"))?;
        let add_all = args["add_all"].as_bool().unwrap_or(true);

        if add_all {
            if let Err(e) = run_git(&["add", "-A"], &ctx.working_dir).await {
                return Ok(ResultatAbeille::err(format!("git add failed: {}", e)));
            }
        }

        match run_git(&["commit", "-m", message], &ctx.working_dir).await {
            Ok(out) => Ok(ResultatAbeille::ok(format!("Committed: {}\n{}", message, out))),
            Err(e) => Ok(ResultatAbeille::err(format!("git commit failed: {}", e))),
        }
    }
}
