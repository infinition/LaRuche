# Lanceurs LaRuche

Les scripts se lancent depuis n'importe quel répertoire : ils retrouvent le code
source dans `laruche/` à partir de leur propre emplacement.

| Dossier | Scripts |
| --- | --- |
| `windows/` | `.bat` : Butinage, bureau, bureau client, découverte réseau et embeddings |
| `macos/` | `.command` : Butinage et bureau, par double-clic dans le Finder |
| `Linux/` | `.sh` : Butinage et bureau, à exécuter dans un terminal |

- **Butinage** compile le nœud, démarre le serveur puis ouvre le navigateur lorsque
  le serveur répond. Sur Linux sans `xdg-open`, l'adresse est affichée.
  Utiliser Ctrl+C pour arrêter le serveur.
- **Bureau** compile le projet puis ouvre LaRuche Desktop. La fenêtre gère le
  démarrage et l'arrêt de son nœud local.

Rust/Cargo et les dépendances de compilation de la plateforme doivent être
installés (voir le README principal). Les lanceurs Unix utilisent aussi `rsync`
pour les skills et `curl` pour la sonde de Butinage. La première compilation peut
être longue. Les paramètres facultatifs restent commentés dans les scripts.

Les lanceurs macOS et Linux respectent `LARUCHE_DATA_DIR` et les ruches déjà
présentes dans le dossier source. Sinon, le foyer se trouve dans
`~/Library/Application Support/LaRuche` sur macOS et
`${XDG_DATA_HOME:-~/.local/share}/laruche` sur Linux.

Les scripts de maintenance restent dans `scripts/` et ceux de distribution dans
`bin/` : ce dossier rassemble les lanceurs utilisateur auparavant à la racine.
