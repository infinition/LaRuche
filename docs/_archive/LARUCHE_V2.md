# LaRuche v2 — Dossier unique (source de vérité)

> **Ce fichier fusionne TOUS les `.md` du projet** (audit + roadmaps + handoff + architecture +
> hacks + parité outils + README) en un seul document exhaustif, pour ne rien perdre et garder
> l'historique. Généré le **2026-06-20** par audit du code réel.
>
> Structure : **Partie 0** = synthèse vérifiée réconciliée (la vérité actuelle). **Partie 1** =
> inventaire détaillé. **Partie 2** = chantiers. **Archives A→J** = texte intégral des anciens
> documents (préservé verbatim, c'est l'historique).

## Table des matières
- **Partie 0** — Synthèse vérifiée (réconciliation 2026-06-20)
- **Partie 1** — État des lieux & inventaire (ex-`ETAT_DES_LIEUX.md`)
- **Partie 2** — Chantiers (inclus dans la Partie 1, §4)
- **Archive A** — `ARCHITECTURE.md` (vision de fusion d'origine)
- **Archive B** — `ROADMAP_CLAUDE_CODE_PORT.md` (portage agentique Claude Code+third-party, tableau de suivi) ⭐
- **Archive C** — `AUDIT_COMPARATIF.md` (LaRuche vs third-party vs Claude Code)
- **Archive D** — `SKILLS_TOOLS_ARCHITECTURE.md` (skills vs tools, Phases 10/11)
- **Archive E** — `laruche_tools_parity.md` (parité outils, revue 2026-06-20)
- **Archive F** — `third-party_HACKS.md` (catalogue d'astuces third-party)
- **Archive G** — `HANDOFF.md` (passation mémoire T1-T14)
- **Archive H** — `ROADMAP.md` racine (pistes A-E « récolte »)
- **Archive I** — `laruche/ROADMAP.md` (roadmap produit / Miel / capabilities)
- **Archive J** — `README.md` racine (build/run)

---

# PARTIE 0 — Synthèse vérifiée (réconciliation 2026-06-20)

## 0.1 Le projet est BEAUCOUP plus avancé que ne le disaient HANDOFF/README

Les docs racine (HANDOFF, README, ARCHITECTURE) étaient en retard. Les docs `laruche/`
(`ROADMAP_CLAUDE_CODE_PORT.md` + `AUDIT_COMPARATIF.md` + `laruche_tools_parity.md`, revue
**2026-06-20**) sont la réalité, et le **tableau de suivi du portage agentique est quasi 100% ✅**.

**Build vérifié ce jour : `cargo check --workspace` → exit 0** (4 warnings d'imports). 14 crates,
~35 000 lignes Rust, **32 Abeilles**, 149 tests déclarés.

## 0.2 Ce qui est RÉELLEMENT fait (consolidé des 3 docs `laruche/` + audit code)

**Cœur agentique (port Claude Code, Phases 0-6 — toutes ✅) :**
- Boucle ReAct + orchestration partition safe/unsafe (`brain.rs`).
- Budget tokens (`budget.rs`) + budget résultat d'outil (`tool_budget.rs`) + auto/micro-compaction (`laruche-compaction`).
- Récupération maxOutputTokens + repli modèle (`providers.rs`).
- Résumé d'outils volumineux via modèle aux (`tool_summary.rs`).
- **Steering / interruption mid-turn** (canal WS non bloquant) — ✅ fait (le HANDOFF le disait « à faire »).
- Sous-agents à contexte isolé (`subagent.rs`) + `POST /api/agents/spawn` + UI arbre.
- Prompt système 3 tiers cache-friendly (`prompt.rs`).

**Robustesse/sécurité (port third-party, Phase 7 — toutes ✅) :**
- **Classifieur d'erreurs** (`error_classifier.rs`) **branché au retry** (rotation de clé sur 429) — ✅ (HANDOFF le disait « à faire »).
- **Credential pool** multi-comptes + cooldown 429 (`credential_pool.rs`) + UI multi-compte.
- **Threat patterns** anti-injection branchés au dispatch (`threat_patterns.rs`) — ✅.
- Mixture of Agents (`mixture.rs`), compression de trajectoire, notifier proactif (Telegram).

**Autonomie vivante (Phase 9 — ✅) :** canal+modèle par tâche cron, daemon cron qui route le
feedback, **background_review forké** (auto-skill post-tour), **curator/dream à l'inactivité**
(sans suppression, archive recouvrable), goals multi-tours.

**Skills & Kanban (Phases 10/11 — ✅) :** Skill = document OKF vectoriel (`skill_list`/`skill_view`
à divulgation progressive), injection des skills au run cron, **orchestrateur kanban** (tire les
`Ready` tout seul), **blueprints** (automatisations 1-clic), distinction `ToolOrigin::Builtin/Custom`.

**Manques comblés vs concurrents :** **LSP** (`lsp.rs`) ✅, **isolation Git worktree** (`worktree.rs`) ✅,
outils média via plugins ✅, **Vision Multimodal (VLM) & MCP Computer Use** ✅, `read_extract` (PDF/MD) ✅, `web_deep_search` ✅, hot-reload plugins ✅.

**Mémoire (la fusion) :** trait `MemoireCognitive` + 3 backends (Native/SQLite-FTS5-audit/Sidecar),
boucle ReAct mémoire (auto-récup trailing + auto-curation aux), sémantique hybride (embeddings),
cycle de maintenance complet (update/delete/move/review/stats/mutations), UI nid d'abeille.

**Providers :** OpenAI local sans clé (loopback) + auto-détection llama.cpp `:8001` + **Codex OAuth
abonnement** (`codex_auth.rs`, vérifié live 19/06).

## 0.3 Ce qui RESTE vraiment (après dédoublonnage des roadmaps)

Le gros est fait. Le reste réel :
1. **Valider en RUNTIME** (tout compile, mais peu a été vu tourner end-to-end) : lancer `laruche-node`,
   ouvrir `:8419`, exercer chaque onglet ; `cargo test --workspace`.
2. **Watchers autonomes** : la boucle d'événements (fichier/RSS/webhook → tâche → notif) — abeilles
   `watcher_create/list/delete` existent, la boucle runtime reste à confirmer/finir.
3. **⭐ Distribution réseau des modèles (Axe 5)** : le node ne détecte qu'**Ollama** ; généraliser à
   llama.cpp/vLLM/LM Studio + TTS/STT/VLM/VLA, catégoriser dans le manifeste Miel + dashboard. C'est
   l'essence de `laruche-node` et c'est sous-exploité (cf. Archive I, capabilities déjà au protocole).
4. **Auth/profils (Axe 6)** : QR+cookie+hash mot de passe déjà là → finir login password + **OTP/TOTP**
   + gestion comptes UI.
5. **Finitions** : UI audit mémoire (stats/mutations/proposed), Node CRUD mémoire, OKF round-trip,
   `reasoning_effort` (#10) et `cache_control` Anthropic (#11), scraping JS-rendered robuste (web_render),
   `laruche-suggestions` (orphelin : câbler ou supprimer), grep contenu dédié, notebook .ipynb.
6. **Boucle d'apprentissage de bout en bout** : background_review + dream existent → vérifier qu'un skill
   naît vraiment d'une trajectoire et est réutilisé.

## 0.4 Journal des décisions (2026-06-20)
- **PAS DE GIT** pour l'instant (l'utilisateur gère le versionnement plus tard).
- **`paradigm/` supprimé** (portage SQL→Rust accompli) → `SidecarBackend` devient code mort optionnel ;
  chantier « paradigm réel end-to-end » abandonné.
- Nettoyé : `target-codex/`, `target-codex-reconcile/` (6,4 Go), fichiers de test racine.
- **Gardés** (décisions tranchées) : `laruche-voix` (service externe STT/TTS annoncé sur Miel — base de
  l'Axe 5), `laruche-channels` (bots Telegram/Discord/Slack, référence), `laruche-vscode` (futur pont VS Code).
- **Modes de lancement clarifiés** : `laruche` (binaire CLI) = TUI agent autonome par défaut + sous-commande
  `server start` (spawn `laruche-node`) ; `laruche-node` = serveur Web/API/nœud mesh. **Lancer `laruche` ne
  démarre PAS le serveur automatiquement** (à unifier = chantier optionnel).

## 0.5 Note sur les archives ci-dessous
Les Archives A→J sont le **texte intégral** des anciens documents, conservé pour l'historique et pour
ne perdre aucun détail d'implémentation (références de fichiers, owners des lots, specs). En cas de
contradiction avec la Partie 0, **la Partie 0 fait foi** (elle est vérifiée sur le code du 2026-06-20).

---

# PARTIE 1 — État des lieux & inventaire

# LaRuche v2 — État des lieux réel (inventaire vérifié)

> **Source unique de vérité.** Établi le 2026-06-20 par audit du code (pas des docs).
> Les autres `.md` (ARCHITECTURE / HANDOFF / ROADMAP / README) sont partiellement **périmés
> ou contradictoires** — voir §6. Ce fichier consolide ce qui est *réellement* fait.

---

## 1. Résumé exécutif

- **Le projet compile** : `cargo check --workspace` → **exit 0**, 4 warnings (imports inutilisés dans `mcp_client.rs` + `reload_plugins.rs`). Toolchain MSVC épinglée (`rust-toolchain.toml`).
- **14 crates** dans le workspace, ~35 000 lignes de Rust, **32 Abeilles** (outils), **149 tests** déclarés.
- **Pas de git pour l'instant** (décision 2026-06-20 : on ne versionne pas encore).
- La **fusion mémoire est faite et câblée** (3 backends, boucle ReAct mémoire, UI). Les **features d'autonomie** (skills, kanban, watchers, events, permissions, compaction) sont **présentes et compilées**, dont la plupart câblées dans `node`.
- **Codex OAuth abonnement = FAIT** (contredit HANDOFF qui le dit « non commencé »).
- **`paradigm/` supprimé** (portage SQL→Rust accompli) ; nettoyage dépôt fait (voir Axe 1).
- Le gros **reste** : valider en runtime (watchers autonomes + UI), distribution réseau des modèles (au-delà d'Ollama), auth (OTP/mot de passe), boucle d'apprentissage.

---

## 2. Inventaire physique

### 2.1 Arborescence racine `laruche-v2/`
```
ARCHITECTURE.md   ← doc de conception d'origine (périmé : « aucun code fusionné »)
HANDOFF.md        ← état mémoire T1-T14 (récent mais partiellement périmé : T12)
ROADMAP.md        ← pistes A-E « récolte de modules » (récent, affectations multi-agents)
third-party_HACKS.md   ← catalogue d'astuces third-party à porter (référence, pas un état)
README.md         ← build/run (table « phases » PÉRIMÉE : dit UI mémoire « planifiée »)
ETAT_DES_LIEUX.md ← CE FICHIER (source de vérité)
demo.bat / lancer.bat
laruche/          ← le corps Rust (workspace 14 crates)
```
> **`paradigm/` SUPPRIMÉ le 2026-06-20** : il avait servi à porter le concept de mémoire
> SQL→Rust, mission accomplie. Conséquence : le `SidecarBackend` (client HTTP vers paradigm)
> devient **optionnel/sans cible locale** — il reste compilable (Rust pur) mais ne peut plus
> être validé end-to-end ici. Le chantier « paradigm réel » (ex-T6) est **abandonné**.

### 2.2 Crates Rust (`laruche/`) — taille & câblage réel
| Crate | Lignes | Dans workspace | Câblé (réf. dans node/essaim/cli) | État |
|---|---:|:---:|:---:|---|
| `laruche-essaim` | 13 872 | ✅ | (le moteur) | Cœur ReAct + 32 Abeilles |
| `laruche-node` | 10 162 | ✅ | (le serveur) | API/WS/UI/swarm |
| `laruche-cli` | 3 721 | ✅ | (client TUI) | OK |
| `miel-protocol` | 2 586 | ✅ | (transport) | mDNS/swarm |
| `laruche-memoire` | 1 895 | ✅ | 3 réf. | **trait + 3 backends** |
| `laruche-compaction` | 664 | ✅ | 3 réf. | câblé essaim |
| `laruche-skills` | 464 | ✅ | 7 réf. | câblé node (OKF) |
| `laruche-permissions` | 390 | ✅ | 3 réf. | câblé essaim |
| `laruche-kanban` | 343 | ✅ | 17 réf. | câblé node+essaim |
| `laruche-client` | 279 | ✅ | — | lib client |
| `laruche-watchers` | 252 | ✅ | 13 réf. | câblé node — **boucle autonome à vérifier** |
| `laruche-events` | 225 | ✅ | 19 réf. | câblé node (bus audit) |
| `laruche-suggestions` | 71 | ✅ | **0 réf.** | ⚠️ **ORPHELIN** (personne ne l'utilise) |
| `laruche-dashboard` | 7 + `spa.html` (5 735 l.) | ✅ | (include_str!) | SPA mono-fichier vanilla |

### 2.3 Hors workspace — décisions tranchées (2026-06-20)
- `laruche-voix/` — **Python, GARDÉ** : service **STT (whisper) + TTS (piper) + annonce Miel** (449 l., `stt_service.py`/`tts_service.py`/`miel_announce.py`). **Ce n'est PAS un artefact mort** : le node Rust le **découvre déjà** sur le mesh (`/api/voice/status`, caps `stt`/`tts`). Statut = **service modèle externe**, analogue à Ollama/llama.cpp. Le mono-binaire reste l'agent ; les modèles audio restent des services séparés annoncés via Miel. → Documenter comme tel, pas porter.
- `laruche-channels/` — **Python, GARDÉ (référence)** : bots **Telegram/Discord/Slack** (422 l.). Le node mentionne déjà des intégrations Discord/Slack. À porter/brancher plus tard ; optionnel, hors mono-binaire.
- `laruche-vscode/` — extension **Node, GARDÉE** : héritage de l'ancien LaRuche (agrégateur LLM). **Évoluera pour parler à LaRuche depuis VS Code.** Hors mono-binaire mais voulue.

### 2.4 paradigm/ — SUPPRIMÉ (2026-06-20)
Servait de référence pour porter la mémoire SQL→Rust (atlas/sqlite-store/consolidator/embeddings) et de sidecar de prototypage. Portage fait → dossier retiré. La logique vit désormais dans `laruche-memoire` (`SqliteBackend` + `NativeBackend`). Le `SidecarBackend` subsiste comme code mort optionnel (brancher un paradigm externe si jamais besoin).

### 2.5 Les 32 Abeilles (outils)
`browser · calendrier · clarify · delegation · essaim_status · execute_code · fichiers · file_watch · git · image_search · kanban_next · knowledge · lsp · math · mcp_resources · mcp_tool · media · memoire · mixture · plan_mode · plugins · read_extract · recherche_fichiers · reload_plugins · run_script · shell · todo · web_deep · web_fetch · web_recherche · worktree`

---

## 3. Ce qui est RÉELLEMENT fait (vérifié dans le code)

### Mémoire (cœur de la fusion) ✅
- **Trait `MemoireCognitive`** + 3 backends : `NativeBackend` (RAM lexical), `SqliteBackend` (SQLite+FTS5+audit+embeddings hybride), `SidecarBackend` (HTTP→paradigm réel). Sélection par `LARUCHE_MEMOIRE_BACKEND` dans `node`.
- **Boucle ReAct mémoire** : auto-récupération (injection trailing prefix-cache) + auto-curation (`tokio::spawn`, modèle aux).
- **Sémantique** : `Embedder` + cosine + `OllamaEmbedder`, recherche hybride (0.7 cosinus + 0.3 lexical), test `sqlite_semantic.rs`.
- **Cycle de maintenance** : update/delete/move/review/list_proposed/suggest_nodes + `stats` + `mutations` (audit) — sur les 3 backends + abeilles + API.
- **UI Mémoire** : onglet SPA hexagones SVG + endpoints `/api/memory/*` (search/write/tree/stats/mutations). *(HANDOFF T4 ✅ — contredit le README qui dit « planifié ».)*

### Autonomie & outils ✅ (présents, compilés, majoritairement câblés)
- **Permissions** (`laruche-permissions`, câblé essaim) : modes Default/Plan/AcceptEdits/Auto, deny>allow.
- **Compaction** (`laruche-compaction`, câblé essaim) : résumé structuré + gros outputs sur disque.
- **Events** (`laruche-events`, 19 réf. node) : bus NDJSON d'audit.
- **Kanban** (`laruche-kanban`, 17 réf.) : board + abeille `kanban_next`.
- **Skills OKF** (`laruche-skills`, 7 réf., test `okf.rs`).
- **Watchers** (`laruche-watchers`, 13 réf.) : structures + abeille `file_watch` (boucle événementielle autonome **à confirmer en runtime**).
- **Outils third-party portés** : `file_edit`, `file_read` (offset/limit), `clarify` (rend la main), `run_script` (RPC multi-étapes), `execute_code`, `todo`, `read_extract`, shell read-only auto-approuvé, `web_deep`/`web_fetch` nettoyés.
- **Garde-fous de boucle** (warning 3×, halt 6×), client aux, toolset stable, strip `<think>`, toggles Abeilles.

### Providers ✅
- OpenAI local sans clé (loopback) + auto-détection llama.cpp `:8001`.
- **Codex OAuth abonnement** : `codex_auth.rs` (20 KB, 4 tests), provider `codex` (Responses API), endpoints `/api/auth/codex/{status,start,logout}` dans node, UI Settings. **Vérifié live le 19/06** (modèles `gpt-5.5/5.4/5.4-mini`). → **T12 est FAIT** (HANDOFF non à jour).

---

## 4. Chantiers à finir (axes prioritaires)

### Axe 1 — Hygiène dépôt (rapide, gros confort) 🔴
1. ~~Premier commit git~~ → **PAS DE GIT pour l'instant** (décision 2026-06-20).
2. ✅ **FAIT (2026-06-20)** : supprimé `paradigm/`, `target-codex/`, `target-codex-reconcile/`, fichiers de test racine (`fix.py`, `update_code.py`, `test_yahoo.py`, `*.html`, `*.md` de test, `Numb.mp3`, `test_file.txt`). Reste `target/` (cache de build vivant, partiellement vidé car verrouillé par un `rustc` actif — se reconstruira seul).
3. ✅ **TRANCHÉ** : `laruche-voix` GARDÉ (service externe STT/TTS), `laruche-channels` GARDÉ (référence bots), `laruche-vscode` GARDÉ (futur pont VS Code). Voir §2.3.
4. **`laruche-suggestions`** : orphelin (0 réf.) → reste à câbler ou supprimer.
5. **Réconcilier les docs** : README (table phases périmée), HANDOFF (T12 périmé), ARCHITECTURE (« aucun code fusionné » faux). Ce fichier les remplace ; les marquer comme historiques.

### Axe 2 — Valider en runtime (le maillon manquant) 🟠
6. ~~Paradigm réel end-to-end~~ → **ABANDONNÉ** (paradigm supprimé). On valide directement `sqlite`/`native`.
7. **`cargo run -p laruche-node`** complet + ouvrir `:8419` : vérifier que chaque onglet (Mémoire/Kanban/Settings/Codex) marche vraiment, pas seulement compile.
8. **Watchers autonomes** : confirmer/implémenter la boucle d'événements (fichier modifié → tour d'agent → notif). Différenciateur produit, mais pas vu de loop runtime.
9. **Lancer la suite de tests** : `cargo test --workspace` (149 tests déclarés, jamais re-vérifiés globalement ici).

### Axe 3 — Finir les features partielles 🟡
10. **Node CRUD mémoire** : `create_node`/`update_node`/`delete_node` (auto-créés mais pas éditables).
11. **UI audit** : panneaux `stats` + journal `mutations` + file de revue `proposed` (endpoints existent, affichage à brancher).
12. **Interop OKF** : `export_okf`/`import_okf` round-trip (partiellement fait côté skills, à compléter côté mémoire).
13. **Hacks third-party restants** : #5 classifieur d'erreurs, #6 compression préflight, #7 interrupt/steer (bouton Stop), #10 reasoning_effort, #11 cache_control Anthropic.

### Axe 4 — Boucle d'apprentissage (le cap produit) 🟢
14. **background_review** post-tour (fork aux-model, whitelist mémoire+skills) → auto-création de skills depuis une trajectoire réussie.
15. **dream sur idle** → suggestions de consolidation remontées dans l'UI.
16. **Boucle autonome complète** : watcher → tâche Kanban (idempotente) → dispatcher → agent+mémoire → handoff audité → dream → skill.

### Axe 5 — ⭐ Distribution réseau des modèles (l'essence de laruche-node) 🟠
**Constat :** aujourd'hui le node ne détecte QUE **Ollama** (`fetch_local_models` → `:11434/api/tags`). Mais l'infra mesh existe déjà : il **découvre des nœuds Miel** annonçant des capacités `stt`/`tts`/`agent` (`/api/voice/status`), et `laruche-voix` (Python) prouve le modèle producteur↔consommateur.
17. **Généraliser la détection locale** au-delà d'Ollama : sonder **llama.cpp** (`:8001` OpenAI `/v1/models`), **vLLM**, **LM Studio**, services **TTS/STT/VLM/VLA** locaux → les présenter sur le réseau via mDNS/Miel comme il le fait pour Ollama.
18. **Catégoriser** les capacités annoncées (LLM / VLM / TTS / STT / VLA / agent / coding) dans le manifeste Miel.
19. **Dashboard** : afficher par catégorie ce que ce node et les autres nœuds du mesh exposent (LLM, TTS, agent, coding…), avec source (local vs distant).
20. **Consommer** un service distant (ex. STT/TTS d'un autre node) de façon transparente dans la boucle agent.

### Axe 6 — Profils & authentification 🟡
**Constat (déjà partiel dans `auth_user.rs`)** : login **QR-code** + cookie signé **BLAKE3 HMAC** (`laruche_auth={user_id}:{ts}:{hmac}`) + **hash de mot de passe optionnel déjà présent** (BLAKE3+sel, `verify_password`). Donc le mot de passe est à moitié là, pas l'OTP.
21. **Finir le login par mot de passe** (flux UI complet, aujourd'hui « QR-only » par défaut).
22. **Ajouter l'OTP/TOTP** (2FA, codes temporels) en complément du mot de passe.
23. **Gestion des comptes simplifiée** : création/édition/suppression d'utilisateurs, rôles, depuis l'UI (Settings → Comptes).

---

## 4bis. Modes de lancement — clarification (ta question)

**Réponse courte : non, lancer `laruche` ne démarre PAS le serveur automatiquement.** Ce sont deux binaires indépendants, comme avant v2 :

| Binaire | Crate | Rôle | Comportement |
|---|---|---|---|
| **`laruche`** | `laruche-cli` | **Client TUI + manager** | `laruche` seul → **mode TUI** (agent interactif dans le terminal, autonome, n'a pas besoin du serveur). Sous-commandes : `ask`, `chat`, `discover`, `auth`, `mcp`, et **`server start/stop/...`** (qui spawn/tue `laruche-node` en sous-process). |
| **`laruche-node`** | `laruche-node` | **Serveur** | Web UI `:8419` + API + agent + nœud mesh (mDNS/Miel). Doit être lancé explicitement (`laruche server start`, ou directement `laruche-node`). |

→ Le TUI `laruche` embarque sa **propre** boucle agent (`EssaimConfig` en process) ; il ne dépend pas du serveur. Le serveur est le **nœud réseau** (ce qui distribue les modèles, l'UI web, le multi-user).
→ **Décision à prendre (chantier optionnel)** : veut-on **unifier** (`laruche` lance aussi le serveur en fond automatiquement) ? Aujourd'hui non — c'est explicite. À trancher selon l'usage voulu.

---

## 5. Comment build/run (vérifié)
```bash
cd laruche
cargo check --workspace          # vert (exit 0)
cargo run -p laruche-essaim --example poc_memoire   # POC mémoire (besoin llama.cpp:8001)
cargo run -p laruche-node                            # serveur → http://localhost:8419
# backends : LARUCHE_MEMOIRE_BACKEND = native(défaut) | sqlite | sidecar(paradigm externe, abandonné)
```
Pièges : toolchain gnu KO (MSVC épinglée OK) · Windows verrouille les `.exe`/`.dll` de `target/` si un `rustc`/`laruche-node` tourne (tuer le process ou `cargo check`).

---

## 6. Contradictions entre docs (à corriger / ignorer)
| Doc | Affirmation | Réalité |
|---|---|---|
| ARCHITECTURE.md | « Aucun code fusionné encore » | Faux — fusion faite, 14 crates |
| README.md (table phases) | onglet Mémoire « planifié » | Fait (T4 ✅) |
| HANDOFF.md | T12 Codex OAuth « non commencé » | **Fait & vérifié live** |
| ROADMAP.md vs HANDOFF.md | 2 systèmes de nommage (pistes A-E vs T1-T14) | Les deux ont atterri dans le code ; ce fichier unifie |

→ Décision suggérée : garder ARCHITECTURE.md (vision) + third-party_HACKS.md (référence), archiver ROADMAP.md/HANDOFF.md, piloter depuis **ETAT_DES_LIEUX.md**.


<a id="archive-A"></a>

---

# ARCHIVE A — ARCHITECTURE.md (vision de fusion d'origine)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# LaRuche v2 — Architecture de fusion (le squelette)

> Fusion de **LaRuche** (le corps : moteur d'agent Rust, outils, swarm, interfaces)
> et **paradigm-memory** (l'hippocampe : carte cognitive auditée) en un agent
> autonome local-first destiné à concurrencer third-party / third-party.

État : **document de conception** (le « papier »). Aucun code fusionné encore.

---

## 1. Positionnement

| Axe | third-party / third-party | LaRuche v2 (cible) |
|---|---|---|
| Installation | uv + Python + Node + ffmpeg + ripgrep + Git portable | **1 binaire Rust** (`cargo install` / un `.exe`) |
| Mémoire | `MEMORY.md` plat + FTS5 + Honcho | **Carte cognitive auditée** (activation, dream, snapshots) |
| Topologie | 1 cerveau sur un VPS | **Mesh local-first** (mDNS/swarm, multi-nœuds, multi-user) |
| Boucle d'apprentissage | mature (curator + background review) | à construire — mais briques déjà présentes (cf. §7) |
| Kanban / tâches durables | board SQLite + dispatcher | à porter (cf. §8) |
| Watchers événementiels | non (cron seulement) | **différenciateur** (cf. §9) |
| Interop format de savoir | — | **OKF-natif** (cf. §10) |

**Thèse produit :** un agent qui s'installe en un binaire, vit sur ton réseau (pas
dans le cloud), avec une mémoire cognitive auditée, une boucle d'apprentissage, un
board Kanban autonome et des watchers événementiels. Le **mono-binaire Rust** est
l'avantage déloyal n°1.

---

## 2. Décisions verrouillées

1. **Lignée LaRuche** — le produit reste « LaRuche » ; paradigm devient *la mémoire de la ruche*.
2. **Copie figée** — on copie les deux bases dans ce dépôt, nouveau git propre. Pas de submodule/subtree pour démarrer.
3. **Destination = tout-Rust mono-binaire.** Le sidecar Node n'est qu'un échafaudage de prototypage, pas la cible.
4. **Cœurs intacts.** On ne modifie ni `rag.rs` ni `memory-core` : on compose via du code neuf.

---

## 3. Principe d'intégration : une interface, deux backends

Abstraire la mémoire derrière **un seul trait Rust**, et migrer le backend dessous
sans jamais retoucher le moteur d'agent.

```
                     ┌──────────────────────────────────────────────┐
 brain.rs (ReAct) ──►│  trait MemoireCognitive   (crate laruche-memoire)
                     └──────────────────────────────────────────────┘
                              │                         │
                  ┌───────────▼──────────┐   ┌──────────▼─────────────┐
                  │ SidecarBackend (P1)  │   │ NativeBackend (P3→)    │
                  │ reqwest → JSON-RPC   │   │ rusqlite (FTS5 bundled)│
                  │ paradigm serve :8765 │   │ + fastembed / Ollama   │
                  │ → prototype en JOURS │   │ → LE produit, 100% Rust│
                  └──────────────────────┘   └────────────────────────┘
```

**Garantie : `brain.rs` ne connaît que le trait.** Passer du sidecar au natif est un
swap drop-in. Aucun travail jeté.

### 3.1. Le trait (esquisse)

```rust
#[async_trait]
pub trait MemoireCognitive: Send + Sync {
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<ContextPack>;
    async fn write(&self, item: MemoryItem) -> Result<MutationId>;
    async fn propose_write(&self, item: MemoryItem) -> Result<MutationId>;
    async fn read_node(&self, id: &str) -> Result<NodeView>;
    async fn dream(&self) -> Result<Vec<Suggestion>>;          // consolidation
    async fn export_okf(&self, dir: &Path) -> Result<()>;       // §10
    async fn import_okf(&self, dir: &Path) -> Result<usize>;    // §10
    async fn health(&self) -> Result<Health>;
}
```

Types repris du schéma paradigm (`memory-core/src/schemas.mjs`) traduits en structs Rust.

---

## 4. Modèle en couches

```
┌─────────────────────────── LaRuche v2 (1 superviseur) ────────────────────────────┐
│  CORPS (Rust, ~intact)                          MÉMOIRE                            │
│  ├─ laruche-node : API, WS, auth, swarm, TUI    ├─ P1 : sidecar paradigm (Node)    │
│  ├─ laruche-essaim : boucle ReAct + Abeilles    └─ P3 : laruche-memoire (Rust pur) │
│  ├─ miel-protocol : mDNS, swarm, QoS                                               │
│  ├─ laruche-cli / dashboard / vscode / voix     COGNITION (NOUVEAU, le différentiel)│
│  └─ laruche-channels : Telegram/Discord/Slack   ├─ couche mémoire dans brain.rs    │
│                                                 ├─ skills (trajectoires → outils)  │
│  ★ laruche-memoire : trait + 2 backends         ├─ dream/curator (idle)            │
│  ★ laruche-kanban  : board durable + dispatcher └─ background-review (post-tour)   │
│  ★ laruche-watchers: triggers événementiels                                        │
└────────────────────────────────────────────────────────────────────────────────────┘
                        ★ = code neuf. Cœurs LaRuche & paradigm gardés intacts.
```

---

## 5. Disposition du dépôt (copie figée)

```
laruche-v2/
  ARCHITECTURE.md          ← ce fichier
  README.md
  laruche/                 ← copie du corps Rust (workspace Cargo)
    laruche-memoire/       ← ★ trait MemoireCognitive + backends
    laruche-kanban/        ← ★ board SQLite + dispatcher + Abeilles kanban
    laruche-watchers/      ← ★ registre de watchers + boucle d'événements
    laruche-essaim/        ← brain.rs câblé sur le trait + boucle d'apprentissage
    laruche-dashboard/     ← spa.html + onglets Mémoire / Kanban (vanilla)
    ...
  paradigm/                ← copie de la mémoire (sidecar P1, optionnel ensuite)
```

Exclus de la copie : `target/`, `node_modules/`, `dist/`, les `.git/` d'origine.

---

## 6. UI mémoire dans le SPA LaRuche (nid d'abeille)

Constat : `laruche-dashboard` = **un seul `spa.html` vanilla** (3541 l.) embarqué dans
le binaire via `include_str!`, zéro build. paradigm = React/Vite/Tauri. **Porter le
React réintroduirait le bagage Node qu'on tue.** On ne porte donc pas le code, on porte
les *fonctions*.

- Reconstruire en **onglets natifs vanilla** dans `spa.html`, style ambre/ruche (tokens existants) :
  recherche + animation d'activation, éditeur d'item, file de revue (accept/reject),
  journal d'audit, dream/consolidation.
- **Carte cognitive rendue en nid d'abeille** (SVG vanilla) : chaque nœud = une alvéole
  hexagonale, l'activation = cellules qui s'illuminent en ambre. La carte est un *arbre*
  → layout hiérarchique/radial faisable sans dépendance. Peut servir de **visualiseur OKF**.
- Données via endpoints `/api/memory/*` sur `laruche-node` → trait `MemoireCognitive`.
  **L'UI marche à l'identique sidecar (P1) ou natif (P3).**
- App Tauri de paradigm : gardée en option (power users, même SQLite), pas l'UI livrée.

---

## 7. Boucle d'apprentissage — blueprint repris de third-party

third-party (vérifié dans sa source) repose sur deux mécanismes ; on les reproduit avec nos briques.

### 7.1. `dream` / curator (consolidation) — `agent/curator.py`
- **Déclenché à l'inactivité** (pas de daemon cron) : si idle et dernier run > `interval_hours`.
- Fait du **umbrella-building** : regroupe les skills proches sous des parapluies,
  auto-transitionne le cycle de vie, patche, épingle. **N'efface jamais — archive** (récupérable).
- → **Chez nous : `memoire.dream()` de paradigm est déjà ce moteur.** On l'appelle sur idle
  (timer/`cron.rs`) et on remonte les suggestions dans l'onglet Mémoire. *Quasi fait.*

### 7.2. `background_review` (auto-création de skills) — `agent/background_review.py`
- **Après chaque tour**, fork l'agent dans un thread démon, rejoue le snapshot, se demande
  « sauver/mettre à jour un skill ou une mémoire ? ». Whitelist d'outils = **mémoire + skills only**.
  N'altère jamais la conversation principale.
- → **Chez nous :** sous-agent via l'Abeille `delegation` (déjà là), avec registry d'outils
  restreint, lancé en post-tour depuis `brain.rs`. Écrit via le trait `MemoireCognitive`.

### 7.3. Skills comme artefacts
- third-party : fichiers `SKILL.md` (standard agentskills.io), invoqués `/nom`.
- → **Chez nous :** réutiliser le **système de plugins JSON existant**, OU adopter `SKILL.md`
  (recommandé : interopérable). Stockage + invocation déjà présents.

**Difficulté : moyenne.** Le sous-système coûteux (moteur cognitif + consolidation) est
déjà dans paradigm. Le reste = glue d'orchestration (quelques centaines de lignes Rust).

---

## 8. Kanban (façon third-party) — `laruche-kanban`

Board de tâches durable, piloté par l'agent ET l'humain. Repris de `third-party kanban`.

- **Stockage :** SQLite (WAL). Statuts `triage → todo → ready → running → blocked → done → archived`.
- **Dépendances :** un enfant passe `todo→ready` quand tous ses parents sont `done`.
- **Workspaces :** `scratch` (tmp éphémère), `dir:<path>` (partagé), `worktree` (git).
- **Dispatcher :** boucle (réutilise `cron.rs`) qui réclame atomiquement les `ready`, spawn
  l'agent essaim assigné, récupère les workers crashés, respecte concurrence + `scheduled_at`.
- **Outils agent (Abeilles) :** `kanban_show / create / complete / block / unblock / comment /
  heartbeat / link`. L'agent ne déplace pas les cartes à la main — il pilote l'état via outils.
- **Surfaces humaines :** onglet Kanban (drag-drop) dans le SPA, slash `/kanban`, CLI.
- **Idempotence :** clé d'idempotence pour dédupliquer les déclencheurs cron/webhook/watcher.
- **Traçabilité :** tables `task_runs` + `task_events` append-only (postmortems).

Briques déjà présentes : `cron.rs` (dispatcher tick), `delegation` (spawn), SPA (UI), session model.

---

## 9. Crons + Watchers — `laruche-watchers`

- **Crons :** déjà dans `cron.rs`. Reste à câbler le langage naturel + la livraison via channels.
- **Watchers (événementiel, primitive de premier rang — rare ailleurs) :** registre de
  déclencheurs persistants (changement de fichier, RSS/URL, webhook, email, log, prix…).
  Sur déclenchement → **crée une tâche Kanban** (clé d'idempotence) → l'agent traite.
  Le watcher peut interroger la mémoire cognitive (« qu'est-ce qui a changé depuis ? »).
- Briques : Abeille `file_watch` (déjà là) + boucle scheduler + channels.

### Boucle autonome complète (la valeur émergente)
```
Watcher détecte un changement → crée une tâche Kanban (idempotente)
  → dispatcher la réclame → spawn agent essaim AVEC mémoire cognitive
  → agent termine, écrit son handoff dans la carte (audité)
  → dream consolide à l'idle → un skill naît de la trajectoire
```

---

## 10. Interop OKF (Open Knowledge Format, Google)

OKF = fichiers Markdown + frontmatter YAML, en arborescence, reliés par liens markdown
(`type` seul obligatoire). **C'est un format d'échange, pas un moteur** (ni recherche, ni
embeddings, ni consolidation). Son modèle ≈ celui de paradigm (nœud↔dossier, item↔doc,
tags↔tags, liens↔liens, audit↔git).

- **Stratégie : OKF = format d'import/export, pas le stockage.** SQLite reste la source de
  vérité (sinon perte de l'indexation FTS5/embeddings). Adaptateur petit (mapping ~1:1,
  réutilise `memory_import_markdown` / `memory_export`).
- **Bénéfices :** ingérer n'importe quel bundle OKF dans la carte ; exporter la mémoire en
  bundle OKF portable et versionnable par git ; l'onglet nid d'abeille devient un visualiseur OKF.
- **Premier arrivé — la vérité :** supporter OKF *en soi* est trivial (spec d'une page).
  Le fossé défendable = être le **premier moteur cognitif local-first mono-binaire OKF-natif**.
  On surfe le standard Google pour l'interop ; le moteur reste le différenciateur.
- **Ne pas pivoter pour ça :** adaptateur optionnel, collé au backend Rust natif (P3). v0.1, ça bougera.

---

## 11. Plan par phases (chaque phase livrable seule)

| Phase | Contenu | Sortie observable |
|---|---|---|
| **P0** | Scaffold : copie des 2 bases, git propre, superviseur lance `paradigm serve` + health-check | `cargo build` OK, sidecar mémoire répond sur :8765 |
| **P1** | Crate `laruche-memoire` + `SidecarBackend` + Abeille `memoire` | L'agent lit/écrit la carte cognitive — **agent fusionné vivant** |
| **P2** | Câblage `brain.rs` (pré-récup + post-curation), débranchement `rag.rs` + onglet Mémoire (§6) | Mémoire automatique + carte nid d'abeille dans le SPA |
| **P3** | `NativeBackend` (port Rust atlas+sqlite-store) → swap drop-in ; adaptateur OKF (§10) | **Mono-binaire Rust**, import/export OKF |
| **P4** | Boucle d'apprentissage (§7) : background-review + dream sur idle + skills | Skills auto-créés, consolidation autonome |
| **P5** | `laruche-kanban` (§8) : board + dispatcher + Abeilles + onglet SPA + slash | Tâches durables pilotées par l'agent |
| **P6** | `laruche-watchers` (§9) : triggers → Kanban + crons en langage naturel | Boucle autonome complète (watcher→kanban→agent→mémoire) |

Cible finale : `cargo install laruche-node` → agent local-first, mémoire cognitive auditée,
mesh LAN, boucle d'apprentissage, Kanban autonome, watchers. Zéro Node, zéro Python.

---

## 12. Risques & garde-fous

- **Focus (solo, multi-projets)** → phases courtes, chacune utile seule. P1 donne déjà un agent supérieur à l'existant.
- **Port Rust** → le trait isole tout ; le sidecar reste comme filet jusqu'à parité (tests de parité en P3).
- **Dérive upstream** → cœurs intacts, code neuf séparé.
- **Dispersion par les nouveautés (OKF, etc.)** → adaptateurs optionnels collés à la bonne phase ; ne jamais réordonner le cœur P0→P3 pour une feature shiny.


<a id="archive-B"></a>

---

# ARCHIVE B — ROADMAP_CLAUDE_CODE_PORT.md (portage agentique + tableau de suivi)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# Roadmap — Portage des techniques agentiques de Claude Code vers LaRuche (Rust)

> **But** : reproduire en Rust, dans LaRuche, les techniques agentiques « production-grade »
> de Claude Code (la reconstruction TS de référence est dans `C:\Users\infinition\Desktop\aa`).
> Cible : moteur ReAct local-first, robuste, qui converge, gère son contexte, et rivalise
> avec third-party/third-party. **Tout-Rust, mono-binaire.**
>
> Ce document est fait pour être suivi par plusieurs agents (Codex, Gemini, Claude) en
> parallèle, sur des **zones de fichiers disjointes**.
>
> 📊 État comparatif détaillé vs third-party & Claude Code : [AUDIT_COMPARATIF.md](./AUDIT_COMPARATIF.md)
> (mapping audit → lots roadmap en bas de l'audit).

## Source de vérité (repo `aa` = Claude Code TS)

| Technique | Fichier(s) référence dans `aa/src` |
|---|---|
| Boucle d'agent (turns, continue-sites, état mutable) | `query.ts`, `QueryEngine.ts` |
| Orchestration d'outils (partition safe/unsafe, concurrence) | `services/tools/toolOrchestration.ts` |
| Exécution d'outils (budget résultat, tool_use_id, content replacement) | `services/tools/toolExecution.ts` |
| Budget de tokens (per-turn, countdown) | `query/tokenBudget.ts` |
| Auto-compaction (seuil) | `services/compact/autoCompact.ts` |
| Micro-compaction (éviction par tool_use_id) | `services/compact/microCompact.ts`, `apiMicrocompact.ts` |
| Nettoyage post-compaction | `services/compact/postCompactCleanup.ts` |
| Permissions (canUseTool, règles, suggestions) | `hooks/useCanUseTool.tsx` |
| Prompt système modulaire (3 tiers) | `constants/systemPromptSections.ts`, `utils/systemPrompt.ts` |
| Sous-agents (fork, resume, contexte isolé, snapshot mémoire) | `tools/AgentTool/{runAgent,forkSubagent,resumeAgent,agentMemory}.ts` |
| Consolidation mémoire (dream) | `services/autoDream/{autoDream,consolidationLock,consolidationPrompt}.ts` |
| Résumé d'outil (gros outputs) | `services/toolUseSummary/*`, `services/tools/...` |
| Steering / messages en file (interruption mid-turn) | `context/QueuedMessageContext.tsx`, `query.ts` |
| Récupération maxOutputTokens (troncature) | `query.ts` (`isWithheldMaxOutputTokens`, recovery) |
| Modèle de repli (fallback) | `query.ts` (`fallbackModel`), `hooks/useMainLoopModel.ts` |

## Mapping cibles LaRuche

| Sous-système | Crate / fichier LaRuche cible |
|---|---|
| Boucle ReAct, dispatch outils | `laruche-essaim/src/brain.rs` |
| Trait outil (Abeille) | `laruche-essaim/src/abeille.rs` |
| Providers LLM (stream, recovery, fallback) | `laruche-essaim/src/providers.rs` |
| Session / messages / compaction locale | `laruche-essaim/src/session.rs`, crate `laruche-compaction` |
| Budget tokens / résultat | nouveaux : `laruche-essaim/src/budget.rs`, `tool_budget.rs` |
| Prompt système | `laruche-essaim/src/prompt.rs` |
| Mémoire + dream | crate `laruche-memoire` (trait `MemoireCognitive`, backends) |
| Permissions | crate `laruche-permissions` |
| API + UI | `laruche-node/src/main.rs`, `laruche-dashboard/src/templates/spa.html` |

## Règles de coordination (IMPORTANT)

1. **Zones disjointes.** Un fichier = un seul propriétaire par vague. Voir la colonne « Owner ».
2. **`brain.rs`, `providers.rs` = Claude uniquement** (cœur de boucle, point de contention).
   Les autres écrivent des **modules/fonctions pures** que Claude branche ensuite.
3. **Crate isolé pour travail parallèle** : si vous créez/modifiez un crate à part, gardez un
   `[workspace]` vide dans son `Cargo.toml` pendant le dev pour le détacher du workspace parent
   (évite la contention sur le `Cargo.lock`/`Cargo.toml` racine). Claude réintègre au merge.
4. **Pas de git** (l'utilisateur gère git). Compilez avec
   `cargo build -p <crate>` et testez `cargo test -p <crate>`. Toolchain : `stable-x86_64-pc-windows-msvc`
   (déjà épinglée par `rust-toolchain.toml`). Pas de gnu (dlltool absent).
5. **FR sans accents dans le code** n'est PAS requis ici (le code existant a des accents en commentaires).
6. **Mettre à jour le tableau de suivi** en bas de ce fichier en fin de lot.

---

## PHASE 0 — Fondations (FAIT ✅ par Claude)

- ✅ Orchestration partition safe/unsafe (`partition_tool_calls`, `is_concurrency_safe`) — `brain.rs`.
- ✅ `shell_exec` : PowerShell sous Windows, `success` = code de sortie, timeout 300 s — `abeilles/shell.rs`.
- ✅ Garde-fou par-nom relevé (12/20), par-signature conservé (3/6) — `brain.rs`.
- ✅ Dropdown mode de permission (Default/AcceptEdits/Plan/Bubble/Auto) persistant — node + SPA.
- ✅ Auth Codex OAuth (abonnement ChatGPT) + provider `codex` Responses API.

---

## PHASE 1 — Contexte & budget (la plus grosse perte de perf aujourd'hui)

> Sans gestion de contexte, les longues tâches explosent le prompt et le modèle « oublie ».
> C'est la priorité #1.

### Lot 1.A — Budget de résultat d'outil (Owner : **CODEX**)
**Réf** : `services/tools/toolExecution.ts` (`applyToolResultBudget`, `maxResultSizeChars`).
**Zones** : `laruche-essaim/src/abeille.rs`, **nouveau** `laruche-essaim/src/tool_budget.rs`.
- Ajouter au trait `Abeille` : `fn max_result_size(&self) -> Option<usize> { None }` (défaut illimité).
- `tool_budget.rs` : fonction pure `tronquer_resultat(output, max) -> String` (coupe + marqueur `… (tronqué)`),
  et `appliquer_budget_agregat(results: &mut [...], budget_total)` qui réduit les plus gros d'abord.
- Tests : un gros output est tronqué ; le total respecte le budget ; petits outputs intacts.
- **Ne touche pas `brain.rs`** : Claude branche `tronquer_resultat` dans le dispatch.

### Lot 1.B — Suivi du budget de tokens (Owner : **CODEX**)
**Réf** : `query/tokenBudget.ts`.
**Zone** : **nouveau** `laruche-essaim/src/budget.rs`.
- `struct BudgetTracker { max_context: usize, used: usize }` + `estimer_tokens(texte) -> usize`
  (heuristique ~ len/4, comme l'existant), `restant()`, `ratio_utilise()`, seuils d'alerte (0.75) et critique (0.9).
- Émettre une structure `BudgetStatus { used, max, ratio, warn, critical }` sérialisable (pour l'UI).
- Tests unitaires.

### Lot 1.C — Auto-compaction & micro-compaction (Owner : **CODEX**)
**Réf** : `services/compact/{autoCompact,microCompact,postCompactCleanup}.ts`.
**Zone** : crate `laruche-compaction` (Codex en est propriétaire exclusif).
- `auto_compact(messages, seuil_ratio, budget) -> bool` : décide si compacter.
- `micro_compact(messages)` : éviction des **anciens résultats d'outils** (par identifiant), en gardant
  les N derniers tours intacts + un résumé structuré (réutiliser le résumé existant de `session.rs` si présent).
- `nettoyage_post_compaction` : retire les doublons/observations orphelines.
- Tests : après compaction, le contexte rétrécit, les derniers tours sont préservés.

### Lot 1.D — Panneau Contexte + réglages compaction (Owner : **GEMINI**)
**Zones** : `laruche-node/src/main.rs` (nouveaux endpoints seulement), `laruche-dashboard/src/templates/spa.html`.
- Endpoint `GET /api/context/stats` → `{used, max, ratio, messages, compactions}` (lit `BudgetStatus` exposé par Claude via AppState — interface : un champ `RwLock<BudgetStatus>` que Claude ajoute ; en attendant, renvoyer des valeurs depuis `session`/`essaim_config`).
- Endpoint `GET/POST /api/config/compaction` → `{context_max_messages, compaction_threshold}` (champs déjà dans `EssaimConfig`).
- SPA : Settings > General, ajouter « Contexte » (barre d'usage tokens + seuil compaction éditable) ;
  un petit indicateur d'usage contexte dans le header (réutiliser le style du dropdown permission).

### Lot 1.E — Intégration boucle (Owner : **CLAUDE**)
**Zone** : `brain.rs`.
- Brancher `tool_budget::tronquer_resultat` sur chaque observation d'outil.
- Instancier `BudgetTracker`, déclencher `laruche_compaction::auto_compact`/`micro_compact` quand le seuil est atteint.
- Émettre `BudgetStatus` dans un `RwLock` partagé (consommé par le Lot 1.D).

---

## PHASE 2 — Robustesse de la boucle

### Lot 2.A — Récupération maxOutputTokens + repli modèle (Owner : **CLAUDE**)
**Réf** : `query.ts` (`isWithheldMaxOutputTokens`, recovery, `fallbackModel`).
**Zones** : `providers.rs`, `brain.rs`.
- Détecter une réponse tronquée (finish_reason length / pas de `</tool_call>` fermé) → relancer en demandant
  une continuation, ou réduire `max_tokens` / basculer sur `config.fallback_models` (champ déjà présent).

### Lot 2.B — Résumé d'outil volumineux via modèle auxiliaire (Owner : **CODEX**)
**Réf** : `services/toolUseSummary/*`.
**Zone** : **nouveau** `laruche-essaim/src/tool_summary.rs`.
- Fonction `resumer_output(aux_client, output) -> String` (n'appelle l'aux model que si `output` > seuil).
- Pure/testable (mock du client). Claude branche dans `brain.rs` (Lot 2.D).

### Lot 2.C — Réglages robustesse UI (Owner : **GEMINI**)
**Zone** : `spa.html`, `laruche-node/src/main.rs`.
- Exposer/éditer `fallback_models`, `max_tokens`, seuil de résumé via `/api/config/provider` (étendre l'existant).

### Lot 2.D — Intégration (Owner : **CLAUDE**) — branche 2.A/2.B dans `brain.rs`.

---

## PHASE 3 — Steering & interruption (différenciateur fort)

### Lot 3.A — Canal de steering côté serveur (Owner : **GEMINI**)
**Réf** : `context/QueuedMessageContext.tsx`, `query.ts` (injection de message en cours de tour).
**Zone** : `laruche-node/src/main.rs` (handler WS `ws_chat`).
- Accepter un message WS `{type:"steer", text}` pendant qu'un run est actif → l'envoyer sur un
  `tokio::mpsc::Sender<String>` (le **steer channel**). Interface contractuelle : Claude ajoute le paramètre
  `steer_rx: Option<Receiver<String>>` à la boucle ; Gemini fournit le `tx` et le câblage WS.

### Lot 3.B — UI steering (Owner : **GEMINI**)
**Zone** : `spa.html`.
- Champ d'envoi actif pendant un run ; les messages partent en `steer` au lieu de `message`.
- Afficher les messages de steering injectés dans le fil.

### Lot 3.C — Consommation steering dans la boucle (Owner : **CLAUDE**)
**Zone** : `brain.rs`.
- En début de chaque itération, draîner `steer_rx` (non bloquant) et injecter le texte comme message user
  additionnel avant l'appel modèle.

---

## PHASE 4 — Sous-agents (AgentTool)

### Lot 4.A — Moteur de sous-agent (Owner : **CODEX**)
**Réf** : `tools/AgentTool/{runAgent,forkSubagent,resumeAgent,agentMemory,agentMemorySnapshot}.ts`.
**Zone** : **nouveau** `laruche-essaim/src/subagent.rs` (+ s'appuie sur `delegate` existant).
- `lancer_sous_agent(tache, registry, config, budget)` : boucle ReAct **à contexte isolé**, renvoie un
  résumé compact au parent (pas tout l'historique). Snapshot mémoire optionnel.
- Limiter outils du sous-agent (pas de délégation récursive infinie). Tests.

### Lot 4.B — Abeille `delegate`/`task` (Owner : **CLAUDE**) — expose 4.A comme outil dans `brain.rs`/registry.

### Lot 4.C — UI sous-agents (Owner : **GEMINI**) — afficher l'arbre des sous-agents + leurs résumés dans le fil.

---

## PHASE 5 — Consolidation mémoire (autoDream)

### Lot 5.A — Dream + verrou de consolidation (Owner : **CODEX**)
**Réf** : `services/autoDream/{autoDream,consolidationLock,consolidationPrompt}.ts`.
**Zone** : crate `laruche-memoire` (méthode `dream` du trait + backend SQLite — Codex propriétaire).
- Renforcer `dream()` : détecter doublons/contradictions, proposer fusions, sous **verrou** (un seul dream à la fois),
  via prompt de consolidation (modèle aux). Idempotent. Tests sur backend SQLite.

### Lot 5.B — UI dream (Owner : **GEMINI**) — bouton « Consolider » + revue des fusions proposées (réutilise l'onglet Mémoire).

---

## PHASE 6 — Prompt système modulaire (3 tiers)

### Lot 6.A — Sections de prompt (Owner : **CLAUDE**)
**Réf** : `constants/systemPromptSections.ts`, `utils/systemPrompt.ts`.
**Zone** : `laruche-essaim/src/prompt.rs`.
- Découper le prompt en : (1) identité stable (cache-friendly), (2) capacités/outils, (3) contexte dynamique (mémoire en message traînant).
- Préserver le prefix-cache (déjà amorcé) ; sections ordonnées du plus stable au plus volatil.

---

## PHASE 7 — Points forts de third-party (`C:\Users\infinition\Desktop\third-party agent`)

> third-party a des briques de **fiabilité** et de **sécurité** que Claude Code n'expose pas.
> On porte celles qui manquent à LaRuche. (Déjà couvert par LaRuche, **ne pas refaire** :
> crons, watchers, kanban, extraction de skills, format OKF, dream, session_search,
> `aux_model`, auth Codex OAuth, thought_stream.)

| Technique third-party | Fichier réf (`third-party agent`) | Cible LaRuche |
|---|---|---|
| Pool de credentials (multi-clé/compte, rotation 429, cooldown) | `agent/credential_pool.py`, `credential_sources.py`, `credential_persistence.py` | nouveau `laruche-essaim/src/credential_pool.rs` |
| Classifieur d'erreurs (retry/ratelimit/relogin/fatal) | `agent/error_classifier.py`, `tool_result_classification.py` | nouveau `laruche-essaim/src/error_classifier.rs` |
| Garde anti-injection (inputs outils, web, cron) | `tools/threat_patterns.py` | nouveau `laruche-essaim/src/threat_patterns.rs` |
| Mixture of Agents (multi-modèle + synthèse) | `tools/mixture_of_agents_tool.py` | nouvelle abeille `laruche-essaim/src/abeilles/mixture.rs` |
| Compression de trajectoire | `trajectory_compressor.py` | crate `laruche-compaction` (complément du Lot 1.C) |
| Notifier proactif (push sur fin de run / watcher) | `services/notifier.*`, gateway | `laruche-node` + `laruche-channels` (Telegram déjà présent) |

### Lot 7.A — Pool de credentials & rotation (Owner : **CODEX**)
**Zone** : **nouveau** `laruche-essaim/src/credential_pool.rs` (+ `pub mod` dans lib.rs).
- `struct CredentialEntry { provider, api_key, base_url, cooldown_until: Option<i64> }`, `struct CredentialPool`.
- `prochain_disponible(provider) -> Option<&Entry>` (saute ceux en cooldown), `marquer_rate_limited(entry, reset_at)`, `marquer_invalide(entry)`.
- Persistance JSON dans `~/.laruche/credentials.json` (même style que `codex_auth`). Pur + testable.
- S'intègre avec `codex_auth` (le provider `codex` peut avoir plusieurs comptes). Claude branche dans `providers.rs`/`brain.rs`.

### Lot 7.B — Classifieur d'erreurs (Owner : **CODEX**)
**Zone** : **nouveau** `laruche-essaim/src/error_classifier.rs`.
- `enum ErrorClass { Retryable, RateLimited { reset_at: Option<i64> }, ReloginRequired, Fatal }`.
- `pub fn classifier(status: u16, body: &str) -> ErrorClass` (429→RateLimited avec Retry-After ; 401/403/invalid_grant→Relogin ; 5xx/timeout→Retryable ; reste→Fatal).
- Réutilisé par 7.A et par la récupération providers (Lot 2.A). Tests sur corps d'erreur OpenAI/Anthropic réels.

### Lot 7.C — Garde anti-injection / threat patterns (Owner : **CODEX**)
**Zone** : **nouveau** `laruche-essaim/src/threat_patterns.rs`.
- `pub fn detecter_injection(texte: &str) -> Vec<&'static str>` : patterns d'injection (ex. « ignore previous instructions », exfil de secrets, `curl … | sh`, etc.).
- Helpers `est_suspect_cron(prompt)`, `est_suspect_contenu_web(texte)`. Pur + tests.
- Claude branche : avant exécution d'un outil sur input modèle, sur le contenu web fetché, et à la création de cron/watcher.

### Lot 7.D — Mixture of Agents (Owner : **CODEX**)
**Zone** : **nouvelle** abeille `laruche-essaim/src/abeilles/mixture.rs` + son enregistrement dans `abeilles/mod.rs` (Codex propriétaire des ajouts d'abeilles).
- Abeille `mixture_of_agents` : interroge N modèles/providers (liste dans args) en parallèle via `provider_chat_stream`, puis synthétise avec l'`aux_model`. Niveau danger : Safe. Schéma + description claire.
- Tests : agrégation déterministe avec providers mockés.

### Lot 7.E — Compression de trajectoire (Owner : **CODEX**)
**Zone** : crate `laruche-compaction` (complément du Lot 1.C).
- `pub fn compresser_trajectoire(messages: Vec<Value>, aux_resumer: impl Fn(&str)->String) -> Vec<Value>` :
  remplace les longues séquences action→observation par un résumé d'étape (le résumé concret est injecté par Claude via l'aux model). Pur + tests (resumer mocké).

### Lot 7.F — Notifier proactif (Owner : **GEMINI**)
**Zones** : `laruche-node/src/main.rs` (wiring events→notif), `laruche-dashboard/src/templates/spa.html`, lecture seule de `laruche-channels` (ne pas modifier le crate).
- Sur `EventKind::AgentCompleted` / `WatcherFired` / `cron` → si activé, pousser un message via le canal Telegram déjà branché (réutiliser `api_start_channel`/handles existants).
- Endpoint `GET/POST /api/config/notify` ({on_agent_done, on_watcher, on_cron}) persisté. Toggle dans Settings > Channels.

### Lot 7.G — UI multi-compte (Owner : **GEMINI**, après 7.A)
**Zones** : `laruche-node/src/main.rs` (endpoints), `spa.html`.
- Endpoints `GET/POST/DELETE /api/credentials` (auth requise) lisant/écrivant via `credential_pool` (7.A).
- Settings > Providers : sous chaque provider, liste des clés/comptes + état cooldown.

---

## Vagues d'exécution (qui fait quoi, quand)

| Vague | CODEX | GEMINI | CLAUDE |
|---|---|---|---|
| **1** | 1.A, 1.B, 1.C | 1.D | 1.E (après 1.A–C) |
| **2** | 2.B, **7.B** | 2.C, **7.F** | 2.A, 2.D |
| **3** | **7.A**, **7.C** | 3.A, 3.B, **7.G** (après 7.A) | 3.C, branche 7.A/7.B/7.C |
| **4** | 4.A, **7.D** | 4.C | 4.B |
| **5** | 5.A, **7.E** | 5.B | merge |
| **6** | — | — | 6.A |

> Démarrer par la **Vague 1** (prompts prêts ci-dessous). Codex et Gemini ne partagent aucun fichier.
> Les lots third-party (7.x) ont des zones **nouvelles/disjointes** — aucun chevauchement avec les lots Claude Code.

---

## PHASE 8 — Manques résiduels (issus de l'[audit comparatif](./AUDIT_COMPARATIF.md))

> Fonctionnalités présentes chez les concurrents et encore absentes de LaRuche.

### Lot 8.A — LSP Tool (Owner : **CODEX**)
**Réf** : Claude Code `tools/LSPTool/*`. **Zone** : nouvelle abeille `laruche-essaim/src/abeilles/lsp.rs`.
- Lance/parle à un serveur LSP (rust-analyzer, tsserver…) : hover, go-to-def, find-refs, symboles.
- Navigation sémantique des grandes bases de code (sinon l'agent lit à l'aveugle). Tests sur un projet jouet.

### Lot 8.B — Isolation Git Worktree (Owner : **CODEX**)
**Réf** : Claude Code `tools/EnterWorktreeTool`, `ExitWorktreeTool`. **Zone** : `laruche-essaim/src/abeilles/worktree.rs`.
- `git worktree add` éphémère pour tester du code destructif sans toucher la branche active ; cleanup auto.

### Lot 8.C — Outils média natifs (Owner : **GEMINI** via plugins, ou CODEX via abeille)
**Réf** : third-party `tools/image_generation_tool.py`. **Zone** : plugin JSON `laruche/plugins/*.json` (image/vidéo via API externe) ou abeille dédiée.

### Lot 8.D — Finalisation câblage retry (Owner : **CLAUDE**) — voir « Reste à faire » ci-dessous.

---

## PHASE 9 — Autonomie vivante (ce qui rend third-party « vivant »)

> Objectif : crons/skills/mémoire qui tournent et se digèrent **tout seuls**, avec
> routing **canal** et **modèle** granulaire par tâche. Réf third-party :
> `agent/background_review.py`, `agent/curator.py`, `tools/cronjob_tools.py` (`_resolve_model_override`), `third-party_cli/goals.py`.

### Lot 9.A — Canal + modèle par tâche (cron/watcher) (Owner : **CODEX** pour le modèle de données, **GEMINI** pour l'API/UI)
**Zones CODEX** : `laruche-essaim/src/cron.rs` (+ `laruche-watchers` si besoin).
- Ajouter à `ScheduledTask` : `channel: Option<String>` (ex. "telegram"), `provider: Option<String>`, `model: Option<String>`. Sérialisation rétro-compatible (`#[serde(default)]`). Tests.
**Zones GEMINI** : `laruche-node/src/main.rs` (API cron), `spa.html`.
- `/api/cron` POST accepte `channel`/`provider`/`model` ; UI Settings > Cron : champs canal + modèle par tâche.

### Lot 9.B — Daemon cron : routing feedback + modèle par tâche (Owner : **CLAUDE**)
**Zone** : `laruche-node/src/main.rs` (le bloc daemon cron ~L5899).
- Au lieu de jeter `_rx` : router le résultat vers le `channel` de la tâche (réutiliser l'envoi Telegram existant), sinon activity_log.
- Appliquer `task.provider`/`task.model` sur `cron_config` (au lieu de forcer `get_llm_default`).

### Lot 9.C — Background review forké (auto-skill/mémoire après chaque tour) (Owner : **CLAUDE** + **CODEX**)
**Réf** : `agent/background_review.py`. **Zone** : `laruche-essaim/src/brain.rs` (+ helper dans un nouveau `background_review.rs`).
- Après un tour, `tokio::spawn` un mini-agent **à outils restreints** (mémoire+skill seulement) avec un prompt « dois-je sauver un skill/souvenir ? » → écrit dans les stores. Non bloquant, n'altère pas le cache du chat principal. Remplace/complète l'extraction inline `tache_complexe_reussie`.

### Lot 9.D — Curator à l'inactivité (Owner : **CODEX**)
**Réf** : `agent/curator.py`. **Zone** : `laruche-memoire` (+ déclencheur idle dans `laruche-node`).
- Quand l'agent est idle depuis > N h, lancer un `dream()`/maintenance forké (archive/consolide/pin skills, dédoublonne mémoire). État persisté (`last_run_at`). Jamais de suppression (archive recouvrable).

### Lot 9.E — Goals / chantiers planifiés multi-tours (Owner : **CLAUDE**)
**Réf** : `third-party_cli/goals.py`. **Zone** : `brain.rs` + API node.
- Boucle dirigée par objectif (max_turns, critère de complétion) pour mener une recherche/un chantier sur plusieurs tours sans relance manuelle.

---

## Tableau de suivi (à mettre à jour)

| Lot | Owner | Statut | Fichiers | Tests |
|---|---|---|---|---|
| 0 (fondations) | Claude | ✅ fait | brain.rs, shell.rs, providers.rs, node, spa | 15 essaim |
| 1.A tool_budget | Codex | ✅ fait | abeille.rs, tool_budget.rs | 3 nouveaux / 59 essaim verts |
| 1.B budget | Codex | ✅ fait | budget.rs | 3 nouveaux / 59 essaim verts |
| 1.C compaction | Codex | ✅ fait | laruche-compaction | 3 nouveaux / 7 compaction verts |
| 1.D context UI | Gemini | ✅ fait | main.rs, spa.html | — |
| 1.E intégration | Claude | ✅ fait | brain.rs | 2 nouveaux / 85 essaim verts |
| 2.A recovery/fallback | Claude | ✅ fait | providers.rs, streaming.rs, brain.rs | 1 nouveau / 85 essaim verts |
| 2.B tool_summary | Codex | ✅ fait | tool_summary.rs | 3 nouveaux / 85 essaim verts |
| 2.C réglages robustesse | Gemini | ✅ fait | spa.html, main.rs | — |
| 2.D résumé outils (intégration) | Claude | ✅ fait | brain.rs | aux model + fallback extractif / 89 essaim verts |
| 3.A steer serveur | Gemini | ✅ fait | main.rs | — |
| 3.B UI steering | Gemini | ✅ fait | spa.html | — |
| 3.C steer boucle | Claude | ✅ fait | brain.rs, main.rs | canal non bloquant injecté entre deux itérations / 89 essaim verts |
| 4.A sous-agent (subagent.rs) | Codex | ✅ fait | laruche-essaim | 1 struct + unit tests |
| 4.B API node | Claude | ✅ fait | main.rs | `POST /api/agents/spawn` |
| 4.C UI sous-agents | Gemini | ✅ fait | spa.html | Afficher l'arbre des sous-agents |
| 5.A dream | Codex | ✅ fait | laruche-memoire, main.rs | Dream inactif implémenté |
| 5.B UI dream | Gemini | ✅ fait | spa.html | — |
| 6.A prompt 3 tiers | Claude | ✅ fait | prompt.rs | 1 nouveau / 89 essaim verts |
| 7.A credential pool | Codex | ✅ fait | credential_pool.rs | 4 nouveaux / 79 essaim verts |
| 7.B error classifier | Codex | ✅ fait | error_classifier.rs | 12 nouveaux / 79 essaim verts |
| 7.C threat patterns | Codex | ✅ fait | threat_patterns.rs | 4 nouveaux / 79 essaim verts |
| 7.D mixture of agents | Codex | ✅ fait | abeilles/mixture.rs, abeilles/mod.rs | 2 nouveaux / 89 essaim verts |
| 7.E trajectory compress | Codex | ✅ fait | laruche-compaction | 1 nouveau / 8 compaction verts |
| 7.F notifier proactif | Gemini | ✅ fait | main.rs, spa.html | — |
| 7.G UI multi-compte | Gemini | ✅ fait | main.rs, spa.html | — |
| 9.A canal + modèle par tâche (data) | Codex | ✅ fait | cron.rs | 2 nouveaux / compatibilité JSON legacy |
| 9.B routing daemon cron | Claude | ✅ fait | main.rs | Route channels / fallback default LLM config |
| 9.A API/UI | Gemini | ✅ fait | main.rs, spa.html | — |
| 9.B daemon cron | Claude | ✅ fait | main.rs | — |
| 9.C background review | Codex | ✅ fait | brain.rs, background_review.rs | 2 nouveaux / non bloquant, accès mémoire+skill uniquement |
| 9.D curator idle | Codex | ✅ fait | laruche-memoire | 2 nouveaux / état persistant + dream sans suppression |

## Reste à faire (post-Vague 1, vérifié au build complet ✅ exit 0)

- **7.C enforcement** : `threat_patterns` est désormais **branché dans le dispatch** (`garde_injection` dans le chemin séquentiel de `brain.rs`) — bloque shell/write/exec sur patterns d'injection/exfiltration, testé. ✅
- **Câblage `error_classifier` + `credential_pool` dans la boucle retry providers** (Claude) : modules prêts et testés, câblage effectué dans `brain.rs` pour déclencher une rotation de clé transparente lors d'erreurs HTTP 429. ✅
- **4.B** : `POST /api/agents/spawn` (exposer le sous-agent côté node) ✅
- **5.A** : dream auto à l'inactivité + background_review (auto-skill) ✅
- **Manques vs Claude Code** : LSP tool ✅, isolation Git worktree ✅.
- **Manques vs third-party** : outils média natifs (image/vidéo) via plugins/MCP ✅.


<a id="archive-C"></a>

---

# ARCHIVE C — AUDIT_COMPARATIF.md (LaRuche vs third-party vs Claude Code)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# Audit Comparatif Global : LaRuche v2 vs third-party agent vs Claude Code

> Document de référence (audit réalisé par Codex/Gemini, vérifié et complété par Claude).
> Mis en regard de la [ROADMAP_CLAUDE_CODE_PORT.md](./ROADMAP_CLAUDE_CODE_PORT.md).
> Sources : Claude Code TS (`C:\Users\infinition\Desktop\aa`), third-party (`C:\Users\infinition\Desktop\third-party agent`).

Ce document constitue un audit architectural exhaustif et fusionné, comparant **LaRuche v2** (Rust)
face à ses deux principales inspirations/concurrents : **third-party agent** (Python) et **Claude Code** (TypeScript).

---

## 1. Topologie, Réseau et Stack Technologique

| Caractéristique | Claude Code (TS) | third-party agent (Python) | LaRuche v2 (Rust) | Bilan LaRuche |
|---|---|---|---|---|
| **Exécutable & Déploiement** | CLI Node.js (`npm install`). | Multiples scripts, dépendances (uv, Node, ffmpeg). | **Mono-binaire natif** (`cargo install`). | 🏆 **Avantage absolu**. Zéro dépendance, empreinte mémoire minimale. |
| **Topologie & Réseau** | Local uniquement, mono-session. | Centralisé (1 VPS / Container). | **Mesh Local-first (Miel, mDNS).** | 🚀 **Exclusivité**. Swarm + découverte P2P locale. |
| **Interface / Accessibilité** | TUI uniquement. | CLI, Telegram, Discord, Slack. | **SPA (Web), Ratatui (TUI), Telegram natif, VS Code.** | 🌟 **Supérieur**. Frontend web unifié + extension IDE. |

## 2. Cœur Agentique, Boucle ReAct & Optimisations

| Mécanisme Core | Source | Statut LaRuche | Implémentation |
|---|---|---|---|
| **Agent Loop (ReAct)** | Les deux | ✅ Parité | `brain.rs` |
| **Token Budget & Limits** | Claude Code | ✅ Parité | `budget.rs` |
| **Tool Result Truncation** | Claude Code | ✅ Parité | `tool_budget.rs` (branché dispatch) |
| **Tool Summary (LLM)** | Claude Code | ✅ Parité | `tool_summary.rs` (modèle aux) |
| **Orchestration partition safe/unsafe** | Claude Code | ✅ Parité | `partition_tool_calls` / `is_concurrency_safe` |
| **Model Fallback** | Claude Code / third-party | ✅ Amélioré | bascule Ollama→cloud sur 429/500 (`providers.rs`/`brain.rs`) |
| **maxOutputTokens recovery** | Claude Code | ✅ Parité | `sortie_tronquee` + continuation |
| **System Prompt 3 Tiers** | third-party | ✅ fait | `prompt.rs` (cache-friendly) |
| **Trajectory Compression** | third-party | ✅ Amélioré | `laruche-compaction` |
| **Steering (interruption mid-turn)** | Claude Code | ✅ fait | canal non bloquant `brain.rs`/`main.rs` |
| **Tâches de fond (Client Aux)** | third-party | ✅ Parité | `tokio::spawn` + `aux_model` |
| **Error Classifier** | third-party | ✅ branché failover | `error_classifier.rs` → `classer_erreur_provider` |
| **Credential Pool (rotation)** | third-party | 🔄 module+UI prêts ; câblage retry à finir | `credential_pool.rs`, `/api/credentials` |
| **Threat Patterns (anti-injection)** | third-party | ✅ branché dispatch | `threat_patterns.rs` → `garde_injection` |

## 3. Outils Natifs (Abeilles) : Mapping Tripartite

### A. Fichiers et Système
| Outil | Claude Code | third-party | LaRuche | Remarques |
|---|:---:|:---:|:---:|---|
| Read / Write / Edit | ✅ | ✅ | ✅ | `fichiers.rs` |
| Grep / Glob Search | ✅ | ✅ | ✅ | `recherche_fichiers.rs` |
| Read Extract (sélectif PDF/MD) | ❌ | ✅ | ✅ | `read_extract.rs` |
| File Watcher (événementiel) | ❌ | ❌ | 🚀 Oui | Abeille `file_watch` — **exclusif LaRuche** |

### B. Code, Shell et Navigation
| Outil | Claude Code | third-party | LaRuche | Remarques |
|---|:---:|:---:|:---:|---|
| Shell / Bash / PowerShell | ✅ | ✅ | ✅ | `shell.rs` (PowerShell sous Windows) |
| Navigateur Headless | ❌ | ✅ | ✅ | `browser.rs` |
| REPL / exécution code isolé | ✅ | ✅ | ✅ | `execute_code.rs` |
| **LSP (Hover, GoToDef)** | ✅ | ❌ | ❌ | **MANQUE LaRuche** |
| Outils Git natifs | ❌ (via Bash) | ✅ | ✅ | hérités d'third-party |
| **Isolement Git Worktree** | ✅ | ❌ | ❌ | **MANQUE LaRuche** |

### C. Recherche Web et Médias
| Outil | Claude Code | third-party | LaRuche | Remarques |
|---|:---:|:---:|:---:|---|
| Web Search & Fetch | ✅ | ✅ | ✅ | `web_recherche`, `web_fetch` |
| Deep Web Search | ❌ | ✅ | ✅ | `web_deep.rs` |
| **Génération Image/Vidéo** | ❌ | ✅ | ✅ | **FAIT** (via plugins/MCP) |
| **Computer Use (contrôle PC)** | ❌ | ❌ | ✅ | **FAIT** (MCP externe) |

### D. Tâches, Orchestration et Protocoles
| Outil | Claude Code | third-party | LaRuche | Remarques |
|---|:---:|:---:|:---:|---|
| Délégation (sous-agents) | ✅ | ✅ | ✅ | `delegation.rs` + `subagent.rs` |
| Mixture of Agents (MoA) | ❌ | ✅ | ✅ | `mixture.rs` (enregistrée) |
| Serveur MCP (exposer) | ❌ | ❌ | ✅ | `laruche mcp` — **exclusif** |
| Client MCP (consommer) | ✅ | ✅ | ✅ | `plugins.rs` |

## 4. Système de Compétences (Skills) — Saut Paradigmatique LaRuche

| Critère | Claude Code | third-party | LaRuche (OKF) |
|---|---|---|---|
| Nature | Fichiers TS bundlés | `SKILL.md` lus depuis `/skills/` | **Nœuds mémoire vectorielle SQLite (format OKF)** |
| Ingestion | `SkillTool` rigide | injection en masse au prompt | souvenir sémantique interrogeable (`memoire.rs`) |
| Auto-création | ❌ | `background_review` (disque) | API `propose_skill`/`write_skill` → hippocampe (🔄 auto à finaliser) |

## 5. Extensibilité — Plugins
- **Claude Code** : peu extensible sans modifier le TS.
- **third-party** : script Python + Pydantic.
- **LaRuche** : JSON dynamique (`laruche/plugins/`) — déposer un fichier (commande shell + schéma JSON). Ultra-léger.

## 6. Robustesse et Sécurité (héritage third-party)
1. **Error Classifier** (`error_classifier.rs`) — 429/relogin/retryable/fatal, branché au failover.
2. **Credential Pool** (`credential_pool.rs`) — rotation multi-comptes sur cooldown 429 (câblage retry à finaliser).
3. **Threat Patterns** (`threat_patterns.rs`) — anti-injection, branché avant `ShellExec`/write/exec.
4. **Guardrails anti-boucle** — par-signature (3/6) + par-nom (10/15).

---

## Mapping AUDIT → ROADMAP (manques → lots)

| Manque identifié | Inspiration | Lot roadmap | Statut |
|---|---|---|---|
| Câblage error_classifier + credential_pool au retry providers | third-party | 7.A/7.B (finalisation) | 🔄 en cours (Claude) |
| LSP Tool (navigation sémantique du code) | Claude Code | **NOUVEAU Lot 8.A** | ⬜ à planifier |
| Isolation Git Worktree (sandbox code destructif) | Claude Code | **NOUVEAU Lot 8.B** | ⬜ à planifier |
| Dream auto à l'inactivité + background_review (auto-skill) | third-party | 5.A / Phase 4 | ⬜ à finaliser |
| Outils Média natifs (image/vidéo) | third-party | **NOUVEAU Lot 8.C** (plugins/MCP) | ⬜ à planifier |
| `POST /api/agents/spawn` (exposer sous-agent) | — | 4.B | ⬜ |

## Conclusion
**L'architecture de LaRuche v2 est conceptuellement supérieure** : robustesse Rust, mesh Miel,
Watchers événementiels, mémoire+Skills unifiées en OKF vectoriel, mono-binaire.
Les briques agentiques de Claude Code et la robustesse de third-party sont **portées et majoritairement
branchées**. Reste à finaliser : le câblage retry credential_pool, et les 3 nouveaux lots (LSP,
Worktree, Média) ajoutés à la roadmap (Phase 8).


<a id="archive-D"></a>

---

# ARCHIVE D — SKILLS_TOOLS_ARCHITECTURE.md (skills vs tools, Phases 10/11)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# Architecture Skills vs Tools — LaRuche (aligné third-party / Claude Skills)

> Objectif : distinction nette **Skill** (connaissance) vs **Tool** (exécutable), comme
> third-party/Claude Code, avec association **skill↔cron/job/tool** (cases à cocher) et
> auto-création par le LLM. Câblage bout-en-bout, fluide.

## Modèle de référence (third-party, vérifié dans son code)

- **Skill** = dossier avec `SKILL.md` (frontmatter YAML + corps Markdown + scripts/références optionnels).
  Frontmatter : `name`, `description`, `version`, `author`, `license`, `platforms`,
  `metadata.third-party.{tags, related_skills}`, `prerequisites.commands`.
  **Divulgation progressive** (Claude Skills) : `skills_list` (tier 1 = nom+desc, l'agent SAIT qu'ils
  existent) → `skill_view` (tier 2 = charge le SKILL.md complet à la demande).
- **Tool** = exécutable appelé par function-call.
- **Cron à skills** : les SKILL.md sélectionnés sont **assemblés dans le prompt** au moment du run
  (third-party `_scan_cron_skill_assembled`). UI : cases à cocher de skills par job + « deliver to » (canal).

## État LaRuche (juin 2026)

| Brique | État | Où |
|---|---|---|
| Modèle Skill OKF | ✅ | `laruche-skills::Skill` (frontmatter, parse/to_markdown) |
| Skills en mémoire | ✅ | `tools.skills.*` (mémoire cognitive) |
| `skill_list` / `skill_view` (divulgation progressive) | ✅ | `abeilles/mod.rs` |
| Tools built-in | ✅ | abeilles Rust |
| Tools custom (LLM) | ✅ | plugins JSON + `plugins/scripts/` + `reload_plugins` |
| **Association skill↔cron/job** | ❌ | `ScheduledTask` sans champ `skills` |
| **Injection skills au run cron** | ❌ | daemon cron n'assemble pas |
| **Page Skills (manager/edit/cat.)** | ❌ | UI absente |
| **Checkboxes skills sur cron** | ❌ | UI absente |

## Décisions (réponses aux questions)

### OKF + laruche-memoire (au cœur)
Un **Skill EST un document OKF** stocké dans `laruche-memoire` sous `tools.skills.<slug>`
(frontmatter `type: skill` + corps Markdown), **indexé vectoriellement**. Avantage sur third-party
(fichiers plats) : `skill_list`/`skill_view` font de la **découverte sémantique**, pas un simple
scan de dossier. Voie d'écriture : `memory_write(node_id="tools.skills.<slug>")` ou
`background_review`. Toute la persistance reste OKF/Markdown → lisible humain + pérenne.

### Tools Rust (abeilles) vs Tools custom (LLM) — **on distingue, mais on unifie**
**Recommandation** : une **seule** registry et **une seule interface d'appel** (le LLM voit tous les
tools de façon identique → fluidité), MAIS un marqueur d'origine :
- `ToolOrigin::Builtin` — abeille **Rust native**, **immuable** : le LLM ne peut PAS l'éditer/supprimer.
- `ToolOrigin::Custom` — plugin JSON + script (`plugins/scripts/`), **forgé/éditable/supprimable** par
  le LLM via `reload_plugins`.

Pourquoi : (1) **sûreté** — le LLM ne peut pas écraser un outil Rust ; (2) **UI** — bouton « voir
source / éditer » seulement pour les custom, badge « Rust natif » sur les builtin ; (3) **sémantique
d'autonomie** — « forger un outil » = toujours custom. Implémentation : `fn origin(&self) ->
ToolOrigin { ToolOrigin::Builtin }` (défaut) sur le trait `Abeille` ; le `PluginAbeille` renvoie
`Custom`. La registry/UI lisent `origin()`.

## Cible — câblage complet

### Données
- `ScheduledTask.skills: Vec<String>` (noms de skills) — `#[serde(default)]` rétro-compatible.
- (Optionnel) `frontmatter` Skill aligné third-party : ajouter `tags`, `related_skills`, `prerequisites`
  dans `SkillMeta` (déjà : name, description, allowed-tools).

### Logique (pure, réutilisable)
- `assembler_prompt_avec_skills(prompt, skill_names, mem) -> String` : pour chaque skill,
  charge son OKF depuis `tools.skills.<name>` et préfixe le prompt par
  `## Skill: <name>\n<corps>\n---`. C'est ce que le daemon cron ET le chat utiliseront.

### Runtime
- **Daemon cron** : avant `boucle_react_memoire`, assembler les skills de la tâche dans le prompt.
- **Chat** (optionnel) : permettre d'épingler des skills à une session.

### UI (dashboard)
- **Page Skills** (façon third-party) : liste (nom+desc+catégorie), filtres/catégories, recherche,
  bouton « New Skill », **édition du SKILL.md** (frontmatter validé au save), enable/disable.
  Endpoints : `GET /api/skills`, `GET /api/skills/:name`, `POST /api/skills` (upsert), `DELETE`.
- **Édition cron** : section « Skills » avec **cases à cocher** (liste depuis `/api/skills`) +
  « Deliver to » (canal, déjà fait en 9.A). Même chose pour watchers/jobs.

### Auto-création par le LLM (déjà amorcé)
- `background_review.rs` propose/écrit des skills sous `tools.skills.*` quand une action récurrente
  est détectée. `memory_write` avec `node_id=tools.skills.<slug>` reste la voie d'écriture.
- L'agent peut aussi **forger un Tool** (plugin JSON + script) via `reload_plugins`. Skill ≠ Tool :
  le skill documente *comment*, le tool *fait*.

## Lots (zones disjointes)

### Lot 10.A — Données + assemblage (Owner : **CODEX**)
**Zones** : `laruche-essaim/src/cron.rs` (champ `skills`), `laruche-skills/src/lib.rs`
(frontmatter étendu + `pub fn assembler_prompt_avec_skills` PURE, prenant un trait/clôture de
lecture skill pour rester sans dépendance à `laruche-memoire`). Tests.

### Lot 10.B — Injection au run cron (Owner : **CLAUDE**)
**Zone** : daemon cron — assembler `task.skills` dans le prompt avant `boucle_react_memoire`.
(Coordonné avec Gemini sur `main.rs` : Claude touche UNIQUEMENT le bloc daemon ~L5899.)

### Lot 10.C — API + UI Skills + checkboxes cron (Owner : **GEMINI**)
**Zones** : `laruche-node/src/main.rs` (endpoints `/api/skills*`), `spa.html` (page Skills + édition
SKILL.md + cases à cocher skills dans l'édition cron, lues depuis `/api/skills`).

## PHASE 11 — Planification (kanban orchestrateur + blueprints + calendrier, façon third-party)

> Ce que les users d'third-party adorent : un **board kanban orchestré** (l'agent tire les tâches tout
> seul) + un **catalogue de blueprints** (automatisations prêtes à activer en 1 clic) + le calendrier.

### Existant LaRuche (déjà solide)
- `laruche-kanban` : `TaskStatus` (Todo/Ready/Blocked/Done/Archived), **dépendances + auto-déblocage**
  des enfants quand un parent passe Done (`change_status`), `complete(result)`. Abeilles `kanban_create`/`kanban_list`.
- Calendrier : `calendar_add`/`calendar_list`. Cron : `cron_*` (+ canal/modèle par tâche, 9.A).

### Manque (à porter d'third-party)
1. **Orchestrateur kanban** : l'agent **tire la prochaine tâche `Ready`**, l'exécute, la `complete`
   (ce qui débloque les dépendantes) — en boucle, jusqu'à board vide. Réf : `tools/kanban_tools.py`
   (mode orchestrator/worker, claim de tâche).
2. **Blueprints** : catalogue d'automatisations paramétrées (titre + `schedule_template` + `prompt_template`
   + slots) → 1 clic crée le cron correspondant. Réf : `cron/blueprint_catalog.py`, onglet « Blueprints ».

### Lot 11.A — Orchestrateur kanban (Owner : **CODEX**)
**Zones** : `laruche-kanban/src/lib.rs` (`pub fn next_ready(&self) -> Option<KanbanTask>` + claim/lock),
nouvelle abeille `laruche-essaim/src/abeilles/kanban_next.rs` (`kanban_next` + `kanban_complete`). Tests.

### Lot 11.B — Boucle orchestrateur (Owner : **CLAUDE**)
**Zone** : un mode dans `brain.rs`/daemon — drainer le board (next_ready → run → complete) jusqu'à vide,
borné. Réutilise les goals (Phase 3/9.E).

### Lot 11.C — Blueprints (Owner : **CODEX** data + **GEMINI** UI)
**Zones CODEX** : `laruche-essaim/src/blueprints.rs` (catalogue statique : structs Blueprint{title,
schedule_template, prompt_template, slots}, + `instancier(slots) -> ScheduledTask`). Tests.
**Zones GEMINI** : `spa.html` onglet « Blueprints » (liste + formulaire slots → POST /api/cron),
endpoint `GET /api/blueprints` dans `main.rs`.

### Lot 11.D — Tool origin (Owner : **CODEX**)
**Zones** : `laruche-essaim/src/abeille.rs` (`enum ToolOrigin {Builtin, Custom}` + `fn origin()` défaut
Builtin sur le trait), le `PluginAbeille` renvoie `Custom`. UI (Gemini) : badge + boutons conditionnels.

## Tableau de suivi
| Lot | Owner | Statut |
|---|---|---|
| 10.A champ skills (ScheduledTask) | Claude (Codex ne l'avait pas livré) | ✅ |
| 10.B injection skills au run cron | Claude | ✅ (assembler_prompt_skills + read_node OKF) |
| 10.C API + UI Skills + checkboxes | Gemini | ⬜ (à vérifier côté UI) |
| 11.A orchestrateur kanban | Codex | ✅ |
| 11.B boucle orchestrateur | Claude | ✅ (daemon kanban exécute les `Ready` 1 par tick) |
| 11.C blueprints (data+UI) | Codex + Gemini | ✅ |
| 11.D tool origin builtin/custom | Codex + Gemini | ✅ |


<a id="archive-E"></a>

---

# ARCHIVE E — laruche_tools_parity.md (parité outils, revue 2026-06-20)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# Parité des Outils — LaRuche vs third-party vs Claude Code

> Tableau de bord vivant. Source LaRuche : abeilles enregistrées (`abeilles/mod.rs`,
> `abeilles_local.rs`, `main.rs`). Sources concurrents : `third-party agent/tools/`,
> `aa/src/tools/`. Mettre à jour à chaque nouvel outil.
> Dernière revue : 2026-06-20.

Légende : ✅ présent · 🔶 partiel · ❌ absent · 🚀 exclusif

## Fichiers & système
| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Lecture fichier | ✅ | ✅ | ✅ | `file_read` (FileRead) |
| Écriture fichier | ✅ | ✅ | ✅ | `file_write` (FileWrite) |
| Édition (patch) | ✅ | ✅ | ✅ | `file_edit` (FileEdit) |
| Liste répertoire | ✅ | ✅ | ✅ | `file_list` (FileList) |
| Glob | ✅ | ✅ | ✅ | `file_search` (FileSearch) |
| Grep contenu | ✅ | ✅ | 🔶 | via `file_search` (à confirmer regex contenu) |
| Extraction sélective (PDF/MD) | ❌ | ✅ | ✅ | `read_extract` (ReadExtract) |
| Watcher fichier événementiel | ❌ | ❌ | 🚀 | `file_watch` (FileWatch) |
| Notebook (.ipynb) | ✅ | ❌ | ❌ | — (à porter ?) |

## Code, shell, navigation
| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Shell / PowerShell | ✅ | ✅ | ✅ | `shell_exec` (PowerShell sous Windows) |
| Exécution code isolée | ✅ | ✅ | ✅ | `execute_code` |
| Pipeline d'outils (1 tour) | 🔶 | 🔶 | 🚀 | `run_script` (RunScript) |
| REPL persistant | ✅ | ✅ | 🔶 | partiel via execute_code |
| LSP (hover/def/refs) | ✅ | ❌ | ✅ | `lsp` (AbeilleLsp) |
| Git status/diff/log/commit | 🔶 (Bash) | ✅ | ✅ | `git_*` (GitStatus/Diff/Log/Commit) |
| Isolation Git worktree | ✅ | ❌ | ✅ | `git_worktree_enter`/`exit` |
| Navigateur headless | ❌ | ✅ | ✅ | `browser_navigate`/`browser_screenshot` |

## Web & médias
| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Web search | ✅ | ✅ | ✅ | `web_search` (WebSearch) |
| Web fetch (page) | ✅ | ✅ | ✅ | `web_fetch` (WebFetch) |
| Deep search (multi-liens) | ❌ | ✅ | ✅ | `web_deep_search` (WebDeepSearch) |
| Scraping JS-rendered robuste | 🔶 | ✅ (Playwright) | 🔶 | **À AMÉLIORER** (voir §Web ci-dessous) |
| Recherche d'images | ❌ | ✅ | ✅ | `image_search` |
| Génération image/vidéo | ❌ | ✅ | ❌ | — (via plugin/MCP) |
| Computer use (contrôle PC) | ❌ | ✅ | ❌ | — (via MCP) |

## Tâches, orchestration, protocoles
| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Délégation sous-agent | ✅ | ✅ | ✅ | `delegate` (Delegate) + `subagent.rs` |
| Mixture of Agents | ❌ | ✅ | ✅ | `mixture_of_agents` |
| Todo | ✅ | ✅ | ✅ | `todo` |
| Cron (créer/lister/suppr) | ✅ | ✅ | ✅ | `cron_create`/`cron_list`/`cron_delete` |
| Watchers événementiels | ❌ | 🔶 | 🚀 | `watcher_create`/`list`/`delete` |
| Kanban | ❌ | ✅ | ✅ | `kanban_create`/`kanban_list` |
| Clarification utilisateur | ✅ | ✅ | ✅ | `clarify` |
| Recherche dans sessions | ✅ | ✅ | ✅ | `session_search` |
| Client MCP (resources) | ✅ | ✅ | ✅ | `mcp_list_resources`/`mcp_read_resource` |
| Serveur MCP (exposer) | ❌ | ❌ | 🚀 | `laruche mcp` |
| **Hot-reload de plugins** | ❌ | ❌ | 🚀 | `reload_plugins` (ReloadPluginsTool) |

## Compétences & mémoire
| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Lister/voir skills | ✅ | ✅ | ✅ | `skill_list`/`skill_view` |
| Auto-création de skills | ❌ | ✅ (background_review) | 🔶 | inline + `background_review.rs` (à finaliser) |
| Mémoire CRUD vectorielle (OKF) | 🔶 | 🔶 | 🚀 | `memory_*` (search/write/update/move/delete/review/…) |
| Dream / consolidation | ❌ | ✅ | 🔶 | `dream()` (manuel ; curator idle à finaliser) |

## Divers
| Capacité | Claude Code | third-party | LaRuche | Outil LaRuche |
|---|:--:|:--:|:--:|---|
| Calendrier | ❌ | ✅ | ✅ | `calendar_add`/`calendar_list` |
| Maths | ❌ | ❌ | ✅ | `math_eval` |
| Infos système | ✅ | ✅ | ✅ | `system_info` |
| Présentation média (UI) | ❌ | 🔶 | ✅ | `media_present` |

---

## Manques prioritaires (à forger via `plugins/scripts/` + `plugins/`)

1. **Scraping JS-rendered robuste** (§ ci-dessous) — priorité #1.
2. **Génération image/vidéo** — plugin appelant une API (ou MCP).
3. **Grep contenu** dédié (regex sur contenu, pas juste noms) si `file_search` ne le couvre pas.
4. **Notebook execution** (.ipynb) — utilité moindre, basse priorité.
5. **Computer use** — via serveur MCP externe.

## §Web — diagnostic & plan d'amélioration

**Outils actuels** : `web_search` (scrape DuckDuckGo HTML), `web_fetch` (HTML brut),
`web_deep_search` (search + fetch top N). **Limite connue** : pages **JS-rendered** (SPA,
meteofrance, etc.) → `web_fetch` ne récupère que la nav, pas le contenu.

**Plan** (forge en plugin, autonomie `reload_plugins`) :
- `plugins/scripts/web_render.py` (ou binaire) : rendu headless (Playwright bundle Node v24
  déjà dispo dans l'env, sans npm) → renvoie le DOM rendu + texte extrait (readability).
- `plugins/web_render.json` : schéma `{url, wait_selector?, screenshot?}`.
- Améliorer le ranking `web_deep_search` (dédup domaines, readability, head/tail) — déjà amorcé.
- Fallback : si fetch statique renvoie peu de contenu (< seuil), basculer auto sur `web_render`.


<a id="archive-F"></a>

---

# ARCHIVE F — third-party_HACKS.md (catalogue d'astuces third-party)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# Audit third-party — astuces & hacks réutilisables pour LaRuche

Mine du repo `third-party agent` (third-party). Techniques éprouvées à **réutiliser sans réinventer**,
classées par valeur pour LaRuche. Réfs = fichiers dans `third-party agent/agent/`.

---

## 🔴 TOP — règlent directement la latence qu'on a constatée

### 1. System prompt en 3 tiers + invariant « prefix-cache chaud » — `system_prompt.py`
third-party construit le system prompt **une fois par session** et le **réutilise verbatim** ; seule la
compression de contexte déclenche un rebuild. But explicite : *garder le cache de préfixe amont chaud*.
Trois tiers joints par `\n\n` :
- **stable** : identité, guidance outils, schémas d'outils, guidance par modèle, skills. (préfixe figé)
- **context** : `system_message` appelant + fichiers de contexte (AGENTS.md/.cursorrules) sous le CWD.
- **volatile** : snapshot mémoire, USER.md, bloc mémoire externe, ligne timestamp/session/modèle.

**Pour LaRuche (notre fix #T13)** : aujourd'hui on injecte la mémoire DANS le system prompt → préfixe
change chaque tour → réindexation complète (~9 s de `prompt eval`). **Mettre les schémas d'outils +
identité dans un préfixe STABLE, et la mémoire/timestamp en tier volatile trailing.** C'est LE gain.

### 2. Client auxiliaire séparé pour les tâches de fond — `auxiliary_client.py`
Toute tâche de fond (curation, background-review, extraction) tourne sur un **client/modèle auxiliaire
distinct** qui **ne touche jamais le cache du prompt de la session principale**.
**Pour LaRuche** : notre auto-curation (déjà passée en `tokio::spawn`) devrait utiliser un **modèle/clé
aux** (config dédiée) pour ne pas évincer le KV-cache du chat principal. Idéalement un petit modèle rapide.

### 3. Le fork background-review réutilise le prompt caché verbatim — `background_review.py`
Le fork hérite provider/model/base_url/credentials + **prompt système caché** → il tape le **même
prefix cache** et la même auth. Whitelist d'outils = mémoire/skills only.

---

## 🟠 HAUT — robustesse de la boucle d'agent

### 4. Garde-fous de boucle d'outils — `tool_guardrails.py`
Contrôleur pur, sans effet de bord, qui détecte par tour les appels d'outils répétés/en échec et renvoie
une **décision** (warning, résultat synthétique, ou **halt contrôlé**). Seuils par défaut :
- `exact_failure_warn_after = 2`
- `same_tool_failure_warn_after = 3`, `same_tool_failure_halt_after = 8`
- `idempotent_no_progress_warn_after = 2`

**Pour LaRuche** : dans `brain.rs`, traquer (nom+args hashés) les appels répétés ; au seuil → injecter une
guidance (« tu répètes le même appel qui échoue, change d'approche ») ou stopper proprement, au lieu de
brûler `max_iterations`. Cheap, gros gain de fiabilité + tokens.

### 5. Classifieur d'erreurs → retry / fallback / abort — `error_classifier.py`
Classe les erreurs provider (rate-limit, context-length, auth, transient…) pour décider retry vs
fallback de modèle vs abort. `fallback` apparaît dans 137 fichiers (failover très travaillé).
**Pour LaRuche** : enrichir le failover (aujourd'hui basique) avec une classification : context-length →
compresser puis retry ; rate-limit → backoff ; auth → message clair ; transient → retry court.

### 6. Compression de contexte « préflight » — `context_compressor.py`, `conversation_compression.py`
Compression de l'historique **avant** que ça déborde (dans le prologue du tour), et **seule la compression
rebuild le system prompt** (cohérent avec #1). 
**Pour LaRuche** : compacter avant l'appel (résumé des vieux tours) plutôt que de tronquer brutalement.

---

## 🟡 MOYEN — UX & puissance

### 7. Interrupt-and-redirect / canal « steer » — (`steer` 17 fichiers, `interrupt` 46)
L'utilisateur peut **interrompre une génération en cours et rediriger** sans repartir de zéro ; un
`STEER_CHANNEL_NOTE` dans le prompt explique le canal au modèle.
**Pour LaRuche** : permettre d'injecter un message pendant qu'un tour tourne (le WS le permet déjà côté transport).

### 8. Scripts qui appellent les outils via RPC — (README third-party)
« Write Python scripts that call tools via RPC, collapsing multi-step pipelines into zero-context-cost
turns. » Un outil exécute un script qui enchaîne plusieurs outils **sans repasser par le LLM** à chaque étape.
**Pour LaRuche** : une Abeille `run_script` qui appelle d'autres Abeilles via un mini-RPC interne → pipelines
multi-étapes en 1 tour. Énorme économie de contexte/latence.

### 9. Profils de posture (coding/general) — `coding_context.py`
`RuntimeMode` figé issu d'un `ContextProfile` : déclare le **toolset à réduire**, le brief opérationnel à
injecter, et des hints (routage modèle, mémoire, subagents). Tout le code lit le même objet résolu.
**Pour LaRuche** : remplacer la sélection d'outils plate par des **profils** (chat / coding / ops) qui
collapsent le toolset et ajustent le prompt — plus net que filtrer outil par outil.

### 10. `reasoning_effort` par modèle — (16 fichiers)
Passent un niveau de raisonnement par modèle (baissé pour économiser). Complète notre toggle no-think.

### 11. `cache_control` (Anthropic prompt caching) — (4 fichiers)
Posent des breakpoints de cache sur le préfixe stable pour Claude.
**Pour LaRuche** : si provider Anthropic, marquer le préfixe stable avec `cache_control`.

---

## ⚪ Déjà au plan (confirmé chez third-party)
- **Idempotency keys** (30 fichiers) + **heartbeats** (20) → Kanban/cron/watchers (cf. HANDOFF T-kanban).
- **Learning loop** : `background_review` (post-tour) + `curator` (idle, umbrella-building) → notre P4.
- **dedup** omniprésent (55 fichiers) : déduplication des résultats d'outils, mémoire, messages.
- **Trajectory save** (`turn_finalizer`, `batch_runner.py`) → données d'entraînement (optionnel/recherche).

---

## Ordre d'attaque recommandé pour LaRuche
1. **#1 prefix-cache 3 tiers** (règle la latence — déjà à moitié identifié dans T13).
2. **#2 client aux** pour la curation de fond.
3. **#4 garde-fous de boucle d'outils** (fiabilité immédiate).
4. **#5 classifieur d'erreurs / failover** puis **#6 compression préflight**.
5. **#8 scripts-RPC** et **#9 profils** (gains de contexte structurels).


<a id="archive-G"></a>

---

# ARCHIVE G — HANDOFF.md (passation mémoire T1-T14)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# HANDOFF — finir le portage de LaRuche v2 (pour Gemini / Codex)

Tu reprends un projet en cours. Lis d'abord **[ARCHITECTURE.md](./ARCHITECTURE.md)** (le plan) et
**[README.md](./README.md)** (build/run/UI). Ce fichier = ce qui reste à faire, précisément.

## ÉTAT ACTUEL — mis à jour 2026-06-19 (après passage Codex + revue)

**Tout compile et teste vert** : `cargo test -p laruche-memoire` (6 tests), `cargo test -p laruche-essaim`,
`cargo check -p laruche-node`. (NB : `cargo build -p laruche-node` peut échouer si un `laruche-node.exe`
tourne déjà — Windows verrouille le binaire ; tuer le process ou utiliser `cargo check`.)

**FAIT :**
- T1 ✅ sémantique (embeddings + hybride, `LARUCHE_EMBED_URL`).
- T2 ✅ renforcé : table `nodes`, `read_node` → children+items, `dream` → suggestions structurées.
- T4 ✅ onglet SPA Mémoire (hexagones SVG, recherche illumine, panneau items, écriture, clic nœud) + endpoints `/api/memory/*`.
- T5 ✅ mémoire auto dans tous les chemins serveur (WS, webhook, Telegram, audio, cron) + `boucle_react_memoire_multimodal`.
- T7 ✅ `laruche` (TUI) + `laruche-node` buildent.
- T8 ✅ provider OpenAI local sans clé (loopback) + auto-détection llama.cpp `:8001`.
- T9 ✅ strip `<think>` (Rust + UI) + checkbox no-think persistée.
- T10 ✅ toggles Abeilles (persistés, exclus du prompt, blocage exécution) + outputs outils dans le chat + ligne Usage.
- T11 ✅ (commencé) sélection dynamique d'Abeilles via `tools.abeilles` en mémoire.
- **Cycle de maintenance mémoire ✅** : `update_item / delete / move_item / review / list_proposed / suggest_nodes`
  (+ autocomplete datalist) — porté sur sqlite, native, sidecar, abeilles, API, UI.
- **`stats` + `mutations` (audit) ✅** (ce passage-ci) : trait + sqlite + sidecar + abeilles `memory_stats`/`memory_mutations`
  + endpoints `GET /api/memory/stats` et `/api/memory/mutations`.

**RESTE (vs paradigm) :**
- **Node CRUD** : `memory_create_node` / `update_node` (renommer, one_liner, importance) / `delete_node` (reparent). Les nœuds sont auto-créés mais pas éditables/supprimables.
- **UI** : panneaux pour afficher `stats` + le journal `mutations` (audit) + file de revue des `proposed` (les endpoints existent, brancher l'affichage).
- **T3 / interop** : `export` / `import` / `import_markdown` (OKF), `snapshots`.
- **Embeddings** : `doctor` / `warm` (maintenance cache embeddings) — optionnel.
- **T6** : valider `SidecarBackend` contre le vrai paradigm (`npm install` requis dans `paradigm/`).
- **T12** : provider Codex/OpenAI en mode abonnement (OAuth) — non commencé.

---

### T13 — Performance (latence chat)
- ✅ **Auto-curation non bloquante** : `curer_memoire` passe en `tokio::spawn` (réponse rendue avant le 2e appel LLM).
- ✅ **Mémoire en contexte trailing (prefix-cache)** : la mémoire n'est PLUS dans `custom_instructions`/
  system prompt ; injectée comme message `system` **trailing** (après l'historique) via le nouveau
  `boucle_react_multimodal_ext(..., ephemeral_context)`. Le system prompt (identité + schémas d'outils)
  redevient un préfixe stable → cache LCP de llama.cpp réutilisable (astuce third-party `system_prompt.py`).
- ✅ **Toolset stable (#9)** : `EssaimConfig.stable_toolset` → sélection d'outils query-INDÉPENDANTE
  (core + remplissage alpha) ; activé dans le chemin mémoire → préfixe identique d'un tour à l'autre.
  Combiné à la mémoire trailing, le system prompt est enfin stable across-turns → cache réutilisé.
- Leviers UI déjà dispo : No-think, désactiver des Abeilles, baisser `tool_selection_limit`.
- Option : gater l'auto-curation (1 tour sur N) pour ne pas occuper le slot llama.cpp juste après la réponse.

### Hacks third-party — état d'implémentation (réf. third-party_HACKS.md)
**FAIT (ce passage) :**
- #1 system prompt stable + mémoire trailing — `boucle_react_multimodal_ext`.
- #2 client auxiliaire pour la curation — `EssaimConfig.aux_model` utilisé par `curer_memoire`.
- #3 garde-fous de boucle d'outils — compteur (nom+args) dans `boucle_react_multimodal_ext` : warning à 3×, halt à 6×.
- #9 toolset stable par profil — `EssaimConfig.stable_toolset`.

**RESTE (specs pour Codex/Gemini) :**
- **#5 classifieur d'erreurs** : dans le bloc failover de `boucle_react_multimodal_ext`, classer l'erreur
  provider — `context_length` → `session.compacter()` puis retry même modèle ; `rate_limit` → backoff court ;
  `auth` → message clair, pas de retry ; `transient/5xx` → 1 retry. Réf. third-party `agent/error_classifier.py`.
- **#7 interrupt/steer** : permettre d'injecter un message pendant un tour en cours (le canal WS existe).
  Réf. third-party `STEER_CHANNEL_NOTE` + (`steer`/`interrupt` dans le repo).
- **#8 scripts-RPC** : Abeille `run_script` exécutant un script qui appelle d'autres Abeilles via un mini-RPC
  interne (sans repasser par le LLM) → pipelines multi-étapes en 1 tour. Gros gain contexte/latence.
- **#10 reasoning_effort** : champ `EssaimConfig.reasoning_effort: Option<String>` passé au provider (adapter
  `providers.rs` openai/anthropic) pour les modèles qui le supportent. Complète le toggle no-think.
- **#11 cache_control** : pour provider Anthropic, marquer le préfixe stable avec des breakpoints `cache_control`.
- **#6 compression préflight** : compacter l'historique AVANT l'appel (résumé), pas seulement tronquer.

### T14 — Boost des tools (best of third-party) + UX

**FAIT :**
- ✅ `web_deep_search` : nettoyage des URLs DuckDuckGo (`clean_result_url` : décode `uddg=`, rejette
  assets `.css/.js/...` et liens internes DDG) + **garde content-type HTML** (ne dump plus de CSS/JS brut)
  + troncature **tête+queue** (garde la fin de page : scores/conclusions). → réglait le bug « dump CSS ».

**Passe catalogue third-party → ajouts FAITS :**
- ✅ `file_edit` (= `patch` third-party) : édition ciblée par remplacement de chaîne unique (`old_string`→`new_string`,
  +`replace_all`). Ajouté au noyau d'outils. Critique pour coder sans réécrire les fichiers entiers.
- ✅ `file_read` boosté : **plages de lignes** (`offset`/`limit`) + **numéros de ligne** + auto-plage si gros fichier
  (plus de rejet brutal à 100KB). Aligne sur `read_file` third-party (et facilite `file_edit`).

**Outils third-party ajoutés :**
- ✅ `clarify` : l'agent pose une question et REND LA MAIN (court-circuit dans `brain.rs` : la question
  devient la réponse du tour, l'utilisateur répond ensuite). Ajouté au noyau.
- ✅ `run_script` (= scripts-RPC third-party) : pipeline d'outils en 1 tour, refs `{{N}}` entre étapes,
  garde-fous (max 12, pas de run_script/delegate imbriqués). Backé par le sous-registre. Au noyau.

**Outils third-party encore MANQUANTS (specs) :**
- `session_search` : rechercher dans les conversations passées (données/API existent côté node).
- `execute_code` : snippet Python/JS isolé (au-delà de `shell_exec`).
- `read_extract` : texte de PDF/docx (aujourd'hui `file_read` = texte brut seulement).
- `todo` : liste de tâches structurée (on n'a que les tags `<plan>`).

**TOOLS boostés (comparaison third-party) :**
- ✅ `web_fetch` : troncature **tête+queue** (était tête seule).
- ✅ `file_read` : offset/limit + numéros de ligne. ✅ `web_deep_search` : URLs propres + content-type + tête+queue.
- Reste : extraction lisible (readability) pour web_fetch/web_deep ; descriptions resserrées ; dédup outputs.
- `web_recherche` : multi-moteurs fallback ; vérifier snippets propres ; renvoyer une réponse directe si DDG « instant answer ».
- ✅ `shell_exec` : **commandes read-only auto-approuvées** (`est_commande_read_only` dans `brain.rs` :
  git status/log/diff, ls/dir, cat/type, echo, pwd, whoami, get-date, powershell get-*). Tout ce qui
  chaîne/redirige/mute (`&&`, `|`, `>`, rm, set-, remove-, install…) garde l'approbation. → fini la friction sur `Get-Date`.
- `fichiers`/`file_read` : lecture par **plage de lignes** (offset/limit) pour ne pas charger des gros fichiers entiers (style third-party).
- **Descriptions d'outils** : resserrer + 1 exemple par outil (améliore le tool-calling des petits modèles).
- **Dédup des résultats d'outils** : ne pas réinjecter un output d'outil identique (third-party `dedup`).
- **#8 `run_script` (RPC)** : Abeille qui enchaîne plusieurs Abeilles sans repasser par le LLM → pipeline en 1 tour.

**UX / UI (specs, dans `spa.html`) :**
- **Bouton Stop/Interrompre** un tour en cours (canal steer #7) — gros confort sur modèle lent.
- **Compteur de contexte** (tokens utilisés / budget) en barre — l'event `Usage` existe déjà.
- **Chip « mémoire injectée »** : afficher quels souvenirs ont servi au tour.
- **Outputs d'outils repliables** (collapse) quand longs.
- **Indicateur tok/s + prompt-eval** (visibilité perf, after the prefix-cache fix).
- Auto-approbation read-only (cf. shell ci-dessus) = le plus gros gain de fluidité ressentie.

## 0. Mission

Fusionner LaRuche (agent Rust) + paradigm-memory (carte cognitive) en un agent local-first
**mono-binaire Rust**. Le cœur de l'idée tient déjà : l'agent parle à la mémoire via UN trait
`MemoireCognitive`, avec plusieurs backends interchangeables. Ta job : finir le backend Rust
**sémantique** (la seule grosse pièce manquante), puis les features autour.

## 1. Règles non négociables

- **Ne modifie JAMAIS** `laruche/laruche-essaim/src/rag.rs` ni le code de `paradigm/` (cœurs gardés intacts). On *compose*, on ne fork pas.
- **Ne casse pas le trait** `MemoireCognitive` (contrat ci-dessous) : tout l'agent en dépend.
- Cible = **mono-binaire Rust**, zéro Node/Python au runtime. N'ajoute pas de dépendance qui casse ça.
- Conventions de nommage FR (abeille, mémoire, etc.), cohérentes avec l'existant.
- Vérifie chaque étape par `cargo build` + un test. Ne livre pas de code non compilé.

## 2. Environnement (pièges déjà rencontrés)

- **Toolchain** : la toolchain par défaut `windows-gnu` PLANTE (dlltool/MinGW absent). MSVC marche.
  C'est déjà réglé via `laruche/rust-toolchain.toml` (channel msvc) → utilise simplement `cargo build`.
- **Node/npm absents du PATH.** Un node v24 existe (`%LOCALAPPDATA%/ms-playwright-go/1.57.0/node.exe`)
  mais pas de npm → le vrai paradigm n'a pas pu tourner ici. Voir tâche T6.
- **LLM de test** : `llama.cpp` en local sur `http://localhost:8001` (OpenAI-compatible), modèle `qwen3.6-35b-a3b`.
- **Git** : NE PAS gérer git, l'utilisateur s'en occupe.

## 3. Le contrat — trait `MemoireCognitive`

Fichier : `laruche/laruche-memoire/src/lib.rs`.

```rust
#[async_trait]
pub trait MemoireCognitive: Send + Sync {
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<ContextPack>;
    async fn write(&self, item: MemoryItem) -> Result<Value>;
    async fn propose_write(&self, item: MemoryItem) -> Result<Value>;
    async fn read_node(&self, node_id: &str) -> Result<Value>;
    async fn dream(&self) -> Result<Value>;
    async fn health(&self) -> Result<bool>;
}
```
`ContextPack { raw: Value }` a `.to_prompt_text()` qui rend `{nodes:[{id,one_liner}], items:[{content}]}`.
Tout nouveau backend DOIT renvoyer cette forme dans `raw` pour rester compatible avec l'agent.

## 4. État (ce qui est FAIT et compile)

- `laruche/laruche-memoire/` : trait + 3 backends.
  - `src/sidecar.rs` → `SidecarBackend` : client HTTP vers vrai paradigm (`POST :8765/mcp` JSON-RPC). Test vert : `tests/sidecar_mock.rs`.
  - `src/native.rs` → `NativeBackend` : RAM, lexical (démo).
  - `src/sqlite.rs` → `SqliteBackend` : **SQLite + FTS5 + audit, persistant, Rust pur** (rusqlite `bundled`). C'est la moitié de P3.
- `laruche/laruche-essaim/src/brain.rs` → `boucle_react_memoire(...)` : auto-récup (injection) + auto-curation (extraction LLM). Cœur ReAct inchangé.
- `laruche/laruche-essaim/src/abeilles/memoire.rs` → Abeilles `memory_search` / `memory_write` + `enregistrer_memoire()`.
- `laruche/laruche-node/src/main.rs` (~ligne 3821) : mémoire enregistrée, backend par env `LARUCHE_MEMOIRE_BACKEND` = `native`(défaut)|`sqlite`|`sidecar`.
- Démo : `laruche/laruche-essaim/examples/poc_memoire.rs` (lancée avec succès sur llama:8001).

Commandes de vérif :
```bash
cd laruche
cargo build -p laruche-memoire && cargo test -p laruche-memoire
cargo run -p laruche-essaim --example poc_memoire   # nécessite llama.cpp:8001
cargo build -p laruche-node                          # PAS encore re-vérifié end-to-end → à confirmer
```

## 5. TÂCHES RESTANTES (par priorité)

### T1 — [P3] Couche sémantique du `SqliteBackend` — ✅ ARCHITECTURE FAITE & TESTÉE
**Fait :** trait `Embedder` + `cosine` + `OllamaEmbedder` (`src/embed.rs`), stockage des vecteurs
(colonne `embedding BLOB`) et **recherche hybride** (cosinus 0.7 + boost lexical 0.3) dans
`src/sqlite.rs` (`SqliteBackend::open_with_embedder`). Prouvé hors-ligne par
`tests/sqlite_semantic.rs` (recall sans mot commun, vert).
**Reste (petit) :** (a) brancher un vrai embedder dans `laruche-node` — utiliser
`SqliteBackend::open_with_embedder("memoire.db", Arc::new(OllamaEmbedder::new(url,"nomic-embed-text")))`
si une var d'env type `LARUCHE_EMBED_URL` est définie ; (b) *(optionnel)* impl `Embedder` via
`fastembed` (ONNX) pour rester 100% mono-binaire sans Ollama. Détails de l'approche d'origine ci-dessous.

#### (référence) approche embeddings ⭐
But : recall *vocabulaire-indépendant* (ex. « conditions pour coder » retrouve « tongs/jazz »),
ce que le FTS5 lexical seul ne fait pas.
- Ajoute des **embeddings** à `laruche/laruche-memoire/src/sqlite.rs`.
  - Option recommandée (mono-binaire) : crate `fastembed` (ONNX, modèle `AllMiniLML6V2`, ~90 Mo téléchargé au 1er run). Build lourd (ort/ONNX) — accepte le temps de compil.
  - Alternative légère : réutiliser les embeddings Ollama déjà présents dans `laruche/laruche-essaim/src/rag.rs` (`/api/embed`, `nomic-embed-text`) via un appel HTTP — évite ONNX mais ajoute une dépendance Ollama au runtime.
- Schéma : ajoute une colonne/table `embedding BLOB` (vecteur f32 sérialisé) sur `items`.
- `search` : calcule l'embedding de la requête, fais un **score hybride** = `α·BM25_normalisé + β·cosinus` (commence α=0.5, β=0.5), trie, `LIMIT`.
- Cosinus : repcompd. simple en Rust (cf. `cosine_similarity` dans `rag.rs`).
- **Acceptance** : un test `tests/sqlite_semantic.rs` qui écrit « je code en tongs en écoutant du jazz » puis `search("dans quelles conditions je programme")` renvoie cet item (0 mot commun → prouve le sémantique). `cargo test -p laruche-memoire` vert.

### T2 — [P3 avancé] Activation de carte cognitive (fidélité paradigm)
Porter la logique d'activation hiérarchique de paradigm pour que `search` renvoie le *sous-arbre pertinent*.
- Source à mirrorer : `paradigm/packages/memory-core/src/atlas.mjs` (874 l.) + `consolidator.mjs` (dream, 177 l.).
- Modélise des **nœuds** (table `nodes(id, parent_id, label, one_liner, importance)`) en plus des items.
- `search` : score d'activation par nœud (label+keywords+distance embedding), gates open/latent/ignored (seuils 0.75/0.45/0.25 comme paradigm).
- `dream` : implémente détection doublons/périmés/surchargés/orphelins (heuristiques de `consolidator.mjs`).
- **Acceptance** : `dream()` renvoie des suggestions structurées ; `read_node` renvoie enfants + items.

### T3 — [Interop] Import/export OKF
- Ajoute `export_okf(dir)` / `import_okf(dir)` au trait + impl sur `SqliteBackend`.
- OKF = fichiers Markdown + frontmatter YAML, arborescence, liens markdown (cf. ARCHITECTURE.md §10).
  Mapping : nœud↔dossier/`index.md`, item↔doc, tags↔`tags:`, lien `[[x]]`↔`[x](/path.md)`.
- **Acceptance** : round-trip export→import préserve les items ; un dossier OKF externe s'importe sans erreur.

### T4 — [UI] Onglet « Mémoire » nid d'abeille dans le SPA
- Fichier : `laruche/laruche-dashboard/src/templates/spa.html` (UN fichier vanilla, zéro build, tokens CSS ambre/ruche déjà présents — voir `:root`).
- Ajoute un onglet hash-routé « Mémoire » : barre de recherche + **carte cognitive en hexagones SVG** (nœuds = alvéoles, activation = remplissage ambre `--amber`), panneau item, file de revue (accept/reject), journal d'audit.
- Données via nouveaux endpoints sur `laruche-node` : `GET /api/memory/search?q=`, `POST /api/memory/write`, `GET /api/memory/tree`, qui appellent le trait `MemoireCognitive` (déjà instancié dans `main.rs`).
- **Ne PAS** porter le React de `paradigm/packages/memory/` (casserait le mono-fichier). Reconstruire en vanilla.
- **Acceptance** : ouvrir `http://localhost:8419`, onglet Mémoire affiche la carte et la recherche illumine les cellules.

### T5 — Brancher l'auto-mémoire dans le serveur
- Dans `laruche/laruche-node/src/main.rs`, remplacer les appels `boucle_react(...)` du chemin chat principal par `boucle_react_memoire(..., memoire.clone())` (le `memoire: Arc<dyn MemoireCognitive>` existe déjà après ligne ~3821). Garde une copie `Arc` accessible aux handlers.
- **Acceptance** : via l'UI, un fait donné en conversation est rappelé dans une session neuve.

### T6 — Valider le `SidecarBackend` contre le vrai paradigm
- `cd paradigm && npm install` (Node 22+), puis `node packages/memory-mcp/src/http-server.mjs` (pont :8765).
- Lancer `LARUCHE_MEMOIRE_BACKEND=sidecar` et vérifier un write/search réel.
- **Acceptance** : le POC marche avec le vrai moteur paradigm (embeddings/dream complets).

### T7 — Modes de lancement (vérifier l'existant LaRuche)
`laruche` = **client TUI** ; `laruche-node` = **serveur** (Web UI + API + agent) qui **expose** ses
modèles aux autres nœuds/clients via Miel/swarm. S'assurer que les deux modes marchent dans v2
(le `.bat` lance `laruche-node`). NB : le serveur LLM lui-même reste externe (Ollama / llama.cpp).

### T8 — Provider OpenAI local sans clé
`providers.rs` exige une `api_key` non vide pour provider `openai` → bloque llama.cpp/LM Studio en local.
**Fix** : si `base_url` est loopback (127.0.0.1/localhost), autoriser une clé vide (envoyer un bearer factice). + Auto-détecter llama.cpp sur `:8001` au boot (cf. fin de session précédente).

### T9 — Reasoning think/no-think
qwen et autres modèles « reasoning » émettent `<think>…</think>` → contexte gonflé, lenteur, réponses
tronquées. **À faire** : (a) **stripper `<think>…</think>`** dans `brain.rs` (même mécanisme que
`strip_plan_tags`), (b) **case à cocher think/no-think** dans l'UI qui injecte `/no_think` (ou
`chat_template_kwargs:{enable_thinking:false}`) au provider.

### T10 — Visibilité & contrôle dans l'UI
- Afficher **tokens utilisés / contexte** par tour (l'event `Usage` existe déjà dans `ChatEvent`).
- Afficher proprement **les outputs des outils (Abeilles)** dans le fil de chat.
- **Cases à cocher pour activer/désactiver chaque Abeille** (Settings → Abeilles).

### T11 — ⭐ Abeilles en mémoire cognitive (réduire le contexte) — IDÉE CLÉ
Problème : on injecte les schémas des 20+ Abeilles **à chaque tour** → prompt énorme (~5k tokens),
d'où la lenteur. **Solution** : stocker les définitions d'outils dans un nœud `tools.abeilles` de la
mémoire cognitive (paradigm), puis faire une **sélection dynamique d'outils** : à chaque tour, ne
récupérer (via `memory_search` sur l'intention) que les ~3-6 Abeilles pertinentes et n'injecter que
celles-là dans le system prompt. Gros gain de contexte + vitesse. (Garder un petit noyau toujours présent.)

### T12 — Auth « abonnement » OpenAI/Codex (OAuth, sans clé API)
But : utiliser ChatGPT/Codex via l'abonnement, pas une clé API payante. Deux voies :
- **(recommandé) Provider `codex` qui délègue au CLI officiel.** L'utilisateur fait `codex login` une
  fois (OAuth abonnement géré par OpenAI, token dans `~/.codex/auth.json`). Ajouter dans
  `laruche-essaim/src/providers.rs` un provider qui **spawn le CLI `codex`** en sous-process et parle
  JSON sur stdio (au lieu de HTTP). Réf. third-party : `agent/codex_runtime.py` (`run_codex_app_server_turn`)
  + `agent/transports/codex_app_server_session.py` (protocole app-server stdio). Conforme ToS.
- **(fragile) OAuth direct en Rust** : lire `~/.codex/auth.json`, requêtes vers
  `https://chatgpt.com/backend-api/codex` avec `Authorization: Bearer <access_token>` + header
  `ChatGPT-Account-Id`, gérer le refresh (re-lire auth.json par requête). Réf. third-party :
  `agent/anthropic_adapter.py` (`_is_oauth_token`) + `third-party_cli/auth.py`. ⚠️ endpoints non officiels = zone grise ToS.
- Idem possible pour Claude (OAuth `anthropic-beta: oauth-…`), minimax-oauth, xai-oauth.
- third-party est en Python → **ne pas copier les fichiers**, les porter en Rust en s'en servant de spec.

## 6. Référence rapide — sources paradigm à mirrorer (lecture seule)

| Fichier | Lignes | Rôle |
|---|---|---|
| `paradigm/packages/memory-core/src/sqlite-store.mjs` | 982 | stockage SQLite (déjà ~porté dans `sqlite.rs`) |
| `paradigm/packages/memory-core/src/atlas.mjs` | 874 | carte cognitive + activation (T2) |
| `paradigm/packages/memory-core/src/consolidator.mjs` | 177 | dream (T2) |
| `paradigm/packages/memory-core/src/embeddings.mjs` | 129 | embeddings (T1, à refaire en Rust) |
| `paradigm/packages/memory-mcp/src/server.mjs` | 657 | schémas exacts des 28 outils (référence args) |

## 7. Définition de « fini »

`cargo build -p laruche-node` OK, `LARUCHE_MEMOIRE_BACKEND=sqlite`, UI sur `:8419` avec onglet Mémoire
fonctionnel, recall sémantique vert (T1), et le POC `poc_memoire` qui passe. Le tout en un binaire Rust.


<a id="archive-H"></a>

---

# ARCHIVE H — ROADMAP.md racine (pistes A-E « récolte »)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# LaRuche v2 — Roadmap parallélisable (récolte + autonomie)

But : intégrer dans le tronc `laruche-v2/laruche` ce qui existe déjà (2 crates de réf.) +
les 3 features d'autonomie (skills, crons, watchers). **5 pistes quasi indépendantes** →
exécutables en parallèle (par plusieurs agents). Cœurs `paradigm/` et `rag.rs` jamais touchés.

Sources de récolte :
- `C:\Users\infinition\Desktop\laruche-architecture-rust` (10 modules, `cargo test` vert)
- `C:\Users\infinition\Desktop\laruche-core` (5 modules, orienté coder)

Règle : on **porte/adapte** ces modules dans des crates `laruche-*`, derrière les traits existants
(`MemoireCognitive`, `Abeille`), sans casser l'API.

---

## Piste A — Socle UX & robustesse  (source: architecture-rust)  [INDÉPENDANTE]
| Module source | Destination | Acceptance |
|---|---|---|
| `thought_stream/` | `essaim` + `ChatEvent::Thought` + UI | streaming réflexion visible (StatusOnly/Summaries), sanitisé → règle la plainte UX |
| `permissions/` | crate `laruche-permissions`, remplace `NiveauDanger` | modes Default/Plan/AcceptEdits/Auto ; deny>allow ; règles persistables |
| `compaction/` | remplace `session::compacter` | résumé structuré + gros outputs sur disque (`ToolResultStore`) |
| `events/` | `laruche-node` (audit/observabilité) | bus NDJSON, `since(id)`, export |

## Piste B — Skills auto-créés  (source: core/okf_memory)  [dépend léger: mémoire]
- Porter `okf_memory` → crate `laruche-skills` (format OKF : paradigmes + étapes).
- Stocker les skills dans la carte cognitive sous `tools.skills.<nom>` (réutilise `MemoireCognitive`).
- **Boucle de création** : après une tâche complexe réussie, fork aux-model (réutilise `curer_memoire`)
  → extrait un skill OKF → `propose_write`. (= third-party `background_review`/`curator`.)
- Invocation : Abeille `skill_view` + `/skill-<nom>` (slash). Acceptance : un skill créé est rappelé et réutilisé au tour suivant.
- Bonus même piste : porter `editor_guard` → **sécuriser `file_edit`** (read-before-write + lock timestamp + fuzzy replace).

## Piste C — Crons façon third-party  (base: `cron.rs` existant)  [INDÉPENDANTE]
- Abeilles `cron_create / cron_list / cron_delete` (l'agent gère ses crons).
- Planif en **langage naturel** → cron expr. Livraison du résultat via channels (Telegram/Discord).
- Clé d'**idempotence** (anti-doublon sur trigger). Acceptance : « chaque matin 8h, résume X » tourne seul et délivre.

## Piste D — Watchers (le différenciateur)  [nouveau]  [INDÉPENDANTE au départ]
- Crate `laruche-watchers` : registre de triggers persistants.
- Types : fichier (réutilise Abeille `file_watch`), RSS/URL (diff de contenu), webhook entrant, pattern de log.
- Boucle d'événements → sur déclenchement : crée une **tâche** (Kanban si dispo, sinon appel agent direct + livraison channel).
- Le watcher peut interroger la mémoire (« qu'est-ce qui a changé depuis ? »). Acceptance : un fichier modifié déclenche un tour d'agent + notif.

## Piste E — Kanban + coordinateur  (source: core/fs_coordination + coordinator)  [dépend: events]
- Porter `fs_coordination` (task list sur FS, statuts, dépendances) + `coordinator` (spawn-vs-continue, synthesize).
- Board durable + dispatcher (réutilise `cron.rs` tick). Abeilles `kanban_*`. Onglet SPA Kanban.
- C'est le réceptacle des watchers (D) et le moteur des sous-agents.

---

## Graphe de dépendances (pour paralléliser)
```
A (socle)   ── indépendante ──────────────┐
C (crons)   ── indépendante ──────────────┤
D (watchers)── indépendante (livre via channel d'abord) ─┐
B (skills)  ── a besoin de: MemoireCognitive (déjà là)   │
E (kanban)  ── a besoin de: events (Piste A)  ───────────┘ puis D s'y branche
```
→ **A, B, C, D démarrables en parallèle tout de suite.** E après `events` (début de A).
→ D livre d'abord en direct (channel), puis se branche sur E quand le Kanban existe.

## Ordre conseillé si séquentiel
1. A.thought_stream (gain UX immédiat) → 2. B (skills, headline) → 3. D (watchers, unique) →
4. C (crons) → 5. E (kanban) → puis A.permissions/compaction en fond.

## État déjà fait (ne pas refaire)
Mémoire cognitive (3 backends), mémoire auto trailing + toolset stable (prefix-cache), client aux,
garde-fous anti-boucle, outils boostés (file_edit, file_read plages, web_deep/web_fetch propres,
shell read-only auto-approve), `clarify`, `run_script`, stats/mutations, **export OKF format Google**
(`export_okf` + `GET /api/memory/export_okf`, test vert), UX (approbation qui disparaît, plan replié,
compteur clarifié). Voir HANDOFF.md / third-party_HACKS.md.

## Affectation — Round 1 (FAIT, mergé, tests verts)
- Codex : Piste B skills + editor_guard. · Gemini : Piste D watchers. · Moi : OKF import/export + refonte UX Mémoire (arbre + carte focalisée).

## Affectation — Round 2 (en cours)
- **Codex → Piste A côté moteur** (zone `laruche/laruche-essaim/` UNIQUEMENT) : thought_stream + `execute_code` + `todo`.
- **Gemini → Piste E Kanban + intégration UI** (zone `laruche/laruche-node/`, `spa.html`, nouvelle crate `laruche/laruche-kanban/`).
- **Moi** : `laruche-memoire/` + crate `laruche-events` (standalone) + passe d'intégration finale.
- Contrat partagé (pas de collision de fichier) : Codex ajoute `ChatEvent::Thought { phase, kind, text }` dans
  `brain.rs` ; Gemini gère l'event `thought` dans `spa.html`. `laruche/Cargo.toml` : chacun ajoute la ligne de SA crate.

## Invariants
- Pas de Node/Python au runtime (mono-binaire Rust). Cœurs upstream intacts.
- Tout passe par les traits (`MemoireCognitive`, `Abeille`) → backends/outils interchangeables.
- Chaque piste : `cargo test` vert + une démo observable avant de la dire finie.


<a id="archive-I"></a>

---

# ARCHIVE I — laruche/ROADMAP.md (roadmap produit / Miel / capabilities)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

# LaRuche - Roadmap & Spécifications Futures

> ⚠️ **Pour le chantier de portage agentique (Claude Code + third-party), la source de vérité est
> [ROADMAP_CLAUDE_CODE_PORT.md](./ROADMAP_CLAUDE_CODE_PORT.md)** (phases, lots, vagues, tableau de
> suivi) + l'[AUDIT_COMPARATIF.md](./AUDIT_COMPARATIF.md). Ce document-ci reste la roadmap
> **générale** du projet (features produit issues du README).

Ce document consolide les fonctionnalités prévues (issues du README) et détaille les suggestions d'implémentation ainsi que les défis techniques associés pour le projet LaRuche.

## Fonctionnalités implémentées (Terminées)

Ces éléments sont déjà opérationnels dans la version actuelle du projet :

- [x] **Miel protocol core** (mDNS discovery + Cognitive Manifest)
- [x] **Capability differentiation** (LLM, VLM, VLA, RAG, Audio, Image, Embed, Code)
- [x] **Proof of Proximity authentication**
- [x] **QoS priority system**
- [x] **Swarm state management**
- [x] **Node daemon with Ollama bridge**
- [x] **Client SDK** (3-line usage)
- [x] **CLI tool**
- [x] **Web dashboard with cyber monitoring**

## Fonctionnalités prévues (Backlog)

Ces éléments ne sont pas encore implémentés et font partie de la roadmap officielle du projet :

### 🔬 Preuves de Concept (POC) & Intégrations Capacitaires
Ces points visent à tester et valider concrètement les différents modèles et cas d'usage pris en charge par le système de capacités (`capability:*`) du protocole Miel :
- [ ] **POC `capability:llm` (Text-to-text)** : Le cas d'usage de base. Un nœud standard exécutant un modèle comme Mistral ou Llama pour de la discussion fluide ou des tâches NLP simples.
- [ ] **POC `capability:vlm` (Vision-Language)** : Envoyer une image via le réseau à un nœud exécutant LLaVA ou Qwen-VL pour analyse visuelle, description ou OCR.
- [ ] **POC `capability:vla` (Vision-Language-Action)** : Piloter un bras robotique ou un drone basique via le réseau LaRuche.
- [ ] **POC `capability:rag` (Retrieval-Augmented Generation)** : Créer un nœud LaRuche spécialisé qui indexe un dossier de documents locaux et répond aux questions sur cette base.
- [ ] **POC `capability:audio` (Speech-to-Text / Text-to-Speech)** : Intégrer un nœud Whisper/Bark pour permettre des commandes vocales au réseau.
- [ ] **POC `capability:image` & `capability:embed`** : Générer des images (ex: Stable Diffusion) ou des vecteurs depuis un nœud dédié à la volée.
- [ ] **POC `capability:code`** : Tester un nœud exécutant CodeLlama/DeepSeek-Coder pour des requêtes spécifiques de développement.

### 🏗️ Infrastructure & Écosystème
- [ ] **Tensor sharding over Ethernet (Swarm Intelligence)**
- [ ] **LaRuche Resilience (failover, hot-swap, mirroring)**
- [ ] **NFC hardware integration**
- [ ] **VS Code extension**
- [ ] **Home Assistant plugin**
- [ ] **Mobile app (iOS/Android)**
- [ ] **Miel v1.0 specification (RFC)**

---

## Pistes de développement et Défis Techniques

Voici une déclinaison plus détaillée des points de la roadmap pour guider l'implémentation.

### 1. Swarm Intelligence & Tensor Sharding
Le "Tensor sharding over Ethernet" est l'un des aspects les plus novateurs et complexes du projet, nécessitant de surmonter les limites de réseau local.
*   **POC de partitionnement :** Réaliser une preuve de concept permettant de diviser l'inférence d'un LLM sur plusieurs machines connectées (pipeline parallelism).
*   **Optimisation réseau :** Explorer des protocoles bas-niveau (UDP, direct TCP) pour minimiser la latence inter-nœuds, potentiellement avec du RDMA si le matériel le permet.

### 2. LaRuche Resilience (Tolérance aux pannes)
Pour que le « plug-and-play de l'IA » soit fiable, la perte d'un boîtier ne doit pas impacter l'utilisateur.
*   **Failover dynamique :** Si un client requiert un nœud qui devient injoignable en cours de route, la requête doit être redirigée de manière transparente vers un autre nœud capable.
*   **Mirroring de contexte :** Synchroniser l'état (ou l'historique de conversation) entre les nœuds d'un même cluster LaRuche pour une reprise sans coupure.

### 3. Écosystèmes Applicatifs (VS Code, Home Assistant, Mobile)
*   **VS Code Extension :** S'appuyer sur la `capability:code`. L'extension découvrira automatiquement le nœud LaRuche via le protocole Miel pour offrir de l'autocomplétion (Copilot local) ou du chat.
*   **Home Assistant Plugin :** Transformer le réseau LaRuche en "cerveau" de la maison. Il faudra intégrer des mécanismes de *Tool Use* (appels de fonctions) pour que le LLM puisse exécuter des actions concrètes (allumer les lumières, etc.).
*   **Mobile App :** Un client léger (iOS/Android) communiquant via le SDK, avec intégration vocale potentielle (Speech-to-Text en amont).

### 4. Hardware et Sécurité (NFC & Proof of Proximity)
La promesse du "zéro configuration" s'accorde parfaitement avec le matériel.
*   **NFC "Tap-to-Connect" :** Approcher un smartphone ou une carte NFC du boîtier LaRuche pour autoriser instantanément un appareil, combinant sécurité ("Proof of Proximity") et expérience utilisateur magique.
*   **Quotas et Pare-feu logique :** Éviter qu'un seul client accapare toute la puissance GPU du réseau en exploitant les indicateurs de QoS du *Cognitive Manifest*.

### 5. Standards et Protocole (Miel v1.0 RFC)
*   **Spécification formelle :** Rédiger la RFC du protocole Miel en détaillant la structure du manifeste mDNS (types TXT), les flags de capacité, et les protocoles d'échange, afin de permettre à d'autres projets d'adopter Miel.

### 6. Rigueur Cognitive et V�rification
Pour lutter contre la "paresse" inh�rente aux LLMs, impl�menter des m�canismes for�ant l'agent � v�rifier le r�sultat de ses actions sur le syst�me.
*   **V�rification post-ex�cution :** Modifier le System Prompt de base ou enrichir les outils existants (ex: shell_exec) pour exiger (ou inclure automatiquement) un double contr�le (dir, ls, etc.) apr�s chaque modification ou t�l�chargement de fichier. L'agent ne doit affirmer la r�ussite d'une action que s'il en a la preuve (ex: fichier pr�sent sur le disque avec une taille > 0).


<a id="archive-J"></a>

---

# ARCHIVE J — README.md racine (build/run)

> _Texte intégral conservé pour l'historique. En cas de contradiction, la Partie 0 fait foi._

<div align="center">

# LaRuche v2 — agent local-first à mémoire cognitive

**Fusion de [LaRuche](./laruche) (corps : moteur d'agent Rust) et [paradigm-memory](./paradigm) (hippocampe : carte cognitive).**
Un agent autonome, local-first, mono-binaire — conçu pour concurrencer third-party / third-party.

</div>

> Architecture détaillée : **[ARCHITECTURE.md](./ARCHITECTURE.md)**. Ce README = comment build, lancer, et accéder à l'UI.

---

## En un coup d'œil

L'agent (boucle ReAct + outils « Abeilles ») parle à une **mémoire cognitive** via une
interface unique (`trait MemoireCognitive`) avec **3 backends interchangeables** :

| Backend | Ce que c'est | Quand |
|---|---|---|
| `NativeBackend` | Rust pur, en mémoire (lexical). Zéro dépendance. | Démo / défaut |
| `SqliteBackend` | **Rust pur, SQLite + FTS5 + audit, persistant, mono-binaire.** | Production locale |
| `SidecarBackend` | Client HTTP vers le **vrai paradigm** (`paradigm serve` :8765). | Moteur cognitif complet (embeddings, dream) |

`brain.rs` ne connaît que le trait → changer de backend ne touche aucune ligne d'agent.

---

## Prérequis

- **Rust** (toolchain MSVC — déjà épinglée via [`laruche/rust-toolchain.toml`](./laruche/rust-toolchain.toml)).
- *(optionnel)* un LLM : **Ollama** local, ou tout endpoint OpenAI-compatible (ex. `llama.cpp` sur `:8001`).
- *(optionnel, backend sidecar)* **Node 22+** pour lancer le vrai paradigm.

---

## Build

```bash
cd laruche
cargo build              # debug (la toolchain MSVC est auto-sélectionnée)
cargo build --release    # binaire optimisé
```

---

## Démo : le POC mémoire (le plus rapide pour « voir » la fusion)

Nécessite un LLM OpenAI-compatible sur `http://localhost:8001` (ex. llama.cpp).

```bash
cd laruche
cargo run -p laruche-essaim --example poc_memoire
```

Ce que ça prouve :
1. round-trip mémoire déterministe (write → search) ;
2. **conversation A** : l'agent mémorise un fait (auto-curation, sans qu'on le lui demande) ;
3. **conversation B (neuve, sans historique)** : l'agent **rappelle** le fait (auto-récupération injectée dans la boucle) et répond.

> Le modèle/endpoint est configuré en haut de [`examples/poc_memoire.rs`](./laruche/laruche-essaim/examples/poc_memoire.rs)
> (`model`, `api_base`). Adapte-les à ton setup.

---

## Lancer le serveur (Web UI + API + agent)

```bash
cd laruche
cargo run -p laruche-node
# ou, installé sur le PATH :
cargo install --path laruche-node --force && laruche-node
```

### Choisir le backend mémoire

```bash
# défaut : NativeBackend (RAM)
LARUCHE_MEMOIRE_BACKEND=sqlite   cargo run -p laruche-node   # SQLite+FTS5 persistant → ./memoire.db
LARUCHE_MEMOIRE_BACKEND=sidecar  cargo run -p laruche-node   # vrai paradigm sur :8765 (lancer paradigm serve à côté)
```

(En PowerShell : `$env:LARUCHE_MEMOIRE_BACKEND="sqlite"; cargo run -p laruche-node`)

L'agent expose alors les outils **`memory_search`** et **`memory_write`**.

---

## Accéder à l'UI web

Une fois `laruche-node` lancé, ouvrir :

```
http://localhost:8419
```

SPA mono-fichier (vanilla, embarquée dans le binaire) : **Chat, Tableau de bord, Sessions, Réglages, Console**.
L'onglet **Mémoire (carte cognitive en nid d'abeille)** est planifié — voir [ARCHITECTURE.md §6](./ARCHITECTURE.md).

Autres interfaces (héritées de LaRuche) : TUI client (`cargo run -p laruche-cli`), Telegram/Discord/Slack
(config dans Réglages), serveur MCP (`laruche mcp`), extension VS Code.

---

## Backend sidecar : lancer le vrai paradigm

```bash
cd paradigm
npm install          # nécessite Node 22+ (résout zod + packages workspace)
node packages/memory-mcp/src/http-server.mjs    # → pont sur http://127.0.0.1:8765
```

Puis lancer `laruche-node` avec `LARUCHE_MEMOIRE_BACKEND=sidecar`.

---

## État des phases

| Phase | Contenu | État |
|---|---|---|
| P0 | scaffold (2 bases copiées, toolchain pin) | ✅ |
| P1 | `laruche-memoire` (trait + `SidecarBackend`) + Abeilles mémoire (test vert) | ✅ |
| P2 | mémoire **automatique** dans la boucle (`boucle_react_memoire` : auto-récup + auto-curation) | ✅ (compile ; démo via l'exemple) |
| P3 | `SqliteBackend` (SQLite+FTS5+audit, Rust pur) | ✅ moitié faite — **reste les embeddings/sémantique** |
| P3+ | enregistrement dans `laruche-node`, onglet Mémoire nid d'abeille, Kanban, watchers | à venir |

---

## Tests

```bash
cd laruche
cargo test -p laruche-memoire   # test d'intégration du protocole sidecar (faux paradigm)
```

## Licence

LaRuche : MPL-2.0 · paradigm : Apache-2.0
