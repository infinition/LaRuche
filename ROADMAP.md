# LaRuche - Roadmap

> Reste-à-faire **récupéré des anciens docs de conception** (avant archivage dans `docs/_archive/`) + chantiers en cours. Source de vérité du travail restant. Coché = fait.

## 🔒 Audit sécurité & hygiène (2026-07-02) - PRIORITÉ ABSOLUE avant toute feature

> Audit complet du projet (probes ciblés + connaissance du moteur/mémoire/outils déjà lus). Le cœur agentique est solide ; la **couche d'exposition réseau était le point faible critique**. Cluster critique + XSS + mesh + vendorisation **CORRIGÉS** (commits `e610365`, `9b1ccca`, `ee44d11`, 2026-07-02). Hygiène repo vérifiée propre (aucun secret committé, aucun fichier runtime tracké). Reste : dette (main.rs monolithique, brain.rs) + audit profond shell/crates (non bloquant, gate d'approbation = vrai contrôle).

### ✅ CRITIQUE - Cluster exposition réseau (CORRIGÉ, commit e610365 + ee44d11)
- [x] **Bind `127.0.0.1` par défaut** (était `0.0.0.0`) : exposition LAN = opt-in explicite `LARUCHE_BIND_LAN=1`, loggué en warn.
- [x] **CORS restreint** : prédicat n'autorisant que les origines `localhost`/`127.0.0.1`/`[::1]` (était `AllowOrigin::any()`).
- [x] **Middleware `auth_guard` global** : exige le cookie sur les requêtes MUTANTES (POST/PUT/DELETE/PATCH) vers `/api/*`, uniquement si un compte à mot de passe existe (fresh install + onboarding restent ouverts) ; GET passe (lectures UI) ; allowlist auth flow + sync interne. Ferme le scénario site-piégé/LAN → mutation.
- [x] **Auto-sync mémoire mesh en opt-in** `LARUCHE_MESH_MEMORY_SYNC=1` (était auto toutes les 5 min sans vérif de pair). Faits importés provenance-taggés + traités en REFERENCE DATA. `import_changes` POST couvert par `auth_guard`.

### ✅ CRITIQUE - XSS chat (CORRIGÉ, commit 9b1ccca)
- [x] **`LaRuche.Utils.safeMarkdown` = `marked.parse` + `DOMPurify.sanitize`** sur les 2 rendus innerHTML du chat (streaming + restauration). Un `<img onerror>` ramené par `web_fetch` ne s'exécute plus dans l'UI.

### ✅ MAJEUR (CORRIGÉ, commit 9b1ccca)
- [x] **marked + DOMPurify + highlight.js vendorisés** dans `templates/vendor/`, servis via `/vendor/:name` (`web::vendor_js`), thème hljs inline dans app.css, `sw.js` v2 cache les vendors. Fin des `<script>` CDN (offline réel, zéro supply-chain).
- [x] **Onboarding embeddings = vraie sonde** (déjà fait `0943092`) ; checks voix/Chrome vérifiés RÉELS (pas des stubs) ; "warning" STT/TTS/TLS = état honnête (services non lancés).
- [ ] **Config d'agents (hors projet)** : `~/.claude.json` pointe le modèle des sous-agents Claude Code sur `deepseek-v4-flash` indisponible → fan-out d'audit KO. Réglage APP Claude Code, à corriger côté user (pas dans le repo).

### 🟡 MINEUR / dette (non bloquant)
- [ ] **`main.rs` ingérable** (~3000 lignes). → extraire le routeur + centraliser l'auth (le `auth_guard` global est un premier pas : l'auth n'est plus éparpillée).
- [ ] **Polling front cumulé** : plusieurs `setInterval` tournent même onglet inactif. → pause sur `visibilitychange`.
- [ ] **Blocklist shell contournable** (`shell.rs` : `BLOCKED_PATTERNS` par sous-chaîne, `rm  -rf  /` double-espace passe) : ralentisseur, le VRAI contrôle est le gate d'approbation (`niveau_danger`=NeedsApproval + popup) + timeout. Sandbox OS dure = chantier différé. `secrets.rs` vérifié : valeurs jamais sérialisées/logguées (bon).
- [ ] **À AUDITER en profondeur (fan-out KO)** : `execute_code.rs`, `mcp_client.rs`, micro-crates (`laruche-compaction` vs `escale` ? `laruche-events` vs `feed_journal` ? crates morts ?).

### 🗑️ Dette moteur
- [ ] **Tuer ou promouvoir l'ancien moteur `brain.rs` (~4000 lignes)** : encore le DÉFAUT sans `RUCHE_MOTEUR=butinage`. → décision : butinage par défaut, brain déprécié puis supprimé.

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
- [ ] **Mystère qwen3:8b aux évals** : 0/8, zéro tool call exécuté - trancher bug d'intégration tools Ollama vs modèle (`RUCHE_DEBUG_SSE=1`), re-lancer les évals sur `llamacpp` + gemma-4-12b, puis **figer la baseline** (`--save-baseline`).
- [ ] **Check LLM de contradiction au write** (mémoire) : un UPDATE de fait (4070→5080) mesure 0.71 de similarité - hors de portée des embeddings. Bande 0.60-0.83 → micro-appel `aux_model` (« même fait / mise à jour / sans rapport ») → supersede auto ou proposition. Le chaînon d'une mémoire qui se met à jour seule.
- [ ] **dream→reine_queue** : les suggestions du dream 6h (doublons legacy, surcharges, orphelins) deviennent des propositions actionnables dans la file LaReine (gate humain existant) = auto-nettoyage supervisé.
- [ ] **Hebbien niveau 2** : ne renforcer que les rappels réellement UTILISÉS dans la réponse (mesurable par le juge des évals).
- [ ] **OKF + git** : auto-commit du bundle exporté = mémoire time-travel (diff/rollback) puis **fédération mesh des faits** entre nœuds (provenance) - session dédiée.
- [ ] **FTS moins permissive** : exiger ≥2 tokens matchés (un token commun fait remonter des synthèses hors-sujet, classées derrière mais bruit).
- [ ] **Évals mémoire** : scénarios « bruit du recall » et « supersede » en missions protégées dans `evals/missions.json`.
- [ ] **Audit des checks d'onboarding restants** (STT/TTS, TLS) - suspects d'être des stubs comme l'était le check embeddings.
- [ ] **Référentiel multi-provider (doc Sonnet)** : validation client-side des args vs JSON Schema avant exécution (filet non-négociable modèles locaux), puis `tool_choice`/`parallel_tool_calls` par provider, parser pythonic, tests de non-régression par modèle (jeu de tool calls fixes).

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
- [ ] **Charte LaReine** (skill hand-écrit, EN) : valeurs, lexique de marque, à quoi ressemble une bonne réponse / un bon skill / un bon tool / une bonne entrée mémoire, anti-patterns. C'est sa **rubrique de jugement**, stable.
- [ ] **Introspection live** : droit de LIRE l'état réel au moment de juger (registre skills, registre tools, schéma mémoire, roadmap) plutôt qu'une carte figée qui pourrit.
- [ ] **Maintenue par le curateur** : la charte se met à jour quand LaRuche évolue.

### UI (à câbler après le split de main.rs)
- [ ] **Bouton couronne activable** à côté du dropdown de choix LLM (header chat).
- [ ] **Slider nb max de revues** : **0 = OFF**, **1 à 10 = tours max** (elle s'arrête dès que la réponse passe ; pas d'infini = pas de runaway tokens/latence).
- [ ] **Sélecteur de mode** : Off / Auto / Hybride / Humaine.
- [ ] **Provider/modèle de la Reine** dans Settings (comme le provider par canal ; juge fort ou petit/local au choix).
- [ ] **LaReine visible dans le chat** : locuteur distinct (avatar couronne), affiche verdict + instruction renvoyée, trace méthodo repliable. Mode Humaine = l'utilisateur prend son siège.
- [x] **Voix de la Reine** (Kokoro/Voicebox TTS) : système voix complet livré (TTS streamé, appel plein écran, barge-in, mot d'éveil, Telegram bidirectionnel) - cf. section Mode Reine ambiant.

### Tiers d'autorité (activables séparément)
- [ ] **Tier 1 - Revue de réponse** (cas chat). Risque bas, à livrer en premier. Modes Auto/Hybride/Humaine.
- [ ] **Tier 2 - Revue d'artefacts via file de propositions (façon pull requests)** : à la création d'un skill / tool / édition mémoire / mission auto-générée, la proposition entre dans une **file durable** que la Reine draine (valide / corrige / rejette). Séparation créateur (curateur) / relecteur (Reine). Décisions actées :
  - **File découplée de la Reine** : c'est un store de premier rang, pas la propriété du toggle. **Désactiver la Reine ne perd jamais le backlog** : les propositions en attente restent gelées et visibles (Mémoire > En attente), actionnables à la main ; seules les **nouvelles** écritures repassent en direct. Notice non bloquante à la désactivation. (invariant testé : `transition_desactivation`).
  - **Risk-tier** pour ne pas noyer l'utilisateur : auto-approuve le **sûr** (fait nouveau non contradictoire) ; **toujours** mettre en file / escalader le **critique** (suppression, écrasement, contradiction) - jamais d'auto-apply destructif.
  - **Détection d'obsolescence** : une proposition versionnée contre sa cible passe en **Obsolète** (« needs rebase ») si la cible a bougé depuis, au lieu d'écraser à l'aveugle.
  - **Péremption** : TTL propre à la file, indépendant de la Reine, pour éviter un backlog qui pourrit.
  - Hook existant : `background_review.rs` (`memory_write`, `skill_propose`) à rediriger de l'auto-apply vers la file. Cœur pur fait : `laruche-essaim/src/reine_file.rs` (10 tests).
- [ ] **Tier 3 - Orchestration proactive** (boucle superviseur OFF par défaut) : elle initie, missionne des abeilles (ranger une section mémoire, fusionner des tools en doublon), vérifie le résultat.

### Garde-fous (non négociables)
- [ ] **Destructif = soft-delete réversible + log d'audit**, jamais de hard delete. memoire.db jamais touché à la légère.
- [ ] **Confirmation humaine** au-dessus d'un seuil de risque (au moins tant que la confiance n'est pas établie).
- [ ] **La Reine ne se relit pas elle-même** (anti-récursion) ; l'utilisateur peut **toujours la surcharger**.
- [ ] **Anti-régression** : une révision qui n'améliore pas un signal mesurable peut revenir au brouillon d'origine.
- [ ] **Borné en coût** : slider de tours + cadence de la boucle superviseur.

### Bonus (forte valeur)
- [ ] **Gardienne de marque et de style** : applique automatiquement le lexique FR de marque, l'anglais dans le code, zéro em dash, ton pro, sur chaque artefact généré. Automatise ce qui a été fait à la main.
- [ ] **Scorecard** : chaque intervention émet un score structuré (pertinence / méthodo / objectif / conformité marque) -> tableau de bord d'éval + carburant pour le curateur.
- [ ] **Étoile polaire** : la Reine tient l'objectif d'une conversation/mission et détecte la dérive (la boussole oriente DANS une boucle ; la Reine tient le cap SUR toute l'interaction).

### Ordre de construction
1. Cœur pur `cap/reine.rs` (types + contrôleur de tours borné + tests) + charte skill + clés i18n. **(en cours, isolé de main.rs)**
2. Hook de revue synchrone (Tier 1) dans `brain.rs` + appel LLM du juge.
3. Câblage UI (bouton/slider/mode/provider) + endpoint `reine_api.rs` + route Router **(après le split de main.rs)**.
4. Tier 2 : brancher le même juge sur la création d'artefacts.
5. Tier 3 : boucle superviseur optionnelle + garde-fous destructifs.

## ✅ Revue projet exhaustive & corrections (audit complet)

> Revue de tout le code (6 agents en parallèle sur des zones disjointes + relecture manuelle Opus du cœur `providers.rs`/`brain.rs`/sécurité), puis **correction de ~45 bugs/manques en 17 commits thématiques**. Workspace vert, `cargo test` **191 verts**, 0 em dash. Hygiène secrets/git vérifiée **clean** (aucun secret commité).

- [x] **Sécurité backend** : `check_admin` sur tous les endpoints mutables non gardés (channels start/stop, missions create/run/update/delete/decompose, crons run/update/delete, profiles active/visibility/use, `/auth/approve`) · GET `/api/config/channels` **masque les `bot_token`** sauf admin (fuite) · bulk-sync ne donne le `cookie_secret` qu'à un pair **mesh-signé** (plus d'IP seule) · `/infer` gaté (admin ou pair mesh) · `/mcp` exige le token hors localhost (était open).
- [x] **XSS frontend** : `esc()` sur données mesh/MCP/doctor injectées en `innerHTML` · `encodeURIComponent` sur URLs · checkbox transparence réparée.
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
- [~] **Split `main.rs`** : **démarré** - `web.rs` extrait (assets + i18n injection, ~100 lignes, sans couplage `AppState`, build vert). **Reste** : la majorité des handlers couplent à `AppState` (privé dans main.rs) ; les extraire impose de passer `AppState` + ~15 types privés (`CustomService`, `MetricsSnapshot`, `NodeEvent`, `ActivityLogEntry`...) en `pub(crate)`, puis sortir par domaine (`channels.rs`, `config_api.rs`, `missions_api.rs`). Gros refacto séquentiel, à faire d'un bloc en contexte frais avec `cargo build` à chaque étape.
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
