# Briefing Gemini — Axe 5 : Dashboard « services du mesh par catégorie »

> Projet : `C:\Users\infinition\Desktop\laruche-v2\laruche`. Contexte global :
> `..\LARUCHE_V2.md` (Partie 0 + §4 Axe 5). Chantier précédent (boucle d'apprentissage UI) = terminé, merci.

## 0. Mission

`laruche-node` est un **nœud réseau** qui découvre et expose des modèles/services sur le mesh Miel.
Claude vient de **généraliser la détection locale** : en plus d'Ollama, le node détecte maintenant les
backends **OpenAI-compatibles locaux** (llama.cpp, vLLM, LM Studio) et les nœuds Miel distants
(STT/TTS/Agent…). **Ta mission = un dashboard qui présente tout ça, groupé par catégorie de capacité,
en distinguant local vs distant (mesh).**

## 1. Ta zone — UNIQUEMENT

- `laruche-dashboard/src/templates/spa.html` (onglet Dashboard / Réseau). **Rien d'autre.**
- Tu n'as **pas** besoin de toucher `main.rs` cette fois : l'endpoint te sert déjà tout (voir §2).
  Si vraiment un champ te manque, demande — ne modifie pas la détection (zone Claude `local_inference.rs`).

> ⚠️ `spa.html` est embarquée via `include_str!` → **rebuild `cargo run -p laruche-node`** après chaque edit.

## 2. Contrat — l'endpoint à consommer

`GET /swarm/models` → `SwarmModelsResponse { models: [SwarmModelInfo], ... }`.
Chaque `SwarmModelInfo` :
```jsonc
{
  "host": "llama.cpp",            // backend/source : "llama.cpp" | "lmstudio" | "vllm" | <ip nœud Miel> | <provider>
  "node_name": "http://127.0.0.1:8001 (local)",
  "node_id": null,                // présent pour un nœud Miel distant
  "name": "qwen3.6-35b-a3b",      // nom du modèle
  "size_gb": 0.0,
  "is_default": false,
  "is_local": true,               // true = sur CETTE machine ; false = autre nœud du mesh
  "capability": "llm"             // "llm"|"vlm"|"vla"|"rag"|"audio"|"image"|"embed"|"code"|"agent"|"stt"|"tts"
}
```
> Nouveauté Claude : les modèles **llama.cpp/vLLM/LM Studio locaux** apparaissent désormais avec
> `is_local: true` et `host` = le nom du backend. Les services Miel distants (TTS/STT/Agent) ont
> `is_local: false` + `node_id`. La catégorie est dans `capability`.

Référence capacités (libellés `capability`) : `llm, vlm, vla, rag, audio, image, embed, code, agent, stt, tts`
(défini dans `miel-protocol/src/capabilities.rs`, méthode `description()` pour un libellé humain).

## 3. Tâches

### G1 — Vue « Services par catégorie »
Dans l'onglet Dashboard (ou un nouvel onglet « Réseau »), afficher les modèles **groupés par
`capability`** (une section par catégorie présente : LLM, VLM, TTS, STT, Code, Embed, Image, Agent…),
style nid d'abeille ambre cohérent avec l'existant. Catégories vides masquées.

### G2 — Distinction Local vs Mesh
Dans chaque catégorie, séparer / marquer visuellement :
- **Local** (`is_local: true`) — badge du backend (`host` : llama.cpp / lmstudio / vllm / ollama).
- **Mesh / distant** (`is_local: false`) — nom du nœud (`node_name`) + indicateur « distant ».
Afficher un compteur par catégorie et un total (« N modèles sur M sources »).

### G3 — Rafraîchissement
Bouton « Rescanner » qui re-fetch `GET /swarm/models` (la détection locale est refaite côté serveur à
chaque appel). Idéalement auto-refresh léger (ex. toutes les 15 s) tant que l'onglet est visible.

## 4. Acceptation
- `cargo check -p laruche-node` vert (tu ne touches que `spa.html`, donc surtout : pas de JS cassé).
- Sur `http://localhost:8419`, l'onglet montre les modèles groupés par catégorie, local vs mesh distingués.
- Si llama.cpp tourne sur `:8001`, son modèle apparaît dans la catégorie **LLM**, badge « llama.cpp », local.
- Un nœud Miel TTS distant apparaît dans la catégorie **TTS**, marqué distant.

## 5. Contraintes
Toolchain MSVC épinglée (n'y touche pas). Pas de git. Conventions FR. Tuer `laruche-node.exe` avant `cargo run`
si verrouillé. Vérifie avant de dire « fini ».

## 6. Test rapide sans matériel
Tu peux simuler des backends locaux via la var d'env (lue par Claude côté serveur) :
`LARUCHE_OPENAI_ENDPOINTS="llama.cpp=http://127.0.0.1:8001,vllm=http://127.0.0.1:8000"`.
Pour l'UI, le plus simple est de vérifier le rendu avec la vraie réponse `/swarm/models` (même vide,
les catégories doivent se construire dynamiquement à partir des `capability` reçues).
