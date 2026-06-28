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
- [ ] *Note* : i18n FR/EN ne couvre que l'UI webapp. Les **strings runtime Rust** (logs, messages serveur) sont en anglais simple (pas de runtime i18n Rust) ; chip mémoire `chat.js` dépend d'un status à compteur que le Rust n'émet plus (cosmétique).

## 🔧 Reste / near-term (chantiers actifs)
Suivis dans la liste de tâches du dépôt.
- [x] **i18n UI - 2ᵉ passe exhaustive** : audit ligne-à-ligne des 9 modules (parser dédié comments/dico/identifiants/CSS exclus). **16 dernières chaînes affichées** migrées (settings ×12, chat ×4). Reste = skips documentés (termes de marque `Abeille`, sentinels backend, identifiants).
- [x] **i18n sentinels runtime Rust↔JS** : `Synthèse/Erreur LaRuche`, verbes de feed `a demandé/a répondu`, `réponse finale` traduits des deux côtés ensemble (voir section Localisation).
- [ ] Décider du sort de la page **Sessions** orpheline (`laruche/_archive/test_script.js`).
- [ ] **Split `main.rs`** (11.7k) en modules node.
- [ ] **Settings : section « Avancé »** + migrer les params de tuning.
- [ ] Affinage injection skill body (top-1 + gate stricte).
- [ ] `/lang fr|en` messages Telegram directs (optionnel).
- [ ] Découper `app.js`-modules plus finement si besoin (settings/chat sont gros) - optionnel.

## 🩹 Dette / récupéré des archives (à trancher)
- [ ] **`laruche-suggestions`** : crate **orphelin** (0 référence ailleurs) → le **câbler** ou le **supprimer**.
- [ ] **Compression préflight** : compresser le contexte AVANT qu'il déborde (pas seulement tronquer/fenêtre glissante).
- [ ] **`cache_control`** (Anthropic prompt caching) : exploiter le cache de préfixe côté cloud.
- [ ] **`reasoning_effort` par modèle** (effort de raisonnement configurable).
- [ ] **Vérifier la boucle runtime des watchers** (création OK ; confirmer le déclenchement/livraison de bout en bout).
- [ ] **Câblage retry `credential_pool`** (rotation de clé sur 429) - à finaliser/vérifier.
- [ ] **`cargo test --workspace`** complet - jamais relancé en entier.

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
