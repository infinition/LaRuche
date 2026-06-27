# LaRuche — Vision & Innovation (se démarquer de third-party / third-party / Claude Code)

> Établi le 2026-06-21. Doc stratégique : la thèse de différenciation + les leviers concrets,
> séquencés par ROI, ancrés sur les briques que LaRuche a DÉJÀ. Source de vérité technique : `LARUCHE_V2.md`.

## La thèse
Tous les concurrents se battent contre la **fenêtre de contexte** : ils empilent, puis **compactent/résument**
quand ça déborde (et perdent de l'info). C'est un pansement. LaRuche a **deux actifs qu'aucun n'a réunis** :
une **mémoire cognitive SQL auditée** (FTS5 + embeddings + activation + dream + audit) **et** un **mesh
local-first** (Miel/mDNS). Le pari :

> **La carte cognitive EST le cerveau. La fenêtre LLM n'est qu'une mémoire de travail transitoire,
> reconstruite à chaque tour par activation. On ne compacte pas le contexte — on le *récupère*.**

C'est l'inverse de tout le monde. Pitch produit :
> *« LaRuche : le premier agent local-first à **mémoire cognitive partagée**. Pas une fenêtre de contexte
> qu'on compacte — un **cerveau** qui apprend, se cite, et se **synchronise entre tes machines en local**.
> Plus tu l'utilises, plus la ruche est intelligente. »*

---

## Levier 1 — Contexte cognitif infini (working-set, pas fenêtre à compacter) ⭐⭐⭐
À chaque tour, l'agent assemble un **set de travail minimal** depuis la carte (nœuds/items/skills/outils
activés, sous budget). Les vieux tours ne sont pas tronqués → **consolidés dans la carte par `dream`** et
re-récupérés si pertinents.
- **Preuve qui fait le buzz** : *« 2h de conversation, prompt stable à ~3k tokens, l'agent n'oublie rien. »*
  third-party/third-party résument et perdent ; LaRuche consolide et peut retrouver.
- **Briques existantes** : activation (atlas), retrieval hybride embeddings+FTS5, `dream`, injection trailing.
- **Manque** : faire du retrieval l'**assembleur primaire** du contexte + un gestionnaire de working-set
  (budget token → sélection nœuds/items par activation, le reste reste en mémoire).

## Levier 2 — Outils & skills sémantiques (capacités illimitées, coût contexte constant) ⭐⭐⭐  [ON COMMENCE ICI]
Aujourd'hui : ~30 schémas d'outils injectés à chaque tour (même pour un « Salut »). À la place : les outils
vivent dans la carte (`tools.abeilles.*`, `tools.skills.*`). Par tour = **noyau minuscule toujours présent**
(memory_search, clarify, run_script) **+ 3-6 outils récupérés par intention**.
- **Conséquence** : l'agent peut avoir **1000 outils, en injecter 5**. Les outils forgés deviennent
  *cherchables*, pas du poids mort. L'usage des outils **réalimente la mémoire** (routage appris).
- **Design clé (cache + pertinence)** : **noyau stable** (caché par le prefix-cache) **+ queue dynamique**
  courte (récupérée par intention). On garde l'astuce third-party du préfixe stable, mais la queue s'adapte.
- **Briques existantes** : abeilles en mémoire (T11 amorcé), skill_list/view, flag `dynamic_tool_selection`.
- **Manque** : **gater l'injection sur le retrieval**. Corrige aussi le bug « 30 outils pour un Salut ».

## Levier 3 — Mémoire COLLECTIVE du mesh (l'intelligence de la Ruche) ⭐⭐⭐⭐  [MOONSHOT / le titre]
Le différenciateur qui **porte le nom** et qu'**aucun concurrent ne peut faire** : plusieurs nœuds LaRuche
**partagent/fédèrent la carte cognitive** via Miel. Skill forgé sur le PC → dispo sur le laptop. Fait appris
par le nœud familial → connu de tous. **Sync CRDT de la mémoire SQL** (le journal d'audit `mutations` est une
base CRDT naturelle : append-only, horodaté, idempotent).
- **Hook viral** : *« Tes agents forment une ruche qui apprend collectivement — PC, laptop, serveur maison.
  Local, sans cloud. »* third-party = 1 cerveau/VPS ; Claude Code = 1 CLI isolé ; LaRuche = **cognition distribuée**.
- **Manque** : protocole de sync/fédération des `mutations` sur Miel (réconciliation, dédup par idempotency key).

## Levier 4 — Raisonnement structurel + cité (ce que le SQL débloque, vecteurs seuls non) ⭐⭐
Parce que la mémoire est **SQL** (pas un `MEMORY.md` plat comme third-party, pas que des vecteurs) :
- **Requêtes structurelles** : agréger, joindre, filtrer par date/provenance. *« Toutes mes décisions sur
  projet X, triées par date. »* Retrieval **hybride : sémantique + SQL**.
- **Citations** : chaque réponse cite ses souvenirs (provenance via l'audit) → **anti-hallucination par
  construction** + explicabilité (*« pourquoi ? → voici la chaîne »*). Les concurrents ne peuvent ni
  requêter ni citer leur mémoire plate.

## Levier 5 (bonus) — Mémoire épisodique + contradiction (l'autobiographie de l'agent) ⭐
L'audit = machine à remonter le temps. *« Qu'est-ce que je savais le 12 juin ? »* Détection de
**contradictions** (« tu disais X, maintenant Y »), décroissance des faits périmés, *« qu'est-ce qui a changé
depuis ? »*. Split épisodique vs sémantique. Un agent avec une vraie histoire.

---

## Séquencement (ROI décroissant, en partant de l'existant)
| Phase | Levier | Pourquoi |
|---|---|---|
| **1 (en cours)** | **L2** outils sémantiques + fixes prompt | corrige le vrai bug (30 outils) + fondation, démontrable vite |
| **2** | **L1** contexte working-set | le gros pari archi, le claim « contexte infini » |
| **3** | **L4** SQL + citations | différenciateur rapide et marketable |
| **4 (moonshot)** | **L3** mémoire collective mesh | le titre, le moment « wow », l'engouement |

L1, L2, L4 reposent à ~80% sur des briques déjà présentes (atlas, dream, FTS5+embeddings, audit,
abeilles-en-mémoire). L3 demande un protocole de sync mais transforme le produit.

## Chantiers délégables à Gemini (UI, faible risque — PAS de main.rs critique)
- **L2** : panneau « Outils actifs ce tour » (transparence : montrer les 3-6 abeilles injectées + pourquoi) — lit un champ d'event/endpoint.
- **L4** : panneau « Sources » sous chaque réponse (les items mémoire cités) + un onglet « Requête mémoire » (champ → `GET /api/memory/query`).
- **L1** : indicateur « contexte : N souvenirs actifs / M en mémoire » dans le header (au lieu de CTX %).
Tous = `spa.html` + endpoints fournis par Claude. Briefings détaillés à la demande.

---

## CHANTIER PARKÉ — Context Compiler (Levier 1, version produit)
> Gardé sous le coude. À faire AVANT le refacto orchestration (prouve L1, moins risqué, buildable sur l'existant).

Un **Tiny LLM routeur** (= l'`aux_model` déjà présent) compile le contexte de CHAQUE tour substantiel :
il voit tout (outils + candidats mémoire), **raisonne** sur la pertinence, et ne laisse passer au gros
modèle QUE l'essentiel (3-5 outils de niche + souvenirs utiles), **nettoie le superflu**.
- Bloate le petit modèle (cheap), garde le gros (cher) lean.
- Meilleur que les embeddings (il comprend « internet »→web là où le retrieval FR↔EN rate).
- Pièges gérés : latence (skip les tours triviaux), cache (noyau stable + queue routée), filet (`tool_search`).
- Hybride final : **noyau stable** (fait) + **routeur** (Idée 1) + **tool_search/carte de catégories** (Idée 2, fallback).

---

# ARCHITECTURE D'ORCHESTRATION — « La Reine » (le substrat central) ⭐⭐⭐⭐⭐

> Le vrai saut. Aujourd'hui cron/watcher/kanban = **daemons parallèles**. Demain : **tout orbite la carte
> cognitive**. Une MISSION devient un **processus long-vécu qui VIT dans la mémoire**, avancé par des
> itérations d'agent, avec **`dream` comme moteur de progrès** (pas juste du nettoyage).

## Thèse
third-party/third-party consolident + forgent des skills au niveau **session** (réactif). LaRuche va faire du
**long-horizon piloté par objectif** : une mission qui s'approfondit sur des **semaines**, **capitalise**
dans un graphe structuré, **s'auto-étend** via dream, et **apprend** (skills). = un agent qui *mène une thèse*,
pas un agent qui *répond*.

## 1. Missions = nœuds persistants (`missions.<slug>`)
Première classe dans la carte cognitive : `{objectif, statut (active/pausée/done), cadence (cron), itérations,
synthèse}` + sous-arbres `findings.*` / `questions.*` / `sources.*` / `contradictions.*`.

## 2. La Reine = orchestrateur unique (la boucle qui « marche » le graphe)
Par tick :
1. Lit les missions **actives & dues**.
2. Assemble l'état de la mission → demande à l'agent « **prochaine itération ?** » → crée des **tâches kanban**.
3. **Dispatche** (réutilise le dispatcher kanban) → tours d'agent (contexte working-set) → **findings écrits
   dans le sous-arbre de la mission** (capitalisés, dédupliqués).
4. Lance **`dream` scopé à la mission** → consolide + détecte **trous & contradictions** → génère de
   **NOUVELLES questions/tâches** (la mission s'auto-étend).
5. **Régénère la synthèse** (le dossier).
6. **Forge des skills** depuis les patterns réussis (l'agent devient meilleur sur le sujet à chaque itération).
7. Met à jour l'état, planifie le prochain tick.

## 3. L'orbite enfin réalisée
`watcher` (events → nœuds) → **déclenche** mission → `kanban` (tâches) → `agent` (tours) →
`mémoire` (capitalisation) → `dream` (moteur de progrès) → `skill` (compétence apprise) → **boucle**.
Tout passe par la carte cognitive = l'état d'orchestration. Plus de daemons isolés.

## 4. La démo qui bluffe
> *« Mission ton agent sur un sujet (ex. veille scientifique). Reviens dans un mois : il a construit un
> **dossier structuré, sourcé, qui s'approfondit tout seul chaque semaine**, résout ses propres
> contradictions, et est devenu **expert** du sujet. »*
Le dossier **EST** le sous-arbre de la carte → **visualisable** (nid d'abeille), **citable** (Levier 4),
**exportable** (OKF). Aucun concurrent ne fait de la **recherche autonome long-horizon capitalisante**.

## 5. Build incrémental (réutilise TOUT l'existant)
- **P1** : type `Mission` (nœud map) + CRUD + un cron qui « tick » une mission.
- **P2** : la Reine = boucle mission → plan → kanban (dispatcher existant) → findings en mémoire.
- **P3** : `dream` scopé mission = moteur (trous/contradictions → nouvelles tâches).
- **P4** : synthèse régénérée + forge de skills.
- **P5** : UI « dossier » (nid d'abeille de la mission) + déclencheurs (cron hebdo / watcher).

## 6. Pourquoi ça démarque
| | third-party / third-party | LaRuche « Reine » |
|---|---|---|
| Consolidation | au niveau session | **long-horizon, capitalisante** |
| Skills | post-tour | **appris sur des semaines, par mission** |
| Sortie | réponses | **dossier structuré qui s'approfondit seul** |
| Moteur | réactif | **dream pilote le progrès** (gaps→tâches) |
| Substrat | mémoire = annexe | **mémoire = l'orchestrateur** |
