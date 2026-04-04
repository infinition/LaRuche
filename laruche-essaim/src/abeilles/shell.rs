use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

/// Execute a shell command.
pub struct ShellExec;

/// Commands that are always blocked — too dangerous.
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

/// Patterns that look like secrets/credentials — warn before executing.
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
    "ghp_",      // GitHub personal token
    "sk-",       // OpenAI key
    "xoxb-",     // Slack bot token
    "xoxp-",     // Slack user token
];

#[async_trait]
impl Abeille for ShellExec {
    fn nom(&self) -> &str {
        "shell_exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output (stdout + stderr). \
         Use this for system tasks like checking disk space, listing processes, \
         running git commands, etc. Dangerous commands are blocked."
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

        // Check if Docker sandbox is requested and available
        let use_docker = std::env::var("ESSAIM_SANDBOX_DOCKER").unwrap_or_default() == "1"
            && which::which("docker").is_ok();

        let result = if use_docker {
            // Execute in Docker sandbox (isolated, no access to host filesystem)
            tracing::info!(command = %command, "Executing in Docker sandbox");
            tokio::time::timeout(
                std::time::Duration::from_secs(60),
                Command::new("docker")
                    .args([
                        "run", "--rm",
                        "--network=none",       // No network access
                        "--memory=256m",         // Memory limit
                        "--cpus=1",              // CPU limit
                        "--pids-limit=100",      // Process limit
                        "-w", "/workspace",
                        "alpine:latest",
                        "sh", "-c", command,
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output(),
            )
            .await
        } else {
            // Direct execution on host
            let shell = if cfg!(windows) { "cmd" } else { "sh" };
            let flag = if cfg!(windows) { "/C" } else { "-c" };

            tokio::time::timeout(
                std::time::Duration::from_secs(30),
                Command::new(shell)
                    .arg(flag)
                    .arg(command)
                    .current_dir(&ctx.working_dir)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output(),
            )
            .await
        };

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

                Ok(ResultatAbeille::ok(combined))
            }
            Ok(Err(e)) => Ok(ResultatAbeille::err(format!("Failed to execute: {}", e))),
            Err(_) => Ok(ResultatAbeille::err(
                "Command timed out after 30 seconds",
            )),
        }
    }
}
