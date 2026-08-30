//! Configuration for the Essaim agent engine (`EssaimConfig`) and the
//! LaReine supervisor settings mirror (`ReineConfig`).

use laruche_permissions::{PermissionMode, PermissionRule};
use serde::{Deserialize, Serialize};

/// Configuration for the Essaim agent engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EssaimConfig {
    /// Ollama API URL (default: http://127.0.0.1:11434)
    pub ollama_url: String,
    /// Default model for inference
    pub model: String,
    /// Model used for specific reviews and missions
    pub review_model: Option<String>,
    /// Fallback models (tried in order if primary fails)
    #[serde(default)]
    pub fallback_models: Vec<String>,
    /// Maximum ReAct iterations before giving up
    pub max_iterations: usize,
    /// Temperature for LLM sampling
    pub temperature: f32,
    /// Maximum tokens per response
    pub max_tokens: u32,
    /// Custom system prompt instructions
    pub custom_instructions: Option<String>,
    /// Max messages in context before auto-compaction (default: 30)
    pub context_max_messages: usize,
    /// Actual context window of the current model/provider in tokens (default: 128000)
    /// Used for the UI context gauge and token-aware decisions.
    pub context_max_tokens: u32,
    /// Context compaction threshold ratio (default: 0.75)
    pub compaction_threshold: f32,
    /// Cost per 1k input tokens in USD (default: 0.0)
    #[serde(default)]
    pub cost_per_1k_input: f32,
    /// Cost per 1k output tokens in USD (default: 0.0)
    #[serde(default)]
    pub cost_per_1k_output: f32,
    /// LLM provider: "ollama" (default), "openai", "anthropic"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// API key for cloud providers (empty for Ollama)
    #[serde(default)]
    pub api_key: String,
    /// API base URL override (e.g., for OpenAI-compatible servers)
    #[serde(default)]
    pub api_base: Option<String>,
    /// Tool names disabled for prompt injection and execution.
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    /// Disabled skill names (not injected / not attachable). Persisted state.
    #[serde(default)]
    pub disabled_skills: Vec<String>,
    /// Curateur (background auto-creation of verified skills/tools). Persistent toggle
    /// driven from Settings; env fallback `RUCHE_CURATEUR=1`. Off by default (anti-bloat).
    #[serde(default)]
    pub curateur_actif: bool,
    /// Age au-dela duquel un episode est efface, en jours. `0` = on garde tout.
    ///
    /// Les episodes sont la trace de ce que l'agent a fait, un noeud par mission.
    /// Ils s'accumulent sans fin: apres quelques mois d'usage quotidien la carte
    /// cognitive est surtout faite de comptes rendus de missions, et le rappel
    /// remonte des souvenirs de l'an dernier plutot que le fait qu'on cherchait.
    /// Zero par defaut, parce qu'effacer la memoire de quelqu'un sans le lui
    /// demander ne se fait pas.
    #[serde(default)]
    pub episodes_retention_jours: u32,
    /// Le decor que l'agent dessine quand il pilote: cadre ambre, panneau
    /// flottant, curseur qui glisse, frappe progressive.
    ///
    /// Actif par defaut, et ce n'est pas cosmetique: une machine qui bouge toute
    /// seule sans rien annoncer est inquietante, et le panneau est le seul
    /// endroit ou l'humain lit ce qui est vise pendant que ca se produit.
    ///
    /// On le coupe pour trois raisons legitimes: une capture propre, une longue
    /// sequence ou l'animation coute plus qu'elle n'apporte, et une machine
    /// distante que personne ne regarde.
    #[serde(default = "vrai")]
    pub halo_actif: bool,
    /// Does LaRuche expose its OWN tools as an MCP server, so an external client can drive
    /// it? Off by default: that surface hands the whole registry to whoever reaches the
    /// port, shell_exec and file_write included, and it is not needed in order to USE
    /// other MCP servers.
    #[serde(default)]
    pub mcp_server_actif: bool,
    /// IP allowlist for the MCP server surface. Off by default so turning the server on
    /// keeps the behaviour it had; once on, an address that is not on the list is refused
    /// before any tool is looked up, and the refusal is audited. Entries are plain
    /// addresses (`192.168.1.10`, `::1`) or CIDR blocks (`192.168.1.0/24`).
    #[serde(default)]
    pub mcp_pare_feu_actif: bool,
    #[serde(default)]
    pub mcp_ip_autorisees: Vec<String>,
    /// Require `x-laruche-mcp-token` on the MCP surfaces. Off = loopback is trusted,
    /// which means any local process can call the tools, `shell_exec` included.
    #[serde(default)]
    pub mcp_token_actif: bool,
    /// The expected token. Generated in the UI, never shown in a log or an error: a
    /// refusal says "bad token", never which one was expected.
    #[serde(default)]
    pub mcp_token: String,
    /// **Smart approvals**: an auxiliary LLM judges a flagged call before bothering
    /// the human (approve / deny / escalate), and approving once approves the whole
    /// PATTERN CLASS for the session. On by default: it removes most popups while
    /// ADDING a check on the autonomous path (which used to execute blindly).
    #[serde(default = "vrai")]
    pub smart_approvals: bool,
    /// Fail-closed: when no human is reachable (cron, scout) and the call is still
    /// unresolved, REFUSE instead of executing. Off by default - LaRuche's
    /// autonomous runs must keep working - but recommended for exposed nodes.
    #[serde(default)]
    pub approbation_stricte: bool,
    /// **Reasoning effort** for thinking-capable models (`minimal|low|medium|high|max|
    /// ultra`, provider-dependent). Empty = the provider default. Mapped per backend:
    /// `reasoning_effort` (OpenAI), a thinking budget (Anthropic), `reasoning.effort`
    /// (Codex); ignored by backends that have no such knob.
    #[serde(default)]
    pub reasoning_effort: String,
    /// Effort for AUXILIARY tasks (curateur, judge, compaction, memory extraction).
    /// Empty = no thinking: a background pass must never burn a deep-reasoning
    /// budget. Kept separate from the main one on purpose.
    #[serde(default)]
    pub reasoning_effort_aux: String,
    /// Origin channel of the current run (e.g. `telegram:12345`, `discord:bob`, `web`). Runtime
    /// only (never persisted): lets tools (`cron_create`) know where the request came from
    /// and route the recurring output back there.
    #[serde(skip)]
    pub origin_channel: Option<String>,
    /// Home channel (set by the user via `/sethome`): default destination for proactive
    /// messages (cron/missions) when no origin channel is known. Persisted.
    #[serde(default)]
    pub home_channel: Option<String>,
    /// Let the agent leave an emoji reaction on the user's message (`>>up`, `>>haha`...).
    ///
    /// OFF by default, and it should stay a choice. It spends instruction budget on
    /// every turn for something decorative, and a marker syntax the model must emit
    /// and we must strip is the class of bug that gave us a hallucinated `$TOOL_NAME`
    /// and a `[SYSTEM]` paragraph leaking into an episode title. A small model will
    /// put the marker mid-sentence sooner or later.
    #[serde(default)]
    pub reactions_agent: bool,
    /// Dynamically inject only the most relevant Abeilles into the prompt.
    #[serde(default)]
    pub dynamic_tool_selection: bool,
    /// Maximum tool schemas injected when dynamic selection is enabled.
    #[serde(default = "default_tool_selection_limit")]
    pub tool_selection_limit: usize,
    /// Stable, query-INDEPENDENT toolset (profile) -> identical prefix from one turn to the next,
    /// so the prefix cache is reusable. Combine with `dynamic_tool_selection`.
    #[serde(default)]
    pub stable_toolset: bool,
    /// Lever 2 - tools deemed relevant for THIS turn (semantically retrieved from the
    /// cognitive map `tools.abeilles.*`). If `Some`, inject the minimal core + these,
    /// instead of the ~30 schemas. `None` = legacy behavior. Filled per turn, not persisted.
    #[serde(skip)]
    pub relevant_tools: Option<Vec<String>>,
    /// Editable identity (node `system.prompt`). If `Some`+non-empty, replaces the hardcoded identity.
    /// Filled per turn (hot-reload). The protocol stays locked.
    #[serde(skip)]
    pub system_prompt_override: Option<String>,
    /// Editable behavior (node `system.behavior`). Same idea, replaces the default behavior.
    #[serde(skip)]
    pub behavior_override: Option<String>,
    /// Editable planning section (node `system.prompt_planning`). Hot-reload.
    #[serde(skip)]
    pub planning_override: Option<String>,
    /// Compact index of available skills (`name - description`), built per turn from the
    /// cognitive map. Always injected in the stable prefix so the model knows its full
    /// repertoire (body via `skill_view` on demand). `None` outside memory context.
    #[serde(skip)]
    pub skills_index: Option<String>,
    /// List of reachable mesh hives (`name - laruche_id`), injected so the agent can
    /// contact (`mesh_send`) / coordinate them. Filled by the node (listener access). `None` if solo.
    #[serde(skip)]
    pub mesh_peers_hint: Option<String>,
    /// Auxiliary model for background tasks (curation/extraction). `None` = same model.
    /// Pointing at a small fast model avoids competing with the main chat's KV cache.
    #[serde(default)]
    pub aux_model: Option<String>,
    /// Threshold (tokens) below which the context is deemed "narrow" -> dynamic selection of tools
    /// AND of the skill catalog (the semantic DB surfaces only what's relevant). Tunable.
    #[serde(default = "default_dynamic_context_threshold")]
    pub dynamic_context_threshold: u32,
    #[serde(default = "default_permission_mode")]
    pub permission_mode: PermissionMode,
    #[serde(default)]
    pub permission_rules: Vec<PermissionRule>,
    /// LaReine supervisor settings, mirrored from `laruche-reine.json` and set per
    /// turn by the node. Off by default (no effect on normal operation).
    #[serde(default)]
    pub reine: ReineConfig,
    #[serde(skip)]
    pub credential_pool:
        Option<std::sync::Arc<tokio::sync::RwLock<crate::credential_pool::CredentialPool>>>,
}

/// LaReine settings as carried by the engine (a serde-friendly mirror of the
/// node's `ReineSettings`). Maps to the pure [`laruche_butinage::cap::reine::ConfigReine`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReineConfig {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub max_revues: u8,
    #[serde(default)]
    pub seuil_confiance: u8,
    #[serde(default)]
    pub tier_reponse: bool,
    #[serde(default)]
    pub tier_artefacts: bool,
    #[serde(default)]
    pub tier_supervision: bool,
    #[serde(default)]
    pub queue_gate: bool,
    #[serde(default)]
    pub provider_profile: Option<String>,
}

impl ReineConfig {
    /// Convert to the pure decision config.
    pub fn to_core(&self) -> laruche_butinage::cap::reine::ConfigReine {
        use laruche_butinage::cap::reine::{ConfigReine, ModeReine};
        ConfigReine {
            mode: ModeReine::depuis_str(&self.mode),
            max_revues: self.max_revues,
            seuil_confiance: if self.seuil_confiance == 0 {
                60
            } else {
                self.seuil_confiance
            },
            tier_reponse: self.tier_reponse,
            tier_artefacts: self.tier_artefacts,
            tier_supervision: self.tier_supervision,
        }
    }

    /// Is response review (Tier 1) active?
    pub fn actif_reponse(&self) -> bool {
        let c = self.to_core();
        c.active() && c.tier_reponse
    }
}

fn default_provider() -> String {
    "ollama".to_string()
}

fn default_tool_selection_limit() -> usize {
    10
}

fn default_permission_mode() -> PermissionMode {
    PermissionMode::Default
}

fn default_dynamic_context_threshold() -> u32 {
    40_000
}

/// serde default for opt-OUT booleans (absent in an old config file = enabled).
fn vrai() -> bool {
    true
}

impl Default for EssaimConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://127.0.0.1:11434".to_string(),
            model: "gemma4:e4b".to_string(),
            fallback_models: vec![],
            max_iterations: 100,
            temperature: 0.7,
            max_tokens: 0, // 0 = no limit (natural model stop)
            custom_instructions: None,
            context_max_messages: 30,
            context_max_tokens: 128000,
            compaction_threshold: 0.75,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            review_model: None,
            provider: "ollama".to_string(),
            api_key: String::new(),
            api_base: None,
            disabled_tools: Vec::new(),
            disabled_skills: Vec::new(),
            curateur_actif: false,
            episodes_retention_jours: 0,
            halo_actif: true,
            // Off unless the user turns it on: exposing the registry is a decision.
            mcp_server_actif: false,
            mcp_pare_feu_actif: false,
            mcp_ip_autorisees: Vec::new(),
            mcp_token_actif: false,
            mcp_token: String::new(),
            smart_approvals: true,
            approbation_stricte: false,
            reasoning_effort: String::new(),
            reasoning_effort_aux: String::new(),
            origin_channel: None,
            home_channel: None,
            reactions_agent: false,
            dynamic_tool_selection: false,
            tool_selection_limit: default_tool_selection_limit(),
            stable_toolset: false,
            relevant_tools: None,
            system_prompt_override: None,
            behavior_override: None,
            planning_override: None,
            skills_index: None,
            mesh_peers_hint: None,
            aux_model: None,
            dynamic_context_threshold: default_dynamic_context_threshold(),
            permission_mode: default_permission_mode(),
            permission_rules: Vec::new(),
            reine: ReineConfig::default(),
            credential_pool: None,
        }
    }
}

/// L'etat du decor, lisible depuis les outils.
///
/// Les outils sont des valeurs sans etat, atteintes par un registre statique:
/// ils ne recoivent ni la configuration ni l'etat du noeud, et `ContextExecution`
/// ne porte que ce qui concerne la session. Passer la config jusqu'a eux
/// demanderait de la faire traverser tout le chemin d'execution pour un seul
/// booleen. Un atome partage dit la meme chose, coute une lecture, et se met a
/// jour depuis les reglages sans redemarrage.
static HALO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Le decor doit-il etre dessine? Defaut des parametres `glow` des outils.
pub fn halo_actif() -> bool {
    HALO.load(std::sync::atomic::Ordering::Relaxed)
}

/// Applique le reglage, au demarrage et a chaque changement dans les reglages.
pub fn definir_halo(actif: bool) {
    HALO.store(actif, std::sync::atomic::Ordering::Relaxed);
}
