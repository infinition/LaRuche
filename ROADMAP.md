# LaRuche - Roadmap

> Reste-à-faire **récupéré des anciens docs de conception** (avant archivage dans `docs/_archive/`) + chantiers en cours. Source de vérité du travail restant. Coché = fait.

## 👑 LaReine - superviseur de la ruche (nouveau chantier)

> Mode activable qui place une **Reine** au-dessus des butineuses : elle juge la pertinence et la méthodologie d'une réponse (ou d'un artefact), renvoie au LLM des instructions correctives jusqu'à atteindre l'objectif, et peut gouverner l'auto-modification de LaRuche. Principe directeur : **le curateur propose, la Reine dispose.**
>
> Règles de build : tout en **anglais par défaut** (code, chaînes, commentaires propres non « LLM-like »), **chaînes UI variabilisées + i18n** (`lang/strings.json`, `t()`), **aucun em dash**, termes de marque FR conservés. La Reine n'est PAS la vigie : la **vigie garde la boucle de l'intérieur** (anti-loop/budget), la **Reine supervise le résultat de l'extérieur** (pertinence, méthodo, objectif).

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
- [ ] **Voix de la Reine** (Kokoro TTS, cf. backlog voix) : elle peut parler ses verdicts.

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
- [ ] **Vérifs runtime** watchers loop + `credential_pool` retry : nécessitent l'app en marche + un 429 réel.

## ✅ Fait cette session (dette)
- [x] **`laruche-suggestions`** orphelin → **supprimé** (stub 71 lignes jamais câblé).
- [x] **`cargo test --workspace`** : **33 cibles vertes**.

## ⚫ Bloqué (matériel)
- [ ] **Validation mesh A** (fédération des skills) - nécessite **2 nœuds réels**.

## 🔭 Vision / long-terme (Miel & essaim)

### 👑🎙️ Mode Reine ambiant (Jarvis local, voix plein écran)
> Aboutissement de LaReine + la voix : on parle à la Reine en continu, plein écran, et elle supervise et pilote l'essaim. **Local-first** (Kokoro + Whisper offline), donc différenciant face aux assistants cloud. La Reine est l'hôte naturel : elle voit déjà tout ce que LaRuche indexe (mémoire, registres, carnet, feed, mesh), elle agit en ton nom **mais gatée** par la file de propositions, et elle se relit avant de parler (anti-hallucination).
- [ ] **UI ambiante plein écran** : mode voix d'abord de la SPA, montre la trace méthodo, la file de propositions et le feed live.
- [ ] **Boucle voix temps réel** : STT bas-latence -> Reine -> butinage si action -> TTS streamé **phrase par phrase** (Kokoro déjà benché, brancher sur butinage).
- [ ] **Provider STT/TTS pluggable** (comme provider par canal) : Kokoro, Whisper, ou autre.
- [ ] **Barge-in** (la couper pendant qu'elle parle) + wake-word / push-to-talk (vie privée).
- [ ] **Session ambiante persistante** (contexte continu, pas tour-par-tour).
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
