# Briefing Gemini — Sélection de service PAR CAPACITÉ (LLM / Code / STT / TTS / VLM / VLA) + intégration voix

> Projet : `C:\Users\infinition\Desktop\laruche-v2\laruche`. Contexte : `..\LARUCHE_V2.md` (Partie 0, Axe 5).
> Chantiers précédents (dashboard par catégorie, visibilité providers) = terminés, merci.

## 0. Mission

Aujourd'hui on choisit *un* modèle actif (LLM). On veut une sélection **par capacité** : un LLM de chat,
**un modèle de Coding distinct**, un STT, un TTS, éventuellement VLM/VLA — chacun choisi indépendamment,
local **ou** découvert sur le mesh Miel. Et surtout : quand l'utilisateur **active la dictée vocale**, il
choisit *quel STT* transcrit ; quand il **active l'auto-TTS**, il choisit *quel TTS* parle (un service du réseau).
Ta mission = l'UI. Le back-end (état + endpoints + routing voix) est fait par Claude.

## 1. Ta zone — UNIQUEMENT
- `laruche-dashboard/src/templates/spa.html` (Dashboard/Réseau + réglages Voix).
- **Ne touche PAS** `main.rs`/`profiles.rs`/`providers.rs` (zone Claude). Rebuild `cargo run -p laruche-node` après edit.

## 2. Contrat (Claude fournit, tu consommes)
| Méthode + route | Rôle |
|---|---|
| `GET /swarm/models` *(déjà)* | tous les modèles : `{models:[{host,name,capability,is_local,node_id,node_name,is_default}]}` |
| `GET /api/capabilities/selection` **(nouveau)** | sélection courante par capacité : `{ "selection": { "stt": {capability,model,backend,node_id?,is_local}, "code": {...}, "llm": {...}, ... } }` |
| `POST /api/models/use` *(déjà)* | body `{host,name,capability,node_id?,base_url?}` → choisit ce service POUR sa `capability` (le met aussi en sélection ci-dessus) |
| `GET /api/voice/status` **(enrichi)** | `{ "stt": {available,url,selected_model,is_selected}, "tts": {available,url,selected_model,is_selected} }` — honore désormais le STT/TTS **choisi** (sinon premier trouvé) |

Capacités (`capability`) : `llm, code, vlm, vla, rag, audio, image, embed, agent, stt, tts`.

## 3. Tâches

### G1 — Sections par capacité (Dashboard/Réseau)
- Une **section par capacité présente** dans `/swarm/models` : **LLM**, **Coding** (distincte, mise en avant),
  **STT**, **TTS**, **VLM**, **VLA**, Embed, Image, Agent.
- Dans chaque section : les services (local vs mesh, déjà fait au chantier précédent) + **marquer le service
  sélectionné** pour cette capacité (depuis `GET /api/capabilities/selection`) + bouton **« Utiliser »**
  (`POST /api/models/use` avec la `capability` de la section).
- Le « modèle actif » global = la sélection `llm`.

### G2 — Intégration voix (le cœur de la demande)
- **Dictée vocale (STT)** : à côté du toggle de dictée, un **sélecteur** des services STT détectés
  (catégorie `stt` de `/swarm/models`) → « Utiliser » = `POST /api/models/use {capability:"stt", ...}`.
  Afficher le STT actif (`/api/voice/status` → `stt.selected_model` / `stt.url`, `is_selected`).
- **Auto-TTS** : idem, sélecteur des services **TTS** détectés (local ou mesh) → `POST /api/models/use {capability:"tts", ...}`.
  Afficher le TTS actif via `/api/voice/status` (`tts.*`).
- Si aucun STT/TTS choisi : indiquer « auto (premier détecté) » (c'est le fallback serveur).

### G3 — Lisibilité (léger)
- Petit récap « Pour cette discussion : LLM = X · Coding = Y · STT = Z · TTS = W » à partir de
  `GET /api/capabilities/selection`, pour que l'utilisateur voie d'un coup d'œil sa config.

## 4. Acceptation
- `cargo check -p laruche-node` vert (tu ne touches que `spa.html`).
- Sur `:8419` : sections par capacité visibles ; choisir un STT/TTS via les réglages voix persiste et se reflète
  dans `/api/voice/status` (`is_selected:true`, `selected_model`) ; une section Coding distincte permet de choisir
  un modèle de code indépendant du LLM de chat ; le récap G3 affiche la sélection courante.

## 4bis — Mode Coding dans le chat (NOUVEAU, branché côté Claude)
Le tour de chat accepte maintenant un champ **`capability`** dans le message WS envoyé à l'agent.
- Ajoute un **toggle « Mode Coding »** près de la zone de saisie. Quand il est ON, ajoute `"capability":"code"`
  au payload du message WS (à côté de `provider`/`model`). Le serveur route alors le tour vers le **modèle
  de Coding sélectionné** (section Coding de G1) au lieu du LLM de chat. OFF (ou absent) = LLM de chat normal.
- Rien d'autre à faire : la résolution provider/modèle est côté serveur.

## 5. Contraintes
spa.html ONLY. Toolchain MSVC (n'y touche pas). Pas de git. FR. Tuer `laruche-node.exe` avant `cargo run` si verrouillé.
Vérifie avant de dire « fini ».
