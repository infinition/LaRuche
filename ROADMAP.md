# LaRuche - Roadmap

> Reste-à-faire **récupéré des anciens docs de conception** (avant archivage dans `docs/_archive/`) + chantiers en cours. Source de vérité du travail restant. Coché = fait.

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
- [ ] **Phase 2b - intégration canaux** (reste) : (1) store langue **par canal/utilisateur** dans `AppState` + persistance `laruche-state.json` ; (2) commande `/lang fr|en` (Telegram d'abord, près de `/help` main.rs:7416) + auto-détection ; (3) le chat web envoie sa langue (cookie déjà posé) ; (4) router les chaînes **visibles utilisateur** via `i18n::t` (réponses commandes, messages canaux, erreurs montrées). Ajouter les clés Rust dans `lang/*.json`. **À faire avec test canal live** (Telegram/Discord) — touche le messaging.
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
