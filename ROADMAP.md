# LaRuche — Roadmap

> Reste-à-faire **récupéré des anciens docs de conception** (avant archivage dans `docs/_archive/`) + chantiers en cours. Source de vérité du travail restant. Coché = fait.

## ✅ Fait récemment
- [x] **Rangement racine** : `README.md` à jour + archivage des docs/scripts/lanceurs (`docs/_archive/`, `_archive/`, `laruche/_archive/`) + `.gitignore` durci.
- [x] **Split `spa.html`** → `app.css` + `app.js` + routes node (`/app.css`, `/app.js`).
- [x] **Split `app.js`** → 9 modules dans `templates/js/` (concat compile-time, un seul `/app.js` servi).
- [x] **i18n UI FR/EN** : infra `LaRuche.i18n` (`t()` + toggle header + dico par module) + **760 chaînes migrées** (workflow 8 agents). Marque non traduite.
- [x] **Localisation EN** : prompts système, abeilles (318 chaînes), plugins, **71 skills** (de-third-party + lean + `tools:` liés).
- [x] **Optimisations contexte** : champ `tools:` natif respecte la sélection · catalogue outils dé-dupliqué · catalogue **skills dynamique** · pas de corps de skill sur smalltalk · **seuil contexte configurable** (Settings).
- [x] **Sonde n_ctx** → `context_max_tokens` auto · commandes Telegram (`/help /status /crons /delcron`) · `@@secret` autocomplete · fixes crons (anti-spam/runaway/réplication).

## 🔧 Reste / near-term (chantiers actifs)
Suivis dans la liste de tâches du dépôt.
- [x] **i18n UI — 2ᵉ passe exhaustive** : audit ligne-à-ligne des 9 modules (parser dédié comments/dico/identifiants/CSS exclus). **16 dernières chaînes affichées** migrées (settings ×12, chat ×4). Reste = skips documentés (termes de marque `Abeille`, sentinels backend, identifiants).
- [ ] **i18n strings runtime côté Rust** : libellés émis par le serveur et streamés dans l'UI — `**Synthèse LaRuche :**` / `**Erreur LaRuche :**` (main.rs ~2279), actions éclaireuse `a demandé` / `a répondu` (main.rs ~3363). **Encore FR**, et les *matchers JS* (memory.js:680, capabilities.js:536) s'y accordent → traduire **les deux ensemble** (sinon on casse la détection). Pas de runtime i18n Rust pour l'instant → effort dédié.
- [ ] Décider du sort de la page **Sessions** orpheline (`laruche/_archive/test_script.js`).
- [ ] **Split `main.rs`** (11.7k) en modules node.
- [ ] **Settings : section « Avancé »** + migrer les params de tuning.
- [ ] Affinage injection skill body (top-1 + gate stricte).
- [ ] `/lang fr|en` messages Telegram directs (optionnel).
- [ ] Découper `app.js`-modules plus finement si besoin (settings/chat sont gros) — optionnel.

## 🩹 Dette / récupéré des archives (à trancher)
- [ ] **`laruche-suggestions`** : crate **orphelin** (0 référence ailleurs) → le **câbler** ou le **supprimer**.
- [ ] **Compression préflight** : compresser le contexte AVANT qu'il déborde (pas seulement tronquer/fenêtre glissante).
- [ ] **`cache_control`** (Anthropic prompt caching) : exploiter le cache de préfixe côté cloud.
- [ ] **`reasoning_effort` par modèle** (effort de raisonnement configurable).
- [ ] **Vérifier la boucle runtime des watchers** (création OK ; confirmer le déclenchement/livraison de bout en bout).
- [ ] **Câblage retry `credential_pool`** (rotation de clé sur 429) — à finaliser/vérifier.
- [ ] **`cargo test --workspace`** complet — jamais relancé en entier.

## ⚫ Bloqué (matériel)
- [ ] **Validation mesh A** (fédération des skills) — nécessite **2 nœuds réels**.

## 🔭 Vision / long-terme (Miel & essaim)
*POCs de nœuds spécialisés par capacité (le mesh annonce déjà les `capability:*`) :*
- [ ] `capability:llm` — nœud texte standard (Mistral/Llama)
- [ ] `capability:vlm` — Vision-Language (LLaVA, Qwen-VL) : analyse d'image via le réseau
- [ ] `capability:vla` — Vision-Language-Action : piloter bras robotique / drone
- [ ] `capability:rag` — nœud indexant des documents locaux, Q&A
- [ ] `capability:audio` — Whisper/Bark (STT/TTS), commandes vocales réseau
- [ ] `capability:image` & `capability:embed` — Stable Diffusion / vecteurs à la volée
- [ ] `capability:code` — CodeLlama/DeepSeek-Coder

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
