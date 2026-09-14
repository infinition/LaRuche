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
export LARUCHE_SANS_NOEUD=1
# Adresse facultative, sinon découverte automatique des ruches du réseau.
# export LARUCHE_URL=http://192.168.1.20:8419
unset LARUCHE_DECOUVRIR
printf '\n=== LaRuche : bureau client ===\n'
echo "La ruche distante doit autoriser le réseau avec LARUCHE_BIND_LAN=1."
cargo build --release -p laruche-bureau
cargo run --release -p laruche-bureau --bin laruche
