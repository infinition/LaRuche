# LaRuche — Refacto UX / Architecture d'Information (IA)

> 2026-06-21. Cible : un produit qui se comprend tout seul. Problème actuel : Settings est un
> fourre-tout (Cron/Watchers/Kanban/Skills/MCP n'y ont pas leur place). Contrainte : garder le
> **style ambre/ruche + animations CSS actuelles**, **responsive mobile/PC**, fluide, cohérent.

## Modèle mental (3 familles)
1. **CE QU'ON FAIT** (surfaces) : Chat · Mémoire · Missions · Automatisations.
2. **CE QUE L'AGENT PEUT FAIRE** (écosystème) : Capacités (skills + abeilles + MCP + plugins).
3. **CE QU'ON CONFIGURE** (vrais réglages) : Settings.

## Navigation cible (top-level)
| Onglet | Contenu | Changement |
|---|---|---|
| **Chat** | conversations + **historique/recherche intégré** (overlay) | absorbe Sessions |
| **Mémoire** 🧠 | carte cognitive Obsidian (arbre + markdown + graphe) | inchangé |
| **Missions** 👑 | La Reine — **même libellé desktop ET mobile** (icône 👑) | fix naming |
| **Automatisations** ⚙️ | **Cron · Watchers · Kanban · Blueprints · Timeline** | sortis de Settings |
| **Capacités** 🛠️ | **Skills · Abeilles · MCP · Plugins** (un seul écosystème) | unifie Skills(×2) + MCP |
| **Dashboard** | mesh · ressources · audit | inchangé |
| **Settings** ⚙ | **uniquement** : Général · Providers (LLM+clés+visibilité) · Channels · Réseau · Onboarding | nettoyé |

## Détails par chantier

### 1. Chat absorbe Sessions (supprimer l'onglet Sessions)
- Un **overlay « Historique »** dans le Chat : liste des conversations + **recherche** (loop), **date de MAJ** affichée, et par conversation des actions **Exporter** (à côté de Supprimer) + ouvrir.
- L'export d'une conversation = **client-side** (Blob .md/.json depuis les messages chargés).
- Supprimer l'onglet/section Sessions séparé.

### 2. Capacités = un seul écosystème (skills/abeilles/MCP/plugins)
- Techniquement ils vivent déjà sous `tools.*` de la carte (`tools.skills.*`, `tools.abeilles.*`).
- **Affichage en TABLEAU ligne par ligne** (l'actuel est trop lourd) : colonnes `Nom · Type(Abeille/Skill/MCP/Plugin) · Origine(natif/custom) · Description courte · Statut(actif) · Actions`.
- 4 familles filtrables. Abeilles = badge « natif » (immuable) ; Skills/Plugins éditables ; MCP = serveurs (add/remove).
- **MCP sort de Settings>Providers** (Providers = backends LLM uniquement) → ici.
- Endpoints : `GET /api/tools` (abeilles+schémas), `GET /api/skills`, `GET /api/mcp/servers` (+ POST/DELETE).

### 3. Automatisations (sortir Cron/Watchers/Kanban de Settings)
- Regroupe Cron · Watchers · Kanban · Blueprints + la **Timeline**.

### 4. Timeline ⭐ (vue planning globale — CRUCIAL)
Vue **temporelle unifiée** de tout ce qui est prévu dans le temps : crons, missions à cadence,
(et watchers = monitors continus dans une section « surveillance active »).
- Source : agrège `GET /api/cron` (cron_expr) + `GET /api/missions` (cadence) + `GET /api/watchers`.
- Calcule la **prochaine occurrence** côté JS depuis l'expression cron (5 champs) ; affiche un
  libellé humain (« chaque lundi 9h ») + dernière exécution.
- Présentation : timeline/agenda (aujourd'hui / cette semaine / à venir) + section « monitors actifs ».
- But : voir d'un coup d'œil **tout le planning de LaRuche dans le temps**.

### 5. Settings nettoyé
- Retirer Cron/Watchers/Kanban/Skills + l'onglet MCP-dans-Providers.
- **Retirer le compteur de contexte de Settings>Général** (non-sens hors conversation) — il reste
  dans le **header du Chat**, lié à la session active.
- Garder : Général (thème, modèle défaut, permissions), Providers, Channels, Réseau, Onboarding.

## Contraintes transverses
- **Style** : tokens CSS ambre/ruche existants, animations conservées (halo action en cours, etc.).
- **Responsive** : nav desktop (barre) + mobile (tabs/burger) cohérents, **mêmes libellés**.
- **Fluide** : MAJ dynamique sans F5 (déjà en place), re-fetch après action.
- **Zéro régression** : ne pas casser le chat, la mémoire, les missions existants. Commits par lot.
- Tout est `spa.html` (un seul fichier, include_str! → rebuild). Endpoints backend déjà existants ;
  si l'un manque, le signaler (Claude l'ajoute).
