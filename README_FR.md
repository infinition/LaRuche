<div align="center">
<img width="3533" height="997" alt="image" src="https://github.com/user-attachments/assets/458f9822-8c9e-44da-896c-20ba238925d3" />
</div>

# LaRuche - Systeme IA edge en reseau

<div align="center">
  <img width="287" height="317" alt="icon-removebg-preview" src="https://github.com/user-attachments/assets/a6a37836-7e97-4203-b041-1edb7ec36263" />
</div>

**"Branchez l'IA. C'est tout."**

Branchez un noeud LaRuche sur votre reseau local et l'IA devient disponible pour les appareils connectes.
Zero configuration, zero cloud impose, et confidentialite au coeur.

## Architecture

```text
Reseau local

  LaRuche Core (LLM/RAG) <---- protocole LAND (mDNS + HTTP) ----> LaRuche Pro (VLM/Code)
            |                                                             |
            +----------------------- intelligence Swarm -------------------+

Clients
  - Extension VS Code
  - Interface web
  - CLI / SDK
  - Integrations IoT
```

## Structure du workspace

```text
laruche/
|-- land-protocol/        # Bibliotheque coeur LAND
|   `-- src/
|       |-- lib.rs
|       |-- capabilities.rs
|       |-- manifest.rs
|       |-- discovery.rs
|       |-- auth.rs
|       |-- qos.rs
|       |-- swarm.rs
|       `-- error.rs
|
|-- laruche-node/         # Daemon noeud
|   `-- src/main.rs
|
|-- laruche-client/       # SDK client Rust
|   `-- src/lib.rs
|
|-- laruche-cli/          # Outil CLI
|   `-- src/main.rs
|
`-- laruche-dashboard/    # Dashboard web
    `-- src/
        |-- main.rs
        `-- templates/dashboard.html
```

## Demarrage rapide

### Prerequis

- Rust 1.75+
- Ollama lance en local ou sur le LAN

Notes Windows:

- Option 1 (recommandee): toolchain MSVC
  - Installer Visual Studio Build Tools avec "Desktop development with C++"
  - `rustup default stable-x86_64-pc-windows-msvc`
- Option 2: GNU/MSYS2
  - Installer MSYS2 et MinGW binutils
  - Ajouter `C:\msys64\mingw64\bin` au `PATH`
  - `rustup default stable-x86_64-pc-windows-gnu`

### 1. Telecharger un modele

```bash
ollama pull mistral
```

### 2. Compiler

```bash
cargo check --workspace
```

### 3. Lancer un noeud

```bash
cargo run -p laruche-node
```

### 4. Ouvrir le dashboard

```text
http://localhost:8419/dashboard
```

## Notes d'implementation actuelles

- Re-annonce mDNS LAND periodique (`2s`) pour garder les noeuds visibles.
- Timeout stale du listener LAND a `45s` (depuis `land-protocol`).
- `/swarm` inclut `port` par noeud et fusionne les capabilities (HTTP + fallback mDNS).
- `/health` renvoie le texte `OK`.
- Le dashboard conserve un noeud transitoirement manquant pendant `10` polls.
- L'extension VS Code utilise:
  - grace mDNS avant perte de noeud: `12s`
  - grace stale swarm: `6` polls

## Configuration du noeud

Le noeud charge la configuration dans cet ordre:

1. `laruche.toml` (ou `LARUCHE_CONFIG=<path>`)
2. Variables d'environnement (prioritaires sur le fichier)

### Variables d'environnement

- `LARUCHE_NAME` (defaut aleatoire `laruche-xxxxxx`)
- `LARUCHE_TIER` (`nano|core|pro|max`)
- `LARUCHE_PORT` (defaut `8419`)
- `LARUCHE_DASH_PORT` (defaut `8420`)
- `OLLAMA_URL` (defaut `http://127.0.0.1:11434`)
- `LARUCHE_MODEL` (modele par defaut)
- `LARUCHE_CAP` (capability principale, ex: `llm`)
- `LARUCHE_CAP2` + `LARUCHE_MODEL2` (2e capability/modele optionnels)

### Exemple `laruche.toml`

```toml
node_name = "laruche-salon"
tier = "core"
ollama_url = "http://127.0.0.1:11434"
default_model = "mistral"
api_port = 8419
dashboard_port = 8420

[[capabilities]]
capability = "llm"
model_name = "mistral"
model_size = "7B"
quantization = "Q4_K_M"

[[capabilities]]
capability = "code"
model_name = "deepseek-coder"
```

## API

### Endpoints principaux

- `GET /` - Etat du noeud (CPU/RAM, queue, capabilities)
- `GET /health` - Health check (`OK`)
- `GET /nodes` - Pairs decouverts (vue mDNS)
- `GET /swarm` - Vue collective (local + pairs, avec `port`)
- `GET /models` - Modeles Ollama locaux
- `GET /swarm/models` - Modeles du swarm
- `POST /infer` - Requete d'inference
- `GET /activity` - Journal d'activite recent
- `POST /auth/request` - Demande d'auth (POC)
- `POST /auth/approve` - Approbation auth (POC)
- `GET /dashboard` - Dashboard embarque

### Exemple de requete d'inference

```json
{
  "prompt": "Explique ownership en Rust",
  "capability": "code",
  "model": "deepseek-coder",
  "qos": "normal",
  "max_tokens": 1024,
  "temperature": 0.7
}
```

## Extension VS Code

```bash
cd laruche-vscode
npm install
npm run compile
```

Puis lancer l'Extension Development Host avec `F5` depuis VS Code.

## SDK client Rust

```rust
use laruche_client::{Cap, LaRuche};

#[tokio::main]
async fn main() {
    let client = LaRuche::discover().await.unwrap();
    let answer = client.ask_with("Ecris un iterateur Rust", Cap::Code).await.unwrap();
    println!("{}", answer.text);
}
```

## Notes

- Ce workspace depend actuellement de `land-protocol` via GitHub (`workspace.dependencies`).
- Si vous iterez localement sur le protocole, mettez a jour la resolution des dependances avant release.
