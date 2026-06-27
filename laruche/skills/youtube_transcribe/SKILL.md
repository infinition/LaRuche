---
type: skill
name: youtube-transcribe
description: Transcrire le contenu parlé d'une vidéo YouTube en utilisant le script dédié.
tools: [shell_exec]
scripts: [scripts/fetch_transcript.py]
---

### Procédure de Transcription YouTube

Ce skill orchestre l'exécution d'un script externe pour récupérer la transcription d'une vidéo YouTube donnée.

**Étapes :**
1.  Identifier l'URL de la vidéo YouTube cible.
2.  Exécuter le script `fetch_transcript.py` via `shell_exec`, en passant l'URL en argument.
3.  Utiliser le flag `--text-only` pour s'assurer que seul le contenu textuel de la transcription est retourné.

**Commandes exactes :**
`uv run python3 SKILL_DIR/scripts/fetch_transcript.py "{{url}}" --text-only`

**Pièges :**
*   **Erreur d'exécution (Code 1) :** Si le script échoue, cela peut être dû à des restrictions de l'API YouTube, à des problèmes de format de la vidéo (pas de sous-titres disponibles), ou à des problèmes d'environnement. Vérifier la disponibilité des sous-titres pour l'URL fournie.
*   **Dépendances :** Assurez-vous que l'environnement `uv` et les dépendances du script sont correctement installés.
