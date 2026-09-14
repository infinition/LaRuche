#!/bin/bash
# Compile toutes les apps livrées en archives installables.
set -e

terminer() {
    resultat=$?
    trap - EXIT
    if [ "$resultat" -ne 0 ]; then
        printf '\nLa compilation a échoué (code %s). Voir les erreurs ci-dessus.\n' "$resultat"
    fi
    if [ -t 0 ]; then
        read -r -p "Appuie sur Entrée pour fermer... " || true
    fi
    exit "$resultat"
}
trap terminer EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

cd "$(dirname "$0")/../../apps-library"

if command -v python3 >/dev/null 2>&1; then
    python_apps=python3
elif command -v python >/dev/null 2>&1 && python -c 'import sys; sys.exit(sys.version_info < (3, 9))' 2>/dev/null; then
    python_apps=python
else
    echo "Python 3 est introuvable. Installe Python 3 puis relance."
    exit 1
fi

if ! "$python_apps" -c 'import sys; sys.exit(sys.version_info < (3, 9))'; then
    echo "Python 3.9 ou plus récent est nécessaire."
    exit 1
fi

for build in */build.py; do
    [ -f "$build" ] || continue
    printf '\n=== Compilation : %s ===\n' "${build%/build.py}"
    "$python_apps" "$build"
done
"$python_apps" check_dist.py
printf '\nApps compilées dans apps-library/<app>/dist/.\nDans LaRuche, ouvre Apps puis Installer et sélectionne une archive .laruche-app.\n'
