use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;

/// A chunk emitted by a process-backed Abeille while it is still running.
#[derive(Debug, Clone)]
pub struct ToolOutputChunk {
    pub tool_name: String,
    pub stream: &'static str,
    pub text: String,
}

const LIVE_OUTPUT_SECRET_HINTS: &[&str] = &[
    "api_key",
    "api-key",
    "apikey",
    "authorization:",
    "bearer ",
    "password",
    "passwd",
    "secret",
    "token=",
    "private_key",
    "-----begin",
];

fn redact_live_output(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| {
            let lower = line.to_lowercase();
            if LIVE_OUTPUT_SECRET_HINTS
                .iter()
                .any(|hint| lower.contains(hint))
            {
                if line.ends_with('\n') {
                    "[ligne masquée : donnée sensible détectée]\n"
                } else {
                    "[ligne masquée : donnée sensible détectée]"
                }
            } else {
                line
            }
        })
        .collect()
}

/// Danger level for a tool: determines approval gating behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NiveauDanger {
    /// Safe to execute without confirmation (e.g., file_read, math)
    Safe,
    /// Requires user approval before execution (e.g., file_write, shell)
    NeedsApproval,
    /// Blocked by default: must be explicitly allowlisted (e.g., rm -rf)
    Dangerous,
}

/// Where a tool comes from. Built-in tools are compiled Rust code; custom tools
/// are user-editable JSON plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolOrigin {
    Builtin,
    Custom,
    Mcp,
}

/// Result returned by an Abeille after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultatAbeille {
    /// Whether the tool executed successfully
    pub success: bool,
    /// The output/result text to show the LLM
    pub output: String,
    /// Optional error message
    pub error: Option<String>,
    pub metadata: Option<serde_json::Value>,
    /// Optional global CWD change request (e.g. for worktree entering)
    pub cwd_change: Option<PathBuf>,
    /// Optional multimodal images (base64 encoded) returned by the tool
    pub images: Vec<String>,
}

impl ResultatAbeille {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            metadata: None,
            cwd_change: None,
            images: vec![],
        }
    }

    pub fn ok_with_cwd(output: impl Into<String>, cwd_change: PathBuf) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            metadata: None,
            cwd_change: Some(cwd_change),
            images: vec![],
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            metadata: None,
            cwd_change: None,
            images: vec![],
        }
    }
}

/// Execution context passed to each Abeille: contains sandbox limits and config.
#[derive(Debug, Clone)]
pub struct ContextExecution {
    /// Allowed base directories for file operations
    pub allowed_dirs: Vec<PathBuf>,
    /// Allowed shell commands (if empty, all are blocked)
    pub shell_allowlist: Vec<String>,
    /// Working directory for the current session
    pub working_dir: PathBuf,
    /// Optional live process-output channel consumed by the chat transport.
    pub live_output: Option<mpsc::UnboundedSender<ToolOutputChunk>>,
    /// Originating channel of the request (`telegram:12345`, `discord:bob`, `web`...). Lets
    /// tools like `cron_create` route the recurring job back to where it was requested.
    pub channel: Option<String>,
}

impl Default for ContextExecution {
    fn default() -> Self {
        Self {
            allowed_dirs: vec![],
            shell_allowlist: vec![],
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            live_output: None,
            channel: None,
        }
    }
}

impl ContextExecution {
    /// Sends a display-only stdout/stderr chunk without affecting the LLM context.
    pub fn emit_live_output(&self, tool_name: &str, stream: &'static str, text: &str) {
        if text.is_empty() {
            return;
        }
        let redacted = redact_live_output(text);
        if let Some(sender) = &self.live_output {
            let _ = sender.send(ToolOutputChunk {
                tool_name: tool_name.to_string(),
                stream,
                text: redacted,
            });
        }
    }
}

/// Reads a child-process stream while forwarding bounded chunks to the UI.
///
/// The returned buffer remains bounded as well, so a verbose process cannot exhaust memory
/// while its visible output continues to be streamed to the user.
pub async fn capture_process_stream<R>(
    mut reader: R,
    ctx: ContextExecution,
    tool_name: &'static str,
    stream: &'static str,
    max_bytes: usize,
) -> std::io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut captured = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        ctx.emit_live_output(tool_name, stream, &String::from_utf8_lossy(&buffer[..read]));
        let remaining = max_bytes.saturating_sub(captured.len());
        if remaining > 0 {
            captured.extend_from_slice(&buffer[..read.min(remaining)]);
        }
    }
    Ok(captured)
}

/// The core tool trait. Each tool ("Abeille") implements this.
#[async_trait]
pub trait Abeille: Send + Sync {
    /// Unique tool name (e.g., "file_read", "web_search")
    fn nom(&self) -> &str;

    /// Human-readable description for the LLM
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's parameters.
    /// This is injected into the system prompt so the LLM knows how to call the tool.
    fn schema(&self) -> serde_json::Value;

    /// Danger level: determines if user approval is needed
    fn niveau_danger(&self) -> NiveauDanger;

    /// Tool provenance displayed by the registry and UI.
    fn origin(&self) -> ToolOrigin {
        ToolOrigin::Builtin
    }

    /// Optional maximum size for the tool result exposed to the model.
    fn max_result_size(&self) -> Option<usize> {
        None
    }

    /// Execute the tool with parsed JSON arguments.
    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille>;
}

/// Registry holding all available Abeilles (tools).
use std::sync::Arc;
use std::sync::RwLock;

pub struct AbeilleRegistry {
    abeilles: RwLock<HashMap<String, Arc<dyn Abeille>>>,
}

impl Default for AbeilleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AbeilleRegistry {
    pub fn new() -> Self {
        Self {
            abeilles: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new Abeille.
    pub fn enregistrer(&self, abeille: Box<dyn Abeille>) {
        let nom = abeille.nom().to_string();
        tracing::info!(tool = %nom, "Abeille registered");
        self.abeilles.write().unwrap().insert(nom, abeille.into());
    }

    /// Get a reference to an Abeille by name.
    pub fn get(&self, nom: &str) -> Option<Arc<dyn Abeille>> {
        self.abeilles.read().unwrap().get(nom).cloned()
    }

    pub fn supprimer_par_origine(&self, origin: ToolOrigin) {
        let mut w = self.abeilles.write().unwrap();
        w.retain(|_, v| v.origin() != origin);
    }

    /// Get all tool names.
    pub fn noms(&self) -> Vec<String> {
        self.abeilles.read().unwrap().keys().cloned().collect()
    }

    /// Get the provenance of a registered tool.
    pub fn origin(&self, nom: &str) -> Option<ToolOrigin> {
        self.abeilles.read().unwrap().get(nom).map(|a| a.origin())
    }

    /// Generate the complete JSON schema for all tools: injected into the system prompt.
    pub fn schema_complet(&self) -> serde_json::Value {
        let lock = self.abeilles.read().unwrap();
        let tools: Vec<serde_json::Value> = lock
            .values()
            .map(|a| {
                serde_json::json!({
                    "name": a.nom(),
                    "description": a.description(),
                    "parameters": a.schema(),
                    "origin": a.origin(),
                })
            })
            .collect();
        serde_json::Value::Array(tools)
    }

    /// Execute an Abeille by name. Arguments are validated (and gently coerced)
    /// against the tool's declared schema BEFORE execution: the non-negotiable
    /// safety net for local models, which routinely send numbers as strings or
    /// forget a required field. A mismatch comes back as a corrective
    /// observation so the model can retry properly instead of the tool failing
    /// in an opaque way.
    pub async fn executer(
        &self,
        nom: &str,
        mut args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let abeille = { self.abeilles.read().unwrap().get(nom).cloned() };

        if let Some(a) = abeille {
            if let Err(e) = valider_et_normaliser_args(&a.schema(), &mut args) {
                return Ok(ResultatAbeille::err(format!(
                    "Invalid arguments for {nom}: {e}. Check the tool schema and retry."
                )));
            }
            a.executer(args, ctx).await
        } else {
            Ok(ResultatAbeille::err(format!("Unknown tool: {nom}")))
        }
    }
}

/// JSON type name for error messages.
fn type_json(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Client-side argument gate: checks the call against the tool's JSON schema
/// (top level) and coerces the classic local-model mistakes first:
/// - "10" -> 10 (integer/number), "true" -> true (boolean)
/// - 10.0 -> 10 when the schema says integer
/// - an object/array sent as an embedded JSON STRING is unwrapped
/// Lenient by design: extra keys pass, undeclared keys pass, absent optional
/// keys pass. Only `required` presence and declared top-level types are hard.
pub fn valider_et_normaliser_args(
    schema: &serde_json::Value,
    args: &mut serde_json::Value,
) -> Result<(), String> {
    if args.is_null() {
        *args = serde_json::json!({});
    }
    let Some(obj) = args.as_object_mut() else {
        return Err(format!(
            "arguments must be a JSON object, got: {}",
            type_json(args)
        ));
    };
    let props = schema.get("properties").and_then(|p| p.as_object());

    // Coercions first, so a recoverable mistake never reaches the type check.
    if let Some(props) = props {
        for (key, spec) in props {
            let Some(v) = obj.get_mut(key) else { continue };
            let attendu = spec.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match v {
                serde_json::Value::String(s) => {
                    let s_trim = s.trim();
                    match attendu {
                        "integer" => {
                            if let Ok(n) = s_trim.parse::<i64>() {
                                *v = serde_json::json!(n);
                            }
                        }
                        "number" => {
                            if let Ok(n) = s_trim.parse::<f64>() {
                                *v = serde_json::json!(n);
                            }
                        }
                        "boolean" => match s_trim {
                            "true" | "True" => *v = serde_json::json!(true),
                            "false" | "False" => *v = serde_json::json!(false),
                            _ => {}
                        },
                        "object" | "array" => {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s_trim) {
                                if (attendu == "object" && parsed.is_object())
                                    || (attendu == "array" && parsed.is_array())
                                {
                                    *v = parsed;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                serde_json::Value::Number(n) => {
                    // 10.0 for an integer field: accept the round float.
                    if attendu == "integer" && !n.is_i64() && !n.is_u64() {
                        if let Some(f) = n.as_f64() {
                            if f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
                                *v = serde_json::json!(f as i64);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Required presence (an explicit null does not count as provided).
    if let Some(reqs) = schema.get("required").and_then(|r| r.as_array()) {
        let manquants: Vec<&str> = reqs
            .iter()
            .filter_map(|r| r.as_str())
            .filter(|r| obj.get(*r).map(|v| v.is_null()).unwrap_or(true))
            .collect();
        if !manquants.is_empty() {
            return Err(format!(
                "missing required argument(s): {}",
                manquants.join(", ")
            ));
        }
    }

    // Declared top-level types.
    if let Some(props) = props {
        for (key, spec) in props {
            let Some(v) = obj.get(key) else { continue };
            if v.is_null() {
                continue; // explicit null on an optional key = absent
            }
            let Some(attendu) = spec.get("type").and_then(|t| t.as_str()) else {
                continue;
            };
            let ok = match attendu {
                "string" => v.is_string(),
                "integer" => v.is_i64() || v.is_u64(),
                "number" => v.is_number(),
                "boolean" => v.is_boolean(),
                "object" => v.is_object(),
                "array" => v.is_array(),
                _ => true,
            };
            if !ok {
                return Err(format!(
                    "argument '{key}' must be a {attendu}, got: {}",
                    type_json(v)
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    struct BuiltinTool;

    #[async_trait]
    impl Abeille for BuiltinTool {
        fn nom(&self) -> &str {
            "builtin_test"
        }
        fn description(&self) -> &str {
            "Builtin test tool"
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        fn niveau_danger(&self) -> NiveauDanger {
            NiveauDanger::Safe
        }
        async fn executer(
            &self,
            _args: serde_json::Value,
            _ctx: &ContextExecution,
        ) -> Result<ResultatAbeille> {
            Ok(ResultatAbeille::ok("ok"))
        }
    }

    #[tokio::test]
    async fn process_stream_is_forwarded_without_entering_the_tool_result() {
        let (mut writer, reader) = tokio::io::duplex(64);
        writer.write_all(b"ligne en direct\n").await.unwrap();
        drop(writer);

        let (sender, mut receiver) = mpsc::unbounded_channel();
        let mut ctx = ContextExecution::default();
        ctx.live_output = Some(sender);
        let captured = capture_process_stream(reader, ctx, "shell_exec", "stdout", 1024)
            .await
            .unwrap();

        assert_eq!(captured, b"ligne en direct\n");
        let chunk = receiver.try_recv().unwrap();
        assert_eq!(chunk.tool_name, "shell_exec");
        assert_eq!(chunk.stream, "stdout");
        assert_eq!(chunk.text, "ligne en direct\n");
    }

    #[test]
    fn live_output_masks_lines_that_look_like_secrets() {
        assert_eq!(
            redact_live_output("ok\ntoken=super-secret\nencore ok\n"),
            "ok\n[ligne masquée : donnée sensible détectée]\nencore ok\n"
        );
    }

    #[test]
    fn builtin_origin_is_exposed_by_registry_schema() {
        let registry = AbeilleRegistry::new();
        registry.enregistrer(Box::new(BuiltinTool));

        assert_eq!(registry.origin("builtin_test"), Some(ToolOrigin::Builtin));
        assert_eq!(registry.schema_complet()[0]["origin"], "builtin");
    }

    fn schema_web() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" },
                "max_chars": { "type": "integer" },
                "render": { "type": "boolean" },
                "opts": { "type": "object" }
            },
            "required": ["url"]
        })
    }

    #[test]
    fn args_manquants_sont_signales_avant_execution() {
        let mut args = serde_json::json!({ "max_chars": 10 });
        let err = valider_et_normaliser_args(&schema_web(), &mut args).unwrap_err();
        assert!(err.contains("url"), "{err}");
        // Explicit null does not count as provided.
        let mut args = serde_json::json!({ "url": null });
        assert!(valider_et_normaliser_args(&schema_web(), &mut args).is_err());
    }

    #[test]
    fn coercions_des_erreurs_classiques_de_modeles_locaux() {
        let mut args = serde_json::json!({
            "url": "https://a.b",
            "max_chars": "1200",          // number as string
            "render": "true",              // boolean as string
            "opts": "{\"deep\": true}"    // object as embedded JSON string
        });
        valider_et_normaliser_args(&schema_web(), &mut args).unwrap();
        assert_eq!(args["max_chars"], 1200);
        assert_eq!(args["render"], true);
        assert_eq!(args["opts"]["deep"], true);
        // Round float for an integer field.
        let mut args = serde_json::json!({ "url": "https://a.b", "max_chars": 10.0 });
        valider_et_normaliser_args(&schema_web(), &mut args).unwrap();
        assert_eq!(args["max_chars"], 10);
    }

    #[test]
    fn type_incoercible_rejete_et_extras_toleres() {
        // A non-numeric string for an integer field is a hard error.
        let mut args = serde_json::json!({ "url": "https://a.b", "max_chars": "beaucoup" });
        let err = valider_et_normaliser_args(&schema_web(), &mut args).unwrap_err();
        assert!(err.contains("max_chars"), "{err}");
        // Extra keys and empty schemas pass; null args become {}.
        let mut args = serde_json::json!({ "url": "https://a.b", "bonus": 42 });
        valider_et_normaliser_args(&schema_web(), &mut args).unwrap();
        let mut args = serde_json::Value::Null;
        valider_et_normaliser_args(&serde_json::json!({"type":"object","properties":{}}), &mut args)
            .unwrap();
        assert!(args.is_object());
    }
}
