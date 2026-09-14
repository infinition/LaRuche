# LaRuche 2048 App

This dependency-free reference App verifies the frontend extension path without rebuilding
LaRuche. It covers:

- installation from a `.laruche-app` ZIP archive;
- the sandboxed `LaRucheApp` SDK handshake;
- per-user private persistence;
- locale and theme context;
- keyboard, pointer and touch input;
- a side panel and an independent popup window;
- package and view icons.

## Correction 1.3.0

- Meme correction que pour les dames: le guide precise que rien ne previent
  l'agent quand l'humain joue, que ce sont les boutons de l'App qui le
  rappellent, et qu'un message annoncant son tour se verifie par `game.state`
  avant toute reponse.
- Le mode auto annonce son arret au lieu de se couper sans rien dire.

## Build and test

From the repository root:

```powershell
node apps-library/2048/test/game.test.js
python apps-library/2048/build.py
```

The archive is written to `apps-library/2048/dist/`. Open LaRuche, select **Apps**, choose
**Installer**, then enable 2048. The game state and best score survive navigation and node
restarts through `storage.private`.

The `package/` directory itself is the complete source package. It deliberately uses no CDN,
framework, network request or browser storage, so it remains compatible with the App sandbox.
The host restricts fetches to the App's own package assets.

Choose **Panneau** to keep playing alongside Chat or Settings. Choose **Fenêtre** for a
separate browser popup. The panel shares the same resize handle and mobile bottom-sheet layout
as detached Settings sections. The panel header arrow returns the App to its page.

Wait for **Synchronisé** before moving a game between views: a newly opened view restores the
last saved state. Independent windows do not synchronize live; avoid playing the same saved
game in two windows simultaneously. Closing the main LaRuche window closes its App popups.

See `wiki/guides/Developing-Apps.md` for authoring your own App and the current SDK limits.

## Agent play in 1.2.0

The package guide explains the objective, full-board slides, single-merge rule, random
tile probabilities, score, concrete row examples, strategy, permissions and recovery.
The engine repeats a rules summary in `game.state`. `won`, `needsContinue`, `keepPlaying`
and `waitingFor` distinguish reaching 2048 from a blocked board. The model must yield
to the human's Continue button at victory, not restart or send impossible moves.

Grant `game.state` and `game.move`, approve `agents.invoke` and select the allowed agent
in native App Permissions. Use One turn or Auto. The model must copy a current legal
direction and revision; the engine remains the authority even if it ignores the guide.

## What a refusal says, in 1.4.1

A refused revision names the current value. Without it the caller has to read the
state again before it can retry, and by then the revision has often moved once more.
The guide also says the App's private store on disk is out of bounds: everything
about the board is reachable through the actions, and a reply that looks incomplete
is a reason to read `game.state` again, not to go around it.

## Zoom et mise en page, en 1.5.0

Un zoom, garde entre deux ouvertures, dans le stockage prive de l'App. Il agit
sur toute la mise en page : dezoomer donne au plateau plus de pixels logiques
au lieu de tout rapetisser.

Deux defauts de geometrie partaient avec. Les lignes de la grille n'etaient pas
declarees, et une piste `1fr` garde un plancher a la taille de son contenu :
pour des colonnes de 132 px, on obtenait trois lignes de 145.875 et une
derniere, vide, de 90.375. Les cases n'etaient donc ni carrees ni egales.
`minmax(0, 1fr)` retire ce plancher.

Les tailles etaient exprimees en unites de fenetre, qui mesurent toute la
fenetre de LaRuche et non le panneau : dans un panneau de 870 px au sein d'une
fenetre de 2000, le titre restait a son maximum et le bas de l'App passait hors
du cadre. Ce sont maintenant des unites de conteneur, et les requetes media
sont devenues des requetes de conteneur.

Enfin le plateau tenait sa forme d'un `aspect-ratio` dans un flex, qui ne sait
pas retrecir sa hauteur quand `max-width` mord : il devenait deux fois plus
haut que large des qu'on dezoomait. Il prend desormais le plus petit des deux
cotes de la place qu'on lui laisse.

`apps-library/test/layout.test.cjs` mesure tout cela dans un vrai navigateur.
