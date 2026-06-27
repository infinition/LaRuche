use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub struct ExecuteCode;

const MAX_CODE_CHARS: usize = 20_000;
const MAX_OUTPUT_CHARS: usize = 6_000;

#[async_trait]
impl Abeille for ExecuteCode {
    fn nom(&self) -> &str {
        "execute_code"
    }

    fn description(&self) -> &str {
        "Run a short Python snippet using the system python binary. 30s timeout; output truncated head+tail if too large."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "code": { "type": "string", "description": "Python snippet to execute" }
            },
            "required": ["code"]
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
        let code = args["code"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'code' argument"))?;
        if code.chars().count() > MAX_CODE_CHARS {
            return Ok(ResultatAbeille::err(format!(
                "Snippet too long (max {MAX_CODE_CHARS} chars)"
            )));
        }

        let python = find_python_binary();
        let mut child = match Command::new(&python)
            .args(["-I", "-"])
            .current_dir(&ctx.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                return Ok(ResultatAbeille::err(format!(
                    "Failed to launch python ({python}): {e}"
                )))
            }
        };

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Python stdout unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Python stderr unavailable"))?;
        let stdout_task = tokio::spawn(crate::abeille::capture_process_stream(
            stdout,
            ctx.clone(),
            "execute_code",
            "stdout",
            16_000,
        ));
        let stderr_task = tokio::spawn(crate::abeille::capture_process_stream(
            stderr,
            ctx.clone(),
            "execute_code",
            "stderr",
            8_000,
        ));

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(code.as_bytes()).await?;
        }

        match tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let status = child.wait().await?;
            let stdout = stdout_task.await??;
            let stderr = stderr_task.await??;
            Ok::<std::process::Output, anyhow::Error>(std::process::Output {
                status,
                stdout,
                stderr,
            })
        })
        .await
        {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut combined = String::new();
                if !stdout.is_empty() {
                    combined.push_str(&head_tail(&stdout, MAX_OUTPUT_CHARS));
                }
                if !stderr.is_empty() {
                    if !combined.is_empty() {
                        combined.push_str("\n--- stderr ---\n");
                    }
                    combined.push_str(&head_tail(&stderr, 2000));
                }
                if combined.is_empty() {
                    combined = format!("(no output, exit code: {exit_code})");
                } else if exit_code != 0 {
                    combined.push_str(&format!("\n(exit code: {exit_code})"));
                }
                if output.status.success() {
                    Ok(ResultatAbeille::ok(combined))
                } else {
                    Ok(ResultatAbeille::err(combined))
                }
            }
            Ok(Err(e)) => Ok(ResultatAbeille::err(format!(
                "Python execution failed: {e}"
            ))),
            Err(_) => Ok(ResultatAbeille::err("Python snippet timed out after 30s")),
        }
    }
}

fn find_python_binary() -> String {
    for candidate in ["python", "python3"] {
        if which::which(candidate).is_ok() {
            return candidate.to_string();
        }
    }
    "python".to_string()
}

fn head_tail(input: &str, max_chars: usize) -> String {
    let len = input.chars().count();
    if len <= max_chars {
        return input.to_string();
    }
    let head_len = max_chars / 2;
    let tail_len = max_chars - head_len;
    let head: String = input.chars().take(head_len).collect();
    let tail: String = input
        .chars()
        .rev()
        .take(tail_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}\n... (output truncated, {len} chars total) ...\n{tail}")
}
