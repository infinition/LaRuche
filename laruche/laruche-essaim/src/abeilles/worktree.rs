use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;

pub struct AbeilleGitWorktreeEnter;

#[async_trait]
impl Abeille for AbeilleGitWorktreeEnter {
    fn nom(&self) -> &str {
        "git_worktree_enter"
    }

    fn description(&self) -> &str {
        "Creates an isolated git worktree and switches the current session's working directory into it. Useful for making speculative changes without affecting the main repository."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "branch_name": {
                    "type": "string",
                    "description": "Name of the new branch to create for this worktree (e.g. 'feature/my-test')"
                }
            },
            "required": ["branch_name"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let branch_name = match args.get("branch_name").and_then(|v| v.as_str()) {
            Some(b) => b,
            None => return Ok(ResultatAbeille::err("Missing 'branch_name' parameter")),
        };

        let current_dir = &ctx.working_dir;
        let main_repo_root = find_git_root(current_dir);
        let repo_root = main_repo_root.unwrap_or_else(|| current_dir.clone());

        // We place worktrees in a .worktrees directory alongside or inside the repo?
        // Usually, `git worktree add ../something` is used. We'll use `.laruche/worktrees/<uuid>`.
        let worktree_id = Uuid::new_v4().to_string();
        let worktrees_base = repo_root.join(".laruche").join("worktrees");
        if !worktrees_base.exists() {
            std::fs::create_dir_all(&worktrees_base).unwrap_or_default();
        }

        // Add to git ignore if not already there
        let gitignore = repo_root.join(".gitignore");
        if gitignore.exists() {
            let content = std::fs::read_to_string(&gitignore).unwrap_or_default();
            if !content.contains(".laruche/") {
                let mut new_content = content.clone();
                if !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                new_content.push_str(".laruche/\n");
                let _ = std::fs::write(&gitignore, new_content);
            }
        }

        let worktree_path = worktrees_base.join(&worktree_id);

        let output = Command::new("git")
            .arg("worktree")
            .arg("add")
            .arg("-b")
            .arg(branch_name)
            .arg(&worktree_path)
            .current_dir(&repo_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ResultatAbeille::err(format!(
                "Failed to create worktree: {}",
                stderr
            )));
        }

        // Return the absolute path as the new CWD
        let absolute_path = if worktree_path.is_absolute() {
            worktree_path.clone()
        } else {
            std::env::current_dir()?.join(worktree_path)
        };

        Ok(ResultatAbeille::ok_with_cwd(
            format!(
                "Successfully created and entered worktree for branch '{}' at {:?}",
                branch_name, absolute_path
            ),
            absolute_path,
        ))
    }
}

pub struct AbeilleGitWorktreeExit;

#[async_trait]
impl Abeille for AbeilleGitWorktreeExit {
    fn nom(&self) -> &str {
        "git_worktree_exit"
    }

    fn description(&self) -> &str {
        "Exits the current isolated git worktree and restores the session's working directory to the original repository."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        _args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let current_dir = &ctx.working_dir;
        let git_root = match find_git_root(current_dir) {
            Some(r) => r,
            None => return Ok(ResultatAbeille::err("Not inside a git repository.")),
        };

        // If we are inside .laruche/worktrees, we should go back to the main repo.
        // The main repo root is usually the parent of .laruche.
        let mut maybe_main_root = git_root.clone();

        // Find if we are inside a worktree by checking git worktree list
        let output = Command::new("git")
            .arg("worktree")
            .arg("list")
            .current_dir(&git_root)
            .stdout(Stdio::piped())
            .output()
            .await;

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if let Some(first_line) = lines.first() {
                // The first line of `git worktree list` is the main repository path
                let parts: Vec<&str> = first_line.split_whitespace().collect();
                if let Some(main_path_str) = parts.first() {
                    maybe_main_root = PathBuf::from(main_path_str);
                }
            }
        }

        Ok(ResultatAbeille::ok_with_cwd(
            format!(
                "Exited worktree. Returned to main repository at {:?}",
                maybe_main_root
            ),
            maybe_main_root,
        ))
    }
}

fn find_git_root(current_dir: &Path) -> Option<PathBuf> {
    let mut dir = current_dir.to_path_buf();
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}
