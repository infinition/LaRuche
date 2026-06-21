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
