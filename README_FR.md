<div align="center">
<img width="3533" height="997" alt="image" src="https://github.com/user-attachments/assets/458f9822-8c9e-44da-896c-20ba238925d3" />
</div>

# LaRuche + L'Essaim

<div align="center">
  <img width="287" height="317" alt="icon-removebg-preview" src="https://github.com/user-attachments/assets/a6a37836-7e97-4203-b041-1edb7ec36263" />
</div>

**"Branchez l'IA. C'est tout."**

Plateforme agent IA open-source. Local-first, respect de la vie privee.
Branchez un noeud LaRuche sur votre reseau local et l'IA devient disponible pour tous les appareils connectes.

---

## Fonctionnalites

### Moteur Agent (Essaim)

- **Boucle ReAct** -- Raisonnement multi-etapes avec appels d'outils, planification et delegation de sous-agents
- **23+ outils integres (Abeilles)** -- Fichiers, shell, recherche web, fetch, deep search, maths, git, calendrier, navigateur, base de connaissances RAG, surveillance fichiers, infos systeme, delegation...
- **Compaction du contexte** -- Resume automatique quand le contexte depasse la capacite du modele
- **Failover de modele** -- Basculement automatique vers un modele alternatif en cas d'erreur
- **Validation des outils dangereux** -- Confirmation utilisateur requise avant execution

### Multi-Provider LLM

- **Providers simultanes** -- Ollama (local), OpenAI, Anthropic, Groq, OpenRouter, ou toute API compatible OpenAI, tous actifs en meme temps
- **Profiles de providers** -- Gerez plusieurs cles API et endpoints via l'interface Settings
- **Liste de modeles unifiee** -- Tous les modeles de tous les providers dans un seul menu, groupes par provider

### Reseau (Protocole Miel)

- **Decouverte zero-config** -- Broadcast mDNS (DNS-SD) pour detection automatique sur le LAN
- **Intelligence en essaim** -- Liste collective des modeles, partage de tenseurs entre noeuds
- **Sync cross-noeud** -- Les sessions et utilisateurs se repliquent entre noeuds de maniere transparente
- **Auth par proximite** -- Tokens d'appareil via bouton physique ou NFC

### Authentification Multi-Utilisateur

- **Login par QR code** -- Enrollment par nom, QR permanent sauvegarde sur smartphone
- **Login par challenge** -- QR ephemere (60s) scanne depuis le telephone pour authentifier le navigateur
- **Cookies signes BLAKE3** -- Cookies avec expiration 30 jours, partages dans le cluster
- **Sessions par utilisateur** -- Chaque utilisateur voit uniquement ses conversations

### Interfaces

- **SPA Web** -- Application web monopage avec chat, dashboard, sessions, settings, console
- **TUI Client (CLI)** -- Interface terminale complete avec Ratatui (sidebar, markdown, autocompletion)
- **TUI Serveur** -- Interface terminale pour laruche-node avec logs scrollables et jauges systeme
- **Pipeline vocal** -- STT (Whisper) + TTS (edge-tts / Kokoro / pyttsx3) via WebSocket
- **Bot Telegram** -- Integration native Rust, demarrage auto depuis la config
- **Discord / Slack** -- Integrations par webhook
- **Serveur MCP** -- JSON-RPC pour Claude Desktop, Cursor, etc.
- **Extension VS Code** -- Assistance IA inline, sidebar chat, selection de noeuds

## Installation

### Prerequis

- [Rust 1.75+](https://rustup.rs/)
- [Ollama](https://ollama.com/) (optionnel si cloud uniquement)

### Depuis les sources

```bash
git clone https://github.com/infinition/LaRuche.git
cd LaRuche

# Installer les deux binaires dans le PATH
cargo install --path laruche-node --force
cargo install --path laruche-cli --force
```

Cela installe deux commandes :

| Commande | Description |
|----------|-------------|
| `laruche-node` | Le serveur (API + Web UI + Moteur agent) |
| `laruche` | Le client TUI (se connecte au serveur) |

Les binaires sont dans `~/.cargo/bin/` qui est dans le PATH systeme.

### Telecharger un modele

```bash
ollama pull gemma3:12b
```

## Utilisation

### Option 1 : Serveur avec TUI (interactif)

```bash
laruche-node
```

Affiche une interface terminale avec logs en temps reel, jauges CPU/RAM/GPU.
Appuyez sur `q` pour quitter.

### Option 2 : Serveur en arriere-plan + Client TUI

```bash
laruche-node --no-tui &
laruche
```

### Option 3 : Tout depuis le Client TUI

```bash
laruche
# Puis dans le TUI :
/server start       # Lance laruche-node en arriere-plan
```

### Interface web

```
http://localhost:8419
```

## Commandes CLI

```bash
laruche                    # TUI interactif (defaut)
laruche --classic          # Mode REPL classique
laruche ask "Question"     # Question one-shot
laruche --cwd /chemin      # Demarrer dans un dossier
laruche discover           # Scanner le reseau
laruche doctor             # Diagnostics systeme
laruche server start       # Gerer le serveur
laruche mcp                # Serveur MCP (stdio)
```

### Commandes slash dans le chat

| Commande | Description |
|----------|-------------|
| `/help` | Aide |
| `/tools` | Lister les outils (Abeilles) |
| `/model [nom]` | Changer de modele |
| `/server [cmd]` | Gerer le serveur (start/stop/restart/status/install/update/uninstall) |
| `/clear` | Nouvelle conversation |
| `/export` | Exporter en Markdown |
| `/discover` | Scanner les noeuds LaRuche |
| `/doctor` | Diagnostics |
| `/quit` | Quitter |

## Workflow developpeur

```bash
# Verification rapide (debug, compile vite)
cargo build

# Installer dans le PATH (fait un build release automatiquement)
cargo install --path laruche-node --force
cargo install --path laruche-cli --force

# Ou depuis le TUI :
/server install    # build release + installe laruche-node
```

| Commande | Resultat | Sortie |
|----------|----------|--------|
| `cargo build` | Build debug (rapide a compiler, lent a executer) | `target/debug/` |
| `cargo build --release` | Build release (lent a compiler, rapide a executer) | `target/release/` |
| `cargo install --path X --force` | Build release + copie dans le PATH | `~/.cargo/bin/` |

## Configuration

### laruche.toml (optionnel)

```toml
node_name = "laruche-salon"
tier = "core"                          # nano | core | pro | max
ollama_url = "http://127.0.0.1:11434"
default_model = "gemma3:12b"
api_port = 8419

[[capabilities]]
capability = "llm"
model_name = "gemma3:12b"
model_size = "12B"
quantization = "Q4_K_M"
```

La plupart de la configuration se fait via la page **Settings** de l'interface web.

## Structure du workspace

```text
LaRuche/
  miel-protocol/        # Protocole Miel (decouverte mDNS, auth, QoS, swarm)
  laruche-node/          # Serveur daemon (API, WebSocket, channels, auth, sync, TUI)
  laruche-essaim/        # Moteur agent (cerveau ReAct, 23+ outils, sessions, RAG, providers)
  laruche-cli/           # Client CLI avec TUI (Ratatui)
  laruche-client/        # SDK client Rust
  laruche-dashboard/     # Templates web (SPA HTML, embarque dans laruche-node)
  laruche-vscode/        # Extension VS Code
  laruche-voix/          # Module vocal (Whisper STT + edge-tts/Kokoro TTS)
  laruche-channels/      # Bots channels legacy (Telegram est maintenant natif Rust)
  plugins/               # Plugins d'outils personnalises (JSON)
```

## Licence

MPL-2.0
