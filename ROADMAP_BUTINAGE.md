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

## ✅ Fait (≈30 commits)
Noyau testable (59 tests) · boucle pilotée par stop_reason (**texte seul = fin**, plus de rambling) · traits + pont + flag · récolte parallèle · escale+jauge (compaction + consolidation LLM→mémoire) · éclaireuses (sous-agents) · web multi-moteurs + Brave + fallback fetch (r.jina/Wayback) · plan `<plan>`→itinéraire→UI (3/3) · gardes injection+permissions · **steering live** · **curateur** (auto skills+tools VÉRIFIÉS, arrière-plan, conservateur, opt-in `RUCHE_CURATEUR=1`, **dédup côté code** via memory_search).
**Chat multi-conversation (solide)** : tag `session_id` sur events WS · garde routage front (même conv neuve) · multi-job (un message pendant un run détache+démarre) · bouton Envoyer réinit au switch · nudges internes non persistés (`Message::interne`) · texte dédoublé corrigé (`runningSession`, reattach conditionnel) · backlog feed restauré (`feedCache`) · ordre Feed (ts ms + réponse au-dessus question).
UI : auto-scroll stick-to-bottom · sélection texte mémoire · bouton 👁 PromptDebug.

## 🔜 À FAIRE (priorité ↓) — chantiers de la nuit
1. [ ] **Curateur = toggle dans SETTINGS** (au lieu de l'env var `RUCHE_CURATEUR`). Plus simple pour l'user. → ajouter un champ persisté (EssaimConfig ou settings) + UI Settings + le node lit ce champ au lieu de l'env. Garder fallback env.
2. [ ] **Multimodal** : envoi d'**images MULTIPLES** ET d'**audio**. Le pont butinage ignore `attachments`. Format ollama existant (`session.rs:310`) : `{"role":"user","content":text,"images":[base64],"attachments":[...]}`. Plan : ajouter `attachments: Vec<Attachment>` sur `Message` butinage (ou un champ images/audio) ; thread `attachments` hook→executer→1er message carnet ; `FournisseurPont::convertir_messages` produit le format multimodal (images[] + audio dans attachments). Tester avec modèle vision/audio.
3. [ ] **Tokens réels** : jauge en `chars/4`. Capter `prompt_eval_count` (ollama) / usage (openai/anthropic) dans `OllamaChunk` (streaming.rs) → `ReponseModele.usage` → jauge. ⚠️ streaming.rs/providers.rs sont dans le WIP user (modifs non commitées) → éditer prudemment.
4. [ ] **Popup d'approbation** : `Ask` auto-approuvé en butinage. Câbler `approval_rx` (thread comme steering) → sur outil mutant, émettre `ApprovalRequest` + attendre la réponse dans la récolte.
5. [ ] **Reprise au boot** : carnet écrit (`sessions/butinage/*.carnet.json`) mais rien ne le relit. Hook node au démarrage (lister/reprendre).
6. [ ] **dream/dédup auto** : passe de consolidation mémoire (`memoire.dream()`) périodique (cron ou idle).
7. [ ] **Fédération mesh** : propager skills VÉRIFIÉS aux ruches via `miel-protocol`.
8. [ ] Feed backlog des runs en fond non-attachés (event buffer côté serveur) ; `executionMode` par outil ; tokenizer réel.

## Décisions ouvertes
- Garder `brain.rs` fallback ou supprimer quand butinage validé ? · un seul backend mémoire prod (sidecar/native/sqlite) · `memoire.db` 155 Mo à profiler.

## Tester
Fermer l'instance → `lancer_butinage.bat`. Butinage actif : statuts anglais, `🐝 Éclaireuse`, `Récolte parallèle`. Curateur : Settings (à venir) ou `set RUCHE_CURATEUR=1`. Clé Tavily/Brave + `aux_model` recommandés.
