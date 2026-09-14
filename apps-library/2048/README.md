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
