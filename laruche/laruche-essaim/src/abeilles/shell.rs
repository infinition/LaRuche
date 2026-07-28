use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

/// Execute a shell command.
pub struct ShellExec;

/// PowerShell writes to redirected pipes using the active output encoding. Force
/// UTF-8 so Rust can decode command output without replacing accented characters.
fn powershell_command(command: &str) -> String {
    format!(
        "$utf8 = [System.Text.UTF8Encoding]::new($false); [Console]::OutputEncoding = $utf8; $OutputEncoding = $utf8; {command}"
    )
}

/// Commands that are always blocked: too dangerous.
const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "rm -rf .",
    "format ",
    "mkfs",
    "dd if=",
    ":(){",
    "shutdown",
    "reboot",
    "del /s /q C:\\",
    "rd /s /q C:\\",
];

/// Patterns that look like secrets/credentials: warn before executing.
const SECRET_PATTERNS: &[&str] = &[
    "api_key=",
    "api-key=",
    "apikey=",
    "secret=",
    "password=",
    "passwd=",
    "token=",
    "bearer ",
    "authorization:",
    "aws_access_key",
    "aws_secret_key",
    "private_key",
    "-----BEGIN",
    "ghp_",  // GitHub personal token
    "sk-",   // OpenAI key
    "xoxb-", // Slack bot token
    "xoxp-", // Slack user token
];

#[async_trait]
impl Abeille for ShellExec {
    fn nom(&self) -> &str {
        "shell_exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output (stdout + stderr + exit code). \
         On Windows the command runs in PowerShell (use PowerShell syntax: \
         $env:USERPROFILE, Join-Path, New-Item, etc.); on Unix it runs in sh. \
         Use this for system tasks like checking disk space, listing processes, \
         running git, downloading files, etc. Long tasks may run up to 5 minutes. \
         Dangerous commands are blocked. \
         When walking a project tree, EXCLUDE build and VCS directories (target, \
         node_modules, .git, dist, build, .venv, __pycache__): they hold tens of \
         thousands of generated files and dominate the walk. A recursive listing that \
         includes a Rust target/ takes over a minute where the same listing without it \
         takes under a second. To answer \"what changed recently\", prefer `git status` \
         or `git log` when the directory is a repository: they already ignore what is \
         generated, and they are instant."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;

        // Check blocked patterns
        let cmd_lower = command.to_lowercase();
        for pattern in BLOCKED_PATTERNS {
            if cmd_lower.contains(pattern) {
                return Ok(ResultatAbeille::err(format!(
                    "Command blocked for safety: contains forbidden pattern '{}'",
                    pattern
                )));
            }
        }

        // Whole-drive enumeration: refused with a way FORWARD rather than a flat no,
        // because the agent that reaches for it has usually lost its bearings and needs
        // to be pointed at its working directory, not merely stopped.
        if scan_disque_entier(command) {
            return Ok(ResultatAbeille::err(
                "Refused: this enumerates an entire drive. It takes minutes and almost never \
                 finds what you are after. Your working directory is given in the Environment \
                 section of your instructions: start from there. To locate a file, use \
                 file_search scoped to a folder; to find an installed program, run its own \
                 version command. Never hunt across the disk."
                    .to_string(),
            ));
        }

        // Check for secrets/credentials in command
        for pattern in SECRET_PATTERNS {
            if cmd_lower.contains(pattern) {
                return Ok(ResultatAbeille::err(format!(
                    "WARNING: Command appears to contain a secret/credential (pattern: '{}'). \
                     Refusing to execute. Never include API keys, tokens, or passwords in commands.",
                    pattern
                )));
            }
        }

        // Substitute `${NOM}` secrets AFTER the anti-secret checks (the LLM only provided
        // placeholders; the real value enters here, never passing through the LLM).
        let command_sub = crate::secrets::substituer(command);
        let command = command_sub.as_str();

        // Check allowlist if configured
        if !ctx.shell_allowlist.is_empty() {
            let first_word = command.split_whitespace().next().unwrap_or("");
            if !ctx.shell_allowlist.iter().any(|a| a == first_word) {
                return Ok(ResultatAbeille::err(format!(
                    "Command '{}' not in allowlist. Allowed: {}",
                    first_word,
                    ctx.shell_allowlist.join(", ")
                )));
            }
        }

        // Both direct and Docker processes stream their output to the chat as they run.
        let use_docker = std::env::var("ESSAIM_SANDBOX_DOCKER").unwrap_or_default() == "1"
            && which::which("docker").is_ok();
        let (mut process, timeout_secs) = if use_docker {
            tracing::info!(command = %command, "Executing in Docker sandbox");
            let mut process = Command::new("docker");
            process.args([
                "run",
                "--rm",
                "--network=none",
                "--memory=256m",
                "--cpus=1",
                "--pids-limit=100",
                "-w",
                "/workspace",
                "alpine:latest",
                "sh",
                "-c",
                command,
            ]);
            (process, 60)
        } else {
            let mut process = if cfg!(windows) {
                let mut process = Command::new("powershell");
                let command = powershell_command(command);
                process.args(["-NoProfile", "-NonInteractive", "-Command", &command]);
                process
            } else {
                let mut process = Command::new("sh");
                process.arg("-c").arg(command);
                process
            };
            process.current_dir(&ctx.working_dir);
            (process, 300)
        };
        process
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let ctx_for_process = ctx.clone();
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async move {
                let mut child = process.spawn()?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("stdout unavailable"))?;
                let stderr = child
                    .stderr
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("stderr unavailable"))?;
                let stdout_task = tokio::spawn(crate::abeille::capture_process_stream(
                    stdout,
                    ctx_for_process.clone(),
                    "shell_exec",
                    "stdout",
                    16_000,
                ));
                let stderr_task = tokio::spawn(crate::abeille::capture_process_stream(
                    stderr,
                    ctx_for_process,
                    "shell_exec",
                    "stderr",
                    8_000,
                ));
                let status = child.wait().await?;
                let stdout = stdout_task.await??;
                let stderr = stderr_task.await??;
                Ok::<std::process::Output, anyhow::Error>(std::process::Output {
                    status,
                    stdout,
                    stderr,
                })
            })
            .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                // Truncate output to avoid context explosion
                let max_len = 4000;
                let mut combined = String::new();
                if !stdout.is_empty() {
                    let s: String = stdout.chars().take(max_len).collect();
                    combined.push_str(&s);
                    if stdout.len() > max_len {
                        combined.push_str("\n... (output truncated)");
                    }
                }
                if !stderr.is_empty() {
                    if !combined.is_empty() {
                        combined.push_str("\n--- stderr ---\n");
                    }
                    let s: String = stderr.chars().take(1000).collect();
                    combined.push_str(&s);
                }

                if combined.is_empty() {
                    combined = format!("(no output, exit code: {})", exit_code);
                } else if exit_code != 0 {
                    combined.push_str(&format!("\n(exit code: {})", exit_code));
                }

                if exit_code == 0 {
                    Ok(ResultatAbeille {
                        success: true,
                        output: combined,
                        error: None,
                        metadata: None,
                        cwd_change: None,
                        images: vec![],
                    })
                } else {
                    Ok(ResultatAbeille {
                        success: false,
                        output: combined,
                        error: Some(format!("Command exited with code {}", exit_code)),
                        metadata: None,
                        cwd_change: None,
                        images: vec![],
                    })
                }
            }
            Ok(Err(e)) => Ok(ResultatAbeille::err(format!("Failed to execute: {}", e))),
            Err(_) => Ok(ResultatAbeille::err(format!(
                "Command timed out after {} seconds",
                timeout_secs
            ))),
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn powershell_command_forces_utf8_before_the_requested_command() {
        let command = powershell_command("Write-Output 'déjà prêt'");
        assert!(command.contains("[Console]::OutputEncoding"));
        assert!(command.ends_with("Write-Output 'déjà prêt'"));
    }
}

/// Is this command about to enumerate an ENTIRE drive or filesystem root?
///
/// Not destructive, so it does not belong in `BLOCKED_PATTERNS`, but it hangs the agent
/// for minutes and almost never finds what it was after. Observed live: an agent that
/// had lost its working directory answered with
/// `Get-ChildItem -Path C:\ -Recurse -Filter "Cargo.toml"` and stalled the whole turn.
///
/// Deliberately narrow: recursion ALONE is fine and common (`-Recurse` inside a project
/// folder is normal). It takes recursion PLUS a root that is a bare drive or `/`.
pub(crate) fn scan_disque_entier(commande: &str) -> bool {
    let c = commande.to_lowercase();
    // `find / -name ...` and friends: the root is glued to the verb.
    if c.contains("find / ") || c.contains("find / -") {
        return true;
    }
    let recursif = c.contains("-recurse")
        || c.contains("-recursive")
        || c.contains(" /s ")
        || c.trim_end().ends_with(" /s")
        || c.contains(" -r ");
    if !recursif {
        return false;
    }
    // A token that IS a root, rather than a path that merely starts at one.
    c.split(|ch: char| ch.is_whitespace() || ch == '"' || ch == '\'')
        .map(|t| t.trim_end_matches(','))
        .any(|t| {
            matches!(t, "/" | "c:" | "c:\\" | "c:/" | "d:" | "d:\\" | "d:/")
                || t == "$env:systemdrive"
                || t == "%systemdrive%"
        })
}

#[cfg(test)]
mod tests_scan {
    use super::scan_disque_entier;

    #[test]
    fn un_balayage_de_disque_entier_est_reconnu() {
        // The exact shape observed in production.
        assert!(scan_disque_entier(
            r#"Get-ChildItem -Path C:\ -Directory -Recurse -Filter "Cargo.toml""#
        ));
        assert!(scan_disque_entier(r"dir /s C:\"));
        assert!(scan_disque_entier("find / -name Cargo.toml"));
        assert!(scan_disque_entier(r"Get-ChildItem -Recurse -Path 'C:'"));
    }

    #[test]
    fn une_recursion_normale_dans_un_dossier_passe() {
        // Recursion is ordinary work and must not be refused: only a ROOT is the problem.
        assert!(!scan_disque_entier(r"Get-ChildItem -Path C:\DEV\laruche -Recurse"));
        assert!(!scan_disque_entier("ls -R ./src"));
        assert!(!scan_disque_entier("find ./src -name '*.rs'"));
        assert!(!scan_disque_entier("dir /s src"));
        assert!(!scan_disque_entier("cargo test"));
    }
}
