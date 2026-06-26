//! # Pont entre `laruche-essaim` et le moteur `laruche-butinage`.
//!
//! Implémente les traits du moteur (`Fournisseur`, `Outils`, `Emetteur`) à partir
//! des briques existantes (providers, `AbeilleRegistry`, `ChatEvent`), et expose
//! [`executer`] : la façade appelée par `boucle_react_multimodal_ext` quand le flag
//! `RUCHE_MOTEUR=butinage` est actif. L'ancien moteur (`brain.rs`) reste intact.

use crate::abeille::{AbeilleRegistry, ContextExecution};
use crate::brain::{
    demande_recherche_longue, parse_tool_calls, schema_outils_pour_prompt, ChatEvent, EssaimConfig,
};
use crate::prompt::build_system_prompt;
use crate::providers::{provider_chat_stream, ProviderError};
use crate::session::Session;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use laruche_butinage as but;
use laruche_memoire::MemoireCognitive;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;

// ───────────────────────── Fournisseur (LLM) ─────────────────────────

struct FournisseurPont {
    provider: String,
    model: String,
    api_key: String,
    api_base: Option<String>,
    ollama_url: String,
    temperature: f32,
    max_tokens: u32,
    tx: broadcast::Sender<ChatEvent>,
}

#[async_trait]
impl but::Fournisseur for FournisseurPont {
    async fn repondre(
        &self,
        messages: &[but::Message],
        schemas: &[serde_json::Value],
    ) -> std::result::Result<but::ReponseModele, but::ErreurFournisseur> {
        let msgs = convertir_messages(messages);
        let tools = if schemas.is_empty() { None } else { Some(schemas) };

        let mut stream = match provider_chat_stream(
            &self.provider,
            &self.model,
            &msgs,
            self.temperature,
            self.max_tokens,
            &self.api_key,
            self.api_base.as_deref(),
            &self.ollama_url,
            tools,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => return Err(classer_erreur(e)),
        };

        let mut texte = String::new();
        let mut finish: Option<String> = None;
        let mut natifs: Option<Vec<crate::brain::ToolCall>> = None;

        while let Some(chunk) = stream.next().await {
            if chunk.finish_reason.is_some() {
                finish = chunk.finish_reason.clone();
            }
            if chunk.tool_calls.is_some() {
                natifs = chunk.tool_calls.clone();
            }
            if !chunk.text.is_empty() {
                texte.push_str(&chunk.text);
                let _ = self.tx.send(ChatEvent::Token { text: chunk.text.clone() });
            }
        }

        // Appels : natifs (API) sinon parsés du texte (rail pour modèles faibles).
        let appels: Vec<but::Appel> = match natifs {
            Some(tcs) if !tcs.is_empty() => tcs.into_iter().map(appel_depuis_toolcall).collect(),
            _ => parse_tool_calls(&texte)
                .into_iter()
                .map(appel_depuis_toolcall)
                .collect(),
        };

        let stop = classer_stop(finish.as_deref(), &appels);
        Ok(but::ReponseModele {
            texte: retirer_think(&texte),
            stop,
            appels,
            usage: None,
        })
    }
}

fn convertir_messages(messages: &[but::Message]) -> Vec<serde_json::Value> {
    use but::Role;
    messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::Systeme => "system",
                Role::Utilisateur | Role::Observation => "user",
                Role::Assistant => "assistant",
            };
            let contenu = match m.role {
                Role::Observation => format!(
                    "[Tool Result: {}]\n{}",
                    m.outil.as_deref().unwrap_or("tool"),
                    m.contenu
                ),
                _ => m.contenu.clone(),
            };
            serde_json::json!({ "role": role, "content": contenu })
        })
        .collect()
}

fn appel_depuis_toolcall(tc: crate::brain::ToolCall) -> but::Appel {
    but::Appel {
        id: tc.id,
        nom: tc.name,
        args: tc.args,
    }
}

fn classer_stop(finish: Option<&str>, appels: &[but::Appel]) -> but::StopReason {
    match finish {
        Some("length") | Some("max_tokens") => but::StopReason::Longueur,
        Some("tool_calls") | Some("tool_use") => but::StopReason::Outils,
        _ if !appels.is_empty() => but::StopReason::Outils,
        Some("stop") | Some("end_turn") | None => but::StopReason::FinTour,
        _ => but::StopReason::Autre,
    }
}

fn classer_erreur(e: anyhow::Error) -> but::ErreurFournisseur {
    if let Some(pe) = e.downcast_ref::<ProviderError>() {
        but::ErreurFournisseur {
            status: pe.status,
            retry_after: pe.retry_after.clone(),
            corps: pe.body.clone(),
        }
    } else {
        // Pas de status HTTP → erreur de transport : on la traite comme passagère (0).
        but::ErreurFournisseur {
            status: 0,
            retry_after: None,
            corps: e.to_string(),
        }
    }
}

/// Retire les blocs `<think>…</think>` (raisonnement interne de certains modèles).
fn retirer_think(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut reste = t;
    while let Some(deb) = reste.find("<think>") {
        out.push_str(&reste[..deb]);
        if let Some(fin) = reste[deb..].find("</think>") {
            reste = &reste[deb + fin + "</think>".len()..];
        } else {
            reste = "";
            break;
        }
    }
    out.push_str(reste);
    out.trim().to_string()
}

// ───────────────────────── Outils (registre) ─────────────────────────

struct OutilsPont<'a> {
    registry: &'a AbeilleRegistry,
    working_dir: Option<PathBuf>,
    disabled: Vec<String>,
    tx: broadcast::Sender<ChatEvent>,
}

#[async_trait]
impl but::Outils for OutilsPont<'_> {
    async fn executer(&self, appel: &but::Appel) -> but::ResultatOutil {
        if self.disabled.iter().any(|d| d == &appel.nom) {
            return but::ResultatOutil::echec("tool disabled in Settings");
        }
        // Événement riche (args complets) pour le dashboard.
        let _ = self.tx.send(ChatEvent::ToolCall {
            name: appel.nom.clone(),
            args: appel.args.clone(),
            iteration: None,
        });

        let mut ctx = ContextExecution::default();
        if let Some(wd) = &self.working_dir {
            ctx.working_dir = wd.clone();
        }

        let t0 = Instant::now();
        let res = match self
            .registry
            .executer(&appel.nom, appel.args.clone(), &ctx)
            .await
        {
            Ok(r) => {
                if r.success {
                    but::ResultatOutil::ok(r.output)
                } else {
                    but::ResultatOutil::echec(r.error.unwrap_or_else(|| "Unknown".into()))
                }
            }
            Err(e) => but::ResultatOutil::echec(format!("tool error: {e}")),
        };
        let ms = t0.elapsed().as_millis() as u64;

        let _ = self.tx.send(ChatEvent::ToolResult {
            name: appel.nom.clone(),
            result: res.sortie.clone(),
            success: res.ok,
            elapsed_ms: Some(ms),
        });
        res
    }

    fn idempotent(&self, nom: &str) -> bool {
        est_lecture_seule(nom)
    }

    fn schemas(&self) -> Vec<serde_json::Value> {
        match self.registry.schema_complet() {
            serde_json::Value::Array(a) => a
                .into_iter()
                .filter(|t| {
                    t.get("name")
                        .and_then(|v| v.as_str())
                        .map(|n| !self.disabled.iter().any(|d| d == n))
                        .unwrap_or(true)
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Outils en lecture seule (sûrs en parallèle, surveillés pour la stagnation).
fn est_lecture_seule(nom: &str) -> bool {
    nom.starts_with("web_")
        || nom.starts_with("memory_search")
        || nom.starts_with("file_read")
        || nom.starts_with("file_list")
        || nom.starts_with("file_search")
        || nom.starts_with("read_extract")
        || nom.starts_with("session_search")
        || nom.starts_with("git_status")
        || nom.starts_with("git_diff")
        || nom.starts_with("git_log")
        || nom == "skill_view"
        || nom == "skill_list"
}

// ───────────────────────── Emetteur (events) ─────────────────────────

struct EmetteurPont {
    tx: broadcast::Sender<ChatEvent>,
}

impl but::Emetteur for EmetteurPont {
    fn emettre(&self, ev: but::Evenement) {
        use but::Evenement as E;
        let ce = match ev {
            E::Statut(m) => ChatEvent::Status { message: m },
            E::Escale { avant, apres } => ChatEvent::Compaction {
                messages_before: avant,
                messages_after: apres,
            },
            E::Fin(t) => ChatEvent::Done { full_response: t },
            // Tokens, appels et résultats d'outils sont déjà émis (plus riches) par
            // FournisseurPont / OutilsPont → on évite les doublons.
            E::Texte(_) | E::AppelOutil { .. } | E::ResultatOutil { .. } => return,
        };
        let _ = self.tx.send(ce);
    }
}

// ───────────────────────── Façade ─────────────────────────

fn profil_pour(config: &EssaimConfig) -> but::ProfilModele {
    match config.provider.as_str() {
        "anthropic" | "codex" => but::ProfilModele::NatifOutils,
        _ => {
            let m = config.model.to_lowercase();
            if m.contains("e4b") || m.contains(":2b") || m.contains("1b") || m.contains("phi") {
                but::ProfilModele::Fragile
            } else {
                but::ProfilModele::Robuste
            }
        }
    }
}

/// Exécute la mission via le moteur `butinage` puis recompose la session (persistance/UI).
pub async fn executer(
    prompt_utilisateur: &str,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    tx: &broadcast::Sender<ChatEvent>,
    ephemeral_context: &Option<String>,
    _memoire: &Option<Arc<dyn MemoireCognitive>>,
) -> Result<String> {
    let _ = tx.send(ChatEvent::Status {
        message: "Moteur butinage actif (RUCHE_MOTEUR=butinage).".into(),
    });

    // System prompt : on réutilise les assembleurs existants (tier stable).
    let tool_schema = schema_outils_pour_prompt(registry, config, prompt_utilisateur);
    let mut systeme = build_system_prompt(
        &tool_schema,
        config.system_prompt_override.as_deref(),
        config.behavior_override.as_deref(),
        None,
        config.custom_instructions.as_deref(),
    );
    if let Some(ctx) = ephemeral_context {
        systeme.push_str(&format!(
            "\n\n[Mémoire cognitive — souvenirs pertinents pour cette requête]\n{ctx}"
        ));
    }

    let mode = if demande_recherche_longue(prompt_utilisateur) {
        but::ModeMission::Exploration
    } else {
        but::ModeMission::Standard
    };

    let reglages = but::Reglages {
        plafond_passes: config.max_iterations.max(1),
        systeme,
        profil: profil_pour(config),
        ..but::Reglages::default()
    };

    let mut carnet = but::Carnet::ouvrir(prompt_utilisateur, mode, chrono::Utc::now());

    let four = FournisseurPont {
        provider: config.provider.clone(),
        model: config.model.clone(),
        api_key: config.api_key.clone(),
        api_base: config.api_base.clone(),
        ollama_url: config.ollama_url.clone(),
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        tx: tx.clone(),
    };
    let outils = OutilsPont {
        registry,
        working_dir: session.working_dir.clone(),
        disabled: config.disabled_tools.clone(),
        tx: tx.clone(),
    };
    let emet = EmetteurPont { tx: tx.clone() };

    let bilan = but::butiner(&mut carnet, &reglages, &four, &outils, &emet).await?;

    // Recompose la session depuis le carnet (persistance disque + relecture UI).
    for m in &carnet.historique {
        match m.role {
            but::Role::Utilisateur => session.ajouter_user(&m.contenu),
            but::Role::Assistant if !m.contenu.is_empty() => session.ajouter_assistant(&m.contenu),
            but::Role::Observation => {
                session.ajouter_observation(m.outil.as_deref().unwrap_or("tool"), &m.contenu)
            }
            _ => {}
        }
    }

    Ok(bilan.texte)
}
