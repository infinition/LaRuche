# Briefing Gemini — Visibilité provider (privé / public-proxy) + sélection 2-click des services mesh

> Projet : `C:\Users\infinition\Desktop\laruche-v2\laruche`. Contexte : `..\LARUCHE_V2.md` (Partie 0, Axe 5 & 6).
> Chantiers précédents (boucle d'apprentissage UI, dashboard services Axe 5) = terminés, merci.

## 0. Mission

On rend les modèles **découverts sur le mesh** réellement *utilisables en 2 clics*, et on introduit une
**visibilité des providers** : un provider reste **privé** (ce node seulement) ou devient **public-proxy**
(le mesh l'utilise *via* ce node — **la clé API ne quitte JAMAIS la machine**, le node relaie).
Ta mission = l'UI de tout ça. Le back-end (logique + endpoints) est fait par Claude.

## 1. Ta zone — UNIQUEMENT

- `laruche-dashboard/src/templates/spa.html` (Settings > Providers, et l'onglet Réseau/dashboard Axe 5).
- **Ne touche PAS** `main.rs`, `profiles.rs`, `providers.rs` (zone Claude pour ce chantier).
- Rebuild `cargo run -p laruche-node` après chaque edit (SPA via `include_str!`).

## 2. Contrat fourni par Claude

### Donnée enrichie
`ProviderProfile` (dans `GET /api/profiles`) a un nouveau champ :
```jsonc
"visibility": "prive" | "public_proxy"   // défaut "prive"
```

### Endpoints (Claude les implémente, tu les consommes)
| Méthode + route | Body | Effet |
|---|---|---|
| `GET /api/profiles` | — | liste des profils, chacun avec `visibility` |
| `POST /api/profiles/:id/visibility` | `{"visibility":"prive"\|"public_proxy"}` | bascule la visibilité (persisté) |
| `POST /api/models/use` | `{ "host","name","capability", "node_id"?, "base_url"? }` | sélectionne ce modèle ; si c'est un node distant, crée/réutilise un profil `provider:"miel"` pointant dessus, puis le met en **défaut de sa `capability`**. Réponse `{status, profile_id, model}` |
| `GET /swarm/models` *(déjà là)* | — | `{ models: [ {host,name,capability,is_local,node_id,node_name,is_default} ] }` |

## 3. Tâches

### G1 — Badges & toggle de visibilité (Settings > Providers)
- Sur chaque provider : badge **🔒 Privé** / **🌐 Public** selon `visibility`, + un **toggle** → `POST /api/profiles/:id/visibility`.
- ⚠️ Quand l'utilisateur passe en **public** un provider **à clé payante** (provider `openai`/`anthropic`/`codex`),
  afficher un avertissement clair : « Public = le mesh utilise ce provider VIA ce node (ta clé reste locale,
  ce node relaie et exécute les appels). N'expose jamais une clé que tu ne veux pas voir consommée par le réseau. »
- Les providers **locaux** (`ollama`, `llama.cpp`/miel local) : public proposé sans friction (c'est juste du GPU partagé).

### G2 — Bouton « Utiliser » sur les services mesh (dashboard Axe 5)
- Sur chaque service de `/swarm/models`, un bouton **« Utiliser »** → `POST /api/models/use` avec
  `{host, name, capability, node_id, base_url?}` (les champs viennent de l'objet du service).
- Au succès : toast « <name> actif pour <capability> » et marquer ce service comme **actif** dans sa catégorie
  (re-fetch `/swarm/models` : le service actif a `is_default: true`).

### G3 — Indicateur passerelle (léger)
- Quand un provider est `public_proxy`, petit pictogramme « passerelle mesh » à côté, pour signaler que ce node
  relaie cette capacité pour le réseau.

## 4. Acceptation
- `cargo check -p laruche-node` vert (tu ne touches que `spa.html`).
- Sur `:8419` : toggler la visibilité d'un provider persiste (re-fetch `/api/profiles` le confirme).
- Cliquer « Utiliser » sur un LLM mesh le rend actif (devient `is_default` pour sa capability ; le chat l'utilise).
- Avertissement affiché avant de rendre public un provider à clé.

## 5. Contraintes
Toolchain MSVC (n'y touche pas). Pas de git. FR. Tuer `laruche-node.exe` avant `cargo run` si verrouillé.
Vérifie (`cargo check -p laruche-node`) avant de dire « fini ». Ne livre pas de JS cassé.
