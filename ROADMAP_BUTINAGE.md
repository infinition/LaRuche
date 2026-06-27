# Roadmap — moteur « butinage » de LaRuche

Branche `butinage`. Moteur ReAct propre (crate `laruche-butinage`) qui cohabite avec l'ancien
`brain.rs` via le flag `RUCHE_MOTEUR=butinage` (lanceur : `lancer_butinage.bat`).
Réfs : `ARCHI_BUTINAGE.md` + mémoire `~/.claude/.../memory/` (third-party-workflow, third-party-workflow, agentic-principles-laruche).
**Le node verrouille `target/debug/laruche-node.exe` quand il tourne** → `cargo build -p laruche-node` finit par « os error 5 » (relink) mais le code COMPILE ; valider via `cargo build -p laruche-essaim` + `cargo test -p laruche-butinage`.

## Architecture clé (pour reprendre vite)
- Crate `laruche-butinage/src/` : `cycle.rs` (boucle `butiner`), `cap/{boussole,vigie,jauge}`, `issue`, `itineraire`, `carnet`, `meteo`, `escale`, `eclaireuse`, `nectar` (trait Source), `messagerie` (Message + champ `interne`), `fournisseur`/`outils`/`evenement`/`reglages` (traits).
- `butiner()` signature : `(carnet, reglages, fournisseur, outils, emet, source: Option<&dyn Source>, steering: Option<&mut Receiver<String>>)`.
- Pont `laruche-essaim/src/butinage_pont.rs` : `FournisseurPont`, `OutilsPont` (run principal, borrows `&`), `OutilsCurateur` (possédé, Arc, pour spawn), `EmetteurPont`, `SourcePont`, façade `pub async fn executer(...)`, `lancer_curateur_arriere_plan(...)`, `PROMPT_CURATEUR`, `skill_proche_existant` (dédup).
- Hook : `brain.rs` → top de `boucle_react_multimodal_ext` : si `RUCHE_MOTEUR=butinage`, délègue à `butinage_pont::executer`.
- Node `main.rs` : `ws_chat_connection` (loop avec `pending_text` pour multi-job), spawn curateur après run (gate `RUCHE_CURATEUR==Ok("1")`), `api_feed` (3088, ts normalisés ms).

## ✅ Fait (≈37 commits)
Noyau testable (60 tests) · boucle pilotée par stop_reason (**texte seul = fin**, plus de rambling) · traits + pont + flag · récolte parallèle · escale+jauge (compaction + consolidation LLM→mémoire) · éclaireuses (sous-agents) · web multi-moteurs + Brave + fallback fetch (r.jina/Wayback) · plan `<plan>`→itinéraire→UI (3/3) · gardes injection+permissions · **steering live** · **curateur** (auto skills+tools VÉRIFIÉS, arrière-plan, conservateur, **dédup côté code**).
**Chat multi-conversation (solide)** : tag `session_id` · garde routage front · multi-job · bouton Envoyer réinit · nudges internes (`Message::interne`) · texte dédoublé corrigé · backlog feed restauré · ordre Feed.
UI : auto-scroll · sélection texte mémoire · bouton 👁 PromptDebug.
**Chantiers nuit (faits)** :
1. ✅ **Curateur = toggle Settings** : `EssaimConfig.curateur_actif` persisté (PersistentState) · `GET/POST /api/config/curateur` · gate `config||env` · UI Settings>General. Fallback env `RUCHE_CURATEUR=1`.
2. ✅ **Multimodal** images multiples + audio : `messagerie::Piece` + `Message.pieces` · `Carnet.pieces` · `convertir_messages` émet `images[]`+`attachments[]` (format Ollama) · hook→executer transmet les attachments · recompose persiste les pièces · input UI accepte `audio/*`.
3. ✅ **Tokens réels** : `OllamaChunk.prompt_eval_count` (Ollama ; None openai/anthropic/codex) → `FournisseurPont` peuple `ReponseModele.usage` → `cycle` appelle `jauge.maj_usage` → **facteur de calibration** appris (réel/estimé, borné, capte le coût images).
4. ✅ **Popup d'approbation** : `OutilsPont.approval: Mutex<ApprovalReceiver>` · outil mutant `Ask` → `ChatEvent::ApprovalRequest` + attente (timeout 180s → refus) · None chez éclaireuses. UI déjà câblée. Mode `auto` ne prompte jamais.
5. ✅ **Reprise/hygiène carnets** : suppression du carnet à la réussite (`executer`) + `purger_carnets_au_boot()` (efface > 3 j, log les repris).
6. ✅ **dream/dédup auto** : `memoire.dream()` périodique (6 h, +10 min au boot ; `LARUCHE_DREAM_INTERVAL_SECS=0` pour couper).

## ✅ Fix de fond (27/06)
- **Mémoire conversationnelle** : butinage ouvrait un carnet vierge par message → amnésie (flagrant Telegram). `executer` réinjecte l'historique de session (`prelude_butinage`) avant le message courant ; recompose `skip(nb_prelude)` → pas de doublon. Images des anciens tours non ré-envoyées. ⚠️ **Limite connue** : si l'escale **compacte** l'historique pendant un run, le `skip(nb_prelude)` peut désaligner la persistance session (cosmétique : fidélité du log/relecture, pas la mémoire live). Rare en chat. À durcir si besoin (marquer les messages neufs autrement qu'en comptant le préfixe).

## 🔜 À FAIRE
7. [ ] **Fédération mesh** : propager skills VÉRIFIÉS aux ruches via `miel-protocol` (GROS — besoin test multi-nœuds).
8. [ ] Tokens réels openai/anthropic/codex (aujourd'hui Ollama seul) · Feed backlog des runs en fond non-attachés (buffer serveur) · `executionMode` par outil · tokenizer réel.
9. [ ] **Reprise effective** d'un carnet inachevé (aujourd'hui : détectés+log seulement ; reste à recharger le carnet dans un run via une UI « missions reprises »).

## Décisions ouvertes
- Garder `brain.rs` fallback ou supprimer quand butinage validé ? · un seul backend mémoire prod (sidecar/native/sqlite) · `memoire.db` 155 Mo à profiler.

## Tester
Fermer l'instance → `lancer_butinage.bat`. Butinage actif : statuts anglais, `🐝 Éclaireuse`, `Récolte parallèle`. Curateur : Settings (à venir) ou `set RUCHE_CURATEUR=1`. Clé Tavily/Brave + `aux_model` recommandés.
