# Roadmap — moteur « butinage » de LaRuche

Branche `butinage`. Moteur ReAct propre (crate `laruche-butinage`) qui cohabite avec l'ancien
`brain.rs` via le flag `RUCHE_MOTEUR=butinage` (lanceur : `lancer_butinage.bat`).
Réfs concurrents étudiés : `ARCHI_BUTINAGE.md` + mémoire `~/.claude/.../memory/` (third-party-workflow, third-party-workflow, agentic-principles-laruche).

## ✅ Fait
- **Noyau testable** : `issue` (Issue/FinDeVol/Bilan), `itineraire` (plan = affichage, pas une condition de fin), `carnet` (état persistable), `cap/vigie` (anti-boucle pur), `cap/boussole` (`cap()` = LA politique), `cap/jauge` (budget), `meteo` (erreurs/retry). ~59 tests verts.
- **Boucle** `cycle::butiner` pilotée par stop_reason. **Texte seul = fin de tour** (état de l'art ; plus de continuation forcée → fin du rambling). Rails bornés (`relance_max=3`) : troncature, tool malformé, exploration. `PlanEnregistre` = poser un plan continue (acte productif).
- **Intégration par traits** : `Fournisseur`/`Outils`/`Emetteur`/`Source`. Pont `butinage_pont.rs` + façade + flag.
- **Récolte parallèle** (read-only ∥ / mutant séquentiel, borné).
- **Longévité** : `escale` (compaction extractive) + **consolidation LLM → mémoire** (Source) + `jauge`.
- **Sous-agents** `eclaireuse` (rôles, budget séparé, streamés, anti-récursion).
- **Cloisonnement chat** : events WS tagués `session_id` ; front route par conversation ; pastille sur terminal seulement.
- **Web boosté** : multi-moteurs ∥ + Brave + **fallback fetch** (r.jina.ai / Wayback sur 403) ; dork plugin réparé.
- **Plan UI** : capté depuis `<plan>`, gardé dans l'historique, poussé `done` à l'UI (fini le 0/3).
- **Gardes** : injection (threat_patterns) + permissions dans le pont.
- **⭐ Curateur** (auto-skills & **tools vérifiés**, killer feature) : sous-run post-mission, outils restreints, `PROMPT_CURATEUR` (arbre patch-first, anti-dup, liste noire, vérif plugin via shell_exec). Gate `RUCHE_CURATEUR=0`.

## 🔜 À faire (priorité ↓)

### Curateur — Phase 2 (le rendre béton)
- [ ] **Exécution en ARRIÈRE-PLAN** (actuellement synchrone après la mission → retarde le run et garde la session occupée → risque de race). Via `Arc<AbeilleRegistry>` du node + `tokio::spawn`. **PRIORITAIRE** (bug latent).
- [ ] **Dédup / dream** : passe de consolidation (`memoire.dream()`) qui fusionne les skills proches, archive les inutilisés (tracking d'usage), évite la prolifération.
- [ ] **Fédération mesh** : propager les skills VÉRIFIÉS aux autres ruches via `miel-protocol` (moat « l'essaim qui apprend »).
- [ ] Résumé curateur visible (le `💾 Self-improvement` de third-party) — actuellement les events tombent après le `Done`.

### Lot node (touche `main.rs`)
- [ ] **Steering live** : injecter un message user pendant un run (file rafraîchie chaque passe, façon third-party `getSteeringMessages`). Pas câblé dans butinage.
- [ ] **Tokens réels** : la `jauge` estime en `chars/4` ; brancher `usage` provider (modif `providers.rs` → capter prompt tokens dans `OllamaChunk`).
- [ ] **Popup d'approbation** : `Ask` est auto-approuvé en butinage (signalé). Câbler `approval_rx`.
- [ ] **Multimodal** : attachments images non transmis (text-first).
- [ ] **Reprise au démarrage** : le carnet est écrit (`sessions/butinage/*.carnet.json`) mais **rien ne le relit** au boot. Ajouter un hook de reprise.
- [ ] **Multi-job concurrent** : une socket/run pour vrais jobs simultanés (aujourd'hui la socket est occupée par le run courant).

### Boucle / qualité
- [ ] **`executionMode` par outil** (façon third-party) plutôt que la partition heuristique read-only.
- [ ] Tuning `min_web_exploration` selon retour terrain.
- [ ] Migrer les **nudges/prompts en anglais** restants (déjà fait pour butinage ; vérifier l'ancien `brain.rs` si on le garde).
- [ ] Tokenizer réel (remplacer `chars/4` partout).

### Décisions ouvertes
- [ ] Garder l'ancien `brain.rs` en fallback, ou supprimer une fois butinage validé ?
- [ ] Un seul backend mémoire de prod (sidecar/native/sqlite cohabitent).
- [ ] `memoire.db` à 155 Mo → profiler/purger.

## Comment tester
`lancer_butinage.bat` (ferme l'instance d'abord — `.exe` verrouillé sinon). Vérif butinage actif : statuts anglais, `🐝 Éclaireuse`, `Récolte parallèle`, `🐝 Curateur`. Idéal : clé Tavily/Brave (`lancer_butinage.bat`) + `aux_model` pour le curateur.
