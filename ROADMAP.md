# LaRuche - Roadmap

> Reste-à-faire **récupéré des anciens docs de conception** (avant archivage dans `docs/_archive/`) + chantiers en cours. Source de vérité du travail restant. Coché = fait.

## 🔒 Audit sécurité & hygiène (2026-07-02) - PRIORITÉ ABSOLUE avant toute feature

> Revue complète du projet (probes ciblés + connaissance du moteur/mémoire/outils déjà lus). Le cœur agentique est solide ; la **couche d'exposition réseau était le point faible critique**. Durcissement réseau + sanitisation du rendu + mesh + vendorisation **LIVRÉS** (commits `e610365`, `9b1ccca`, `ee44d11`, 2026-07-02). Hygiène repo vérifiée propre (aucun secret committé, aucun fichier runtime tracké). Audit profond execute_code/mcp_client/crates FAIT (2026-07-02, cf. MINEUR). Reste : dette main.rs monolithique + suppression de brain.rs (déprécié, butinage est le défaut) + sandbox shell dure (différé, gate d'approbation = vrai contrôle).

### ✅ CRITIQUE - Cluster exposition réseau (CORRIGÉ, commit e610365 + ee44d11)
- [x] **Bind `127.0.0.1` par défaut** (était `0.0.0.0`) : exposition LAN = opt-in explicite `LARUCHE_BIND_LAN=1`, loggué en warn.
- [x] **CORS restreint** : prédicat n'autorisant que les origines `localhost`/`127.0.0.1`/`[::1]` (était `AllowOrigin::any()`).
- [x] **Middleware `auth_guard` global** : exige le cookie sur les requêtes MUTANTES (POST/PUT/DELETE/PATCH) vers `/api/*`, uniquement si un compte à mot de passe existe (fresh install + onboarding restent ouverts) ; GET passe (lectures UI) ; allowlist auth flow + sync interne. Bloque les mutations d'origine tierce non autorisée.
- [x] **Auto-sync mémoire mesh en opt-in** `LARUCHE_MESH_MEMORY_SYNC=1` (était auto toutes les 5 min sans vérif de pair). Faits importés provenance-taggés + traités en REFERENCE DATA. `import_changes` POST couvert par `auth_guard`.

### ✅ CRITIQUE - Sanitisation du rendu chat (LIVRÉ, commit 9b1ccca)
- [x] **`LaRuche.Utils.safeMarkdown` = `marked.parse` + `DOMPurify.sanitize`** sur les 2 rendus innerHTML du chat (streaming + restauration). Le HTML éventuellement présent dans une page récupérée par `web_fetch` est désormais nettoyé avant affichage (rendu, jamais exécuté).

### ✅ MAJEUR (CORRIGÉ, commit 9b1ccca)
- [x] **marked + DOMPurify + highlight.js vendorisés** dans `templates/vendor/`, servis via `/vendor/:name` (`web::vendor_js`), thème hljs inline dans app.css, `sw.js` v2 cache les vendors. Fin des `<script>` CDN (offline réel, zéro dépendance externe au chargement).
- [x] **Onboarding embeddings = vraie sonde** (déjà fait `0943092`) ; checks voix/Chrome vérifiés RÉELS (pas des stubs) ; "warning" STT/TTS/TLS = état honnête (services non lancés).
- [x] **Config d'agents (hors projet) CORRIGÉ (2026-07-02, côté user)** : le launcher `.0xid/apps/claude-code.bat` écrasait `~/.claude/settings.json` avec `CLAUDE_CODE_SUBAGENT_MODEL=deepseek-v4-flash` pour les profils DeepSeek/Local. Fix : profils `CLAUDE_CONFIG_DIR` dédiés (`~/.claude-deepseek`, `~/.claude-local`) + `~/.claude` nettoyé (`env: {}`). Fan-out de sous-agents rétabli.

### 🟡 MINEUR / dette (non bloquant)
- [x] **`main.rs` ingérable** (~3000 lignes). → **FAIT** (2026-07-02) : routeur + `auth_guard` extraits dans `router.rs`, état dans `state.rs`, jobs de fond dans `background.rs` (cf. item Split main.rs, section ⏸️ Reste).
- [x] **Polling front cumulé CORRIGÉ (2026-07-02)** : helper central `LaRuche.Poll.every/stop` (core.js) - tick sauté onglet caché, rattrapage immédiat au retour. 14 pollings convertis ; les flux d'auth (codex, challenge login) restent en `setInterval` volontairement.
- [ ] **Blocklist shell contournable** (`shell.rs` : `BLOCKED_PATTERNS` par sous-chaîne, `rm  -rf  /` double-espace passe) : ralentisseur, le VRAI contrôle est le gate d'approbation (`niveau_danger`=NeedsApproval + popup) + timeout. Sandbox OS dure = chantier différé. `secrets.rs` vérifié : valeurs jamais sérialisées/logguées (bon).
- [x] **Audit profond FAIT (2026-07-02)** : `execute_code.rs` sain (gate approbation, python -I, timeout 30s, caps, troncature char-safe ; fix : broken pipe stdin non fatal). `mcp_client.rs` : **timeout 60s par requête ajouté** (un serveur MCP muet au handshake bloquait le BOOT du node pour toujours) + reap du process (zombies Unix) + outil malformé loggué. Micro-crates : **aucun crate workspace mort** (`laruche-evals` harness binaire et `laruche-dashboard` porte-assets = voulus sans dépendants) ; `laruche-compaction` vs `escale` = doublon assumé, compaction ne sert que brain.rs+session.rs et mourra avec brain ; `laruche-events` (EventBus volatile) vs `feed_journal` (NDJSON persisté) = rôles distincts, pas un doublon.
- [ ] **`laruche-channels/` (Python)** : bots telegram/discord/slack legacy plus référencés nulle part dans le Rust (canaux natifs) → à archiver dans `_archive/` après confirmation user.
- [ ] **Duplication `log_activite`** (repérée au split de main.rs) : le heartbeat Ollama et les dispatchers cron/watcher/kanban (`background.rs`) reconstruisent `ActivityLogEntry` à la main au lieu d'appeler le helper `log_activite`. Pas un bug, juste de la duplication à résorber.
- [ ] **Imports inutilisés préexistants** (~9 warnings : knowledge_api, plugins_api, profiles_api, voice_api, laruche-essaim) → passe `cargo fix` rapide.

### 🗑️ Dette moteur
- [x] **Butinage PAR DÉFAUT (fait 2026-07-02)** : `moteur_butinage_actif()` centralisé dans `butinage_pont` - butinage par défaut, `RUCHE_MOTEUR=brain` = opt-out déprécié avec warn (une fois), l'ancien opt-in `=butinage` reste accepté (no-op). Dispatch unique vérifié (chat, canaux, missions, Reine).
- [ ] **Supprimer `brain.rs` (~4000 lignes)** une fois la dépréciation digérée (quelques semaines de runtime butinage sans regression) ; `laruche-compaction` part avec lui (session.rs à migrer sur `escale`).

## 🐝 Moteur butinage, évals, outils & mémoire - audit expert appliqué (2026-07-01/02)

> Audit complet de la boucle ReAct + implémentation. 15 commits (`5e979b5`..`59e6167`), workspace tests verts. Détails : messages de commit + notes de session.

### ✅ Livré (build vert, testé)
- **Moteur ReAct corrigé (12 fixes)** : transcript tool-calling porté par l'historique (`Message.appels`/`appel_id`), ancre mission épinglée + troncature low-watermark calibrée par la jauge, checkpoint atomique + nudges filtrés + vigie persistée, timeout par outil, cap des observations (30k head+tail), annulation coopérative, budget tokens (`FinDeVol::Budget`), boucle d'appels bloqués stoppée, `mission_accomplie`/`clarify` honorés seuls, compaction LLM par défaut + consolidation auto-portante, escalade superviseur dédiée.
- **Tool-calling NATIF par provider** : pré-passe de corrélation (appel natif ⇔ résultat présent, anti-400), OpenAI-compat `tool_calls`, Anthropic `tool_use`/`tool_result` + images natives, fallback texte pour modèles locaux. Provider **`llamacpp`** (base défaut `:8001`).
- **Deep-research SOTA** : 3 canaux de décision de mode (mots-clés élargis / **outil `research_mode` auto-déclaré intercepté par le moteur** / arg `mode` du plan), `PROTOCOLE_EXPLORATION` (fan-out `delegate` parallèle, gardienne, 403=contournement), scouts avec prompt purgé + nudge adapté au contexte (`delegation_disponible`), autonomie durcie (jamais renvoyer l'utilisateur chercher).
- **Harness d'évals** (`cargo run -p laruche-evals`) : missions fixes vs vrai moteur, checks durs (fin/mode/web/fan-out/démission/fichier), juge LLM optionnel, JSONL + baseline avec régressions signalées. `RapportMission` riche exposé par le pont.
- **Outils machines de guerre** : `web_fetch` paginé + `include_links` + PDF via jina ; `web_deep_search` parallèle (+fix panic UTF-8) ; `file_search` glob + grep contenu + bruit ignoré ; `file_list` arbre trié ; `file_write` atomique.
- **Mémoire - exploitation complète** : embedder universel TOUJOURS actif (`HttpEmbedder` Ollama/llama.cpp auto-détecté, disjoncteur, backfill au boot, `lancer_embeddings.bat` avec téléchargement auto), search v2 (fusion sémantique+FTS, **decay de priorité** = importance persistée + usage hebbien + fraîcheur, jamais de suppression), **supersede à l'écriture** (dedup exact + quasi-doublons à l'échelle du domaine, seuils **calibrés sur mesures réelles** 0.83/0.85), rappel JIT `Source::rappeler` câblé (scouts + reprises + stagnation), mémoire épisodique (`episodes.*`), recall sans bruit skills (`capacities.*`/`system.*` exclus, items plafonnés 600c), **Consolider récursif** (sous-arbre entier).
- **Fixes transverses** : auth (secret cookie sauvé au boot = fini le re-login à chaque lancement, 401 diagnostiqués), onboarding embeddings = vraie sonde (était un stub `done:false`).

### 🔜 En attente (backlog priorisé)
- [x] **Mystère qwen3:8b RÉSOLU** (2026-07-02) : c'était l'intégration tools d'**Ollama**, pas le harness. Via `llamacpp` + gemma-4-e4b/12b, tool_calls natifs OK. Fix usage OpenAI-compat (`b01700a`) : chunk `include_usage` dédié qui était droppé (marche sur Anthropic/DeepSeek ; le `tokens=0` restant est spécifique au streaming llama.cpp).
- [x] **Correctifs deep-research post-évals** (`a1b9b01`) : scouts bornés (Éclaireuse 12 passes/min_web 3, était 30/6) + **cap dur 4 scouts** (`MAX_DELEGATIONS`, le modèle en lançait 8-12) + protocole "4 angles MAX + vérification fraîche même si mémoire a la réponse" + mots-clés multilingues ES/IT/PT/DE + prompt `research_mode` "in any language".
- [x] **Charte curateur durcie** (`d423ae9`) : ne capture plus les impasses de diagnostic ni les méta-skills sur les internes du système (les 2 chemins : PROMPT_CURATEUR + extracteur brain). Cause : skills-poubelle `diagnose_system_discrepancy`/`task_source_diagnoser` vues en attente.
- [~] **Baseline d'évals NON figée** : le 12b local est trop lent+stochastique pour un run deep propre (timeouts, variance run à run) ; DeepSeek gratuit rate-limite sous le volume du fan-out. Le harness+moteur sont prouvés corrects (contrôles verts sur 3 backends, fan-out parfait de 8 scouts au run 2). → figer quand un backend fiable sera dispo. Détails : `COMPTE_RENDU.md`.
- [x] **Check LLM de contradiction au write** (fait `656111a`) : trait `Arbitre`+`VerdictArbitre` (inversion de dépendance), bande d'ambiguïté 0.62-0.83 même domaine → `ArbitreLLM` (modèle aux, REPLACE/DISTINCT) → supersede ; échec = Distinct (jamais destructif), opt-out `LARUCHE_MEMOIRE_ARBITRE=0`. Test de régression déterministe.
- [x] **dream→reine_queue (fait 2026-07-02)** : les suggestions `duplicate` du dream (6h + bouton manuel) deviennent des propositions `MemoireHygiene` dans la file LaReine - classe Critique (jamais auto-appliqué), approbation = dédup soft-delete réversible du nœud, garde anti-flood `deja_en_file` testée. Reste : `overloaded`/`orphan` sont consultatives (pas d'apply mécanique → mission agent, Tier 3).
- [ ] **Hebbien niveau 2** : ne renforcer que les rappels réellement UTILISÉS dans la réponse (mesurable par le juge des évals).
- [ ] **OKF + git** : auto-commit du bundle exporté = mémoire time-travel (diff/rollback) puis **fédération mesh des faits** entre nœuds (provenance) - session dédiée.
- [x] **FTS moins permissive** (fait `04514c3`) : requête riche (≥3 tokens) exige ≥2 tokens matchés ; requêtes courtes gardent le OR permissif.
- [x] **Évals mémoire** (fait `04514c3`) : scénarios « bruit du recall » et « supersede inter-nœuds » protégés par tests de régression déterministes dans `laruche-memoire` (mieux que missions LLM non-déterministes).
- [x] **Audit des checks d'onboarding restants FAIT (2026-07-02)** : STT/TTS ne regardaient que les capability flags mesh (mensongers : 'not found' avec les services locaux up, 'available' pour un nœud mort) → vraie sonde `GET /health` sur les URLs résolues comme au runtime (helper partagé `resolve_voice_urls`/`voice_service_up`, réutilisé par le websocket voix). TLS : vérifie que cert/key sont lisibles (sinon le serveur retombe en HTTP → statut error) + cas `LARUCHE_HTTPS=1` auto-signé couvert.
- [~] **Référentiel multi-provider (doc Sonnet)** : ✅ **validation client-side des args vs JSON Schema faite (2026-07-02)** - gate unique dans `AbeilleRegistry::executer` (chat, scouts, bridge tool_call, scripts) : `required` + types top-level, avec coercitions des erreurs classiques (nombre/booléen en string, float entier, objet JSON embarqué en string) ; mismatch = observation corrective, le modèle réessaie. ✅ parser tolérant forme attributs (2026-07-02). Reste : `tool_choice`/`parallel_tool_calls` par provider, parser pythonic complet, tests de non-régression par modèle (jeu de tool calls fixes).

## 👑 LaReine - superviseur de la ruche (nouveau chantier)

> Mode activable qui place une **Reine** au-dessus des butineuses : elle juge la pertinence et la méthodologie d'une réponse (ou d'un artefact), renvoie au LLM des instructions correctives jusqu'à atteindre l'objectif, et peut gouverner l'auto-modification de LaRuche. Principe directeur : **le curateur propose, la Reine dispose.**
>
> Règles de build : tout en **anglais par défaut** (code, chaînes, commentaires propres non « LLM-like »), **chaînes UI variabilisées + i18n** (`lang/strings.json`, `t()`), **aucun em dash**, termes de marque FR conservés. La Reine n'est PAS la vigie : la **vigie garde la boucle de l'intérieur** (anti-loop/budget), la **Reine supervise le résultat de l'extérieur** (pertinence, méthodo, objectif).

> ### ✅ État (livré, build vert, workspace tests verts)
> - **Tier 1 COMPLET** : config + UI (couronne header / slider = nb de re-runs / mode / provider dropdown), charte complète embarquée + éditable en Mémoire (`system.prompt_reine`), juge robuste (raisonnement borné via champ `ANALYSIS`, `/no_think` + strip `<think>`, format ligne `KEY: value` + repli JSON, résolution du profil actif), **re-run agentique RÉEL par LaRuche visible en direct** (streaming, animation « LaRuche refait le travail »), verdict final explicite + analyse toujours affichée, attribution correcte (abeille = travail, couronne = jugement), persistance de la version refaite, prompt de réécriture « bonne foi » (pas de soumission aveugle).
> - **Tier 2 COMPLET** : file de propositions persistée (`laruche-reine-queue.json`), gate du curateur, **disposition risk-tier** (gate+Off/Humaine = tout en file ; Auto/Hybride = la Reine auto-approuve le sûr, met en file le risqué), **backlog découplé** (désactiver le gate ne le perd jamais, invariant testé), **péremption** 14j. Panneau **déplacé dans l'onglet Mémoire** (cartes risque/cible/aperçu + approuver/rejeter/tout-appliquer-sûr) + **badge type iOS** (compteur en attente, pollé 20s, desktop + mobile). Settings ne garde que le toggle gate + un pointeur.
> - **Tier 3 COMPLET (anti-blocage)** : superviseur pur testé (`cap::reine::superviser`, +3 tests) branché dans la boucle butinage. Mesure l'avancement du plan ; si stagnation N passes -> consigne de recentrage, puis escalade après K relances. Opt-in (`Reglages.supervision`, OFF par défaut), plombé bout en bout (toggle UI -> ws_chat -> butinage_pont -> cycle).
> - **Extras LaReine livrés** : re-runs **illimités** (sentinel 255), **fenêtre de contexte réglable** (la Reine voit N tours précédents), **juge robuste** (clés décorées + verdict inféré des scores), **avatars distincts** (couronne LaReine vs abeille LaRuche).
> - **Provider/robustesse** : 429 « solde vide » -> fatal (stop net, plus de spin), **bouton Test** par provider (affiche l'erreur exacte), **prompt caching Anthropic** + fix system top-level (#48 partie 1, à valider sur clé live), **section Avancé** repliable dans Settings (#43).
> - **Reste** : `reasoning_effort` (#48 partie 2, signature à threader), garde-fous destructifs avancés (soft-delete + confirm seuil), i18n par canal live (#45/#57), mesh 2 noeuds (#46, matériel). **Voix de la Reine : livrée** (cf. section Mode Reine ambiant + revue audit).

### Architecture - décisions actées
- **Pas de nouveau moteur ReAct pour juger.** La revue est un **hook synchrone borné** dans le pipeline chat/butinage (`brain.rs`), 2-3 appels outils max (lire registre skills/tools, lire mémoire). Juger est court.
- **Boucle superviseur dédiée et optionnelle** (Tier 3 seulement), **OFF par défaut** : se réveille, audite, agit via les abeilles existantes, vérifie, dort. Réutilise l'infra cron/missions, pas un moteur neuf.
- **Cœur pur et testé** dans `laruche-butinage/src/cap/reine.rs` (à côté de vigie/boussole) : `ModeReine`, `ConfigReine`, `Verdict`, `Scorecard`, contrôleur de tours borné. La décision est pure ; l'appel LLM du juge vit dans la couche d'intégration.

### Skill « charte » de la Reine (expertise béton)
- [x] **Charte LaReine** (skill hand-écrit, EN) : livrée dans `laruche/skills/lareine-charte/SKILL.md`, embarquée au build (`include_str!` dans `reine_live.rs`) + éditable en Mémoire (`system.prompt_reine`).
- [x] **Introspection live (1re tranche, 2026-07-02)** : le juge reçoit un bloc « atelier » live - outils disponibles (registre réel) + trace des appels du brouillon jugé (échecs marqués) → le score METHODOLOGY se fonde sur les faits. Reste : schéma mémoire + roadmap dans le bloc.
- [ ] **Maintenue par le curateur** : la charte se met à jour quand LaRuche évolue (processus continu).

### UI (câblé, livré avec le Tier 1)
- [x] **Bouton couronne activable** à côté du dropdown de choix LLM (header chat).
- [x] **Slider nb max de revues** : 0 = OFF, 1 à 10 = tours max, + case « illimité » (sentinel 255, opt-in explicite).
- [x] **Sélecteur de mode** : Off / Auto / Hybride / Humaine (Settings > LaReine).
- [x] **Provider/modèle de la Reine** dans Settings (dropdown des profils provider, défaut = même que le chat).
- [x] **LaReine visible dans le chat** : avatar couronne distinct, verdict + analyse affichés, re-run streamé en direct. Mode Humaine = l'utilisateur prend son siège.
- [x] **Voix de la Reine** (Kokoro/Voicebox TTS) : système voix complet livré (TTS streamé, appel plein écran, barge-in, mot d'éveil, Telegram bidirectionnel) - cf. section Mode Reine ambiant.

### Tiers d'autorité (activables séparément)
- [x] **Tier 1 - Revue de réponse** (cas chat). Livré (cf. État) : juge + re-run agentique réel streamé, appliqué au chat ET aux missions (`revue_mission`).
- [x] **Tier 2 - Revue d'artefacts via file de propositions (façon pull requests)** : livré (cf. État) - file durable `reine_file.rs` (statut `Obsolete` + `base_version` = détection d'obsolescence, péremption 14j, backlog découplé invariant testé, risk-tier). Décisions actées (toutes implémentées) :
  - **File découplée de la Reine** : c'est un store de premier rang, pas la propriété du toggle. **Désactiver la Reine ne perd jamais le backlog** : les propositions en attente restent gelées et visibles (Mémoire > En attente), actionnables à la main ; seules les **nouvelles** écritures repassent en direct. Notice non bloquante à la désactivation. (invariant testé : `transition_desactivation`).
  - **Risk-tier** pour ne pas noyer l'utilisateur : auto-approuve le **sûr** (fait nouveau non contradictoire) ; **toujours** mettre en file / escalader le **critique** (suppression, écrasement, contradiction) - jamais d'auto-apply destructif.
  - **Détection d'obsolescence** : une proposition versionnée contre sa cible passe en **Obsolète** (« needs rebase ») si la cible a bougé depuis, au lieu d'écraser à l'aveugle.
  - **Péremption** : TTL propre à la file, indépendant de la Reine, pour éviter un backlog qui pourrit.
  - Hook existant : `background_review.rs` (`memory_write`, `skill_propose`) à rediriger de l'auto-apply vers la file. Cœur pur fait : `laruche-essaim/src/reine_file.rs` (10 tests).
- [x] **Tier 3 - Orchestration proactive** (boucle superviseur OFF par défaut) : livré en anti-blocage (`cap::reine::superviser` : stagnation → recentrage → escalade, opt-in). L'orchestration « elle initie des missions de rangement » reste couverte par dream→reine_queue (backlog moteur).

### Garde-fous (non négociables)
- [ ] **Destructif = soft-delete réversible + log d'audit**, jamais de hard delete. memoire.db jamais touché à la légère. (= « garde-fous destructifs avancés » du reste LaReine)
- [x] **Confirmation humaine** au-dessus d'un seuil de risque : via risk-tier - le critique (suppression, écrasement, contradiction) va TOUJOURS en file (gate humain), jamais d'auto-apply destructif.
- [x] **La Reine ne se relit pas elle-même** (anti-récursion) : structurel - elle juge le travail de l'abeille, jamais son propre verdict ; re-runs bornés ; mode Humaine = l'utilisateur la surcharge.
- [x] **Anti-régression (fait 2026-07-02)** : le meilleur brouillon (score global) est conservé et expédié quand le budget (tours ou temps cumulé `LARUCHE_REINE_BUDGET_SECS`) force la sortie ; `Reine::regression()` enfin câblée.
- [x] **Borné en coût** : slider de tours (illimité = opt-in explicite) + supervision Tier 3 opt-in OFF par défaut.

### Bonus (forte valeur)
- [ ] **Gardienne de marque et de style** : applique automatiquement le lexique FR de marque, l'anglais dans le code, zéro em dash, ton pro, sur chaque artefact généré. Automatise ce qui a été fait à la main.
- [~] **Scorecard** : type + `juger()` + **journal JSONL `evals/reine-scorecards.jsonl` (2026-07-02)** + verdicts persistés en session (thought reine/verdict, restaurés au reload). Reste : le tableau de bord d'éval agrégé (la donnée existe maintenant).
- [ ] **Étoile polaire** : la Reine tient l'objectif d'une conversation/mission et détecte la dérive. Partiellement couvert par la supervision Tier 3 (stagnation de plan) ; le cap sur TOUTE l'interaction reste.

### Ordre de construction (déroulé, historique)
1. ~~Cœur pur `cap/reine.rs` + charte skill + clés i18n~~ **FAIT**
2. ~~Hook de revue synchrone (Tier 1) + appel LLM du juge~~ **FAIT**
3. ~~Câblage UI + endpoint `reine_api.rs`~~ **FAIT**
4. ~~Tier 2 : le juge sur la création d'artefacts (file)~~ **FAIT**
5. Tier 3 : ~~boucle superviseur optionnelle~~ **FAIT** ; garde-fous destructifs avancés (soft-delete + anti-régression) **RESTE**.

## ✅ Revue projet exhaustive & corrections (audit complet)

> Revue de tout le code (6 agents en parallèle sur des zones disjointes + relecture manuelle Opus du cœur `providers.rs`/`brain.rs`/sécurité), puis **correction de ~45 bugs/manques en 17 commits thématiques**. Workspace vert, `cargo test` **191 verts**, 0 em dash. Hygiène secrets/git vérifiée **clean** (aucun secret commité).

- [x] **Sécurité backend** : `check_admin` sur tous les endpoints mutables non gardés (channels start/stop, missions create/run/update/delete/decompose, crons run/update/delete, profiles active/visibility/use, `/auth/approve`) · GET `/api/config/channels` **masque les `bot_token`** sauf admin (fuite) · bulk-sync ne donne le `cookie_secret` qu'à un pair **mesh-signé** (plus d'IP seule) · `/infer` gaté (admin ou pair mesh) · `/mcp` exige le token hors localhost (était open).
- [x] **Échappement HTML frontend** : `esc()` sur données mesh/MCP/doctor injectées en `innerHTML` · `encodeURIComponent` sur URLs · checkbox transparence réparée.
- [x] **Crash / perte de données** : SQL `LIKE` de sous-arbres **échappé + `ESCAPE`** (le `_` snake_case détruisait des sous-arbres non liés) · buffer SSE **Ollama** (perte du chunk final `done`/usage/tool_calls si coupé) · `escale` n'efface plus l'historique sur 0 fait extrait · cert TLS illisible → **fallback HTTP** au lieu de panic · slice Telegram en chars (panic multi-octets) · `addToolMessage` indéfini retiré · underflow `swarm.rs` (saturating + garde `LayerRange::count`).
- [x] **Bugs LLM** : tool_calls **triés par index** (plus par id aléatoire) · buffer **UTF-8** openai/anthropic/codex (caractères coupés en frontière de chunk) · `stream_options.include_usage` (usage OpenAI réel) · `codex_headers` anti-403 · `audio/mpeg`→mp3 · **substitution `@@secret`** de la clé (moteur legacy) · `ToolCallRaw.arguments` `serde(default)` · `error_classifier` branche morte → `Fatal` · **tool_use natif Anthropic** parsé (Claude peut appeler des outils) · **pool credentials piloté dans le moteur butinage** (clé dispo + load-balance).
- [x] **JS robustesse** : `stopAllTts` au stop/switch de conversation · null-guards (checkVoiceStatus, enroll, sharding, missions) · `sendAudio` cappé · `setRunning` au steer · `importOkf` bon id · `Chat.current` mort retiré.
- [x] **Voix Python + tui + cleanup** : codes d'erreur HTTP corrects (plus de 200+JSON joué comme audio) · cap longueur texte · `/health` rapporte le backend actif · pyttsx3 lock module · uptime « 5ss » · vars/no-ops mortes.
- [ ] **Restes assumés** (arbitrages, pas oublis) : rotation de clé **en cours de stream** (mid-call rate-limit) · CORS `*`/bind `0.0.0.0` des services voix (voulu LAN, durcir via token mesh) · format `tool_result` natif Anthropic sur tours multiples (à valider runtime) · code mort JS résiduel inoffensif.

## ✅ Grand batch UX / mobile / auth / PWA / TUI (récent)

> ~16 demandes regroupées, livrées en lots. Workspace vert, 0 em dash, code EN / marque FR. **À tester app en marche** (auth sensible : pose un mot de passe avant de relancer pour éviter le lockout).

- [x] **Voix (avant ce lot)** : `/voice` Telegram exclusif (ON=voix seule / OFF=texte seul) · STT externe optionnel (service vs LLM) · vocaux longs Telegram **découpés** (≤8 notes, frontières de phrase) · **vitesse TTS** réglable · **voix Kokoro par canal** · **barge-in** + mot d'éveil · backend **Voicebox** (voix clonée) + sélecteur de backend · **stream inverse LLM→TTS**.
- [x] **Auth durcie + comptes cloisonnés** : bug d'enroll (un compte créé à **chaque** login) **corrigé** (password ≥6 + rejet doublon de nom) · **mot de passe + TOTP (2FA)** RFC 6238 maison (pas de dépendance fetchée, vecteurs RFC 4226 verts) · **migration de dé-duplication** au démarrage (collapse les doublons par nom → garde le plus récent, hérite admin, ≥1 admin garanti) → **répare le compte admin**.
- [x] **Profil + admin** : section **Profil** (photo avatar redimensionnée 128px, nom, mot de passe, fiche « ce que LaRuche sait », 2FA) · section **Admin** (lister/supprimer/changer rôle, anti-auto-suppression) · **avatar partout** où il y a une miniature (badge, **feed**, **modal profil**, **messages chat** - était codé en dur sur `'U'`).
- [x] **Mobile / cross-platform** : barre de statut **safe-area** (plus tronquée par les coins arrondis) · inputs `font-size:16px` (anti-zoom iOS) · clavier virtuel iOS géré · **collapse sidebar** desktop + fermeture mobile au clic extérieur · **PWA** (manifest + service worker network-first + icône honeycomb, servis par le node) → **« ajouter à l'écran d'accueil »** iPhone/Android, standalone, shell offline.
- [x] **Refonte UX/UI Settings** : nav **verticale** par sections (9), recherche, sections `adminOnly` filtrées, `/infer` **retiré** (route + types + zone Dashboard morts supprimés).
- [x] **Missions × LaReine** : la revue Tier 1 ne s'appliquait qu'au chat → **`revue_mission`** (juge + refait si insuffisant + livre la version approuvée) branchée sur les itérations de mission.
- [x] **Audit logging** : helper **`log_activite`** → les événements **sécurité** (login OK/échec, nouveau compte, suppression/rôle admin) remontent maintenant dans le **volet Audit** du Dashboard (avant : CLI seulement).
- [x] **Refonte TUI (façon Claude Code)** : moniteur mono-log → **5 vues à onglets** (Overview / Logs / Activity / Sessions / Swarm) + **palette de commandes slash** (`/` → complétion live `/overview /logs /activity /sessions /swarm /clear /help /quit`, Tab complète, Entrée exécute) · `Tab`/`1-5` navigation · `?` aide.

## ✅ Batch UX & fiabilité chat/boot/outils (2026-07-02 après-midi)
- [x] **Boot 6s au lieu de plusieurs minutes** : sync skills disque→SQL INCRÉMENTALE (fini le delete+rewrite des 71 skills à chaque boot, qui déclenchait embed+arbitre LLM par skill) + sync et chargement MCP en arrière-plan (computer-use python = 8s d'imports mesurés) + chronos de phase `boot: ... t_ms` permanents.
- [x] **Sessions chat fiables** : miroir live (F5 en plein run = reprise à l'identique, plan/outils/résultats), plan persisté pour les vieilles sessions, titre immédiat (fini « Sans titre » pendant un run), pastille onglet Chat (desktop+mobile) + par conversation.
- [x] **Roue de chargement au boot** (plus de page login pendant le démarrage, 401/403 seuls mènent au login) + launcher `.bat` qui n'ouvre le navigateur que quand le node répond.
- [x] **`tool_call`/`tool_search`/`run_script` sur le registre principal LIVE** (étaient sur le sub_registry builtins-only : 'Unknown tool: cron_list' + hallucination) ; scouts inchangés (sous-registre réduit, pas de fan-out récursif).
- [x] **Parser tool-call tolérant** : forme attributs des modèles locaux (`<tool_call name="x" arguments={...}>`) parsée + masquée au streaming ; chaîne exacte observée gelée en test.
- [x] **Outils missions** : `mission_list`/`mission_create` (approbation)/`mission_delete` + cadences de missions visibles dans `cron_list` et `/api/cron` (kind=mission) = une réponse vraie à « qu'est-ce qui est planifié ? ». Hint de capacités mis à jour.
- [x] **Catalogue skills complet rétabli** (nom + description ~2k tokens, toute requête non triviale) - il était filtré par tokens de la requête via le couplage au Lever 2 ; les corps restent lazy (`skill_view`).
- [x] **Éval deep terminée** (l'autre session) → l'item « figer la baseline » (`--save-baseline`) est actionnable.

## ✅ Fait récemment
- [x] **Rangement racine** : `README.md` à jour + archivage des docs/scripts/lanceurs (`docs/_archive/`, `_archive/`, `laruche/_archive/`) + `.gitignore` durci.
- [x] **Split `spa.html`** → `app.css` + `app.js` + routes node (`/app.css`, `/app.js`).
- [x] **Split `app.js`** → 9 modules dans `templates/js/` (concat compile-time, un seul `/app.js` servi).
- [x] **i18n UI FR/EN** : infra `LaRuche.i18n` (`t()` + toggle header + dico par module) + **760 chaînes migrées** (workflow 8 agents). Marque non traduite.
- [x] **Localisation EN** : prompts système, abeilles (318 chaînes), plugins, **71 skills** (de-third-party + lean + `tools:` liés).
- [x] **Optimisations contexte** : champ `tools:` natif respecte la sélection · catalogue outils dé-dupliqué · catalogue **skills dynamique** · pas de corps de skill sur smalltalk · **seuil contexte configurable** (Settings).
- [x] **Sonde n_ctx** → `context_max_tokens` auto · commandes Telegram (`/help /status /crons /delcron`) · `@@secret` autocomplete · fixes crons (anti-spam/runaway/réplication).

## 🌍 Localisation EN du code (commentaires + chaînes) - FAIT
But : **zéro français dans le code** hors termes de marque LaRuche et fichier i18n. Commentaires pro (pas de tournure « LLM »), **aucun em dash** nulle part. `abeille → tool` (une abeille = un agent qui utilise des tools). Identifiants gardés tels quels (règle utilisateur). Glossaire de marque FR : `LaRuche, ruche, essaim, Miel, butinage, nectar, Source, escale, éclaireuse, curateur, vigie, boussole, jauge, carnet, récolte`.
- [x] **Webapp** : 9 modules JS + `app.css` + `spa.html`. Commentaires + texte → EN, onglet « Abeilles » → **Tools**. Contrats API gardés (`origin`, `visibility 'prive'`).
- [x] **Rust** : ~118 fichiers, crate par crate (gate `cargo build` + `cargo test --workspace`, 33 cibles vertes). Chaînes traduites prudemment (humain seulement ; protocole/match/test/intent-detection laissés). 5 assertions de test recalées sur les chaînes traduites.
- [x] **Revue par fichier** : vérificateur de squelette de code (94/111 fichiers prouvés sans changement de code ; 17 inspectés à la main). A attrapé une **sur-traduction systémique des termes de marque** (Éclaireuse→Scout, Curateur→Curator, Ruche→Hive, Récolte→Harvest) → **restaurés** en FR dans les chaînes/commentaires (identifiants gardés).
- [x] **Sentinels Rust↔JS** traduits **conjointement** : verbes de feed (`a demandé/a répondu`→`asked/replied` + matcher capabilities.js), `Synthèse/Erreur LaRuche`→`LaRuche summary/error` + matcher memory.js, `réponse finale`→`final response`, séparateur third-party `" - "` (au lieu de l'em dash).
- [x] **Sweep em dash = 0** sur tout le projet (3267 retirés : code+docs, puis 3232 dans `skills/**/*.md`).
- [x] **README** réécrit pro/concis/anglais/non-LLM.

## 🌐 i18n complet via fichiers de langue (web fait, Rust à venir)
- [x] **Couverture web 100 %** : ~220 libellés UI hardcodés en anglais routés par `t()` avec `{fr,en}`.
- [x] **Fichiers de langue externes** : `laruche/lang/en.json` + `fr.json` (1016 clés), source de vérité. Le node injecte la langue active (cookie `laruche_lang`) en `window.__I18N__` avant `app.js` ; `t()` fait un lookup plat. Servis sur `/lang/<code>.json`. **Ajouter une langue = déposer `lang/<code>.json` + 1 match arm** (voir `lang/README.md`). Dicos inline gardés en fallback runtime.
- [x] **Phase 2a - moteur i18n Rust** : module `laruche_essaim::i18n` lisant les mêmes `lang/*.json` (`t(key,lang,vars)`, `normalize_lang`, fallback clé). 4 tests verts. Logs/prompts restent EN.
- [ ] **Phase 2b - intégration canaux** (reste) : (1) store langue **par canal/utilisateur** dans `AppState` + persistance `laruche-state.json` ; (2) commande `/lang fr|en` (Telegram d'abord, près de `/help` main.rs:7416) + auto-détection ; (3) le chat web envoie sa langue (cookie déjà posé) ; (4) router les chaînes **visibles utilisateur** via `i18n::t` (réponses commandes, messages canaux, erreurs montrées). Ajouter les clés Rust dans `lang/*.json`. **À faire avec test canal live** (Telegram/Discord), touche le messaging.
- [ ] **Polish FR** : certaines valeurs `fr.json` ont perdu leurs accents (« a la demande » -> « à la demande ») - passe de qualité.
- [ ] *Note* : chip mémoire `chat.js` dépend d'un status à compteur que le Rust n'émet plus (cosmétique).

## 🔧 Reste / near-term (chantiers actifs)
Suivis dans la liste de tâches du dépôt.
- [x] **i18n UI - 2ᵉ passe exhaustive** : audit ligne-à-ligne des 9 modules (parser dédié comments/dico/identifiants/CSS exclus). **16 dernières chaînes affichées** migrées (settings ×12, chat ×4). Reste = skips documentés (termes de marque `Abeille`, sentinels backend, identifiants).
- [x] **i18n sentinels runtime Rust↔JS** : `Synthèse/Erreur LaRuche`, verbes de feed `a demandé/a répondu`, `réponse finale` traduits des deux côtés ensemble (voir section Localisation).
- [x] **Affinage injection skill body** : top-1 (lazy par défaut) + gate `requete_triviale`. 141 tests verts.
- [x] **Provider/modèle par canal** (Settings > Channels) + log diagnostic tool-call. (cf. section Canaux)
- [ ] Décider du sort de la page **Sessions** orpheline (`laruche/_archive/test_script.js`).
- [ ] `/lang fr|en` messages Telegram directs - dépend de la Phase 2b (rien ne lit encore la langue côté Rust).

## ⏸️ Reste - nécessite une condition que je n'ai pas en l'état (honnête)
> Tout le reste est soit un refacto énorme à faire **app fermée**, soit du **comportemental** qui doit être validé **runtime** (API/canaux réels) - le faire à l'aveugle risquerait l'app qui tourne. Plan précis pour chacun :
- [x] **Split `main.rs`** : **FAIT** (2026-07-02, 7 commits gatés build vert). main.rs : 3559 → ~1200 lignes (main() + bootstrap uniquement). Extraits : `state.rs` (AppState + types d'état + persistance, tout en `pub(crate)`), `helpers.rs` (helpers partagés inter-modules), `router.rs` (les 179 routes + CORS + `auth_guard`, strictement identiques), `background.rs` (les 13 jobs de fond en fonctions `spawn_*`), DTOs swarm → `swarm_api.rs`, helpers feed → `feed_api.rs`. Déplacement pur (zéro changement de logique), 330 tests workspace verts, re-export glob à la racine donc aucun module `*_api` existant modifié.
- [ ] **Settings « Avancé »** + migrer params tuning : reorg **frontend** (les params existent déjà dans General via `/api/config/runtime`). Nécessite **validation visuelle** dans le navigateur.
- [ ] **Phase 2b - router les strings Rust visibles** via `i18n::t` + store langue/canal + `/lang` : nécessite **test canal live** (Telegram/Discord).
- [ ] **`cache_control` (Anthropic)** + **`reasoning_effort`** : changent le **corps des requêtes provider** - à valider contre les **vraies API**.
- [ ] **Compression préflight** : comportemental, à **régler runtime** (seuils).
- [~] **Vérifs runtime** watchers loop + `credential_pool` retry : le pool est désormais **piloté dans le moteur butinage** (sélection de clé dispo + load-balance, cf. audit) ; la **rotation en cours de stream** sur 429 réel reste à valider app en marche.

## ✅ Fait cette session (dette)
- [x] **`laruche-suggestions`** orphelin → **supprimé** (stub 71 lignes jamais câblé).
- [x] **`cargo test --workspace`** : **33 cibles vertes**.

## ⚫ Bloqué (matériel)
- [ ] **Validation mesh A** (fédération des skills) - nécessite **2 nœuds réels**.

## 🔭 Vision / long-terme (Miel & essaim)

### 👑🎙️ Mode Reine ambiant (Jarvis local, voix plein écran)
> Aboutissement de LaReine + la voix : on parle à la Reine en continu, plein écran, et elle supervise et pilote l'essaim. **Local-first** (Kokoro + Whisper offline), donc différenciant face aux assistants cloud. La Reine est l'hôte naturel : elle voit déjà tout ce que LaRuche indexe (mémoire, registres, carnet, feed, mesh), elle agit en ton nom **mais gatée** par la file de propositions, et elle se relit avant de parler (anti-hallucination).
> ### ✅ Voix livrée (cette session) - cf. mémoire `laruche-voix-reine`
- [x] **UI ambiante plein écran** : mode « Appeler LaRuche » (bouton 📞, overlay Jarvis, sous-titre des paroles, conversation continue).
- [x] **Boucle voix temps réel** : **stream inverse LLM→TTS** (elle parle dès la 1re phrase finie, sans attendre la fin), file de lecture avec prefetch, repli voix navigateur. Idem Telegram (vocaux découpés en plusieurs notes).
- [x] **Provider STT/TTS pluggable** : backends `kokoro` (défaut) / `edge-tts` / `pyttsx3` / **`voicebox`** (voicebox.sh = voix clonée), **sélecteur dans Settings** (backend + vitesse + voix), réglages persistés (`laruche-voice.json`). STT : service externe OU le modèle lui-même (Gemma) selon le toggle.
- [x] **Barge-in** (la couper en parlant, filtre anti-écho) + **mot d'éveil « LaRuche »** (Web Speech continu, contexte sécurisé requis).
- [ ] **Session ambiante persistante** (contexte continu, pas tour-par-tour) - reste.
- [ ] **Omniscience = sources branchées** : agenda, mails, fichiers, Home Assistant via client MCP + capability nodes (`capability:rag`, `capability:audio`). Se construit source par source.

*POCs de nœuds spécialisés par capacité (le mesh annonce déjà les `capability:*`) :*
- [ ] `capability:llm` - nœud texte standard (Mistral/Llama)
- [ ] `capability:vlm` - Vision-Language (LLaVA, Qwen-VL) : analyse d'image via le réseau
- [ ] `capability:vla` - Vision-Language-Action : piloter bras robotique / drone
- [ ] `capability:rag` - nœud indexant des documents locaux, Q&A
- [ ] `capability:audio` - Whisper/Bark (STT/TTS), commandes vocales réseau
- [ ] `capability:image` & `capability:embed` - Stable Diffusion / vecteurs à la volée
- [ ] `capability:code` - CodeLlama/DeepSeek-Coder

*Grandes briques :*
- [ ] **Tensor sharding over Ethernet** (intelligence d'essaim)
- [ ] **Résilience LaRuche** : failover, hot-swap, mirroring
- [ ] **Spécification Miel v1.0** (RFC du protocole)
- [ ] **Extension VS Code** (crate `laruche-vscode` existe, en cours)
- [ ] **Plugin Home Assistant**
- [ ] **App mobile** (iOS/Android)
- [ ] **Intégration matérielle NFC**

---
*Historique complet de conception : `docs/_archive/` (LARUCHE_V2.md, MASTER_DEV.md, VISION_INNOVATION.md, ARCHI_BUTINAGE.md…).*
