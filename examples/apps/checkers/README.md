# LaRuche Checkers App

Application de Jeu de Dames 8x8 autonome concue pour le bac a sable LaRuche.

## Caracteristiques

- Regles officielles du jeu de dames 8x8 (prises obligatoires et rafles).
- Controle strict du tour par tour : l'agent LLM attend obligatoirement le coup de l'utilisateur.
- Adversaire parametrable : Agent distant LaRuche ou IA heuristique locale (Minimax).
- Integration SDK LaRuche (handshake `/apps-runtime/v1.js`, actions et persistance privee).
- Interface responsive pour panneau lateral (min 360 px), fenetre detachee et mobile tactile.
- Internationalisation native (francais et anglais).

## Build et tests

Depuis la racine du depot :

```powershell
node examples/apps/checkers/test/game.test.js
python examples/apps/checkers/build.py
```

L'archive `.laruche-app` est generee dans `examples/apps/checkers/dist/`.

## Installation dans LaRuche

1. Ouvrir LaRuche et acceder a **Apps**.
2. Cliquer sur **Installer** et selectionner le fichier `.laruche-app` genere.
3. Activer l'application et configurer les permissions si necessaire.
