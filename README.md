# LaRuche v2 🐝

**Agent IA *edge* en Rust** — multi-canal (Telegram / Discord / Slack), mémoire cognitive persistante, auto-amélioration, et fédération en essaim. Un seul binaire, faible empreinte, tourne en local.

> Ce qu'aucune des références (third-party / third-party / Claude Code) ne réunit : **multi-canal natif + mémoire par canal + moteur edge Rust + curateur qui crée skills & outils vérifiés + mesh + MCP client ET serveur**.

---

## 🚀 Lancer

```
lancer_butinage.bat
```

Ça lance le node (`laruche-node`) avec le moteur **butinage** + backend mémoire **sqlite**, puis ouvre le dashboard web. Réglages dans le `.bat` : moteur (`RUCHE_MOTEUR`), backend mémoire (`LARUCHE_MEMOIRE_BACKEND` = `native|sqlite|sidecar`), et clés de recherche web optionnelles (`LARUCHE_TAVILY_KEY`, `LARUCHE_BRAVE_KEY`, `LARUCHE_SEARXNG_URL`).

Tout le reste se règle **à chaud dans Settings** (modèle, providers, contexte, curateur, secrets, MCP, canaux…), sans redémarrage.

---

## 🧠 Architecture (workspace `laruche/`)

| Crate | Rôle |
|---|---|
| `laruche-node` | Serveur HTTP/axum : routes API, dashboard, bots de canaux, serveur MCP, état |
| `laruche-essaim` | Moteur agentique : boucle ReAct, abeilles (outils), prompt, providers, curateur |
| `laruche-butinage` | Noyau ReAct testable : `butiner()`, cap (boussole/jauge/vigie), escale, éclaireuses |
| `laruche-memoire` | Carte cognitive (nœuds + items), backends sqlite / natif |
| `laruche-dashboard` | UI web (SPA) servie par le node |
| `miel-protocol` | Mesh : découverte mDNS, manifeste, fédération de nœuds |
| `laruche-kanban` · `laruche-watchers` · `laruche-channels` · `laruche-compaction` · `laruche-permissions` · `laruche-skills` · `laruche-voix` · `laruche-cli` | Modules dédiés |

**Concepts clés** : *abeilles* = outils, *butinage* = la boucle agentique, *nectar/Source* = mémoire, *escale* = compaction, *éclaireuse* = sous-agent, *curateur* = auto-création de skills/outils vérifiés.

---

## ✨ Ce qui marche

- **Multi-canal** Telegram (Discord/Slack câblés) + **mémoire persistante par canal** (UUIDv5).
- **Moteur butinage** : ReAct, anti-boucle, outils ∥, compaction, sous-agents budgétés, steering live.
- **Mémoire cognitive sémantique** : la DB surface faits & skills pertinents (sélection dynamique des outils ET du catalogue de skills selon l'intention).
- **Auto-amélioration** : le *curateur* crée/patche skills & plugins **vérifiés** en arrière-plan.
- **Prompts système éditables à chaud** (mémoire `system.*` : identité, comportement, planification, curateur, consolidation) avec « restaurer défaut ».
- **Secrets** chiffrés (`${NOM}` / `@@NOM`, le LLM ne voit que les noms) + autocomplétion `@@` partout.
- **MCP client ET serveur** (`POST /mcp`).
- **Mesh** : découverte de nœuds + fédération de skills (`/api/mesh/sync`).
- **Hooks** utilisateur (pre/post-tool, `hooks.json`).
- **Hub d'automatisation** : crons, watchers, kanban, missions (avec canal par tâche).
- **Dashboard** complet : chat, mémoire (éditable), capacités, missions, settings, navigateur de fichiers plugins.
- **71 skills** lean en anglais, liés aux vraies abeilles ; abeilles/plugins en anglais.

---

## ⚙️ Réglage

- **Settings → General** : génération (max passes, température, max tokens, limite outils dynamiques, seuil contexte étroit), contexte/compaction, curateur.
- **Settings → Providers / Secrets / MCP / Channels** : configuration à chaud.
- **Mémoire → `system.*`** : éditer les prompts système.
- **Commandes Telegram** : `/help` `/status` `/crons` `/delcron <nom|all>` `/sethome` `/clear`.

---

## 📦 Docker

```
docker compose up
```

---

## 📚 Historique & docs de dev

Les anciens documents de conception (vision, audit, roadmaps, handoffs, briefings) sont **archivés** dans [`docs/_archive/`](docs/_archive/). Les lanceurs et scripts de dev obsolètes sont dans [`_archive/`](_archive/) et [`laruche/_archive/`](laruche/_archive/).
