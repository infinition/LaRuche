# Lanceurs LaRuche

Les scripts se lancent depuis n'importe quel répertoire : ils retrouvent le code
source dans `laruche/` à partir de leur propre emplacement.

| Dossier | Scripts |
| --- | --- |
| `windows/` | `.bat` : Butinage, bureau, bureau client, découverte réseau, embeddings et compilation des apps |
| `macos/` | `.command` : Butinage, bureau, bureau client, découverte réseau, embeddings et compilation des apps, par double-clic dans le Finder |
| `Linux/` | `.sh` : Butinage, bureau, bureau client, découverte réseau, embeddings et compilation des apps, à exécuter dans un terminal |

- **Butinage** compile le nœud, démarre le serveur puis ouvre le navigateur lorsque
  le serveur répond. Sur Linux sans `xdg-open`, l'adresse est affichée.
  Utiliser Ctrl+C pour arrêter le serveur.
- **Bureau** compile le projet puis ouvre LaRuche Desktop. La fenêtre gère le
  démarrage et l'arrêt de son nœud local.

- **Bureau client** compile et ouvre la coque sans nœud local. Elle découvre les
  ruches du réseau, ou rejoint `LARUCHE_URL` si cette variable est définie. La ruche
  distante doit être lancée avec `LARUCHE_BIND_LAN=1`.
- **Découvrir les ruches** compile la coque puis liste les ruches annoncées en
  mDNS et vérifie leur accessibilité, sans ouvrir le bureau.
- **Embeddings** accepte `auto` (par défaut), `ollama` ou `llamacpp` en argument.
  Il utilise Ollama s'il est installé, sinon cherche `llama-server` dans le PATH
  et sous `LOCAL_AI_DIR`. Sur Linux et macOS, ce dossier vaut par défaut
  `~/.local/share/laruche/local-ai`. Le modèle est téléchargé s'il manque.
  `curl` et Ollama ou llama.cpp sont requis. Ollama reste en arrière-plan sur le
  port 11434 ; llama.cpp reste dans le terminal sur le port 8002. Pour ce dernier,
  définir `LARUCHE_EMBED_URL=http://localhost:8002` dans le lanceur LaRuche.

**Compiler les apps** : lancer `compiler_apps.bat`, `compiler_apps.command` ou
`compiler_apps.sh` dans le dossier de sa plateforme. Python 3.9 ou plus récent suffit : le lanceur
compile toutes les apps de `apps-library/`, vérifie les archives et affiche leurs
emplacements dans `apps-library/<app>/dist/`. Installer ensuite les fichiers
`.laruche-app` depuis **Apps > Installer** dans LaRuche.

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
