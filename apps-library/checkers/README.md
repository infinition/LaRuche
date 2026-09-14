# LaRuche Checkers App

Application de Jeu de Dames 8x8 autonome concue pour le bac a sable LaRuche.

## Correction 1.2.0

- L'agent promettait de « surveiller le jeu » apres son coup. Il n'a aucune
  boucle de fond et rien ne le previent quand l'humain joue: la promesse ne
  pouvait pas etre tenue, et on attendait un tour qui ne venait jamais. Le guide
  dit maintenant qui le rappelle, les boutons Tour agent et Reponse auto, et
  exige un `game.state` avant de repondre a un message qui annonce son tour.
- La reponse auto se coupait en silence sur la moindre erreur. Elle se coupe
  toujours, sinon elle rejouerait la panne en boucle, mais le statut le dit.

## Caracteristiques

- Variante LaRuche 8x8 : dames courtes, pions vers l'avant, prises obligatoires,
  rafles completes, blancs en premier. Ce ne sont pas les dames internationales 10x10.
- Controle strict du tour par tour : l'agent joue uniquement le camp oppose au camp
  humain choisi. Si l'humain choisit les noirs, l'agent blanc joue en premier.
- Adversaire parametrable : Agent distant LaRuche ou IA heuristique locale (Minimax).
- Integration SDK LaRuche (handshake `/apps-runtime/v1.js`, actions et persistance privee).
- Interface responsive pour panneau lateral (min 360 px), fenetre detachee et mobile tactile.
- Internationalisation native (francais et anglais).

## Build et tests

Depuis la racine du depot :

```powershell
node apps-library/checkers/test/game.test.js
python apps-library/checkers/build.py
```

L'archive `.laruche-app` est generee dans `apps-library/checkers/dist/`.

## Installation dans LaRuche

1. Ouvrir LaRuche et acceder a **Apps**.
2. Cliquer sur **Installer** et selectionner le fichier `.laruche-app` genere.
3. Activer l'application et configurer les permissions si necessaire.

## Contrat LLM en 1.1.0

Le guide du manifeste couvre but, encodage de la grille, orientation, mouvements,
prises, promotion, strategie, tours humains et erreurs. `game.state` rappelle les
regles et renvoie `agentSide`, `opponentMode` et `waitingFor`.

L'agent copie `from`, `to` et le `path` complet d'un coup de `legalMoves`, avec la
revision courante. Deux rafles de dame peuvent avoir les memes extremites : sans
chemin, un tel coup est refuse comme ambigu. Une rafle entiere compte pour un tour.
Le mode IA locale refuse les actions de jeu du LLM. `game.new` demande une permission
distincte et ne doit servir que si l'utilisateur demande une nouvelle partie.

Il n'y a pas encore de compteur de repetition ou de regle de nulle automatique.
Ces limites sont explicites dans le guide. Les tests couvrent aussi les directions
des pions, les rafles non maximales, la promotion et les chemins ambigus.
