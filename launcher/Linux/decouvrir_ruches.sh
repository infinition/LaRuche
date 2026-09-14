#!/bin/bash
set -euo pipefail

terminer() {
    resultat=$?
    trap - EXIT
    if [ "$resultat" -ne 0 ]; then
        printf '\nÉchec (code %s). Voir les erreurs ci-dessus.\n' "$resultat"
    fi
    if [ -t 0 ]; then
        read -r -p "Appuie sur Entrée pour fermer... " || true
    fi
    exit "$resultat"
}
trap terminer EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

cd "$(dirname "$0")/../../laruche"
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust/Cargo est introuvable. Installe Rust depuis https://rustup.rs puis relance."
    exit 1
fi
export LARUCHE_DECOUVRIR=1
printf '\n=== Recherche des ruches sur le réseau local ===\n'
echo "Écoute mDNS puis vérification de leur accessibilité."
cargo build --release -p laruche-bureau
cargo run --release -p laruche-bureau --bin laruche
