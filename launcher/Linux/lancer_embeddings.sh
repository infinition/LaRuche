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

# Usage : lancer_embeddings [auto|ollama|llamacpp]
# LOCAL_AI_DIR permet de choisir le dossier des binaires et modèles locaux.
mode="${1:-auto}"
local_ai="${LOCAL_AI_DIR:-$HOME/.local/share/laruche/local-ai}"
if ! command -v curl >/dev/null 2>&1; then
    echo "curl est introuvable. Installe curl puis relance."
    exit 1
fi
case "$mode" in
    auto)
        if command -v ollama >/dev/null 2>&1; then mode=ollama; else mode=llamacpp; fi
        ;;
    ollama|llamacpp) ;;
    *) echo "Usage : $0 [auto|ollama|llamacpp]"; exit 2 ;;
esac

if [ "$mode" = ollama ]; then
    if ! command -v ollama >/dev/null 2>&1; then
        echo "Ollama est introuvable. Installe Ollama puis relance."
        exit 1
    fi
    export OLLAMA_HOST=http://127.0.0.1:11434
    if ! curl -fsS --max-time 2 "$OLLAMA_HOST/api/tags" >/dev/null 2>&1; then
        mkdir -p "$local_ai"
        echo "Démarrage d'Ollama. Journal : $local_ai/ollama.log"
        nohup ollama serve >"$local_ai/ollama.log" 2>&1 < /dev/null &
        ollama_pid=$!
        pret=0
        for ((essai=0; essai<30; essai++)); do
            if curl -fsS --max-time 2 "$OLLAMA_HOST/api/tags" >/dev/null 2>&1; then
                pret=1
                break
            fi
            if ! kill -0 "$ollama_pid" 2>/dev/null; then break; fi
            sleep 1
        done
        if [ "$pret" -ne 1 ]; then
            echo "Ollama ne répond pas. Consulte $local_ai/ollama.log."
            exit 1
        fi
    fi
    if ! ollama show nomic-embed-text >/dev/null 2>&1; then
        echo "Téléchargement du modèle nomic-embed-text..."
        ollama pull nomic-embed-text
    fi
    reponse=$(curl -fsS --max-time 60 "$OLLAMA_HOST/api/embed"         -H 'Content-Type: application/json'         -d '{"model":"nomic-embed-text","input":"test"}')
    if ! printf '%s' "$reponse" | grep -Eq '"embeddings"[[:space:]]*:[[:space:]]*\[[[:space:]]*\[[[:space:]]*-?[0-9]'; then
        echo "Le test d'embedding n'a pas renvoyé de vecteur."
        exit 1
    fi
    echo "Embeddings prêts sur http://localhost:11434. Ollama reste en arrière-plan."
    exit 0
fi

llama_exe=""
if command -v llama-server >/dev/null 2>&1; then
    llama_exe=$(command -v llama-server)
else
    for candidat in "$local_ai/llama-server" "$local_ai"/llama-*/llama-server "$local_ai"/llama-*/bin/llama-server "$local_ai"/llama-*/build/bin/llama-server; do
        if [ -x "$candidat" ] && [ -f "$candidat" ]; then llama_exe="$candidat"; fi
    done
fi
if [ -z "$llama_exe" ]; then
    echo "llama-server est introuvable. Installe llama.cpp ou ajuste LOCAL_AI_DIR."
    exit 1
fi
model_dir="$local_ai/.models"
model_path="$model_dir/nomic-embed-text-v1.5.Q8_0.gguf"
model_url=https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/resolve/main/nomic-embed-text-v1.5.Q8_0.gguf
mkdir -p "$model_dir"
if [ ! -s "$model_path" ]; then
    echo "Téléchargement du modèle d'embeddings..."
    temporaire=$(mktemp "$model_path.XXXXXX")
    if curl -fL --retry 3 -o "$temporaire" "$model_url" && [ -s "$temporaire" ]; then
        mv "$temporaire" "$model_path"
    else
        rm -f "$temporaire"
        echo "Le téléchargement a échoué."
        exit 1
    fi
fi
printf '\nDans le lanceur LaRuche, utilise :\nexport LARUCHE_EMBED_URL=http://localhost:8002\n'
echo "Le serveur reste dans ce terminal. Ctrl+C pour l'arrêter."
"$llama_exe" -m "$model_path" --embeddings --pooling mean -c 4096 -b 4096 -ngl 99 --host 127.0.0.1 --port 8002
