#!/bin/bash
# Lanceur Linux : exécuter avec Bash ou depuis un terminal.
set -e

sonde_pid=""
terminer() {
    resultat=$?
    trap - EXIT
    if [ -n "$sonde_pid" ]; then
        kill "$sonde_pid" 2>/dev/null || true
        wait "$sonde_pid" 2>/dev/null || true
    fi
    if [ "$resultat" -ne 0 ] && [ -t 0 ]; then
        printf '\nLe lancement a échoué (code %s). Voir les erreurs ci-dessus.\n' "$resultat"
        read -r -p "Appuie sur Entrée pour fermer… " || true
    fi
    exit "$resultat"
}
trap terminer EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

cd "$(dirname "$0")/../../laruche"
# Retrouver Cargo même si son dossier est absent du PATH.
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust/Cargo est introuvable. Installe Rust depuis https://rustup.rs puis relance."
    exit 1
fi

export RUCHE_MOTEUR=butinage
export LARUCHE_EMBED_URL=http://localhost:11434
export LARUCHE_EMBED_MODEL=nomic-embed-text

# Options, comme dans les .bat : décommenter au besoin.
# export LARUCHE_BIND_LAN=1
# export LARUCHE_DATA_DIR="$PWD"
# export LARUCHE_TAVILY_KEY="..."
# export LARUCHE_BRAVE_KEY="..."
# export LARUCHE_SEARXNG_URL=http://localhost:8888

# Même choix du foyer que l'application, y compris une ruche déjà dans le dépôt.
foyer="${LARUCHE_DATA_DIR:-}"
if [ -z "$foyer" ]; then
    foyer="${XDG_DATA_HOME:-$HOME/.local/share}/laruche"
    for marqueur in memoire.db config.json laruche.toml secrets.enc missions.json cron-tasks.json; do
        if [ -e "$marqueur" ]; then
            foyer="$PWD"
            break
        fi
    done
fi
mkdir -p "$foyer/skills"
# Copie les skills livrés, en préservant les fichiers locaux plus récents.
if [ "$(cd "$foyer" && pwd -P)" != "$(pwd -P)" ]; then
    for skill in skills/*; do
        if [ -d "$skill" ] && [ -f "$skill/SKILL.md" ]; then
            rsync -rtu "$skill/" "$foyer/skills/${skill##*/}/"
        fi
    done
fi

export LARUCHE_MEMOIRE_BACKEND=sqlite
export LARUCHE_NO_BROWSER=1
# export RUCHE_CURATEUR=1

printf '\n=== BUTINAGE - mémoire SQLite ===\nFoyer : %s\n' "$foyer"
echo "=== Build de laruche-node (le premier lancement peut être long) ==="
cargo build --release -p laruche-node

echo "=== Démarrage du serveur - Ctrl+C pour arrêter ==="
# Attend une réponse HTTP avant d'ouvrir un seul onglet. La sonde est arrêtée
# automatiquement si le serveur s'arrête ou si le lancement échoue.
(
    for ((essai=0; essai<600; essai++)); do
        if curl --fail --silent --output /dev/null --max-time 1 http://127.0.0.1:8419/; then
            if command -v xdg-open >/dev/null 2>&1; then
                xdg-open http://localhost:8419
            else
                echo "Interface prête : http://localhost:8419"
            fi
            exit 0
        fi
        sleep 0.5
    done
    echo "Le serveur ne répond pas encore. Ouvre http://localhost:8419 quand il sera prêt."
) &
sonde_pid=$!
cargo run --release -p laruche-node --bin laruche-node
