# LaRuche v2 — Dossier complet de reprise DEV

> **Compilé le 2026-06-22.** Fusion exhaustive de tous les `.md` du projet : LARUCHE_V2.md,
> HANDOFF_GLOBAL.md, VISION_INNOVATION.md, UX_REFACTO.md, BRIEFING_GEMINI*.md, CHANGELOG.md,
> README.md, README_FR.md. **Rien n'a été sacrifié.**
>
> **Règle de priorité** : en cas de contradiction entre sections, la section "État réel consolidé"
> (§4) fait foi — elle est vérifiée sur le code du 2026-06-20 + sessions suivantes.

---

## Table des matières

- [1. Identité & pitch produit](#1-identité--pitch-produit)
- [2. Environnement & règles NON NÉGOCIABLES](#2-environnement--règles-non-négociables)
- [3. Architecture workspace (14 crates)](#3-architecture-workspace-14-crates)
- [4. État réel consolidé (ce qui est FAIT)](#4-état-réel-consolidé-ce-qui-est-fait)
- [5. Ce qui RESTE — récapitulatif actionnable](#5-ce-qui-reste--récapitulatif-actionnable)
  - [5.1 Travaux UI (Gemini) — contrats prêts](#51-travaux-ui-gemini--contrats-prêts)
  - [5.2 Chantiers Rust (Claude/Codex) — par priorité](#52-chantiers-rust-claudecodex--par-priorité)
- [6. Briefings UI détaillés](#6-briefings-ui-détaillés)
  - [6.1 Boucle d'apprentissage (BRIEFING_GEMINI)](#61-boucle-dapprentissage-briefing_gemini)
  - [6.2 Dashboard services Axe 5 (BRIEFING_GEMINI_AXE5)](#62-dashboard-services-axe-5-briefing_gemini_axe5)
  - [6.3 Providers visibilité + Utiliser (BRIEFING_GEMINI_PROVIDERS)](#63-providers-visibilité--utiliser-briefing_gemini_providers)
  - [6.4 Capacités STT/TTS/Coding (BRIEFING_GEMINI_CAPABILITES)](#64-capacités-sttttsco​ding-briefing_gemini_capabilites)
  - [6.5 Cron provider (BRIEFING_GEMINI_CRON_PROVIDER)](#65-cron-provider-briefing_gemini_cron_provider)
- [7. Vision & différenciation](#7-vision--différenciation)
- [8. Architecture "La Reine" (orchestration long-horizon)](#8-architecture-la-reine-orchestration-long-horizon)
- [9. Refacto UX / Navigation cible](#9-refacto-ux--navigation-cible)
- [10. API Reference complète](#10-api-reference-complète)
- [11. Pièges connus](#11-pièges-connus)
- [12. Build & Run](#12-build--run)
- [13. Changelog](#13-changelog)
- [14. Archives historiques (textes intégraux)](#14-archives-historiques-textes-intégraux)

---

# 1. Identité & pitch produit

**LaRuche + L'Essaim** — Plateforme agent IA open-source. **Local-first, privacy-focused.**

> *"Branchez l'IA. C'est tout."*

Branchez un nœud LaRuche sur votre réseau local et l'IA devient disponible pour tous les appareils
connectés. Zéro configuration, zéro dépendance cloud, privacy-first by design.

**Pitch différenciateur :**
> *"LaRuche : le premier agent local-first à **mémoire cognitive partagée**. Pas une fenêtre de
> contexte qu'on compacte — un **cerveau** qui apprend, se cite, et se **synchronise entre tes
> machines en local**. Plus tu l'utilises, plus la ruche est intelligente."*

**La thèse technique :**
Tous les concurrents se battent contre la fenêtre de contexte (empilent puis compactent/résument
quand ça déborde → perte d'info). LaRuche a **deux actifs qu'aucun n'a réunis** :
- **Mémoire cognitive SQL auditée** (FTS5 + embeddings + activation + dream + audit)
- **Mesh local-first** (Miel/mDNS)

> *La carte cognitive EST le cerveau. La fenêtre LLM n'est qu'une mémoire de travail transitoire,
> reconstruite à chaque tour par activation. On ne compacte pas le contexte — on le **récupère**.*

**Comparatif topologique :**

| Axe | third-party / third-party | Claude Code | LaRuche v2 |
|---|---|---|---|
| Installation | uv + Python + Node + ffmpeg | CLI Node.js | **1 binaire Rust** |
| Mémoire | `MEMORY.md` plat + FTS5 | fenêtre + compaction | **Carte cognitive auditée** |
| Topologie | 1 cerveau sur VPS | local mono-session | **Mesh local-first** (mDNS/swarm) |
| Interfaces | CLI + Telegram | TUI uniquement | **SPA, TUI, Telegram natif, VS Code** |
| Boucle apprentissage | mature (curator + bg_review) | — | présent, à finaliser |

---

# 2. Environnement & règles NON NÉGOCIABLES

- **Dépôt** : `C:\Users\infinition\Desktop\laruche-v2\laruche` (workspace Rust, 14 crates)
- **Pas de git** : l'utilisateur gère le versionnement. Ne pas commit/push.
- **Toolchain MSVC épinglée** (`laruche/rust-toolchain.toml`). Ne JAMAIS changer. (`gnu` KO : dlltool absent.)
- **Windows verrouille `laruche-node.exe`** s'il tourne → tuer le process AVANT `cargo run`/`cargo build`. Alternative : `cargo check`.
- **Conventions FR** dans le code (abeille, mémoire, skill, essaim…).
- **VÉRIFIER AVANT DE DIRE FINI** : `cargo check -p <crate>` ou `cargo test` doivent être verts. Ne jamais livrer de code non compilé.
- **Build/run** : `cd laruche ; cargo run -p laruche-node` → `http://localhost:8419`
- **Zone de travail UI** : `laruche-dashboard/src/templates/spa.html` UNIQUEMENT (via `include_str!` → rebuild `cargo run -p laruche-node` après chaque edit).
- **Zone de travail backend** : `laruche-essaim/src/*`, `laruche-node/src/*.rs`, crates `laruche-*`.
- **LLM de test local** : llama.cpp sur `http://localhost:8001` (OpenAI-compatible), modèle `qwen3.6-35b-a3b`.
- **Pas de npm dans le PATH** (Node v24 existe en `%LOCALAPPDATA%/ms-playwright-go/1.57.0/node.exe` mais sans npm).

**Répartition des zones (pour bosser en parallèle SANS collision) :**

| Agent | Zone exclusive |
|---|---|
| **Claude / Codex** (Rust cœur) | `laruche-essaim/src/*` (brain, providers, cron, abeilles…), `laruche-node/src/*.rs` (logique, endpoints), crates `laruche-*` |
| **Gemini** (UI) | `laruche-dashboard/src/templates/spa.html` UNIQUEMENT (+ rebuild node après edit) |

---

# 3. Architecture workspace (14 crates)

```text
laruche-v2/
  laruche/                      ← workspace Rust (14 crates)
    miel-protocol/              # Protocole Miel (mDNS, auth, QoS, swarm)
    laruche-node/               # Serveur daemon (API, WebSocket, channels, auth, sync, TUI)
    laruche-essaim/             # Moteur agent (cerveau ReAct, 32 Abeilles, sessions, RAG, providers)
    laruche-cli/                # Client CLI avec TUI (Ratatui)
    laruche-client/             # SDK client Rust
    laruche-dashboard/          # Templates web (spa.html, embarqué via include_str!)
    laruche-memoire/            # Trait MemoireCognitive + 3 backends
    laruche-compaction/         # Auto-compaction + micro-compaction + compression trajectoire
    laruche-skills/             # Système de skills OKF
    laruche-permissions/        # Modes Default/Plan/AcceptEdits/Auto
    laruche-kanban/             # Board SQLite + dispatcher + orchestrateur
    laruche-events/             # Bus NDJSON d'audit
    laruche-watchers/           # Registre de triggers persistants
    laruche-suggestions/        # ⚠️ ORPHELIN (0 réf) — câbler ou supprimer
  laruche-voix/                 # Python, GARDÉ — service STT (Whisper) + TTS (piper) + annonce Miel
  laruche-channels/             # Python, GARDÉ (référence) — bots Telegram/Discord/Slack
  laruche-vscode/               # Node, GARDÉ — extension VS Code (futur pont)
```

**Taille des crates (audit 2026-06-20) :**

| Crate | Lignes | État |
|---|---:|---|
| `laruche-essaim` | 13 872 | Cœur ReAct + 32 Abeilles |
| `laruche-node` | 10 162 | API/WS/UI/swarm |
| `laruche-cli` | 3 721 | Client TUI OK |
| `miel-protocol` | 2 586 | mDNS/swarm |
| `laruche-memoire` | 1 895 | Trait + 3 backends |
| `laruche-compaction` | 664 | Câblé essaim |
| `laruche-skills` | 464 | Câblé node (OKF) |
| `laruche-permissions` | 390 | Câblé essaim |
| `laruche-kanban` | 343 | Câblé node+essaim |
| `laruche-client` | 279 | Lib client |
| `laruche-watchers` | 252 | Câblé node — **boucle autonome à vérifier** |
| `laruche-events` | 225 | Câblé node (bus audit) |
| `laruche-suggestions` | 71 | ⚠️ **ORPHELIN** (0 réf) |
| `laruche-dashboard` | 7 + spa.html (5 735 l.) | SPA mono-fichier vanilla |

**Couches logiques :**

```
┌─────────────────────────── LaRuche v2 ────────────────────────────┐
│  CORPS (Rust)                              MÉMOIRE                  │
│  laruche-node : API, WS, auth, swarm       laruche-memoire          │
│  laruche-essaim : boucle ReAct + Abeilles  (trait + 3 backends)    │
│  miel-protocol : mDNS, swarm, QoS                                  │
│  laruche-cli / dashboard / vscode / voix   COGNITION               │
│  laruche-channels : Telegram/Discord/Slack  laruche-skills (OKF)   │
│                                             laruche-kanban           │
│                                             laruche-watchers         │
│                                             dream/curator (idle)    │
│                                             background-review (post) │
└────────────────────────────────────────────────────────────────────┘
```

**Les 32 Abeilles (outils) :**
`browser · calendrier · clarify · delegation · essaim_status · execute_code · fichiers · file_watch · git · image_search · kanban_next · knowledge · lsp · math · mcp_resources · mcp_tool · media · memoire · mixture · plan_mode · plugins · read_extract · recherche_fichiers · reload_plugins · run_script · shell · todo · web_deep · web_fetch · web_recherche · worktree`

---

# 4. État réel consolidé (ce qui est FAIT)

> Vérifié sur le code au 2026-06-20 + sessions du handoff du 2026-06-21.
> **`cargo check --workspace` → exit 0** (4 warnings d'imports). 14 crates, ~35 000 lignes Rust, 32 Abeilles, 149 tests.

## 4.1 Cœur agentique (Phases 0-6 — toutes ✅)

- **Boucle ReAct** + orchestration partition safe/unsafe (`brain.rs`)
- **Budget tokens** (`budget.rs`) + budget résultat d'outil (`tool_budget.rs`) + auto/micro-compaction (`laruche-compaction`)
- **Récupération maxOutputTokens** + repli modèle (`providers.rs`, `streaming.rs`)
- **Résumé d'outils volumineux** via modèle aux (`tool_summary.rs`)
- **Steering / interruption mid-turn** (canal WS non bloquant) ✅
- **Sous-agents** à contexte isolé (`subagent.rs`) + `POST /api/agents/spawn` + UI arbre
- **Prompt système 3 tiers** cache-friendly (`prompt.rs`)

## 4.2 Robustesse/sécurité (Phase 7 — toutes ✅)

- **Classifieur d'erreurs** (`error_classifier.rs`) branché au retry (rotation de clé sur 429)
- **Credential pool** multi-comptes + cooldown 429 (`credential_pool.rs`) + UI multi-compte
- **Threat patterns** anti-injection branchés au dispatch (`threat_patterns.rs`) ✅
- Mixture of Agents (`mixture.rs`), compression de trajectoire, notifier proactif (Telegram)

## 4.3 Autonomie vivante (Phase 9 — ✅)

- Canal + modèle par tâche cron
- Daemon cron qui route le feedback
- **background_review forké** (auto-skill post-tour)
- **Curator/dream à l'inactivité** (sans suppression, archive recouvrable)
- Goals multi-tours

## 4.4 Skills & Kanban (Phases 10/11 — ✅)

- Skill = document OKF vectoriel (`skill_list`/`skill_view` à divulgation progressive)
- Injection des skills au run cron
- **Orchestrateur kanban** (tire les `Ready` tout seul)
- **Blueprints** (automatisations 1-clic)
- Distinction `ToolOrigin::Builtin/Custom`

## 4.5 Manques comblés vs concurrents

- **LSP** (`lsp.rs`) ✅
- **Isolation Git worktree** (`worktree.rs`) ✅
- Outils média via plugins ✅
- **Vision Multimodal (VLM) & MCP Computer Use** ✅
- `read_extract` (PDF/MD) ✅
- `web_deep_search` ✅
- Hot-reload plugins ✅

## 4.6 Mémoire (la fusion)

- **Trait `MemoireCognitive`** + 3 backends :
  - `NativeBackend` (RAM lexical)
  - `SqliteBackend` (SQLite+FTS5+audit+embeddings hybride)
  - `SidecarBackend` (HTTP→paradigm, code mort optionnel)
- Sélection par `LARUCHE_MEMOIRE_BACKEND` dans `node`
- **Boucle ReAct mémoire** : auto-récupération (injection trailing prefix-cache) + auto-curation (tokio::spawn, modèle aux)
- **Sémantique** : `Embedder` + cosine + `OllamaEmbedder`, recherche hybride (0.7 cosinus + 0.3 lexical)
- **Cycle de maintenance** : update/delete/move/review/list_proposed/suggest_nodes + `stats` + `mutations` (audit)
- **UI Mémoire** : onglet SPA hexagones SVG + endpoints `/api/memory/*`

## 4.7 Providers & réseau (livré en session 2026-06-20/21)

- **Détection locale élargie** : llama.cpp/vLLM/LM Studio via `/v1/models` + var `LARUCHE_OPENAI_ENDPOINTS`
- `/swarm/models` enrichi (`is_local` + `capability`) — **résilient à Ollama down** (fix §3.6 du handoff)
- Annonce mDNS des modèles locaux au boot
- **Visibilité providers** : `Visibilite{Prive, PublicProxy}` (JSON `"visibility"`) — public = PROXY (clé reste locale)
- `POST /api/profiles/:id/visibility`
- `POST /api/models/use` (sélection 2-clics, crée profil local/miel)
- Provider `"miel"` routé OpenAI vers node distant

## 4.8 Sélection par capacité & voix (livré 2026-06-21)

- `CapabilitySelection{capability, model, backend, node_id, is_local, profile_id}`
- `GET /api/capabilities/selection`
- `/api/voice/status` honore le STT/TTS choisi (`selected_model`/`is_selected`)
- **Mode Coding** : chat WS lit `incoming["capability"]` → `"code"` route vers modèle de code (`appliquer_capacite`)
- Persisté au redémarrage

## 4.9 Cron provider via profil (livré)

- Helper unique `appliquer_profil(state, &mut config, profile_id, model)`
- `ScheduledTask.profile_id`
- Daemon cron résout via profil

## 4.10 Sessions persistantes (livré)

- Session rendue **visible + persistée dès l'envoi** (snapshot avec message user dans `essaim_sessions` + `sauvegarder()` disque)
- Agent tourne dans `tokio::spawn` détaché du WS → F5 ne l'arrête pas
- `Session` a `event_tx: Option<broadcast::Sender<ChatEvent>>` — reconnexion sur même `session_id` reçoit le flux en direct

## 4.11 Re-annonce mDNS périodique (livré)

- Boucle 120s qui ré-annonce le manifeste avec les modèles réels
- Corrige l'annonce du modèle figé (mistral)

## 4.12 Parité watcher/kanban provider (P1 — livré 2026-06-20)

- Daemons watcher + kanban résolvent provider/clé/base_url/modèle via `appliquer_profil(profile_id)`

## 4.13 Section MCP dans Settings (P2 — livré)

- `GET /api/mcp/servers`, `POST /api/mcp/servers/:name`, `DELETE /api/mcp/servers/:name`
- UI Gemini branchée (spa.html appelle ces 3)

## 4.14 Codex OAuth (T12 — fait, vérifié live 19/06)

- `codex_auth.rs` (20 KB, 4 tests)
- Provider `codex` (Responses API)
- Endpoints `/api/auth/codex/{status,start,logout}` dans node
- UI Settings
- Modèles : `gpt-5.5/5.4/5.4-mini`

---

# 5. Ce qui RESTE — récapitulatif actionnable

## 5.1 Travaux UI (Gemini) — contrats prêts

> Backend Rust implémenté, endpoints prêts, il ne manque que `spa.html`.

| Briefing | Statut | Priorité |
|---|---|---|
| `BRIEFING_GEMINI.md` — Boucle d'apprentissage : page Skills, file revue `proposed`, chips `skill_applied`/`skill_proposed` | À re-vérifier (Gemini dit fini) | Haute |
| `BRIEFING_GEMINI_AXE5.md` — Dashboard services mesh par catégorie (local vs mesh) | ⚠️ **Panneau actuellement vide** — rebuild requis (fix §3.6), re-tester | Haute |
| `BRIEFING_GEMINI_PROVIDERS.md` — Badges 🔒/🌐 + toggle visibilité + bouton « Utiliser » + avertissement clé publique | À re-vérifier | Haute |
| `BRIEFING_GEMINI_CAPABILITES.md` — Sections par capacité (LLM/Coding/STT/TTS/VLM/VLA) ; réglages voix ; toggle Mode Coding | À re-vérifier | Haute |
| `BRIEFING_GEMINI_CRON_PROVIDER.md` — Sélecteur Provider (profil) + modèle dans formulaire cron | **NON COMMENCÉ** | Haute |
| P7 — Réactivité UI : après action mutante, re-fetch + re-render en place sans F5 | Non commencé | Moyenne |
| P8 (partiel) — Notification cliquable (poller `GET /api/events?since=`) + live re-attach sur session `running` | Non commencé | Moyenne |
| UI watcher/kanban : sélecteurs provider dans les forms (même contrat que cron) | Non commencé | Basse |
| UX_REFACTO.md — Refonte navigation (Chat absorbe Sessions, Capacités, Automatisations, Timeline) | Non commencé | Future |

### Endpoints clés que l'UI consomme

```
GET  /swarm/models        → {models:[{host,name,capability,is_local,node_id,node_name,is_default}]}
GET  /api/profiles        → {profiles:{id:{provider,name,base_url,models,visibility}}, active_model}
POST /api/profiles/:id/visibility   {visibility: "prive"|"public_proxy"}
POST /api/models/use      {host,name,capability,node_id?,base_url?}
GET  /api/capabilities/selection
GET  /api/voice/status    → {stt:{available,url,selected_model,is_selected}, tts:{...}}
GET  /api/skills          → [{name,description,enabled}]
GET  /api/skills/:name    → SKILL.md (OKF) complet
POST /api/skills          body {content} OKF ou {name,content}
POST /api/skills/:name/toggle
DELETE /api/skills/:name
GET  /api/memory/proposed → file de revue
POST /api/memory/review   → accepter/rejeter
GET  /api/cron ; POST /api/cron {profile_id,model,...} ; PATCH /api/cron/:id {profile_id,model}
GET  /api/events?since=   → events AgentStarted/SessionFinished avec session_id

WS chat message champs : provider (profile id), model, capability ("code"...)
Events WS : skill_applied, skill_proposed (+ token, tool_call, tool_result, etc.)
```

## 5.2 Chantiers Rust (Claude/Codex) — par priorité

### P3 — Fermer la boucle réseau « miel » distant

Le provider `"miel"` route en OpenAI vers un node distant, MAIS le node n'expose aujourd'hui que
`/infer` (style Ollama), pas `/v1/chat/completions`.
→ **Exposer un endpoint chat OpenAI-compatible sur `laruche-node`** pour qu'un node consomme
réellement le LLM d'un autre. (Les backends locaux llama.cpp/vllm marchent déjà.)

### P6 — ⭐ Moteurs d'inférence custom auto-annoncés sur Miel

**(a) OpenAI-compat** : DÉJÀ ok via `LARUCHE_OPENAI_ENDPOINTS`. Documenter.

**(b) Auto-annonce Miel** (plug-and-play) : helper réutilisable pour broadcaster un
`CognitiveManifest` (capability + host/port) :
- Rust : mini-helper au-dessus de `miel-protocol::MielBroadcaster`
- Python : extraire `laruche-voix/src/miel_announce.py` en mini-lib réutilisable

**(c) Register côté node** (protocoles exotiques) : `POST /api/services/register {name,capability,url,protocol}` → ajouté à `/swarm/models` + annoncé sur manifeste ; `DELETE /api/services/register/:name` + UI Settings.

⚠️ **BUG à corriger d'abord** : `laruche-node/src/main.rs` (~l.1197, agrégation des nœuds Miel
distants) **skippe** `llm|code|vlm|embed|image` → un moteur custom annonçant `capability:llm` est
ignoré. **Relâcher ce filtre** (garder uniquement la dédup `already_listed`).

Bonus : couper les heartbeats « Ollama is not responding » quand Ollama n'est pas configuré.

### P7 — ⭐ Réactivité UI / MAJ dynamique (niveau 2 backend)

Option : canal `broadcast::Sender` dashboard (séparé du `ChatEvent` par session) qui émet
`{type:"state_changed", what:"models|profiles|capabilities|voice"}` ; handlers mutants l'émettent ;
WS/SSE dashboard le diffuse ; UI écoute et re-fetch le morceau touché.

Alternative simple : `GET /api/state/version` incrémenté à chaque mutation, que l'UI poll léger.

### P5 — Divers

- `cargo test --workspace` complet (jamais fait)
- Watchers : confirmer/finir la boucle d'événements autonome (fichier/RSS/webhook → tâche → notif)
- UI audit mémoire (stats/mutations/proposed déjà côté endpoints)
- `laruche-suggestions` orphelin (0 réf) : câbler ou supprimer
- third-party #10 `reasoning_effort`, #11 `cache_control` Anthropic
- Node CRUD mémoire (`create_node`/`update_node`/`delete_node`)
- OKF round-trip complet (`export_okf`/`import_okf`)
- Scraping JS-rendered robuste (`web_render` plugin Playwright)
- Notebook `.ipynb`
- Auth OTP/TOTP + login password (flux UI complet)
- Grep contenu dédié (regex sur contenu)

### L2 — Outils & skills sémantiques ⭐⭐⭐ (EN COURS)

Aujourd'hui : ~30 schémas d'outils injectés à chaque tour (même pour un « Salut »). Cible :
- Outils vivent dans la carte (`tools.abeilles.*`, `tools.skills.*`)
- Par tour = **noyau minuscule toujours présent** (memory_search, clarify, run_script) **+ 3-6 outils récupérés par intention**
- Résultat : agent peut avoir **1000 outils, en injecter 5**
- Briques existantes : abeilles en mémoire (T11 amorcé), skill_list/view, flag `dynamic_tool_selection`
- **Manque** : gater l'injection sur le retrieval

### L1 — Contexte cognitif infini (working-set, pas fenêtre à compacter)

À chaque tour, l'agent assemble un **set de travail minimal** depuis la carte (nœuds/items/skills/outils
activés, sous budget). Les vieux tours → **consolidés dans la carte par `dream`** et re-récupérés si pertinents.
- **Preuve** : 2h de conversation, prompt stable à ~3k tokens, l'agent n'oublie rien
- Briques existantes : activation (atlas), retrieval hybride embeddings+FTS5, `dream`, injection trailing
- **Manque** : faire du retrieval l'assembleur primaire + gestionnaire de working-set (budget token → sélection nœuds/items par activation)

---

# 6. Briefings UI détaillés

## 6.1 Boucle d'apprentissage (BRIEFING_GEMINI)

### Mission
Rendre la boucle d'apprentissage visible et pilotable dans l'UI : voir les skills, accepter/rejeter
ceux que l'agent propose, voir en direct quand un skill naît ou est appliqué.

### Zone de travail
- `laruche-node/src/main.rs` → **seulement** des handlers/endpoints HTTP
- `laruche-dashboard/src/templates/spa.html` → l'UI

**NE PAS TOUCHER** : `brain.rs`, `laruche-essaim/*`, `laruche-memoire/*`, `laruche-skills/*`

### Endpoints disponibles

| Méthode + route | Rôle |
|---|---|
| `GET /api/skills` | Liste des skills `{name, description, enabled}` |
| `GET /api/skills/:name` | Le SKILL.md (OKF) complet |
| `POST /api/skills` | Crée/met à jour un skill (`{content}` OKF ou `{name,content}`) |
| `POST /api/skills/:name/toggle` | Active/désactive (persisté) |
| `DELETE /api/skills/:name` | Supprime |
| `GET /api/memory/proposed` | File de revue. **Filtrer côté UI** les items `type: skill` |
| `POST /api/memory/review` | Accepter/rejeter un proposé |

### Events WebSocket

```json
{ "type": "skill_applied",  "name": "<nom>" }   // skill auto-injecté dans CE tour
{ "type": "skill_proposed", "name": "<nom>" }   // nouveau skill proposé (trajectoire réussie)
```

### Tâches

**G1 — Page/onglet « Skills »**
- Liste des skills actifs (`GET /api/skills`), avec description et toggle enable/disable
- Vue de l'OKF complet au clic dans un panneau
- Suppression (`DELETE`)
- Bouton « Nouveau skill » → éditeur OKF (textarea Markdown) → `POST /api/skills`
- Badge **« auto »** sur les skills issus de l'auto-apprentissage

**G2 — File de revue des skills proposés**
- Section lisant `GET /api/memory/proposed`, filtrant items `type: skill`
- Pour chaque proposé : aperçu + boutons **Accepter** / **Rejeter** → `POST /api/memory/review`
- Accepter → apparaît dans la liste G1

**G3 — Chips d'apprentissage dans le fil de chat**
- `skill_applied` → puce inline : « 🧠 Skill appliqué : <name> »
- `skill_proposed` → toast « ✨ Skill né : <name> » + rafraîchir la file G2

### Acceptation
1. `cargo check -p laruche-node` vert
2. Sur `:8419` : onglet Skills liste/crée/voit/supprime/toggle
3. Skill proposé accepté via G2 → actif dans G1
4. Chips dans le chat lors des events

---

## 6.2 Dashboard services Axe 5 (BRIEFING_GEMINI_AXE5)

### Mission
Dashboard présentant tous les modèles/services du mesh **groupés par catégorie de capacité**,
en distinguant local vs distant.

### Zone de travail
`laruche-dashboard/src/templates/spa.html` (onglet Dashboard / Réseau). **Rien d'autre.**

### Endpoint

`GET /swarm/models` → `SwarmModelsResponse { models: [SwarmModelInfo], ... }`

Chaque `SwarmModelInfo` :
```jsonc
{
  "host": "llama.cpp",             // "llama.cpp"|"lmstudio"|"vllm"|<ip nœud Miel>|<provider>
  "node_name": "http://127.0.0.1:8001 (local)",
  "node_id": null,                 // présent pour un nœud Miel distant
  "name": "qwen3.6-35b-a3b",
  "size_gb": 0.0,
  "is_default": false,
  "is_local": true,                // true = sur CETTE machine ; false = autre nœud du mesh
  "capability": "llm"              // "llm"|"vlm"|"vla"|"rag"|"audio"|"image"|"embed"|"code"|"agent"|"stt"|"tts"
}
```

### Tâches

**G1 — Vue « Services par catégorie »**
- Sections groupées par `capability` (LLM, VLM, TTS, STT, Code, Embed, Image, Agent…)
- Style nid d'abeille ambre. Catégories vides masquées.

**G2 — Distinction Local vs Mesh**
- **Local** (`is_local: true`) — badge du backend (`host`)
- **Mesh / distant** (`is_local: false`) — nom du nœud (`node_name`) + indicateur « distant »
- Compteur par catégorie + total

**G3 — Rafraîchissement**
- Bouton « Rescanner » → re-fetch `GET /swarm/models`
- Auto-refresh léger (ex. 15 s) quand onglet visible

### Test rapide sans matériel
```
LARUCHE_OPENAI_ENDPOINTS="llama.cpp=http://127.0.0.1:8001,vllm=http://127.0.0.1:8000"
```

---

## 6.3 Providers visibilité + Utiliser (BRIEFING_GEMINI_PROVIDERS)

### Mission
Rendre les modèles du mesh utilisables en 2 clics + introduire la visibilité des providers
(privé / public-proxy).

### Zone de travail
`laruche-dashboard/src/templates/spa.html` (Settings > Providers + onglet Réseau).

### Contrat Claude

**Donnée enrichie** : `ProviderProfile` (dans `GET /api/profiles`) a un nouveau champ :
```jsonc
"visibility": "prive" | "public_proxy"   // défaut "prive"
```

| Méthode + route | Body | Effet |
|---|---|---|
| `GET /api/profiles` | — | liste des profils avec `visibility` |
| `POST /api/profiles/:id/visibility` | `{"visibility":"prive"|"public_proxy"}` | bascule la visibilité |
| `POST /api/models/use` | `{host,name,capability,node_id?,base_url?}` | sélectionne ce modèle pour sa `capability` |
| `GET /swarm/models` | — | `{models:[{host,name,capability,is_local,node_id,node_name,is_default}]}` |

### Tâches

**G1 — Badges & toggle de visibilité (Settings > Providers)**
- Sur chaque provider : badge **🔒 Privé** / **🌐 Public** selon `visibility` + toggle → `POST /api/profiles/:id/visibility`
- ⚠️ Quand l'utilisateur passe en **public** un provider **à clé payante** (openai/anthropic/codex) : afficher avertissement clair
- Providers **locaux** (ollama, llama.cpp) : public sans friction

**G2 — Bouton « Utiliser » sur les services mesh**
- Sur chaque service de `/swarm/models` : bouton **« Utiliser »** → `POST /api/models/use`
- Au succès : toast « <name> actif pour <capability> » + marquer ce service comme actif (re-fetch `/swarm/models`)

**G3 — Indicateur passerelle**
- Provider `public_proxy` → pictogramme « passerelle mesh »

---

## 6.4 Capacités STT/TTS/Coding (BRIEFING_GEMINI_CAPABILITES)

### Mission
Sélection par capacité : LLM de chat distinct du modèle Coding, un STT, un TTS, etc. Chacun
choisi indépendamment (local ou mesh). + Intégration voix.

### Zone de travail
`laruche-dashboard/src/templates/spa.html` (Dashboard/Réseau + réglages Voix).

### Contrat Claude

| Méthode + route | Rôle |
|---|---|
| `GET /swarm/models` | tous les modèles avec `capability` |
| `GET /api/capabilities/selection` | sélection courante `{selection:{stt:{capability,model,backend,node_id?,is_local}, code:{...}, llm:{...},...}}` |
| `POST /api/models/use` | body `{host,name,capability,node_id?,base_url?}` → choisit ce service POUR sa `capability` |
| `GET /api/voice/status` | `{stt:{available,url,selected_model,is_selected}, tts:{available,url,selected_model,is_selected}}` |

### Tâches

**G1 — Sections par capacité (Dashboard/Réseau)**
- Une section par capacité présente : **LLM**, **Coding** (distincte, mise en avant), **STT**, **TTS**, **VLM**, **VLA**, Embed, Image, Agent
- Dans chaque section : services (local vs mesh) + marquer service sélectionné + bouton **« Utiliser »**
- « Modèle actif » global = sélection `llm`

**G2 — Intégration voix**
- **Dictée vocale (STT)** : à côté du toggle dictée, sélecteur des services STT détectés → « Utiliser » = `POST /api/models/use {capability:"stt",...}`
- **Auto-TTS** : idem, sélecteur des services TTS détectés
- Si aucun STT/TTS choisi : indiquer « auto (premier détecté) »

**G3 — Récap de session**
- Petit récap « Pour cette discussion : LLM = X · Coding = Y · STT = Z · TTS = W » depuis `GET /api/capabilities/selection`

**G4 — Mode Coding dans le chat (§4bis)**
- Toggle **« Mode Coding »** près de la zone de saisie
- Quand ON : ajoute `"capability":"code"` au payload du message WS
- Le serveur route vers le modèle de Coding sélectionné
- OFF (ou absent) = LLM de chat normal

---

## 6.5 Cron provider (BRIEFING_GEMINI_CRON_PROVIDER)

### Mission
Exposer le choix du provider/modèle dans le formulaire cron. **NON COMMENCÉ.**

### Zone de travail
`laruche-dashboard/src/templates/spa.html` (formulaire/list des crons, Settings > Cron).

### Contrat Claude

- `GET /api/profiles` → `{profiles:{<id>:{provider,name,base_url,models,visibility}}, active_model}`
- `POST /api/cron` accepte maintenant `profile_id` (string, optionnel) + `model` (string, optionnel)
- `PATCH /api/cron/:id` accepte aussi `profile_id` et `model`
- `GET /api/cron` renvoie chaque tâche avec `profile_id`/`provider`/`model`

### Tâches

**G1 — Sélecteur Provider dans le formulaire cron**
- Champ **« Provider »** = liste déroulante des profils (`GET /api/profiles`), option « Défaut »
- À la sélection : envoie `profile_id` dans le POST/PATCH
- (optionnel) Sous-sélecteur **« Modèle »** peuplé depuis `profiles[id].models` → envoie `model`

**G2 — Afficher le routage dans la liste des crons**
- Pour chaque cron : `profile_id` → nom du profil (+ modèle) ; sinon `provider`/`model` bruts ; sinon « défaut »

### À venir (NE PAS faire maintenant)
- Même sélecteur pour **Watchers** et **Kanban**
- **Section MCP** dans les Settings (ajouter/lister/supprimer des serveurs MCP)

---

# 7. Vision & différenciation

> Source : `VISION_INNOVATION.md` (2026-06-21)

## La thèse

> La carte cognitive EST le cerveau. La fenêtre LLM n'est qu'une mémoire de travail transitoire,
> reconstruite à chaque tour par activation. On ne compacte pas le contexte — on le *récupère*.

C'est l'inverse de tout le monde.

## Les 5 leviers

### Levier 1 — Contexte cognitif infini ⭐⭐⭐

À chaque tour, l'agent assemble un **set de travail minimal** depuis la carte. Les vieux tours
ne sont pas tronqués → **consolidés dans la carte par `dream`** et re-récupérés si pertinents.

- **Preuve buzz** : *« 2h de conversation, prompt stable à ~3k tokens, l'agent n'oublie rien. »*
- **Briques existantes** : activation (atlas), retrieval hybride embeddings+FTS5, `dream`, injection trailing
- **Manque** : retrieval comme assembleur primaire + gestionnaire de working-set (budget token → sélection nœuds/items)

### Levier 2 — Outils & skills sémantiques ⭐⭐⭐ [ON COMMENCE ICI]

Aujourd'hui : ~30 schémas d'outils injectés à chaque tour. À la place :
- Outils vivent dans la carte → **noyau minuscule toujours présent + 3-6 outils récupérés par intention**
- Agent peut avoir **1000 outils, en injecter 5**
- **Design clé** : **noyau stable** (caché par prefix-cache) **+ queue dynamique** courte (récupérée par intention)
- **Briques existantes** : abeilles en mémoire (T11 amorcé), skill_list/view, flag `dynamic_tool_selection`
- **Manque** : gater l'injection sur le retrieval

### Levier 3 — Mémoire COLLECTIVE du mesh ⭐⭐⭐⭐ [MOONSHOT / le titre]

Plusieurs nœuds LaRuche **partagent/fédèrent la carte cognitive** via Miel.
- Skill forgé sur le PC → dispo sur le laptop
- Fait appris par le nœud familial → connu de tous
- **Sync CRDT de la mémoire SQL** (le journal d'audit `mutations` est une base CRDT naturelle : append-only, horodaté, idempotent)
- **Hook viral** : *« Tes agents forment une ruche qui apprend collectivement — PC, laptop, serveur maison. Local, sans cloud. »*
- **Manque** : protocole de sync/fédération des `mutations` sur Miel

### Levier 4 — Raisonnement structurel + cité ⭐⭐

- **Requêtes structurelles** : agréger, joindre, filtrer par date/provenance sur la mémoire SQL
- **Citations** : chaque réponse cite ses souvenirs (provenance via l'audit) → **anti-hallucination** + explicabilité
- Les concurrents ne peuvent ni requêter ni citer leur mémoire plate

### Levier 5 — Mémoire épisodique + contradiction ⭐

- L'audit = machine à remonter le temps
- Détection de contradictions
- Split épisodique vs sémantique

## Séquencement (ROI décroissant)

| Phase | Levier | Pourquoi |
|---|---|---|
| **1 (en cours)** | **L2** outils sémantiques + fixes prompt | corrige le vrai bug (30 outils) + fondation, démontrable vite |
| **2** | **L1** contexte working-set | le gros pari archi, le claim « contexte infini » |
| **3** | **L4** SQL + citations | différenciateur rapide et marketable |
| **4 (moonshot)** | **L3** mémoire collective mesh | le titre, le moment « wow » |

## Context Compiler (Levier 1, version produit — parké)

Un **Tiny LLM routeur** (= l'`aux_model` déjà présent) compile le contexte de CHAQUE tour :
- Voit tout (outils + candidats mémoire), raisonne sur la pertinence
- Ne laisse passer au gros modèle QUE l'essentiel (3-5 outils + souvenirs utiles)
- Hybride final : **noyau stable** (fait) + **routeur** (Idée 1) + **tool_search/carte de catégories** (Idée 2, fallback)

## Chantiers délégables à Gemini (UI, faible risque)

- **L2** : panneau « Outils actifs ce tour » (les 3-6 abeilles injectées + pourquoi)
- **L4** : panneau « Sources » sous chaque réponse + onglet « Requête mémoire » (`GET /api/memory/query`)
- **L1** : indicateur « contexte : N souvenirs actifs / M en mémoire » dans le header

---

# 8. Architecture "La Reine" (orchestration long-horizon)

> Source : `VISION_INNOVATION.md` — section Architecture d'orchestration

## Thèse

third-party/third-party consolident + forgent des skills au niveau **session** (réactif).
LaRuche va faire du **long-horizon piloté par objectif** : une mission qui s'approfondit sur des
**semaines**, capitalise dans un graphe structuré, s'auto-étend via dream, et **apprend** (skills).

## 1. Missions = nœuds persistants (`missions.<slug>`)

Première classe dans la carte cognitive :
`{objectif, statut (active/pausée/done), cadence (cron), itérations, synthèse}`
+ sous-arbres `findings.*` / `questions.*` / `sources.*` / `contradictions.*`

## 2. La Reine = orchestrateur unique

Par tick :
1. Lit les missions **actives & dues**
2. Assemble l'état de la mission → demande à l'agent « prochaine itération ? » → crée des **tâches kanban**
3. **Dispatche** → tours d'agent (contexte working-set) → **findings écrits dans le sous-arbre de la mission**
4. Lance **`dream` scopé à la mission** → consolide + détecte trous & contradictions → génère **nouvelles questions/tâches** (auto-extension)
5. **Régénère la synthèse** (le dossier)
6. **Forge des skills** depuis les patterns réussis
7. Met à jour l'état, planifie le prochain tick

## 3. L'orbite réalisée

```
watcher (events → nœuds)
  → déclenche mission
  → kanban (tâches)
  → agent (tours)
  → mémoire (capitalisation)
  → dream (moteur de progrès)
  → skill (compétence apprise)
  → boucle
```

## 4. La démo qui bluffe

> *« Mission ton agent sur un sujet. Reviens dans un mois : il a construit un **dossier
> structuré, sourcé, qui s'approfondit tout seul chaque semaine**, résout ses contradictions,
> et est devenu **expert** du sujet. »*

Le dossier **EST** le sous-arbre de la carte → visualisable (nid d'abeille), citable (L4), exportable (OKF).

## 5. Build incrémental

- **P1** : type `Mission` (nœud map) + CRUD + un cron qui « tick »
- **P2** : La Reine = boucle mission → plan → kanban (dispatcher existant) → findings en mémoire
- **P3** : `dream` scopé mission = moteur (trous/contradictions → nouvelles tâches)
- **P4** : synthèse régénérée + forge de skills
- **P5** : UI « dossier » (nid d'abeille de la mission) + déclencheurs

## 6. Différenciation vs concurrents

| | third-party / third-party | LaRuche « Reine » |
|---|---|---|
| Consolidation | au niveau session | **long-horizon, capitalisante** |
| Skills | post-tour | **appris sur des semaines, par mission** |
| Sortie | réponses | **dossier structuré qui s'approfondit seul** |
| Moteur | réactif | **dream pilote le progrès** (gaps→tâches) |
| Substrat | mémoire = annexe | **mémoire = l'orchestrateur** |

---

# 9. Refacto UX / Navigation cible

> Source : `UX_REFACTO.md` (2026-06-21)

## Problème actuel

Settings est un fourre-tout. Cron/Watchers/Kanban/Skills/MCP n'y ont pas leur place.

## Modèle mental (3 familles)

1. **CE QU'ON FAIT** (surfaces) : Chat · Mémoire · Missions · Automatisations
2. **CE QUE L'AGENT PEUT FAIRE** (écosystème) : Capacités (skills + abeilles + MCP + plugins)
3. **CE QU'ON CONFIGURE** (vrais réglages) : Settings

## Navigation cible (top-level)

| Onglet | Contenu | Changement |
|---|---|---|
| **Chat** | conversations + **historique/recherche intégré** (overlay) | absorbe Sessions |
| **Mémoire** 🧠 | carte cognitive (arbre + markdown + graphe) | inchangé |
| **Missions** 👑 | La Reine — même libellé desktop ET mobile (icône 👑) | fix naming |
| **Automatisations** ⚙️ | **Cron · Watchers · Kanban · Blueprints · Timeline** | sortis de Settings |
| **Capacités** 🛠️ | **Skills · Abeilles · MCP · Plugins** (un seul écosystème) | unifie Skills(×2) + MCP |
| **Dashboard** | mesh · ressources · audit | inchangé |
| **Settings** ⚙ | **uniquement** : Général · Providers · Channels · Réseau · Onboarding | nettoyé |

## Détails par chantier

### 1. Chat absorbe Sessions
- **Overlay « Historique »** : liste des conversations + recherche (loop) + date MAJ + Exporter + ouvrir
- Export = client-side (Blob .md/.json depuis messages chargés)
- Supprimer l'onglet Sessions séparé

### 2. Capacités = un seul écosystème
- **TABLEAU ligne par ligne** : `Nom · Type(Abeille/Skill/MCP/Plugin) · Origine(natif/custom) · Description · Statut · Actions`
- 4 familles filtrables. Abeilles = badge « natif » (immuable) ; Skills/Plugins éditables ; MCP = serveurs (add/remove)
- MCP sort de Settings>Providers → ici
- Endpoints : `GET /api/tools`, `GET /api/skills`, `GET /api/mcp/servers` (+ POST/DELETE)

### 3. Automatisations
- Regroupe Cron · Watchers · Kanban · Blueprints + **Timeline**

### 4. Timeline ⭐ (CRUCIAL)
Vue **temporelle unifiée** de tout ce qui est prévu :
- Source : agrège `GET /api/cron` + `GET /api/missions` + `GET /api/watchers`
- Calcule la **prochaine occurrence** côté JS depuis l'expression cron (5 champs)
- Libellé humain (« chaque lundi 9h ») + dernière exécution
- Présentation : timeline/agenda (aujourd'hui / cette semaine / à venir) + section « monitors actifs »

### 5. Settings nettoyé
- Retirer Cron/Watchers/Kanban/Skills + onglet MCP-dans-Providers
- **Retirer le compteur de contexte de Settings>Général** (le mettre dans le header du Chat)
- Garder : Général, Providers, Channels, Réseau, Onboarding

## Contraintes transverses
- Style tokens CSS ambre/ruche existants, animations conservées
- Responsive : nav desktop (barre) + mobile (tabs/burger) cohérents, **mêmes libellés**
- MAJ dynamique sans F5 (déjà en place), re-fetch après action
- Zéro régression sur chat, mémoire, missions existants
- Tout est `spa.html` (include_str! → rebuild)

---

# 10. API Reference complète

## Core

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | SPA web interface |
| `GET` | `/api/status` | Node status (CPU, RAM, GPU, queue, capabilities) |
| `GET` | `/health` | Health check |
| `GET` | `/nodes` | Discovered peers (mDNS) |
| `GET` | `/swarm` | Collective swarm view |
| `GET` | `/swarm/models` | Models across swarm + cloud providers (enrichi : is_local, capability) |
| `GET` | `/models` | Local Ollama models |
| `POST` | `/infer` | Raw inference request |
| `GET` | `/activity` | Activity log |
| `GET` | `/metrics/history` | Time-series metrics (CPU, RAM, GPU, tokens/s) |

## Agent (Essaim)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/ws/chat` | WebSocket chat (streaming tokens, tool calls) |
| `GET` | `/ws/audio` | WebSocket voice (STT/TTS) |
| `GET` | `/api/tools` | List registered tools |
| `GET` | `/api/sessions` | List sessions (filtered by authenticated user) |
| `GET` | `/api/sessions/search?q=` | Full-text search across sessions |
| `GET` | `/api/sessions/:id/messages` | Session messages |
| `GET` | `/api/sessions/:id/export` | Export session as Markdown |
| `POST` | `/api/sessions/:id/fork` | Fork a session |
| `DELETE` | `/api/sessions/:id` | Delete a session |
| `POST` | `/api/webhook` | HTTP webhook (non-streaming) |
| `POST` | `/api/rpc` | JSON-RPC agent calls |
| `POST` | `/api/agents/spawn` | Spawn a sub-agent |
| `GET` | `/api/events?since=` | Events stream (AgentStarted/SessionFinished) |

## Authentication

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/auth/enroll` | Create user identity (returns QR SVG) |
| `GET` | `/api/auth/me` | Current user info |
| `GET` | `/api/auth/challenge` | Generate ephemeral login QR (60s) |
| `GET` | `/api/auth/status/:id` | Poll challenge status |
| `POST` | `/api/auth/logout` | Clear auth cookie |
| `GET` | `/auth/scan/:id` | Phone scans this to resolve login |
| `GET` | `/auth/link/:uid/:secret` | Permanent auth link (enrollment QR) |
| `GET` | `/api/auth/codex/status` | Codex OAuth status |
| `POST` | `/api/auth/codex/start` | Start Codex device auth flow |
| `POST` | `/api/auth/codex/logout` | Logout Codex |

## Provider Profiles

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/profiles` | List all provider profiles (avec `visibility`) |
| `POST` | `/api/profiles` | Create/update a profile |
| `DELETE` | `/api/profiles/:id` | Delete a profile |
| `GET` | `/api/profiles/models` | Unified model list (all providers) |
| `POST` | `/api/profiles/active` | Set active model + provider |
| `POST` | `/api/profiles/:id/visibility` | `{visibility:"prive"|"public_proxy"}` |
| `POST` | `/api/models/use` | `{host,name,capability,node_id?,base_url?}` → sélectionne pour sa capability |

## Capabilities & Voice

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/capabilities/selection` | Sélection courante par capacité |
| `GET` | `/api/voice/status` | `{stt:{...,selected_model,is_selected}, tts:{...}}` |

## Skills

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/skills` | Liste des skills `{name,description,enabled}` |
| `GET` | `/api/skills/:name` | Le SKILL.md (OKF) complet |
| `POST` | `/api/skills` | Crée/met à jour un skill |
| `POST` | `/api/skills/:name/toggle` | Active/désactive |
| `DELETE` | `/api/skills/:name` | Supprime |

## Memory

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/memory/search?q=` | Recherche dans la mémoire |
| `POST` | `/api/memory/write` | Écriture d'un item |
| `GET` | `/api/memory/tree` | Arbre des nœuds |
| `GET` | `/api/memory/stats` | Statistiques |
| `GET` | `/api/memory/mutations` | Journal d'audit |
| `GET` | `/api/memory/proposed` | File de revue (proposés) |
| `POST` | `/api/memory/review` | Accepter/rejeter un proposé |
| `GET` | `/api/memory/export_okf` | Export OKF |

## Configuration

| Method | Path | Description |
|--------|------|-------------|
| `GET/POST` | `/api/config/channels` | Channel bot config |
| `GET/POST` | `/api/cwd` | Working directory |
| `GET/POST` | `/config/default_model` | Default model |
| `GET` | `/api/onboarding` | Onboarding status |
| `GET` | `/api/doctor` | System diagnostics |
| `GET/POST` | `/api/config/compaction` | Compaction settings |
| `GET/POST` | `/api/config/notify` | Notification settings |

## Channels

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/channels/start` | Start a channel bot |
| `POST` | `/api/channels/stop` | Stop a channel bot |
| `GET` | `/api/channels/status` | Channel bot status |
| `POST` | `/api/channels/discord/webhook` | Discord Interactions endpoint |
| `POST` | `/api/channels/slack/events` | Slack Events API endpoint |

## Cron

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/cron` | Liste des tâches cron (avec `profile_id`, `model`) |
| `POST` | `/api/cron` | Créer une tâche (`prompt`,`cron_expr`,`channel`,`skills`,`profile_id`,`model`) |
| `PATCH` | `/api/cron/:id` | Modifier une tâche (dont `profile_id`, `model`) |
| `DELETE` | `/api/cron/:id` | Supprimer |

## Credentials

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/credentials` | Liste des credentials/comptes |
| `POST` | `/api/credentials` | Ajouter un compte |
| `DELETE` | `/api/credentials/:id` | Supprimer |

## MCP

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/mcp` | MCP JSON-RPC endpoint (exposer LaRuche comme serveur MCP) |
| `GET` | `/api/mcp/servers` | Liste des serveurs MCP configurés |
| `POST` | `/api/mcp/servers/:name` | Ajouter/maj un serveur MCP (recharge la registry) |
| `DELETE` | `/api/mcp/servers/:name` | Supprimer (recharge) |

## Knowledge / RAG

| Method | Path | Description |
|--------|------|-------------|
| `GET/POST` | `/api/knowledge` | RAG knowledge base (CRUD) |

## Cross-Node Sync (internal)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/internal/sync/session` | Push session to peer |
| `POST` | `/api/internal/sync/user` | Push user to peer |
| `GET` | `/api/internal/sync/bulk` | Bulk sync (all sessions + users) |

## Services (futur — P6)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/services/register` | Enregistrer un service custom `{name,capability,url,protocol}` |
| `DELETE` | `/api/services/register/:name` | Supprimer |

## Context (futur)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/context/stats` | `{used,max,ratio,messages,compactions}` |
| `GET` | `/api/state/version` | Numéro de version de l'état (pour poll réactivité UI) |

---

# 11. Pièges connus

1. **Ollama souvent DOWN** dans l'env de test → ne JAMAIS faire dépendre un endpoint d'agrégation
   d'Ollama avec `?` (cf. fix §3.6 de HANDOFF_GLOBAL). Même prudence pour tout nouvel agrégateur.

2. **Windows verrouille les `.exe`/`.dll` de `target/`** si un `rustc`/`laruche-node` tourne.
   → Tuer le process avant `cargo run`/`cargo build`. `cargo check` reste utilisable.

3. **SPA via `include_str!`** : après CHAQUE edit de `spa.html`, rebuild obligatoire.
   `cargo run -p laruche-node` (ou `cargo build -p laruche-node && laruche-node`).

4. **Toolchain MSVC uniquement**. `rust-toolchain.toml` épingle déjà la bonne. Ne jamais changer.

5. **Dropdown modèle du SPA** : doit envoyer `provider` (= profile_id) en plus du nom de modèle,
   sinon le handler garde le provider actif.

6. **Panneau « Services du mesh » vide** : fix §3.6 (résilient à Ollama down) est dans le code
   mais nécessite un rebuild du node. Tester APRÈS rebuild.

7. **Pas de npm dans le PATH** : Node v24 existe mais sans npm. Ne pas tenter d'installer des
   packages Node dans l'env.

8. **`laruche-suggestions` orphelin** (0 réf dans le workspace) : ne pas s'appuyer dessus tant
   qu'il n'est pas câblé.

9. **Backend mémoire défaut = RAM non persistée** : pour la persistance, lancer avec
   `LARUCHE_MEMOIRE_BACKEND=sqlite`. En PowerShell :
   `$env:LARUCHE_MEMOIRE_BACKEND="sqlite"; cargo run -p laruche-node`

10. **BUG agrégation nœuds Miel distants** (`main.rs` ~l.1197) : skippe `llm|code|vlm|embed|image`
    → moteur custom annonçant `capability:llm` ignoré. Relâcher le filtre (garder uniquement dédup `already_listed`).

---

# 12. Build & Run

## Commandes essentielles

```bash
cd C:\Users\infinition\Desktop\laruche-v2\laruche

# Vérification rapide (ne verrouille pas le .exe)
cargo check --workspace
cargo check -p laruche-node

# Build debug
cargo build
cargo build -p laruche-node

# Build release
cargo build --release

# Lancer le serveur → http://localhost:8419
cargo run -p laruche-node

# Avec SQLite persistant (PowerShell)
$env:LARUCHE_MEMOIRE_BACKEND="sqlite"; cargo run -p laruche-node

# Simuler des backends locaux pour les tests
$env:LARUCHE_OPENAI_ENDPOINTS="llama.cpp=http://127.0.0.1:8001,vllm=http://127.0.0.1:8000"

# Tests
cargo test -p laruche-memoire
cargo test -p laruche-essaim
cargo test -p laruche-essaim apprentissage_tests   # 3/3 verts
cargo test -p laruche-node local_inference          # 2/2 verts
cargo test --workspace                              # ⚠️ jamais relancé en entier → à faire

# POC mémoire
cargo run -p laruche-essaim --example poc_memoire   # besoin llama.cpp:8001

# Installer dans le PATH
cargo install --path laruche-node --force
cargo install --path laruche-cli --force
```

## Configuration optionnelle (`laruche.toml`)

```toml
node_name = "laruche-salon"
tier = "core"                          # nano | core | pro | max
ollama_url = "http://127.0.0.1:11434"
default_model = "gemma3:12b"
api_port = 8419

[[capabilities]]
capability = "llm"
model_name = "gemma3:12b"
model_size = "12B"
quantization = "Q4_K_M"
```

## Variables d'environnement

| Variable | Défaut | Description |
|---|---|---|
| `LARUCHE_NAME` | `laruche-xxxxxx` | Nom du node |
| `LARUCHE_TIER` | `core` | Tier hardware |
| `LARUCHE_PORT` | `8419` | Port API |
| `OLLAMA_URL` | `http://127.0.0.1:11434` | URL Ollama |
| `LARUCHE_MODEL` | `gemma3:12b` | Modèle par défaut (fallback) |
| `LARUCHE_PROVIDER` | `ollama` | Provider LLM par défaut |
| `LARUCHE_API_KEY` | — | Clé API pour providers cloud |
| `LARUCHE_TLS_CERT` | — | Chemin certificat TLS (active HTTPS) |
| `LARUCHE_TLS_KEY` | — | Chemin clé privée TLS |
| `LARUCHE_MEMOIRE_BACKEND` | `native` | `native`\|`sqlite`\|`sidecar` |
| `LARUCHE_EMBED_URL` | — | URL Ollama embeddings (`/api/embed`) |
| `LARUCHE_OPENAI_ENDPOINTS` | — | `"label=url,..."` pour llama.cpp/vLLM/LM Studio |

## Données persistées (dans le CWD du lancement)

```
votre-dossier/
  sessions/              # Conversations sauvegardées
  users/                 # Identités utilisateur (auth)
  provider-profiles.json # Config providers LLM + clés API
  laruche-state.json     # État persistant (modèle actif, activité)
  channels-config.json   # Config Telegram/Discord/Slack
  cron-tasks.json        # Tâches planifiées
  laruche.toml           # Config serveur (optionnel)
  memoire.db             # Base SQLite (si LARUCHE_MEMOIRE_BACKEND=sqlite)
  mcp_servers.json       # Serveurs MCP configurés
```

> **Tip** : toujours lancer `laruche-node` depuis le même dossier pour retrouver ses données.

## CLI Reference

```bash
laruche                    # TUI interactif (défaut)
laruche --classic          # Mode REPL classique
laruche ask "Question"     # Question one-shot
laruche --cwd /chemin      # Démarrer dans un dossier
laruche discover           # Scanner le réseau
laruche doctor             # Diagnostics système
laruche server start|stop|restart|status|install|update|uninstall|logs
laruche mcp                # Serveur MCP (stdio, pour Claude Desktop)
laruche auth codex         # Auth Codex par device code
```

### Commandes slash dans le chat

| Commande | Description |
|---|---|
| `/help` | Aide |
| `/tools` | Lister les Abeilles |
| `/model [nom]` | Changer de modèle |
| `/cwd [path]` | Changer le répertoire de travail |
| `/clear` | Nouvelle conversation |
| `/export` | Exporter en Markdown |
| `/discover` | Scanner les nœuds LaRuche |
| `/doctor` | Diagnostics |
| `/server [cmd]` | Gérer le serveur |
| `/quit` | Quitter |

---

# 13. Changelog

## [0.2.0] - 2026-04-05

### Added
- L'Essaim agent engine with ReAct loop
- 23+ built-in Abeilles (tools)
- Multi-provider LLM support (Ollama, OpenAI, Anthropic)
- Miel Protocol v0.2.0 (renamed from LAND)
- SPA unified dashboard + chat
- CLI TUI with Ratatui (WebSocket streaming)
- Telegram bot integrated in server
- RAG Knowledge Base with vector search
- Sub-agent delegation
- Parallel tool execution
- Browser control (headless Chrome)
- Dynamic plugin system
- Voice pipeline (STT/TTS)
- GPU/VRAM monitoring
- Interactive approval gating
- Cron scheduler
- MCP server support

### Changed
- LAND Protocol renamed to Miel Protocol
- All capabilities updated (Agent, Stt, Tts added)

## [0.1.0] - 2026-03-30

### Added
- Initial LaRuche node with LAND Protocol
- Ollama inference proxy
- mDNS discovery
- Basic dashboard
- CLI tool
- VS Code extension

---

# 14. Archives historiques (textes intégraux)

> Les archives ci-dessous sont les **textes intégraux** des anciens documents, conservés pour
> l'historique et pour ne perdre aucun détail d'implémentation. En cas de contradiction avec
> les sections §1-§12 ci-dessus, **les sections §1-§12 font foi** (elles sont vérifiées sur
> le code réel).

---

## Archive A — ARCHITECTURE.md (vision de fusion d'origine)

> Document de conception original. Statut : périmé (dit « aucun code fusionné encore » alors
> que la fusion est faite). Conservé pour l'historique et le contexte.

### Positionnement d'origine

| Axe | third-party / third-party | LaRuche v2 (cible) |
|---|---|---|
| Installation | uv + Python + Node + ffmpeg | **1 binaire Rust** |
| Mémoire | `MEMORY.md` plat + FTS5 + Honcho | **Carte cognitive auditée** |
| Topologie | 1 cerveau sur un VPS | **Mesh local-first** (mDNS/swarm) |
| Boucle d'apprentissage | mature (curator + background review) | à construire |
| Kanban / tâches durables | board SQLite + dispatcher | à porter |
| Watchers événementiels | non (cron seulement) | **différenciateur** |
| Interop format de savoir | — | **OKF-natif** |

### Le trait MemoireCognitive (esquisse originale)

```rust
#[async_trait]
pub trait MemoireCognitive: Send + Sync {
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<ContextPack>;
    async fn write(&self, item: MemoryItem) -> Result<MutationId>;
    async fn propose_write(&self, item: MemoryItem) -> Result<MutationId>;
    async fn read_node(&self, id: &str) -> Result<NodeView>;
    async fn dream(&self) -> Result<Vec<Suggestion>>;
    async fn export_okf(&self, dir: &Path) -> Result<()>;
    async fn import_okf(&self, dir: &Path) -> Result<usize>;
    async fn health(&self) -> Result<Health>;
}
```

### Interop OKF

OKF = fichiers Markdown + frontmatter YAML, arborescence, liens markdown. **Stratégie :
OKF = format d'import/export, pas le stockage.** SQLite reste la source de vérité.
Mapping : nœud↔dossier, item↔doc, tags↔tags, liens↔liens, audit↔git.

**Bénéfice** : être le premier moteur cognitif local-first mono-binaire OKF-natif.

---

## Archive B — ROADMAP_CLAUDE_CODE_PORT.md (tableau de suivi du portage)

### Tableau de suivi complet des lots

| Lot | Owner | Statut | Fichiers | Tests |
|---|---|---|---|---|
| 0 (fondations) | Claude | ✅ fait | brain.rs, shell.rs, providers.rs, node, spa | 15 essaim |
| 1.A tool_budget | Codex | ✅ fait | abeille.rs, tool_budget.rs | 3 nouveaux / 59 essaim verts |
| 1.B budget | Codex | ✅ fait | budget.rs | 3 nouveaux / 59 essaim verts |
| 1.C compaction | Codex | ✅ fait | laruche-compaction | 3 nouveaux / 7 compaction verts |
| 1.D context UI | Gemini | ✅ fait | main.rs, spa.html | — |
| 1.E intégration | Claude | ✅ fait | brain.rs | 2 nouveaux / 85 essaim verts |
| 2.A recovery/fallback | Claude | ✅ fait | providers.rs, streaming.rs, brain.rs | 1 nouveau / 85 verts |
| 2.B tool_summary | Codex | ✅ fait | tool_summary.rs | 3 nouveaux / 85 verts |
| 2.C réglages robustesse | Gemini | ✅ fait | spa.html, main.rs | — |
| 2.D résumé outils (intégration) | Claude | ✅ fait | brain.rs | aux model + fallback / 89 verts |
| 3.A steer serveur | Gemini | ✅ fait | main.rs | — |
| 3.B UI steering | Gemini | ✅ fait | spa.html | — |
| 3.C steer boucle | Claude | ✅ fait | brain.rs, main.rs | canal non bloquant / 89 verts |
| 4.A sous-agent | Codex | ✅ fait | laruche-essaim | 1 struct + unit tests |
| 4.B API node spawn | Claude | ✅ fait | main.rs | `POST /api/agents/spawn` |
| 4.C UI sous-agents | Gemini | ✅ fait | spa.html | Arbre des sous-agents |
| 5.A dream | Codex | ✅ fait | laruche-memoire, main.rs | Dream inactif implémenté |
| 5.B UI dream | Gemini | ✅ fait | spa.html | — |
| 6.A prompt 3 tiers | Claude | ✅ fait | prompt.rs | 1 nouveau / 89 verts |
| 7.A credential pool | Codex | ✅ fait | credential_pool.rs | 4 nouveaux / 79 verts |
| 7.B error classifier | Codex | ✅ fait | error_classifier.rs | 12 nouveaux / 79 verts |
| 7.C threat patterns | Codex | ✅ fait | threat_patterns.rs | 4 nouveaux / 79 verts |
| 7.D mixture of agents | Codex | ✅ fait | abeilles/mixture.rs | 2 nouveaux / 89 verts |
| 7.E trajectory compress | Codex | ✅ fait | laruche-compaction | 1 nouveau / 8 compaction verts |
| 7.F notifier proactif | Gemini | ✅ fait | main.rs, spa.html | — |
| 7.G UI multi-compte | Gemini | ✅ fait | main.rs, spa.html | — |
| 9.A canal + modèle par tâche | Codex + Gemini | ✅ fait | cron.rs, main.rs, spa.html | 2 nouveaux |
| 9.B daemon cron | Claude | ✅ fait | main.rs | Route channels / fallback default |
| 9.C background review | Codex | ✅ fait | brain.rs, background_review.rs | 2 nouveaux |
| 9.D curator idle | Codex | ✅ fait | laruche-memoire | 2 nouveaux / état persistant |

### Source de vérité (repo `aa` = Claude Code TS)

| Technique | Fichier(s) référence dans `aa/src` |
|---|---|
| Boucle d'agent (turns, continue-sites, état mutable) | `query.ts`, `QueryEngine.ts` |
| Orchestration d'outils (partition safe/unsafe, concurrence) | `services/tools/toolOrchestration.ts` |
| Exécution d'outils (budget résultat, tool_use_id, content replacement) | `services/tools/toolExecution.ts` |
| Budget de tokens (per-turn, countdown) | `query/tokenBudget.ts` |
| Auto-compaction (seuil) | `services/compact/autoCompact.ts` |
| Micro-compaction (éviction par tool_use_id) | `services/compact/microCompact.ts` |
| Nettoyage post-compaction | `services/compact/postCompactCleanup.ts` |
| Permissions (canUseTool, règles, suggestions) | `hooks/useCanUseTool.tsx` |
| Prompt système modulaire (3 tiers) | `constants/systemPromptSections.ts` |
| Sous-agents (fork, resume, contexte isolé) | `tools/AgentTool/runAgent.ts` |
| Consolidation mémoire (dream) | `services/autoDream/autoDream.ts` |
| Résumé d'outil (gros outputs) | `services/toolUseSummary/*` |
| Steering / messages en file | `context/QueuedMessageContext.tsx` |
| Récupération maxOutputTokens | `query.ts` (`isWithheldMaxOutputTokens`) |
| Modèle de repli (fallback) | `query.ts` (`fallbackModel`) |

---

## Archive C — AUDIT_COMPARATIF.md

### Cœur agentique

| Mécanisme | Source | Statut LaRuche | Implémentation |
|---|---|---|---|
| Agent Loop (ReAct) | Les deux | ✅ Parité | `brain.rs` |
| Token Budget & Limits | Claude Code | ✅ Parité | `budget.rs` |
| Tool Result Truncation | Claude Code | ✅ Parité | `tool_budget.rs` |
| Tool Summary (LLM) | Claude Code | ✅ Parité | `tool_summary.rs` |
| Orchestration safe/unsafe | Claude Code | ✅ Parité | `partition_tool_calls` |
| Model Fallback | Claude Code / third-party | ✅ Amélioré | bascule Ollama→cloud |
| maxOutputTokens recovery | Claude Code | ✅ Parité | `sortie_tronquee` + continuation |
| System Prompt 3 Tiers | third-party | ✅ Fait | `prompt.rs` (cache-friendly) |
| Trajectory Compression | third-party | ✅ Amélioré | `laruche-compaction` |
| Steering (interruption mid-turn) | Claude Code | ✅ Fait | canal non bloquant |
| Tâches de fond (Client Aux) | third-party | ✅ Parité | `tokio::spawn` + `aux_model` |
| Error Classifier | third-party | ✅ branché failover | `error_classifier.rs` |
| Credential Pool (rotation) | third-party | ✅ module+UI prêts | `credential_pool.rs` |
| Threat Patterns (anti-injection) | third-party | ✅ branché dispatch | `threat_patterns.rs` |

### Outils — Mapping tripartite complet

#### Fichiers & Système

| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Lecture fichier | ✅ | ✅ | ✅ | `file_read` (offset/limit + numéros de ligne) |
| Écriture fichier | ✅ | ✅ | ✅ | `file_write` |
| Édition (patch) | ✅ | ✅ | ✅ | `file_edit` (old_string→new_string, +replace_all) |
| Liste répertoire | ✅ | ✅ | ✅ | `file_list` |
| Glob | ✅ | ✅ | ✅ | `file_search` |
| Grep contenu | ✅ | ✅ | 🔶 | via `file_search` (à confirmer regex contenu) |
| Extraction sélective (PDF/MD) | ❌ | ✅ | ✅ | `read_extract` |
| Watcher fichier événementiel | ❌ | ❌ | 🚀 | `file_watch` — **exclusif LaRuche** |
| Notebook (.ipynb) | ✅ | ❌ | ❌ | — (à porter) |

#### Code, Shell, Navigation

| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Shell / PowerShell | ✅ | ✅ | ✅ | `shell_exec` (PowerShell sous Windows, read-only auto-approuvé) |
| Exécution code isolée | ✅ | ✅ | ✅ | `execute_code` |
| Pipeline d'outils (1 tour) | 🔶 | 🔶 | 🚀 | `run_script` (RPC multi-étapes) |
| REPL persistant | ✅ | ✅ | 🔶 | partiel via execute_code |
| LSP (hover/def/refs) | ✅ | ❌ | ✅ | `lsp` (AbeilleLsp) |
| Git status/diff/log/commit | 🔶 (Bash) | ✅ | ✅ | `git_*` |
| Isolation Git worktree | ✅ | ❌ | ✅ | `git_worktree_enter`/`exit` |
| Navigateur headless | ❌ | ✅ | ✅ | `browser_navigate`/`browser_screenshot` |

#### Web & Médias

| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Web search | ✅ | ✅ | ✅ | `web_search` |
| Web fetch (page) | ✅ | ✅ | ✅ | `web_fetch` (tête+queue) |
| Deep search (multi-liens) | ❌ | ✅ | ✅ | `web_deep_search` (URLs propres + content-type) |
| Scraping JS-rendered robuste | 🔶 | ✅ (Playwright) | 🔶 | **À AMÉLIORER** (web_render plugin) |
| Recherche d'images | ❌ | ✅ | ✅ | `image_search` |
| Génération image/vidéo | ❌ | ✅ | ✅ | via plugins/MCP |
| Computer use | ❌ | ✅ | ✅ | via MCP externe |

#### Tâches, Orchestration, Protocoles

| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Délégation sous-agent | ✅ | ✅ | ✅ | `delegation` + `subagent.rs` |
| Mixture of Agents | ❌ | ✅ | ✅ | `mixture_of_agents` |
| Todo | ✅ | ✅ | ✅ | `todo` |
| Cron (créer/lister/suppr) | ✅ | ✅ | ✅ | `cron_create`/`cron_list`/`cron_delete` |
| Watchers événementiels | ❌ | 🔶 | 🚀 | `watcher_create`/`list`/`delete` |
| Kanban | ❌ | ✅ | ✅ | `kanban_create`/`kanban_list`/`kanban_next` |
| Clarification utilisateur | ✅ | ✅ | ✅ | `clarify` |
| Recherche dans sessions | ✅ | ✅ | ✅ | `session_search` |
| Client MCP (resources) | ✅ | ✅ | ✅ | `mcp_list_resources`/`mcp_read_resource` |
| Serveur MCP (exposer) | ❌ | ❌ | 🚀 | `laruche mcp` |
| Hot-reload de plugins | ❌ | ❌ | 🚀 | `reload_plugins` |

#### Compétences & Mémoire

| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Lister/voir skills | ✅ | ✅ | ✅ | `skill_list`/`skill_view` |
| Auto-création de skills | ❌ | ✅ (background_review) | 🔶 | inline + `background_review.rs` |
| Mémoire CRUD vectorielle (OKF) | 🔶 | 🔶 | 🚀 | `memory_*` (search/write/update/move/delete/review/…) |
| Dream / consolidation | ❌ | ✅ | 🔶 | `dream()` (manuel ; curator idle à finaliser) |

#### Divers

| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Calendrier | ❌ | ✅ | ✅ | `calendar_add`/`calendar_list` |
| Maths | ❌ | ❌ | ✅ | `math_eval` |
| Infos système | ✅ | ✅ | ✅ | `system_info` |
| Présentation média (UI) | ❌ | 🔶 | ✅ | `media_present` |

---

## Archive D — SKILLS_TOOLS_ARCHITECTURE.md (Phases 10/11)

### Modèle de référence third-party

- **Skill** = dossier avec `SKILL.md` (frontmatter YAML + corps Markdown + scripts optionnels)
- **Divulgation progressive** : `skills_list` (tier 1 = nom+desc) → `skill_view` (tier 2 = SKILL.md complet)
- **Cron à skills** : les SKILL.md sélectionnés sont **assemblés dans le prompt** au moment du run

### Décisions LaRuche

Un **Skill EST un document OKF** stocké dans `laruche-memoire` sous `tools.skills.<slug>`.
Avantage : `skill_list`/`skill_view` font de la **découverte sémantique**, pas un simple scan de dossier.

**Un seul registry, un marqueur d'origine :**
- `ToolOrigin::Builtin` — abeille Rust native, **immuable** (le LLM ne peut pas l'éditer)
- `ToolOrigin::Custom` — plugin JSON + script, **forgé/éditable** par le LLM via `reload_plugins`

### Tableau de suivi Phases 10/11

| Lot | Owner | Statut |
|---|---|---|
| 10.A champ skills (ScheduledTask) | Claude (Codex ne l'avait pas livré) | ✅ |
| 10.B injection skills au run cron | Claude | ✅ (assembler_prompt_skills + read_node OKF) |
| 10.C API + UI Skills + checkboxes | Gemini | ⬜ (à vérifier côté UI) |
| 11.A orchestrateur kanban | Codex | ✅ |
| 11.B boucle orchestrateur | Claude | ✅ (daemon kanban exécute les `Ready` 1 par tick) |
| 11.C blueprints (data+UI) | Codex + Gemini | ✅ |
| 11.D tool origin builtin/custom | Codex + Gemini | ✅ |

---

## Archive E — laruche_tools_parity.md (parité outils, revue 2026-06-20)

### Plan d'amélioration web (plugin `web_render`)

**Outils actuels** : `web_search` (scrape DDG HTML), `web_fetch` (HTML brut), `web_deep_search`.
**Limite** : pages JS-rendered → `web_fetch` ne récupère que la nav, pas le contenu.

**Plan** (forge en plugin) :
- `plugins/scripts/web_render.py` : rendu headless (Playwright bundle Node v24 sans npm) → DOM rendu + texte (readability)
- `plugins/web_render.json` : schéma `{url, wait_selector?, screenshot?}`
- Fallback : si fetch statique renvoie peu de contenu (< seuil), basculer auto sur `web_render`

**Manques prioritaires restants :**
1. Scraping JS-rendered robuste (`web_render` plugin) — priorité #1
2. Grep contenu dédié (regex sur contenu, pas juste noms)
3. Notebook execution (.ipynb) — basse priorité
4. Computer use — via serveur MCP externe (déjà possible)

---

## Archive F — third-party_HACKS.md (catalogue d'astuces third-party)

### 🔴 TOP — règlent la latence

**#1 System prompt en 3 tiers + invariant prefix-cache** — `system_prompt.py`
- **stable** : identité, guidance outils, schémas d'outils, skills
- **context** : `system_message` appelant + fichiers de contexte (AGENTS.md/.cursorrules)
- **volatile** : snapshot mémoire, USER.md, timestamp/session/modèle
- ✅ FAIT dans LaRuche (`prompt.rs`)

**#2 Client auxiliaire séparé** — `auxiliary_client.py`
- Tâches de fond sur un modèle/clé aux distinct → ne touche jamais le cache du chat principal
- ✅ FAIT dans LaRuche (`aux_model`)

**#3 Fork background-review réutilise le prompt caché verbatim** — `background_review.py`
- Fork hérite provider/model/base_url/credentials + **prompt système caché** → tape le **même prefix cache**
- Whitelist d'outils = mémoire/skills only
- ✅ FAIT dans LaRuche (`background_review.rs`)

### 🟠 HAUT — robustesse

**#4 Garde-fous de boucle d'outils** — seuils : warn 3×, halt 6× (par signature)
- ✅ FAIT (`brain.rs`)

**#5 Classifieur d'erreurs** — `error_classifier.py`
- `context_length` → compresser puis retry ; `rate_limit` → backoff ; `auth` → message clair ; `transient/5xx` → retry court
- ✅ FAIT (`error_classifier.rs` → branché failover)

**#6 Compression préflight** — avant que ça déborde, pas seulement tronquer
- ⬜ RESTE à faire

### 🟡 MOYEN — UX & puissance

**#7 Interrupt-and-redirect / canal steer** — (`steer` 17 fichiers, `interrupt` 46)
- ✅ FAIT dans LaRuche (canal WS non bloquant)

**#8 Scripts RPC** — `run_script` enchaîne des outils sans repasser par le LLM
- ✅ FAIT (`run_script` dans LaRuche)

**#9 Profils de posture** — `RuntimeMode` : toolset réduit, brief opérationnel
- ✅ FAIT (toolset stable `EssaimConfig.stable_toolset`)

**#10 `reasoning_effort` par modèle** — (16 fichiers third-party)
- ⬜ RESTE à faire

**#11 `cache_control` (Anthropic prompt caching)**
- ⬜ RESTE à faire

---

## Archive G — HANDOFF.md (passation mémoire T1-T14)

### Hacks third-party — état final d'implémentation

| Hack | Statut |
|---|---|
| #1 system prompt stable + mémoire trailing | ✅ FAIT (`boucle_react_multimodal_ext`) |
| #2 client auxiliaire pour la curation | ✅ FAIT (`EssaimConfig.aux_model`) |
| #3 garde-fous de boucle d'outils | ✅ FAIT (warning 3×, halt 6× dans brain.rs) |
| #4 ??? | — |
| #5 classifieur d'erreurs + failover | ✅ FAIT (`error_classifier.rs`) |
| #6 compression préflight | ⬜ RESTE |
| #7 interrupt/steer | ✅ FAIT (canal WS) |
| #8 scripts-RPC | ✅ FAIT (`run_script`) |
| #9 toolset stable par profil | ✅ FAIT (`EssaimConfig.stable_toolset`) |
| #10 reasoning_effort | ⬜ RESTE |
| #11 cache_control Anthropic | ⬜ RESTE |

### T13 — Performance (latence chat) — FAIT

- ✅ **Auto-curation non bloquante** : `curer_memoire` passe en `tokio::spawn`
- ✅ **Mémoire en contexte trailing (prefix-cache)** : mémoire injectée comme message `system` trailing (après l'historique). Le system prompt (identité + schémas d'outils) redevient un préfixe stable → cache LCP de llama.cpp réutilisable
- ✅ **Toolset stable** : `EssaimConfig.stable_toolset` → sélection d'outils query-INDÉPENDANTE

### T14 — Boost des tools — FAIT

- ✅ `web_deep_search` : nettoyage URLs DDG + garde content-type HTML + troncature tête+queue
- ✅ `file_edit` (= `patch` third-party) : édition ciblée `old_string`→`new_string` (+`replace_all`)
- ✅ `file_read` boosté : plages de lignes (offset/limit) + numéros de ligne
- ✅ `clarify` : rend la main (court-circuit dans `brain.rs`)
- ✅ `run_script` (scripts-RPC third-party) : pipeline d'outils en 1 tour
- ✅ `shell_exec` : commandes read-only auto-approuvées (`git status/log/diff`, `ls`, `cat`, `echo`…)
- ✅ `web_fetch` : troncature tête+queue

---

## Archive H — ROADMAP.md racine (pistes A-E « récolte »)

### Les 5 pistes (état actuel)

| Piste | Contenu | État |
|---|---|---|
| **A** | Socle UX & robustesse (thought_stream, permissions, compaction, events) | ✅ FAIT |
| **B** | Skills auto-créés (OKF memory, background_review) | ✅ FAIT |
| **C** | Crons façon third-party (abeilles + canal + idempotence) | ✅ FAIT |
| **D** | Watchers (triggers → Kanban) | ✅ FAIT (boucle autonome à confirmer runtime) |
| **E** | Kanban + orchestrateur (dispatcher + blueprints) | ✅ FAIT |

### Invariants

- Pas de Node/Python au runtime (mono-binaire Rust). Cœurs upstream intacts.
- Tout passe par les traits (`MemoireCognitive`, `Abeille`) → backends/outils interchangeables.
- Chaque piste : `cargo test` vert + une démo observable avant de la dire finie.

---

## Archive I — laruche/ROADMAP.md (roadmap produit / Miel / capabilities)

### Fonctionnalités implémentées

- ✅ Miel protocol core (mDNS discovery + Cognitive Manifest)
- ✅ Capability differentiation (LLM, VLM, VLA, RAG, Audio, Image, Embed, Code)
- ✅ Proof of Proximity authentication
- ✅ QoS priority system
- ✅ Swarm state management
- ✅ Node daemon with Ollama bridge
- ✅ Client SDK (3-line usage)
- ✅ CLI tool
- ✅ Web dashboard with cyber monitoring

### Fonctionnalités prévues (backlog long-terme)

#### POCs Capacités
- [ ] POC `capability:llm` — node standard Mistral/Llama
- [ ] POC `capability:vlm` — Vision-Language (LLaVA, Qwen-VL)
- [ ] POC `capability:vla` — Vision-Language-Action (bras robotique/drone)
- [ ] POC `capability:rag` — nœud spécialisé indexation documents locaux
- [ ] POC `capability:audio` — Whisper/Bark STT/TTS
- [ ] POC `capability:image` & `capability:embed` — Stable Diffusion, vecteurs
- [ ] POC `capability:code` — CodeLlama/DeepSeek-Coder

#### Infrastructure & Écosystème
- [ ] Tensor sharding over Ethernet (Swarm Intelligence)
- [ ] LaRuche Resilience (failover, hot-swap, mirroring)
- [ ] NFC hardware integration
- [ ] VS Code extension (en cours, GARDÉ)
- [ ] Home Assistant plugin
- [ ] Mobile app (iOS/Android)
- [ ] Miel v1.0 specification (RFC)

#### Défis techniques notés

1. **Swarm Intelligence & Tensor Sharding** : pipeline parallelism LLM multi-machines, protocoles bas-niveau (UDP, RDMA)
2. **LaRuche Resilience** : failover dynamique si nœud injoignable, mirroring de contexte entre nœuds cluster
3. **VS Code Extension** : découverte automatique via Miel pour autocomplétion / Copilot local
4. **Home Assistant Plugin** : Tool Use pour actions concrètes (allumer les lumières…)
5. **Mobile App** : client léger, intégration vocale STT
6. **NFC « Tap-to-Connect »** : Proof of Proximity + quotas GPU (QoS Cognitive Manifest)
7. **Miel v1.0 RFC** : spec formelle de la structure manifeste mDNS, flags capability, protocoles
8. **Rigueur cognitive** : vérification post-exécution (dir, ls…) → l'agent ne dit réussite qu'avec preuve

---

## Archive J — README.md racine (build/run, état des phases d'origine)

### État des phases (tableau d'origine — périmé, remplacé par §4 ci-dessus)

| Phase | Contenu | État (d'après README) |
|---|---|---|
| P0 | scaffold : copie des 2 bases, toolchain pin | ✅ |
| P1 | `laruche-memoire` + `SidecarBackend` + Abeilles mémoire | ✅ |
| P2 | mémoire automatique dans la boucle (`boucle_react_memoire`) | ✅ (démo via l'exemple) |
| P3 | `SqliteBackend` (SQLite+FTS5+audit, Rust pur) | ✅ moitié faite — reste embeddings/sémantique |
| P3+ | enregistrement dans `laruche-node`, onglet Mémoire, Kanban, watchers | **FAIT** (voir §4) |

> Note : le README datait du début du projet. Tout P3+ est fait (voir §4).

### Backend sidecar (paradigm) — ABANDONNÉ

```bash
# Si jamais besoin de tester le vrai paradigm :
cd paradigm
npm install          # nécessite Node 22+ (pas dispo dans l'env actuel sans npm)
node packages/memory-mcp/src/http-server.mjs    # → pont sur http://127.0.0.1:8765

# Puis lancer avec :
LARUCHE_MEMOIRE_BACKEND=sidecar cargo run -p laruche-node
```

> **Décision 2026-06-20** : paradigm/ SUPPRIMÉ. Le `SidecarBackend` subsiste comme code mort
> optionnel (brancher un paradigm externe si jamais besoin). Le chantier « paradigm réel end-to-end »
> est **abandonné** — on valide directement avec `sqlite`/`native`.

---

## Archive K — Journal des décisions (2026-06-20)

- **PAS DE GIT** pour l'instant (l'utilisateur gère le versionnement plus tard)
- **`paradigm/` supprimé** : portage SQL→Rust accompli, mission accomplie. `SidecarBackend` = code mort optionnel
- Nettoyé : `target-codex/`, `target-codex-reconcile/` (6,4 Go), fichiers de test racine
- **Gardés** : `laruche-voix` (service externe STT/TTS annoncé sur Miel), `laruche-channels` (bots Telegram/Discord/Slack, référence), `laruche-vscode` (futur pont VS Code)
- **Modes de lancement clarifiés** : `laruche` (binaire CLI) = TUI agent autonome par défaut + sous-commande `server start` (spawn `laruche-node`) ; `laruche-node` = serveur Web/API/nœud mesh. **Lancer `laruche` ne démarre PAS le serveur automatiquement**
- **Build vérifié** : `cargo check --workspace` → exit 0, 4 warnings (imports inutilisés dans `mcp_client.rs` + `reload_plugins.rs`)
- Tests vérifés : `cargo test -p laruche-essaim apprentissage_tests` 3/3 ; `cargo test -p laruche-node local_inference` 2/2

---

## Archive L — Contradictions entre anciens docs (pour info)

| Doc | Affirmation | Réalité (vérifié) |
|---|---|---|
| ARCHITECTURE.md | « Aucun code fusionné encore » | Faux — fusion faite, 14 crates |
| README.md (table phases) | onglet Mémoire « planifié » | Fait (T4 ✅) |
| HANDOFF.md | T12 Codex OAuth « non commencé » | **Fait & vérifié live le 19/06** |
| ROADMAP.md vs HANDOFF.md | 2 systèmes de nommage (pistes A-E vs T1-T14) | Les deux ont atterri dans le code |

---

*Fin du dossier — compilé le 2026-06-22 depuis 12 fichiers sources.*

---

# 10. MISE À JOUR — 2026-06-22 (audit code RÉEL, post-session mémoire/forge/feed)

> Ce doc ci-dessus est un snapshot ~20-21 juin. Cette section corrige l'écart, **vérifiée par lecture du code** (pas seulement les commits). Elle prime sur §4 et §5 en cas de divergence.

## 10.1 Faux / périmé dans le doc (à corriger)
- **`tools.abeilles.*` / `tools.skills.*` → renommé `capacities.{tools,skills,plugins,mcp}.*`** (rename effectif : 21 réf brain.rs, 18 main.rs).
- §4.6 « UI Mémoire = hexagones SVG » → remplacé par l'**UI Obsidian** (arbre + markdown + graphe + drag&drop + horodatage + Feed).
- §4.3 « dream sans suppression » → **dream fait la fusion réelle** (`consolider_node/memoire`).
- §4.4 « skill_list/view » → **forge complète** (création/édition/itération de skills & outils).

## 10.2 Listé « RESTE » mais en fait FAIT (vérifié dans le code)
- **P3** — node expose un chat OpenAI-compat : ✅ `main.rs:1618` + route `/v1/chat/completions` (l.8067). Miel distant consomme le LLM d'un autre node.
- **P6** — bug filtre mesh §1197 (capabilities llm/code/vlm skippées) : ✅ **corrigé** (`main.rs:~1328`).
- **L2** — gater l'injection sur le retrieval : ✅ SEMANTIC_CORE + retrieval + forçage par intention + `tool_search`/`tool_call` + index de capacités.
- **Node CRUD mémoire** (create/update/delete_node) : ✅ (+ delete→orphans lossless, node/move, drag&drop, edit mode).
- **OKF round-trip** export/import : ✅ (+ `export.zip` scopé, import par fichier).
- **UX_REFACTO** (nav, Capacités, Automatisations, Timeline) : ✅ (+ Timeline Gantt, kanban horizontal, édition missions, builder cron).
- **BRIEFING skills** (page Skills, file `proposed`, chips) : ✅.

## 10.3 AJOUTS de la session (absents du doc)
- Rename `capacities.*` (4 familles, routage par origine builtin/custom/mcp).
- **`system.prompt`/`identity`/`behavior`/`soul`** éditables en DB, **hot-reload** (sans redémarrage), pré-remplissage du défaut, item unique pour les nœuds-doc.
- **Horodatage** créé/maj sur nœuds+items + `memory_read_node`.
- **`tool_search`/`tool_call`** (divulgation progressive) + **index de capacités** dans le prompt + garde-fou anti-boucle assoupli.
- Skill **`web-research`** seedé + forcé par intention.
- **FORGE auto-amélioration** : `skill_create/patch/delete`, `skill_file_write/read/delete/list`, `plugin_create/list/delete`, `mcp_add/remove/list`. Section « Autonomie » du prompt réécrite (skill=procédure vs plugin=capacité). → l'agent s'est créé tout seul le skill `arxiv_search` avec script.
- **Consolidation LLM réelle** : `consolider_node/memoire` + abeille `memory_consolidate` + bouton « Consolider ».
- **Feed global** : acteur User/LaRuche sur mutations (colonne `src`), endpoint `/api/feed` agrégé, volet drawer (filtres, pin-push, refs cliquables, prochaine action), bruit système silencé.
- Blueprints **réels** (fin du mock) + UI création ; MCP edit ; plugins corrigés (champ `command`).
- (Gemini) `@LaRuche` dans la DB, fixes suppression de nœuds (read_node pur, orphans), UI mobile/dashboard.

## 10.4 Backlog RÉEL restant (haute confiance, code-vérifié)
- **Levier 1** — contexte working-set (« infini ») : ❌ pas fait. Le gros pari archi.
- **Levier 3** — mémoire mesh CRDT (moonshot, le titre) : ❌ pas fait.
- **Feed v2** — logger crons/watchers/missions DÉCLENCHÉS (pour l'instant : « prochaine action » + activity_log). Attribution `move`/`create_node` reste LaRuche par défaut (mineur).
- **P7** — push global `state_changed` (au-delà du poll Feed) : partiel.
- **§P5 audit (0 fichier = NON fait)** : `reasoning_effort`, `cache_control` Anthropic, `web_render` (Playwright), notebook `.ipynb`, **auth OTP/password**, **grep contenu (regex)**.
- **Watchers** : boucle autonome fichier/RSS→tâche→notif = **à confirmer** (webhooks Discord/HTTP présents).
- `cargo test --workspace` complet : jamais lancé.

*Addendum vérifié par audit code le 2026-06-22.*
