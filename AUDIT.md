# Audit & état des lieux — LaRuche vs third-party / third-party / Claude Code

Scan du 27/06/2026 sur les 4 projets du bureau. Légende effort : 🟢 quick · 🟡 moyen · 🔴 gros.

## TL;DR
LaRuche **n'est pas en retard** — c'est déjà le produit le plus complet des quatre. Il réunit ce
qu'aucun autre n'a : multi-canal (Telegram/Discord/Slack) avec mémoire par canal, moteur edge en
Rust, multimodal, vault secrets chiffré, dashboard web complet, mesh (`miel-protocol`), et un
curateur d'auto-amélioration qui crée skills ET tools vérifiés. Les vrais gaps restants = **6 chantiers**.

## Projets de référence
| Projet | Nature | Stack | Réf de |
|---|---|---|---|
| third-party agent | Agent ReAct autonome | Python | auto-amélioration (background_review + curator) |
| third-party | Harness d'agent | TypeScript | boucle propre (stopReason + hooks + steering) |
| claude-code | Source leakée (31/03/2026) | TS | harness produit (Tool/Task/QueryEngine, permissions, hooks, MCP) |
| **laruche-v2** | Agent edge networké | **Rust** (16 crates) | — |

## Comparatif (✅ fait · 🟡 partiel · ❌ absent)
| Capacité | LaRuche | third-party | third-party | CC |
|---|:--:|:--:|:--:|:--:|
| Boucle stop_reason (texte=fin) | ✅ | ✅ | ✅ | ✅ |
| Anti-boucle (contrôleur pur) | ✅ vigie | ✅ | ✅ | ✅ |
| Outils ∥ + bascule séquentielle | ✅ recolte | 🟡 | ✅ | ✅ |
| Sous-agents isolés + budgets | ✅ eclaireuse | ✅ | ✅ | ✅ |
| Compaction entre tours | ✅ escale | ✅ | ✅ | ✅ |
| System prompt en tiers (cache) | ✅ | ✅ | ✅ | ✅ |
| Steering live | ✅ | 🟡 | ✅ | ✅ |
| **Auto-création skills/tools vérifiés** | ✅ curateur | ✅(réf) | ❌ | 🟡 |
| Mémoire + dream/consolidation | ✅ | ✅ | 🟡 | 🟡 |
| Multimodal (images mult.+audio) | ✅ | 🟡 | 🟡 | 🟡 |
| RAG | ✅ | ✅ | 🟡 | ❌ |
| MCP **client** | ✅ | 🟡 | ✅ | ✅ |
| MCP **serveur** (s'exposer) | ❌ | ❌ | 🟡 | ✅ |
| Permissions / popup | ✅ | 🟡 | ✅ | ✅ |
| **Hooks utilisateur** | 🟡 interne | 🟡 | ✅ | ✅ |
| Sandbox durci | 🟡 | ✅ Docker | ✅ | ✅ |
| Vault secrets chiffré + `${NOM}` | ✅ | ❌ | ❌ | 🟡 |
| Browser / LSP / worktree git | ✅ | 🟡 | 🟡 | ✅ |
| **Multi-canal (TG/Discord/Slack)** | ✅ | ❌ | ❌ | ❌ |
| Cron/watchers/kanban/missions | ✅ | ✅ cron | ❌ | ❌ |
| Dashboard web complet | ✅ | 🟡 | ✅ | 🟡 TUI |
| **Mesh / fédération nœuds** | 🟡 miel | ❌ | ❌ | ❌ |
| Tokens/usage réels | 🟡 Ollama | ✅ | ✅ | ✅ |
| Edge / Rust natif | ✅ | ❌ | ❌ | ❌ |

## Moats (à mettre en avant, NE PAS reconstruire)
1. Multi-canal natif + mémoire persistante par canal (UUIDv5 déterministe).
2. `miel-protocol` (mesh) — fédération de nœuds edge.
3. Edge / Rust — un binaire, faible empreinte.
4. Curateur qui crée skills ET tools **vérifiés** (au-delà de third-party).
5. Vault secrets chiffré (LLM voit les NOMS seulement).
6. Hub d'automatisation unifié.

## 🎯 Les 6 gaps « killer » (priorisés)
- **A** 🔴 P0 — **Finir la fédération mesh** : propager les skills vérifiés de nœud à nœud. Test multi-nœuds requis. *Le récit différenciant.* ⏳ RESTE (besoin setup multi-nœuds).
- **B** 🔴 P0 — **Serveur MCP** : LaRuche pilotable par Claude Code/Cursor/third-party. ✅ **FAIT** (`POST /mcp`, commit `7fc4514`).
- **C** 🟠 P1 — **Tokens/usage réels hors-Ollama**. ✅ **FAIT** Anthropic+OpenAI (`efb848e`) ; codex OAuth reste.
- **D** 🟠 P1 — **Hooks utilisateur** (pre/post-tool, events, configurables) façon Claude Code/third-party. ⏳ RESTE (système à concevoir).
- **E** 🟡 P2 — **Sandbox durci** pour shell_exec/execute_code/browser. ⏳ RESTE (durcissement large).
- **F** 🟡 P2 — **Reprise effective des carnets**. ✅ **FAIT** (`reprendre_carnet` + endpoints, `8a8f44b`).

## Séquence recommandée
1. **B** (serveur MCP) — petit effort, branche tout l'écosystème, démontrable.
2. **C + F** — crédibilité (coûts réels) + fiabilité (reprise).
3. **A** (mesh) — chantier de fond, démo sur 2-3 nœuds.
4. **D + E** — maturité production.

Pitch une fois A+B faits : *« agent IA edge en Rust, multi-canal, qui s'auto-améliore, se fédère
en essaim, et s'expose/consomme en MCP »* — aucune des 3 réfs ne coche ces cases ensemble.

## ✅ Déjà fait (rappel, ne pas refaire)
Boucle/anti-boucle/parallélisme/sous-agents/compaction/steering · curateur (skills+tools vérifiés) ·
multimodal · vault secrets+webhooks · multi-canal+mémoire par canal+/sethome · modèles dynamiques+sonde
n_ctx · missions/cron/watchers/kanban/timeline · MCP client+onglet · feed persistant · édition complète
mémoire · **Telegram résolu** (index compact ~4K + auto-sélection dynamique + fenêtre glissante + sonde n_ctx).
