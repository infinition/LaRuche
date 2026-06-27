---
type: skill
name: youtube_video_retrieval
description: Procédure complète pour localiser et télécharger une vidéo YouTube spécifique, gérant les étapes de recherche et les pièges de scraping.
tools: [web_deep_search, web_fetch, browser_navigate, video_downloader]
---

# Workflow de récupération et téléchargement de vidéo YouTube

Ce skill orchestre la recherche d'une vidéo YouTube spécifique (par titre, chaîne, ou mot-clé) et son téléchargement.

## ⚙️ Étapes du processus
1. **Recherche de l'URL (Phase de Découverte) :**
    *   Utiliser `web_deep_search` avec des requêtes ciblées (ex: "Titre de la vidéo youtube", "Chaîne youtube nom dernier video").
    *   Si la recherche échoue ou renvoie des pages de canal complexes (comme YouTube), utiliser `web_fetch` ou `browser_navigate` pour tenter de charger la page, mais être conscient des limitations (politiques de cookies, anti-scraping).
    *   **⚠️ Piège majeur :** Les plateformes comme YouTube bloquent souvent le scraping direct. Si l'URL directe n'est pas trouvée, l'utilisateur doit fournir l'URL manuellement.
2. **Extraction de l'URL :**
    *   Analyser les résultats de recherche pour extraire l'URL canonique de la vidéo.
3. **Téléchargement (Phase d'Action) :**
    *   Une fois l'URL validée, utiliser l'outil `video_downloader` pour effectuer le téléchargement.

## 🛠️ Outils et dépendances
*   `web_deep_search`: Pour la recherche initiale de l'URL.
*   `web_fetch` / `browser_navigate`: Pour l'inspection de pages complexes.
*   `video_downloader`: L'outil final pour l'action de téléchargement.
