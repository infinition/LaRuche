# LaRuche — Roadmap

> Reste-à-faire **récupéré des anciens docs de conception** (avant archivage dans `docs/_archive/`) + chantiers en cours. Source de vérité du travail restant. Coché = fait.

## 🔧 En cours / near-term (chantiers actifs)
Suivis dans la liste de tâches du dépôt. Résumé :
- [ ] Cleanup `laruche/_archive/` ✅ (rangement fait) — reste : décider du sort de la page **Sessions** orpheline (dans `test_script.js`).
- [ ] **Split `spa.html`** → CSS + JS + routes (enabler i18n).
- [ ] **i18n UI FR/EN** (sélecteur de langue ; UI seulement).
- [ ] **Split `main.rs`** (11.7k) en modules node.
- [ ] **Settings : section « Avancé »** + migrer les params de tuning.
- [ ] Affinage injection skill body (top-1 + gate stricte).
- [ ] `/lang fr|en` messages Telegram directs (optionnel).

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
