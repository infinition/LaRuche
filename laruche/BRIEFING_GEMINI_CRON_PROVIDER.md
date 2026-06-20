# Briefing Gemini — Choisir le provider/modèle d'un CRON

> Projet : `C:\Users\infinition\Desktop\laruche-v2\laruche`. Contexte : `..\LARUCHE_V2.md`.
> Chantiers précédents (capacités, providers) = terminés, merci.

## 0. Mission
On veut pouvoir dire « **cette tâche cron tourne sur tel provider / tel modèle** ». Le back-end est fait
(Claude) : une cron peut référencer un **profil** (`profile_id`) ; le daemon résout alors provider + clé +
base_url + modèle via ce profil. Ta mission = exposer ce choix dans le formulaire cron de la SPA.

## 1. Ta zone — UNIQUEMENT
`laruche-dashboard/src/templates/spa.html` (formulaire/list des crons, Settings > Cron).
NE TOUCHE PAS main.rs/cron.rs (zone Claude). Rebuild `cargo run -p laruche-node` après edit.

## 2. Contrat (Claude fournit)
- `GET /api/profiles` → `{ profiles: { "<id>": {provider,name,base_url,models,visibility} }, active_model }`.
- `POST /api/cron` (création) accepte désormais, en plus de `prompt`/`cron_expr`/`channel`/`skills` :
  - `profile_id` (string, optionnel) → la cron tournera sur ce profil.
  - `model` (string, optionnel) → modèle précis ; sinon le 1er modèle du profil.
  - (héritage) `provider` (string brut) reste accepté en fallback si pas de `profile_id`.
- `PATCH /api/cron/:id` (édition) accepte aussi `profile_id` (et `model`).
- La liste des crons (`GET /api/cron`) renvoie chaque tâche avec ses champs `profile_id`/`provider`/`model`.

## 3. Tâches
### G1 — Sélecteur Provider dans le formulaire cron (création + édition)
- Ajoute un champ **« Provider »** = liste déroulante des profils (`GET /api/profiles`), avec une option
  « Défaut (modèle actif) ». À la sélection, envoie `profile_id` dans le POST/PATCH.
- (optionnel) Sous-sélecteur **« Modèle »** peuplé depuis `profiles[id].models` → envoie `model`.
### G2 — Afficher le routage dans la liste des crons
- Pour chaque cron, montrer le provider/modèle effectif : si `profile_id` → nom du profil (+ modèle),
  sinon `provider`/`model` bruts, sinon « défaut ».

## 4. Acceptation
- `cargo check -p laruche-node` vert (tu ne touches que `spa.html`).
- Créer une cron en choisissant un provider → après reload, la cron affiche ce provider ; le `profile_id`
  est bien renvoyé par `GET /api/cron`.

## 5. À venir (NE PAS faire maintenant — Claude prépare le back-end)
- Même sélecteur pour **Watchers** et **Kanban** (Claude ajoutera `profile_id` à ces tâches ensuite).
- **Section MCP** dans les Settings (ajouter/lister/supprimer des serveurs MCP) — chantier dédié à suivre.

## 6. Contraintes
spa.html ONLY. MSVC (n'y touche pas). Pas de git. FR. Tuer `laruche-node.exe` avant `cargo run` si verrouillé.
