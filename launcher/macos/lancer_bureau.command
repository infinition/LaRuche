#!/bin/bash
# Lanceur macOS : double-cliquer dans le Finder.
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
# Le Finder ne charge pas forcément le PATH du terminal.
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
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
    foyer="$HOME/Library/Application Support/LaRuche"
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

# Pour ouvrir une ruche distante sans démarrer de nœud local :
# export LARUCHE_URL=http://192.168.1.20:8419

printf '\n=== LARUCHE — APPLICATION DE BUREAU ===\nFoyer : %s\n' "$foyer"
echo "=== Build du nœud et de la coque (le premier lancement peut être long) ==="
cargo build --release

echo "=== Ouverture de la fenêtre ==="
# La coque démarre le nœud et attend qu'il soit prêt avant d'afficher l'interface.
cargo run --release -p laruche-bureau --bin laruche
