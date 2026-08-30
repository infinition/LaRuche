//! Shared application state (AppState, state types) and persistence - split out of main.rs.

use crate::*;

pub(crate) const ACTIVITY_LOG_LIMIT: usize = 400;

pub(crate) const METRICS_HISTORY_LIMIT: usize = 360; // ~1 hour at 10s intervals
pub(crate) const NODE_EVENTS_LIMIT: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ActivityLogEntry {
    pub(crate) timestamp: String,
    pub(crate) level: String,
    pub(crate) tag: String,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) full_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) full_response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) model_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) tokens_generated: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) latency_ms: Option<u64>,
    /// Owner user ID (for filtering: users see only their own logs, admin sees all)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) user_id: Option<Uuid>,
}

/// Push a simple entry to the activity log shown in the dashboard Audit panel. Use for events
/// that should be auditable (logins, account changes), which otherwise only reach the CLI.
pub(crate) async fn log_activite(
    state: &AppState,
    level: &str,
    tag: &str,
    message: String,
    user_id: Option<Uuid>,
) {
    log_activite_riche(state, level, tag, message, None, None, None, user_id).await;
}

/// Rich variant: activity entry with the prompt/response/model attached. Used by
/// the background dispatchers (cron, watcher, kanban); `log_activite` is the
/// sugar for the common case.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn log_activite_riche(
    state: &AppState,
    level: &str,
    tag: &str,
    message: String,
    full_prompt: Option<String>,
    full_response: Option<String>,
    model_used: Option<String>,
    user_id: Option<Uuid>,
) {
    let mut activity = state.activity_log.write().await;
    if activity.len() >= ACTIVITY_LOG_LIMIT {
        activity.pop_front();
    }
    activity.push_back(ActivityLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: level.into(),
        tag: tag.into(),
        message,
        full_prompt,
        full_response,
        model_used,
        tokens_generated: None,
        latency_ms: None,
        user_id,
    });
}

/// Persistent state saved to disk (survives restarts)
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct PersistentState {
    /// Legacy single default model (kept for backward-compatible deserialization)
    #[serde(default)]
    pub(crate) default_model: Option<String>,
    /// Per-capability default models (new format)
    #[serde(default)]
    pub(crate) default_models: Option<HashMap<String, String>>,
    /// Per-capability service selection (with source): survives restart.
    #[serde(default)]
    pub(crate) capability_selection: Option<HashMap<String, CapabilitySelection>>,
    #[serde(default)]
    pub(crate) activity_log: Vec<ActivityLogEntry>,
    #[serde(default)]
    pub(crate) disabled_tools: Vec<String>,
    #[serde(default)]
    pub(crate) disabled_skills: Vec<String>,
    /// Permission mode ("default" | "plan" | "acceptEdits" | "auto" | "bubble").
    #[serde(default)]
    pub(crate) permission_mode: Option<String>,
    #[serde(default)]
    pub(crate) saved_at: String,
    /// BLAKE3 cookie secret (base64), shared across cluster
    #[serde(default)]
    pub(crate) cookie_secret: Option<String>,
    #[serde(default)]
    pub(crate) context_max_messages: Option<usize>,
    #[serde(default)]
    pub(crate) context_max_tokens: Option<u32>,
    #[serde(default)]
    pub(crate) compaction_threshold: Option<f32>,
    /// Curateur (auto-skills/tools) enabled from Settings: survives restart.
    #[serde(default)]
    pub(crate) curateur_actif: Option<bool>,
    #[serde(default)]
    pub(crate) episodes_retention_jours: Option<u32>,
    #[serde(default)]
    pub(crate) halo_actif: Option<bool>,
    /// MCP server surface, off unless the user turned it on. Absent from a file written
    /// before this field, which reads as off: an upgrade never opens a door by itself.
    #[serde(default)]
    pub(crate) mcp_server_actif: Option<bool>,
    /// IP allowlist for that surface, and whether it is enforced.
    #[serde(default)]
    pub(crate) mcp_pare_feu_actif: Option<bool>,
    #[serde(default)]
    pub(crate) mcp_ip_autorisees: Option<Vec<String>>,
    /// "Home" channel (/sethome): default destination for proactive messages.
    #[serde(default)]
    pub(crate) home_channel: Option<String>,
    /// May the agent leave an emoji reaction on the user's message? A toggle the user
    /// flips in Settings, so it has to survive a restart like every other one.
    #[serde(default)]
    pub(crate) reactions_agent: Option<bool>,
    /// Dynamic tool selection (inject only relevant schemas: lighter prompt
    /// for small-context models). Survives restart.
    #[serde(default)]
    pub(crate) dynamic_tool_selection: Option<bool>,
    /// Hot generation settings edited in the dashboard.
    #[serde(default)]
    pub(crate) max_iterations: Option<usize>,
    #[serde(default)]
    pub(crate) temperature: Option<f32>,
    #[serde(default)]
    pub(crate) max_tokens: Option<u32>,
    #[serde(default)]
    pub(crate) tool_selection_limit: Option<usize>,
    #[serde(default)]
    pub(crate) dynamic_context_threshold: Option<u32>,
    /// Models used only by the Mixture tool when no explicit candidates are supplied.
    #[serde(default)]
    pub(crate) fallback_models: Option<Vec<String>>,
    /// Optional model used by cognitive-memory enrichment.
    #[serde(default)]
    pub(crate) review_model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MetricsSnapshot {
    pub(crate) epoch_ms: u64,
    pub(crate) cpu_pct: f32,
    pub(crate) ram_pct: f32,
    pub(crate) tokens_per_sec: f32,
    pub(crate) queue_depth: u32,
    pub(crate) node_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gpu_pct: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) vram_pct: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NodeEvent {
    pub(crate) epoch_ms: u64,
    pub(crate) event_type: String,
    pub(crate) node_name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MetricsHistoryResponse {
    pub(crate) snapshots: Vec<MetricsSnapshot>,
    pub(crate) events: Vec<NodeEvent>,
}

/// Current service selection for a given capability (stt/tts/code/vlm/vla/llm...).
/// Goes beyond a plain model name: keeps the SOURCE (backend / node mesh)
/// for routing (e.g. voice dictation to the chosen STT, auto-TTS to the chosen TTS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CapabilitySelection {
    pub(crate) capability: String,
    pub(crate) model: String,
    /// Backend/host (local label "llama.cpp"... or mesh node IP).
    pub(crate) backend: String,
    /// Remote Miel node id (None if local service).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) node_id: Option<String>,
    pub(crate) is_local: bool,
    /// Provider profile serving this capability (to resolve provider/base_url/key at runtime).
    pub(crate) profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CustomService {
    pub(crate) name: String,
    pub(crate) capability: String,
    pub(crate) url: String,
    pub(crate) protocol: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ActiveContextStats {
    pub(crate) messages: u32,
    pub(crate) base_tokens: u32,
    pub(crate) streamed_chars: usize,
    pub(crate) extra_tokens: u32,
    pub(crate) streaming_response_open: bool,
    pub(crate) running: bool,
}

impl ActiveContextStats {
    pub(crate) fn from_session(session: &Session, running: bool) -> Self {
        Self {
            messages: session.len() as u32,
            base_tokens: session.estimated_tokens() as u32,
            streamed_chars: 0,
            extra_tokens: 0,
            streaming_response_open: false,
            running,
        }
    }

    pub(crate) fn used_tokens(&self) -> u32 {
        self.base_tokens
            .saturating_add((self.streamed_chars / 4) as u32)
            .saturating_add(self.extra_tokens)
    }

    pub(crate) fn apply_event(&mut self, event: &ChatEvent) {
        match event {
            ChatEvent::Token { text } => {
                if !text.is_empty() {
                    self.streamed_chars = self.streamed_chars.saturating_add(text.len());
                    self.streaming_response_open = true;
                    self.running = true;
                }
            }
            ChatEvent::ToolCall { name, args, agent, .. } => {
                if self.streaming_response_open {
                    self.messages = self.messages.saturating_add(1);
                    self.streaming_response_open = false;
                }
                self.messages = self.messages.saturating_add(1);
                // Sub-agent calls live in an isolated context (see ToolResult below).
                if agent.is_none() {
                    self.extra_tokens = self
                        .extra_tokens
                        .saturating_add(approx_context_tokens(&format!("{name}{args}")));
                }
                self.running = true;
            }
            ChatEvent::ToolResult { name, result, agent, .. } => {
                self.messages = self.messages.saturating_add(1);
                // A SUB-AGENT's tool result never enters the main context: it runs on
                // an isolated context and only its compact report comes back. Counting
                // it here inflated the gauge with work the main agent never sees
                // (measured: a scout fan-out pushed the bar past 100%).
                if agent.is_none() {
                    self.extra_tokens = self
                        .extra_tokens
                        .saturating_add(approx_context_tokens(&format!("{name}{result}")));
                }
                self.running = true;
            }
            // COMPACTION: the engine just shrank its working context. Without this the
            // estimate only ever grew - the bar sat at 105% while the real context had
            // just been halved. Scale the accumulated estimate by the message ratio;
            // the next provider `Usage` re-anchors it on the truth.
            ChatEvent::Compaction {
                messages_before,
                messages_after,
            } => {
                let (avant, apres) = (*messages_before as u64, *messages_after as u64);
                if avant > 0 && apres < avant {
                    let garde = |v: u32| ((v as u64 * apres) / avant) as u32;
                    self.extra_tokens = garde(self.extra_tokens);
                    self.base_tokens = garde(self.base_tokens);
                    self.streamed_chars =
                        ((self.streamed_chars as u64 * apres) / avant) as usize;
                    self.messages = apres as u32;
                }
            }
            ChatEvent::Usage {
                input_tokens,
                output_tokens,
                ..
            } => {
                let usage_total = input_tokens.saturating_add(*output_tokens);
                if usage_total > self.used_tokens() {
                    self.base_tokens = usage_total;
                    self.streamed_chars = 0;
                    self.extra_tokens = 0;
                }
            }
            ChatEvent::Done { full_response } => {
                if self.streaming_response_open || !full_response.is_empty() {
                    self.messages = self.messages.saturating_add(1);
                    self.streaming_response_open = false;
                }
                self.running = false;
            }
            ChatEvent::Error { .. } => {
                self.running = false;
            }
            _ => {}
        }
    }
}

fn approx_context_tokens(text: &str) -> u32 {
    if text.is_empty() {
        0
    } else {
        text.len().div_ceil(4) as u32
    }
}

pub(crate) async fn update_active_context_stats(
    state: &Arc<AppState>,
    session_id: Uuid,
    event: &ChatEvent,
) {
    let mut stats_by_session = state.active_context_stats.write().await;
    let stats = stats_by_session
        .entry(session_id)
        .or_insert_with(|| ActiveContextStats {
            running: true,
            ..ActiveContextStats::default()
        });

    stats.apply_event(event);
}

pub(crate) struct AppState {
    pub(crate) manifest: RwLock<CognitiveManifest>,
    pub(crate) auth: RwLock<ProximityAuth>,
    pub(crate) queue: RwLock<RequestQueue>,
    pub(crate) listener: RwLock<MielListener>,
    pub(crate) config: NodeConfig,
    /// Manually declared mesh services (P6)
    pub(crate) custom_services: RwLock<HashMap<String, CustomService>>,
    /// Per-capability default models (e.g. "llm" → "mistral", "code" → "qwen3-coder:30b")
    /// The "llm" key is the universal fallback for unspecified capabilities.
    pub(crate) default_models: RwLock<HashMap<String, String>>,
    /// Per-capability service selection (with source), for voice/code/vision routing.
    pub(crate) capability_selection: RwLock<HashMap<String, CapabilitySelection>>,
    /// Long-running missions ("La Reine"): metadata; the knowledge lives in the cognitive map.
    /// Arc-shared: the mission_* abeilles hold a clone (registered before AppState exists).
    pub(crate) missions: Arc<RwLock<missions::MissionStore>>,
    pub(crate) sys: RwLock<System>,
    pub(crate) activity_log: RwLock<VecDeque<ActivityLogEntry>>,
    /// Path to laruche-state.json for persistence
    pub(crate) state_file_path: PathBuf,
    /// Time-series metrics for charts
    pub(crate) metrics_history: RwLock<VecDeque<MetricsSnapshot>>,
    /// Node connect/disconnect events
    pub(crate) node_events: RwLock<VecDeque<NodeEvent>>,
    /// Track known node IDs for event detection
    pub(crate) known_node_ids: RwLock<HashSet<String>>,
    /// Essaim agent engine
    pub(crate) essaim_registry: Arc<AbeilleRegistry>,
    pub(crate) essaim_config: RwLock<EssaimConfig>,
    pub(crate) memoire: Arc<dyn laruche_memoire::MemoireCognitive>,
    pub(crate) essaim_sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    pub(crate) active_context_stats: Arc<RwLock<HashMap<Uuid, ActiveContextStats>>>,
    pub(crate) essaim_cron: Arc<RwLock<CronScheduler>>,
    pub(crate) watchers: Arc<RwLock<laruche_watchers::WatchersRegistry>>,
    pub(crate) kanban_board: Arc<RwLock<laruche_kanban::KanbanBoard>>,
    pub(crate) essaim_kb: Arc<tokio::sync::RwLock<laruche_essaim::rag::KnowledgeBase>>,
    pub(crate) events: Arc<RwLock<laruche_events::EventBus>>,
    /// Active channel bots (keyed by channel name)
    pub(crate) channel_handles: RwLock<HashMap<String, tokio::task::JoinHandle<()>>>,
    /// Provider profiles (multi-provider support)
    pub(crate) profiles: RwLock<profiles::ProfilesConfig>,
    /// Path to provider-profiles.json
    pub(crate) profiles_path: PathBuf,
    /// Registered users
    pub(crate) users: RwLock<HashMap<Uuid, auth_user::User>>,
    /// Pending login challenges (ephemeral, 60s TTL)
    pub(crate) auth_challenges: RwLock<HashMap<Uuid, auth_user::AuthChallenge>>,
    /// BLAKE3 key for signing auth cookies (shared across cluster)
    pub(crate) cookie_secret: [u8; 32],
    /// Credential pool for multiple API keys per provider
    pub(crate) credential_pool: Arc<RwLock<laruche_essaim::credential_pool::CredentialPool>>,
    /// Path to credentials.json
    pub(crate) credentials_path: PathBuf,
    /// Last activity timestamp to trigger Dream mode
    pub(crate) last_activity: RwLock<std::time::Instant>,
    /// What LaRuche is doing RIGHT NOW, one entry per running job. The activity log is a
    /// history and answers "what happened"; this answers "what is happening", which is the
    /// only thing a live indicator can show. A std lock, not a tokio one: writes are two
    /// map operations with no await in between, and the guard needs to clean up on Drop,
    /// which cannot await.
    pub(crate) travaux: Arc<std::sync::RwLock<HashMap<Uuid, Travail>>>,
    /// Refusal tracker for the MCP surface (allowlist misses, bad tokens). A std mutex:
    /// every use is a handful of map operations with no await, on the cheapest possible
    /// path, since the whole point of a ban is that it costs nothing to serve.
    pub(crate) mcp_verrou: Arc<std::sync::Mutex<crate::mcp_pare_feu::Verrou>>,
}

/// One job in flight: who is working, with which model, toward which channel.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Travail {
    /// "curateur", "recherche", "cron", "watcher", "lareine" or "laruche".
    pub(crate) acteur: String,
    /// What it is about: a mission slug, a cron name, a watcher name.
    pub(crate) sujet: String,
    pub(crate) fournisseur: String,
    pub(crate) modele: String,
    /// Where the result is headed, when it is headed anywhere.
    pub(crate) canal: Option<String>,
    pub(crate) depuis: String,
}

/// Removes its entry when dropped, including on panic or early return, so the indicator
/// cannot be left showing work that already finished.
pub(crate) struct GardeTravail {
    id: Uuid,
    travaux: Arc<std::sync::RwLock<HashMap<Uuid, Travail>>>,
}

impl GardeTravail {
    pub(crate) fn nouveau(
        travaux: &Arc<std::sync::RwLock<HashMap<Uuid, Travail>>>,
        travail: Travail,
    ) -> Self {
        let id = Uuid::new_v4();
        if let Ok(mut m) = travaux.write() {
            m.insert(id, travail);
        }
        Self { id, travaux: travaux.clone() }
    }
}

impl Drop for GardeTravail {
    fn drop(&mut self) {
        if let Ok(mut m) = self.travaux.write() {
            m.remove(&self.id);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NodeConfig {
    pub(crate) node_name: String,
    pub(crate) tier: HardwareTier,
    pub(crate) ollama_url: String,
    pub(crate) default_model: String,
    pub(crate) api_port: u16,
    pub(crate) dashboard_port: u16,
    pub(crate) capabilities: Vec<CapabilityConfig>,
    /// LLM provider: "ollama" (default), "openai", "anthropic"
    #[serde(default)]
    pub(crate) provider: String,
    /// API key for cloud providers
    #[serde(default)]
    pub(crate) api_key: String,
    /// API base URL override
    #[serde(default)]
    pub(crate) api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CapabilityConfig {
    pub(crate) capability: String,
    pub(crate) model_name: String,
    pub(crate) model_size: Option<String>,
    pub(crate) quantization: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct NodeConfigFile {
    pub(crate) node_name: Option<String>,
    pub(crate) tier: Option<HardwareTier>,
    pub(crate) ollama_url: Option<String>,
    pub(crate) default_model: Option<String>,
    pub(crate) api_port: Option<u16>,
    pub(crate) dashboard_port: Option<u16>,
    pub(crate) capabilities: Option<Vec<CapabilityConfig>>,
    pub(crate) provider: Option<String>,
    pub(crate) api_key: Option<String>,
    pub(crate) api_base: Option<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_name: {
                let id = Uuid::new_v4().to_string();
                format!("laruche-{}", &id[..6])
            },
            tier: HardwareTier::Core,
            ollama_url: "http://127.0.0.1:11434".into(),
            default_model: "mistral".into(),
            api_port: miel_protocol::DEFAULT_API_PORT,
            dashboard_port: miel_protocol::DEFAULT_DASHBOARD_PORT,
            capabilities: vec![CapabilityConfig {
                capability: "llm".into(),
                model_name: "mistral-7b".into(),
                model_size: Some("7B".into()),
                quantization: Some("Q4_K_M".into()),
            }],
            provider: "ollama".into(),
            api_key: String::new(),
            api_base: None,
        }
    }
}

// ── Persistence ──────────────────────────────────────────────────────

pub(crate) fn resolve_state_file_path() -> PathBuf {
    if let Ok(dir) = std::env::var("LARUCHE_DATA_DIR") {
        PathBuf::from(dir).join("laruche-state.json")
    } else {
        PathBuf::from("laruche-state.json")
    }
}

pub(crate) fn load_persistent_state(path: &std::path::Path) -> PersistentState {
    match std::fs::read_to_string(path) {
        Ok(raw) => match serde_json::from_str::<PersistentState>(&raw) {
            Ok(s) => {
                info!(path = %path.display(), entries = s.activity_log.len(), "Loaded persistent state");
                s
            }
            Err(e) => {
                warn!(path = %path.display(), error = %e, "Failed to parse state file, starting fresh");
                PersistentState::default()
            }
        },
        Err(_) => {
            info!(path = %path.display(), "No state file found, starting fresh");
            PersistentState::default()
        }
    }
}

pub(crate) async fn save_persistent_state(state: &Arc<AppState>) {
    let logs = state.activity_log.read().await;
    let dm = state.default_models.read().await;
    let llm_default = dm.get("llm").cloned();
    let persistent = PersistentState {
        default_model: llm_default, // backward compat
        default_models: Some(dm.clone()),
        capability_selection: Some(state.capability_selection.read().await.clone()),
        activity_log: logs.iter().cloned().collect(),
        disabled_tools: state.essaim_config.read().await.disabled_tools.clone(),
        disabled_skills: state.essaim_config.read().await.disabled_skills.clone(),
        permission_mode: Some(
            settings_api::permission_mode_to_str(state.essaim_config.read().await.permission_mode).to_string(),
        ),
        saved_at: chrono::Utc::now().to_rfc3339(),
        cookie_secret: Some(auth_user::cookie_secret_to_base64(&state.cookie_secret)),
        context_max_messages: Some(state.essaim_config.read().await.context_max_messages),
        compaction_threshold: Some(state.essaim_config.read().await.compaction_threshold),
        context_max_tokens: Some(state.essaim_config.read().await.context_max_tokens),
        curateur_actif: Some(state.essaim_config.read().await.curateur_actif),
        episodes_retention_jours: Some(
            state.essaim_config.read().await.episodes_retention_jours,
        ),
        halo_actif: Some(state.essaim_config.read().await.halo_actif),
        mcp_server_actif: Some(state.essaim_config.read().await.mcp_server_actif),
        mcp_pare_feu_actif: Some(state.essaim_config.read().await.mcp_pare_feu_actif),
        mcp_ip_autorisees: Some(state.essaim_config.read().await.mcp_ip_autorisees.clone()),
        dynamic_tool_selection: Some(state.essaim_config.read().await.dynamic_tool_selection),
        reactions_agent: Some(state.essaim_config.read().await.reactions_agent),
        max_iterations: Some(state.essaim_config.read().await.max_iterations),
        temperature: Some(state.essaim_config.read().await.temperature),
        max_tokens: Some(state.essaim_config.read().await.max_tokens),
        tool_selection_limit: Some(state.essaim_config.read().await.tool_selection_limit),
        dynamic_context_threshold: Some(state.essaim_config.read().await.dynamic_context_threshold),
        fallback_models: Some(state.essaim_config.read().await.fallback_models.clone()),
        review_model: state.essaim_config.read().await.review_model.clone(),
        home_channel: state.essaim_config.read().await.home_channel.clone(),
    };
    drop(logs);
    drop(dm);

    let json = match serde_json::to_string_pretty(&persistent) {
        Ok(j) => j,
        Err(e) => {
            warn!(error = %e, "Failed to serialize state");
            return;
        }
    };

    let tmp_path = state.state_file_path.with_extension("json.tmp");
    if let Err(e) = tokio::fs::write(&tmp_path, &json).await {
        warn!(error = %e, "Failed to write state temp file");
        return;
    }
    if let Err(e) = tokio::fs::rename(&tmp_path, &state.state_file_path).await {
        warn!(error = %e, "Failed to rename state file");
    }
}

#[cfg(test)]
mod tests_travaux {
    use super::*;

    fn travail(acteur: &str) -> Travail {
        Travail {
            acteur: acteur.to_string(),
            sujet: "t".into(),
            fournisseur: "ollama".into(),
            modele: "m".into(),
            canal: None,
            depuis: "now".into(),
        }
    }

    /// Several jobs at once is the normal case, not the edge case: a cron can fire while
    /// a watcher triggers and the user is mid-conversation. Each guard must own exactly
    /// its own entry.
    #[test]
    fn plusieurs_travaux_coexistent_et_se_retirent_un_a_un() {
        let travaux: Arc<std::sync::RwLock<HashMap<Uuid, Travail>>> = Default::default();
        let g_chat = GardeTravail::nouveau(&travaux, travail("laruche"));
        let g_cron = GardeTravail::nouveau(&travaux, travail("cron"));
        let g_watch = GardeTravail::nouveau(&travaux, travail("watcher"));
        assert_eq!(travaux.read().unwrap().len(), 3);

        drop(g_cron);
        let restants: Vec<String> = travaux
            .read()
            .unwrap()
            .values()
            .map(|t| t.acteur.clone())
            .collect();
        assert_eq!(restants.len(), 2);
        assert!(!restants.contains(&"cron".to_string()));
        assert!(restants.contains(&"laruche".to_string()));
        assert!(restants.contains(&"watcher".to_string()));

        drop(g_chat);
        drop(g_watch);
        assert!(travaux.read().unwrap().is_empty());
    }

    /// Two jobs from the SAME actor are two jobs: two crons can overlap, and the second
    /// must not be mistaken for the first.
    #[test]
    fn deux_travaux_du_meme_acteur_restent_distincts() {
        let travaux: Arc<std::sync::RwLock<HashMap<Uuid, Travail>>> = Default::default();
        let a = GardeTravail::nouveau(&travaux, travail("cron"));
        let _b = GardeTravail::nouveau(&travaux, travail("cron"));
        assert_eq!(travaux.read().unwrap().len(), 2);
        drop(a);
        assert_eq!(travaux.read().unwrap().len(), 1);
    }

    /// A panicking job must not leave the indicator lit forever.
    #[test]
    fn une_panique_libere_quand_meme_l_entree() {
        let travaux: Arc<std::sync::RwLock<HashMap<Uuid, Travail>>> = Default::default();
        let t2 = travaux.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _garde = GardeTravail::nouveau(&t2, travail("cron"));
            assert_eq!(t2.read().unwrap().len(), 1);
            panic!("boom");
        }));
        assert!(travaux.read().unwrap().is_empty());
    }
}
