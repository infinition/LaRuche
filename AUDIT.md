# Audit & cartographie LaRuche v2 — manques et chantiers

État au 27/06/2026, branche `butinage`. Tout le workspace compile, 33 suites de tests vertes.
Légende effort : 🟢 quick-win · 🟡 moyen · 🔴 gros. Priorité : ⭐⭐⭐ haute → ⭐ basse.

## 1. Contexte & petits modèles (lié au bug Telegram résolu)
- ⭐⭐⭐ 🟢 **Toggle `dynamic_tool_selection` dans Settings** — par défaut `false` → TOUS les schémas
  d'outils sont injectés (~20-30k tokens), ce qui sature les modèles à petit `n_ctx` (gemma/llama.cpp
  32768). L'exposer en toggle (comme le curateur) réduirait drastiquement le prompt. **Quick-win recommandé.**
- ⭐⭐ 🟡 **Auto-activer la sélection dynamique** quand `n_ctx ≤ 32768` (heuristique). Évite à l'utilisateur
  d'y penser. (Le n_ctx réel est maintenant sondé via `/props`.)
- ⭐ 🟢 Indicateur visuel « contexte X/n_ctx » par message (la jauge existe déjà côté stats).

## 2. Channels
- ⭐⭐ 🟡 **Discord/Slack** : commandes `/sethome` + `/clear` (aujourd'hui Telegram seul) ; livraison
  proactive (cron/missions) ne gère QUE Telegram (`livrer_telegram`) → étendre à Discord/Slack.
- ⭐ 🔴 **WhatsApp** : channel non implémenté (mentionné comme futur). L'abstraction session est prête.
- ⭐ 🟡 **Home channel par utilisateur** (aujourd'hui global, OK en POC mono-user).

## 3. Mémoire
- ⭐ 🟡 **dream/dédup plus intelligent** : la passe périodique existe ; pourrait fusionner les quasi-doublons
  sémantiquement (au-delà du nettoyage).
- ⭐ 🟢 Bouton « purge des projections `tools.*` mortes » (auto-régénérées mais s'accumulent).

## 4. Moteur butinage
- ⭐⭐ 🟡 **Tokens réels openai/anthropic/codex** : seul Ollama renvoie `prompt_eval_count` aujourd'hui
  (la jauge se calibre via le facteur appris pour les autres, mais c'est approximatif).
- ⭐⭐ 🔴 **Reprise EFFECTIVE d'un carnet inachevé** : aujourd'hui détectés + journalisés au boot, mais
  pas re-injectés dans un run (il manque l'UI « missions reprises » + le rechargement du carnet).
- ⭐ 🟡 Popup d'approbation : câblé pour butinage ; à étendre aux sous-agents si besoin (volontairement off).

## 5. Fédération mesh
- ⭐ 🔴 **Propager les skills VÉRIFIÉS** aux autres ruches via `miel-protocol`. Infra présente
  (discovery, capabilities, manifest, `sync.rs`) mais **besoin d'un setup multi-nœuds pour tester**.

## 6. Kanban / Missions / Automatisations
- ⭐ 🟢 **Channel par tâche kanban** (modèle par tâche existe déjà ; channel non).
- ⭐ 🟢 Édition d'une mission existante (création OK ; pas d'édition in-place comme cron/watcher).

## 7. UI / QoL
- ⭐⭐ 🟢 **Audit responsive mobile fin** : barres d'onglets rendues scrollables (fait) ; reste à valider
  les modales (skill editor, secrets) et le chat sur petit écran.
- ⭐ 🟢 **Cleanup warnings** : quelques imports inutilisés (`PathBuf`, `ChildStdin`, `RwLock`…) dans main.rs.
- ⭐ 🟡 **Export PDF** (TODO existant `main.rs:3783`) : nécessite une lib (printpdf / headless Chrome).

## 8. Sécurité / Secrets
- ⭐⭐ 🟡 **Substitution `${NOM}` dans plus de surfaces** : aujourd'hui shell_exec + clé API provider.
  À étendre à execute_code (scripts Python) + forge (templates de plugins) + webhooks sortants.
- ⭐ 🟡 Chiffrement : keystream blake3 (correct au repos, sans dépendance) ; passer à AES-GCM si on
  accepte d'ajouter une crate (`chacha20poly1305`).

## ✅ Déjà fait cette session (rappel)
Secrets chiffrés+Webhooks · modèles dynamiques + sonde n_ctx · Missions provider/channel/cadence ·
Timeline interactive · Skill↔tools/plugins (hint + UI cases) · MCP onglet · feed persistant ·
mémoire conv. multi-canal + fenêtre glissante · /sethome · curateur Settings · multimodal ·
**édition complète mémoire (admin peut supprimer system.*/capacities.*)** · cohérence workspace.
