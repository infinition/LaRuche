# HANDOFF GLOBAL — LaRuche v2 (reprise de session)

> **À lire en premier.** Ce fichier est le point d'entrée unique pour reprendre le travail.
> Source de vérité détaillée : [`LARUCHE_V2.md`](./LARUCHE_V2.md) (Partie 0 = état réel du code).
> Dépôt : `C:\Users\infinition\Desktop\laruche-v2\laruche`. Établi le 2026-06-20.

## 0. Règles & environnement (NON négociables)
- **Pas de git** (l'utilisateur gère). Ne commit pas, ne push pas.
- Toolchain **MSVC** épinglée (`rust-toolchain.toml`). Ne pas changer. (gnu KO : dlltool absent.)
- Windows **verrouille `laruche-node.exe`** s'il tourne → le tuer avant `cargo run`/`cargo build`, ou `cargo check`.
- Conventions **FR** dans le code (abeille, mémoire, skill…).
- **Vérifier** chaque étape : `cargo check -p <crate>` (ou `cargo test`) AVANT de dire « fini ». Ne pas livrer de code non compilé.
- Build/run : `cd laruche ; cargo run -p laruche-node` → `http://localhost:8419`.

## 1. Répartition des zones (pour bosser en parallèle SANS collision)
| Agent | Zone exclusive |
|---|---|
| **Claude / Codex** (Rust cœur) | `laruche-essaim/src/*` (brain, providers, cron, abeilles…), `laruche-node/src/*.rs` (logique, endpoints), crates `laruche-*` |
| **Gemini** (UI) | `laruche-dashboard/src/templates/spa.html` UNIQUEMENT (+ rebuild node après edit, SPA via `include_str!`) |

## 2. État du build (au moment du handoff)
- `cargo check -p laruche-node` : **vert** (exit 0).
- `cargo test -p laruche-essaim apprentissage_tests` : 3/3 ; `cargo test -p laruche-node local_inference` : 2/2.
- ⚠️ `cargo test --workspace` **jamais relancé en entier** → à faire à la reprise.

## 3. Ce qui a été LIVRÉ cette session (backend Rust, tout compile)
1. **Boucle d'apprentissage** (`brain.rs`) : events `ChatEvent::SkillApplied`/`SkillProposed` ; rappel auto des skills en chat (`augmenter_ephemere_avec_skills`) ; extraction gatée (`trajectoire_merite_skill`, ≥2 outils) ; POC `examples/poc_apprentissage.rs`.
2. **Axe 5 — distribution réseau** : `laruche-node/src/local_inference.rs` (détecte llama.cpp:8001 / lmstudio:1234 / vllm:8000 via `/v1/models`, surchargeable `LARUCHE_OPENAI_ENDPOINTS`) ; `/swarm/models` enrichi (is_local + capability) ; annonce mDNS des modèles locaux au boot.
3. **Providers** : `profiles::Visibilite{Prive,PublicProxy}` (JSON `"visibility"`) ; `POST /api/profiles/:id/visibility` ; `POST /api/models/use` (sélection 2-clics, crée profil local/miel) ; provider `"miel"` routé OpenAI vers node distant. **Décision : public = PROXY** (la clé reste locale, le node relaie ; JAMAIS de diffusion de clé).
4. **Sélection par capacité** : `CapabilitySelection{capability,model,backend,node_id,is_local,profile_id}` ; `GET /api/capabilities/selection` ; `/api/voice/status` honore le STT/TTS choisi (`selected_model`/`is_selected`) ; **mode Coding** : le chat WS lit `incoming["capability"]` → `"code"` route vers le modèle de code sélectionné (`appliquer_capacite`). Persisté au redémarrage.
5. **Cron provider via profil** : helper unique `appliquer_profil(state,&mut config,profile_id,model)` (résout provider+clé+base_url+modèle) ; `ScheduledTask.profile_id` ; daemon cron résout via profil. (`appliquer_capacite` réutilise `appliquer_profil`.)
6. **FIX dashboard** : `/swarm/models` rendu **résilient à Ollama down** (avant : un `?` faisait échouer tout l'endpoint quand Ollama ne répondait pas → panneau « Services du mesh » bloqué sur « Chargement… »). **→ rebuild node nécessaire pour que le fix prenne effet.**

## 4. TRAVAIL UI restant pour GEMINI (contrats back-end prêts)
Chaque briefing détaillé est un `.md` à la racine de `laruche/`. Statut à **vérifier** par Gemini (l'utilisateur a dit « Gemini a fini » pour plusieurs, mais re-tester).
- **`BRIEFING_GEMINI.md`** — boucle d'apprentissage : page Skills, file de revue `proposed`, chips `skill_applied`/`skill_proposed`.
- **`BRIEFING_GEMINI_AXE5.md`** — dashboard « Services du mesh » par catégorie (local vs mesh). ⚠️ **c'est le panneau actuellement vide** : après le rebuild (fix §3.6), re-tester ; il consomme `GET /swarm/models`.
- **`BRIEFING_GEMINI_PROVIDERS.md`** — badges 🔒/🌐 + toggle visibilité + bouton « Utiliser » + avertissement clé publique.
- **`BRIEFING_GEMINI_CAPABILITES.md`** — sections par capacité (LLM/Coding distinct/STT/TTS/VLM/VLA) ; **réglages voix** : choisir le STT (dictée) / TTS (auto-TTS) parmi les services détectés ; **toggle « Mode Coding »** qui envoie `capability:"code"` dans le message WS (§4bis).
- **`BRIEFING_GEMINI_CRON_PROVIDER.md`** — sélecteur Provider (profil) + modèle dans le formulaire cron (envoie `profile_id`). **NON commencé.**

### Endpoints clés que l'UI consomme
- `GET /swarm/models` → `{models:[{host,name,capability,is_local,node_id,node_name,is_default}]}`
- `GET /api/profiles` ; `POST /api/profiles/:id/visibility {visibility}` ; `POST /api/models/use {host,name,capability,node_id?,base_url?}`
- `GET /api/capabilities/selection` ; `GET /api/voice/status`
- `POST/PATCH /api/cron` (accepte `profile_id`, `model`) ; `GET /api/cron`
- WS chat message : champs `provider` (profile id), `model`, `capability` ("code"…)
- Events WS : `skill_applied`, `skill_proposed` (+ token, tool_call, tool_result, etc.)

## 5. CHANTIERS RUST restants (prochain Claude/Codex) — par priorité
### P1 — Parité provider Watcher + Kanban (même pattern que cron)
But : `profile_id` par tâche watcher/kanban, résolu via `appliquer_profil`.
- `laruche-watchers/src/lib.rs` struct `Watcher` (l.21) : ajouter `#[serde(default)] pub profile_id: Option<String>` (+ `pub model: Option<String>`). Mettre à jour les **3 littéraux** : `lib.rs:222`, `laruche-node/src/abeilles_local.rs:270`, `laruche-node/src/main.rs:2853` (create handler — lire `body["profile_id"]`).
- `laruche-kanban/src/lib.rs` struct `KanbanTask` (l.20) : idem ; littéral `lib.rs:121` ; la fn `create()` (l.104) ne prend pas profile_id → ajouter un param ou un setter (appelée par l'abeille kanban_create + l'API).
- Daemons à câbler (réutiliser `appliquer_profil`) : watcher ~`main.rs:7207`, kanban ~`main.rs:7280` (aujourd'hui ils font juste `config.model = current_model`).
- Briefing Gemini ensuite : mêmes sélecteurs Provider dans les forms watcher/kanban.

### P2 — Section MCP dans Settings (l'utilisateur l'a demandée)
État : serveurs MCP chargés de `mcp_servers.json` au boot (`mcp_client.rs:242 charger_mcp_servers`, struct `McpServerConfig` l.232). **Aucun CRUD UI.** `/api/mcp` = serveur MCP exposé, PAS la config.
- À créer (Rust) : `GET /api/mcp/servers` (lit le fichier), `POST /api/mcp/servers` (ajoute/maj + recharge dans la registry), `DELETE /api/mcp/servers/:name` (retire + recharge). Attention au hot-reload de la registry (Arc partagé).
- Briefing Gemini : Settings > MCP (liste + ajouter {name, command, args/url} + supprimer + test connexion).

### P3 — Fermer la boucle réseau « miel » distant
Le provider `"miel"` route en OpenAI vers un node distant, MAIS le node n'expose aujourd'hui que `/infer` (style Ollama), pas `/v1/chat/completions`. → **Exposer un endpoint chat OpenAI-compatible sur `laruche-node`** pour qu'un node consomme réellement le LLM d'un autre. (Les backends LOCAUX llama.cpp/vllm marchent déjà.)

### P4 — public_proxy → annonce mDNS conditionnelle
Aujourd'hui le manifeste mDNS est construit 1× au boot. Pour qu'un provider passé `public_proxy` (ou un backend lancé à chaud) soit annoncé : **re-register périodique** du manifeste (boucle qui reconstruit + ré-annonce toutes les N min). Pré-requis pour que le mesh voie les providers public-proxy.

### P6 — ⭐ Moteurs d'inférence CUSTOM auto-annoncés sur Miel (demande utilisateur)
But : qu'un moteur écrit maison (Rust/Python, OpenAI-compat ou non) puisse **s'annoncer et être détecté** sur le mesh Miel.
- **(a) OpenAI-compat** : DÉJÀ ok via `LARUCHE_OPENAI_ENDPOINTS="label=url,…"` (lu par `local_inference`). Rien à coder, juste documenter.
- **(b) Auto-annonce Miel** (le vrai plug-and-play) : fournir un **helper réutilisable** pour broadcaster un `CognitiveManifest` (capability + host/port) :
  - Rust : exposer un mini-helper au-dessus de `miel-protocol::MielBroadcaster` (≈ ce que fait `laruche-node` au boot).
  - Python : extraire `laruche-voix/src/miel_announce.py` en mini-lib réutilisable.
- **(c) Register côté node** (protocoles exotiques) : `POST /api/services/register {name, capability, url, protocol}` → ajouté à `/swarm/models` + annoncé sur le manifeste mesh ; `DELETE /api/services/register/:name`. UI Settings.
- ⚠️ **BUG à corriger d'abord** : `laruche-node/src/main.rs` (~l.1197, agrégation des nœuds Miel distants) **skippe** `llm|code|vlm|embed|image` → un moteur custom annonçant `capability:llm` est ignoré. **Relâcher ce filtre** (garder uniquement la dédup `already_listed`) pour que les LLM custom du mesh apparaissent.
- Bonus : couper les heartbeats « Ollama is not responding » quand aucun provider ollama n'est configuré (l'utilisateur n'a PAS Ollama, juste llama.cpp/lmstudio/vllm) → bruit dans l'AUDIT.

### P7 — ⭐ Réactivité UI / MAJ dynamique partout (demande utilisateur)
Problème : choisir un LLM (dashboard « Services du mesh » / `POST /api/models/use`) ne met PAS à jour le
**dropdown de modèle en haut** ni les autres panneaux → il faut **F5**. Le backend expose déjà l'état
(`GET /api/profiles` → `active_model`, `GET /api/capabilities/selection`, `GET /api/voice/status`).
- **Niveau 1 (Gemini, spa.html, prioritaire)** : après CHAQUE action mutante (`/api/models/use`, visibilité,
  sélection STT/TTS, cron…), **re-fetch + re-render en place** le dropdown + le badge capacité + le panneau concerné.
  Idem : un état partagé côté JS (le modèle actif/les sélections) que tous les composants lisent, mis à jour après action.
- **Niveau 2 (backend léger + Gemini, « dynamique partout »)** : **push serveur** sur changement d'état.
  Option simple : un canal `broadcast::Sender` dashboard (séparé du `ChatEvent` par session) qui émet un event
  `{type:"state_changed", what:"models|profiles|capabilities|voice"}` ; les handlers mutants l'émettent ;
  un WS/SSE dashboard le diffuse ; l'UI écoute et re-fetch le morceau touché. → tous les onglets/clients synchro sans F5.
  (Alternative encore plus simple : un endpoint `GET /api/state/version` incrémenté à chaque mutation, que l'UI poll léger.)
- Couvre AUSSI : badges visibilité, sélections de capacité, liste des crons, services mesh — tout doit se rafraîchir vivant.

### P8 — Sessions persistantes / notifications / live re-attach (demande utilisateur)
- ✅ FAIT (Claude) : la session est rendue **visible + persistée dès l'envoi** (snapshot avec message user inséré dans `essaim_sessions` + `sauvegarder()` disque, dans le spawn react de `ws_chat` ~main.rs:6085). Survit au F5 ; apparaît dans Sessions avant la réponse.
- ✅ DÉJÀ le cas : l'agent tourne dans un `tokio::spawn` détaché du WS → un F5 ne l'arrête pas.
- RESTE Gemini : **notification cliquable** → poller `GET /api/events?since=` (events `AgentStarted`/`SessionFinished` avec `session_id` déjà émis) → toast qui ouvre la session.
- RESTE Claude (follow-up + gros) : **live re-attach** — stocker un `broadcast::Sender` PAR session (pas par connexion WS) ; à la reconnexion WS sur un `session_id` en cours, s'abonner au flux existant pour revoir les tokens en direct après F5. Aujourd'hui le stream est par-connexion → perdu au F5 (mais le résultat final est bien écrit dans la session).

### P5 — Divers (notés dans LARUCHE_V2.md §0.3)
- `cargo test --workspace` complet (jamais fait).
- Watchers : confirmer/finir la boucle d'événements autonome (différenciateur).
- UI audit mémoire (stats/mutations/proposed déjà côté endpoints).
- `laruche-suggestions` orphelin (0 réf) : câbler ou supprimer.
- third-party #10 `reasoning_effort`, #11 `cache_control` Anthropic.
- `laruche-voix` (Python STT/TTS) et `laruche-channels` (Python bots) = services externes GARDÉS, annoncés sur Miel. `laruche-vscode` GARDÉ (futur pont).

## 6. Pièges connus
- Ollama souvent **down** dans l'env de test → ne JAMAIS faire dépendre un endpoint d'agrégation d'Ollama avec `?` (cf. fix §3.6). Même prudence pour tout nouvel agrégateur.
- llama.cpp de test sur `:8001` (OpenAI-compatible). Node v24 existe (bundle Playwright) mais **pas de npm** dans le PATH.
- Le dropdown modèle du SPA doit envoyer `provider` (= profile_id) en plus du nom de modèle, sinon le handler garde le provider actif.
- Après tout edit de `spa.html` : **rebuild `laruche-node`** (embarqué via `include_str!`).

## 7. Première action conseillée à la reprise
1. Tuer `laruche-node.exe`, `cargo run -p laruche-node`, ouvrir `:8419` → vérifier que **« Services du mesh » se peuple** (fix §3.6) même si Ollama est down.
2. Gemini : reprendre `BRIEFING_GEMINI_CRON_PROVIDER.md` (non commencé) puis re-tester les autres panneaux.
3. Claude/Codex : P1 (watcher/kanban) puis P2 (MCP).
