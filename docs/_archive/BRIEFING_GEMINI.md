# Briefing Gemini — UI de la boucle d'apprentissage (LaRuche v2)

> Tu reprends un projet en cours, **à froid**. Lis d'abord la **Partie 0** de
> [`LARUCHE_V2.md`](./LARUCHE_V2.md) (source de vérité, état réel du code au 2026-06-20).
> Projet : `C:\Users\infinition\Desktop\laruche-v2\laruche` (workspace Rust, 14 crates).

## 0. Le chantier en une phrase

LaRuche apprend **tout seul** : à partir d'une trajectoire réussie, l'agent crée des **skills**
(documents OKF stockés en mémoire cognitive sous `tools.skills.*`) qu'il réutilise ensuite.
La mécanique back-end est faite (Claude la finit). **Ta mission = rendre la boucle visible et
pilotable dans l'UI** : voir les skills, accepter/rejeter ceux que l'agent propose, et voir en
direct quand un skill naît ou est appliqué.

## 1. Tes zones — et UNIQUEMENT celles-là (sinon collision avec Claude)

- `laruche-node/src/main.rs` → **seulement** des handlers/endpoints HTTP. N'ajoute **rien** dans la boucle agent.
- `laruche-dashboard/src/templates/spa.html` → l'UI (vanilla JS, **zéro build**, tokens CSS ambre/ruche dans `:root`).

> ⚠️ **NE TOUCHE PAS** : `brain.rs`, `laruche-essaim/*`, `laruche-memoire/*`, `laruche-skills/*`
> (zones Claude/Codex). Si tu crois avoir besoin d'y toucher, c'est que le contrat ci-dessous suffit — relis-le.
>
> ⚠️ La SPA est embarquée dans le binaire via `include_str!` → **après chaque edit de `spa.html`,
> rebuild obligatoire** : `cargo run -p laruche-node`.

## 2. Contrat fourni par Claude (ce sur quoi tu t'appuies)

### Endpoints HTTP déjà existants (à consommer, ne pas réécrire)
| Méthode + route | Rôle |
|---|---|
| `GET /api/skills` | Liste des skills `{name, description, enabled}` |
| `GET /api/skills/:name` | Le SKILL.md (OKF) complet |
| `POST /api/skills` | Crée/met à jour un skill (body `{content}` OKF, ou `{name, content}`) |
| `POST /api/skills/:name/toggle` | Active/désactive (persisté) |
| `DELETE /api/skills/:name` | Supprime |
| `GET /api/memory/proposed` | File de revue (items proposés). **Filtre côté UI** ceux dont l'OKF contient `type: skill` |
| `POST /api/memory/review` | Accepter/rejeter un proposé (regarde la signature exacte dans `main.rs`) |

### Deux nouveaux events WebSocket (déjà câblés par Claude, sérialisés automatiquement)
Ils arrivent dans le `ws.onmessage` de la SPA comme tous les autres events (`token`, `tool_call`, …) :
```json
{ "type": "skill_applied",  "name": "<nom>" }   // un skill appris a été auto-injecté dans CE tour
{ "type": "skill_proposed", "name": "<nom>" }   // un nouveau skill vient d'être proposé (trajectoire réussie)
```

## 3. Tâches

### G1 — Page/onglet « Skills »
Onglet dédié (ou section Settings), style nid d'abeille ambre cohérent avec l'existant.
- Liste des skills actifs (`GET /api/skills`), avec description et toggle enable/disable.
- Vue de l'OKF complet au clic (`GET /api/skills/:name`) dans un panneau.
- Suppression (`DELETE`).
- Bouton « Nouveau skill » → petit éditeur OKF (textarea Markdown) → `POST /api/skills`.
- Badge **« auto »** sur les skills issus de l'auto-apprentissage (source `auto-skill` / tag `skill`).

### G2 — File de revue des skills proposés
- Section « Skills proposés » lisant `GET /api/memory/proposed`, en ne gardant que les items `type: skill`.
- Pour chaque proposé : aperçu + boutons **Accepter** / **Rejeter** → `POST /api/memory/review`.
- Accepter un proposé doit le faire apparaître ensuite dans la liste G1 (skill actif).

### G3 — Chips d'apprentissage dans le fil de chat
- À la réception de `skill_applied` → puce inline dans le tour courant : « 🧠 Skill appliqué : <name> ».
- À la réception de `skill_proposed` → toast « ✨ Skill né : <name> » + rafraîchir la file G2.

## 4. Critères d'acceptation (la définition de « fini »)

1. `cargo check -p laruche-node` **vert**.
2. Sur `http://localhost:8419` : l'onglet Skills liste les skills, on peut en créer/voir/supprimer/toggler.
3. Un skill **proposé** accepté via G2 passe en **actif** dans G1.
4. Pendant un chat : la puce « Skill appliqué » et le toast « Skill né » s'affichent quand les events arrivent.

## 5. Build / run / contraintes

```bash
cd C:\Users\infinition\Desktop\laruche-v2\laruche
cargo run -p laruche-node      # → http://localhost:8419
```
- Toolchain **MSVC** déjà épinglée (`rust-toolchain.toml`). Ne change pas la toolchain.
- **Pas de git** (l'utilisateur gère le versionnement).
- Si `laruche-node.exe` tourne déjà, Windows verrouille le binaire → tuer le process avant `cargo run`, ou `cargo check`.
- Conventions de nommage **FR** cohérentes avec l'existant (abeille, mémoire, skill…).
- Vérifie chaque étape (`cargo check -p laruche-node`) avant de dire « fini ». Ne livre pas de code non compilé.

## 6. Coordination

- Claude tient `brain.rs` / `laruche-essaim` (rappel auto des skills, gating de l'extraction, POC end-to-end)
  et **émet** déjà les events `skill_applied` / `skill_proposed`. Toi tu les **affiches**.
- Les endpoints `/api/skills*` et `/api/memory/{proposed,review}` existent déjà : tu les **consommes**.
  Si l'un manque un champ dont tu as besoin, ajoute le champ **dans le handler node** (ta zone), pas dans l'essaim.
- En cas de doute sur la forme exacte d'une réponse API, lis le handler correspondant dans `main.rs`
  (cherche `api_list_skills`, `api_memory_proposed`, `api_memory_review`).
