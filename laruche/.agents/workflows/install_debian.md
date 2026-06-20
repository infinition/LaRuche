---
description: Installation et lancement de LaRuche sur Debian
---

Ce guide vous permet d'installer les outils nécessaires et de lancer LaRuche sur un système Debian/Ubuntu.

### 1. Mise à jour du système
Ouvrez un terminal et mettez à jour vos paquets :
```bash
sudo apt update && sudo apt upgrade -y
```

### 2. Installation des dépendances système
Installez les outils de compilation et les bibliothèques nécessaires :
```bash
sudo apt install -y build-essential pkg-config libssl-dev
```

### 3. Installation de Rust (et Cargo)
Utilisez le script officiel `rustup`. Appuyez sur **1** quand on vous le demande pour l'installation par défaut :
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Après l'installation, rechargez votre configuration :
```bash
source $HOME/.cargo/env
```
Vérifiez que cela fonctionne : `cargo --version`

### 4. Installation d'Ollama
LaRuche utilise Ollama pour faire tourner les modèles localement :
```bash
curl -fsSL https://ollama.com/install.sh | sh
```
Lancez Ollama et téléchargez un modèle léger pour tester :
```bash
ollama run mistral
```

### 5. Lancement de LaRuche
Allez dans votre dossier LaRuche et lancez le Node :
```bash
cd /home/infinition/Documents/LaRuche
cargo run -p laruche-node
```

Dans un **autre terminal**, vous pouvez utiliser le CLI pour lui parler :
```bash
cargo run -p laruche-cli -- ask "Qui es-tu ?"
```

Et pour le Dashboard :
```bash
cargo run -p laruche-dashboard
```
Ouvrez ensuite [http://localhost:8420](http://localhost:8420) dans votre navigateur.
