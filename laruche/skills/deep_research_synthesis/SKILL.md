---
type: skill
name: deep_research_synthesis
description: Mener une recherche académique et web approfondie, récupérer des sources spécifiques (ex: arXiv) et synthétiser les résultats en un rapport structuré.
tools: [web_deep_search, web_fetch, task_complete]
---

# Procédure de Recherche Académique Approfondie et Synthèse
Ce skill orchestre une recherche structurée sur des sujets complexes, en ciblant spécifiquement les sources académiques (arXiv) et les ressources web.

## Étapes de la Procédure
1.  **Planification (Plan):** Définir les objectifs de recherche, identifier les mots-clés principaux et les types de sources (ex: arXiv, web général).
2.  **Recherche Ciblé (Search):** Utiliser `web_deep_search` pour obtenir une vue d'ensemble des sujets et identifier les papiers clés (ex: numéros arXiv).
3.  **Récupération Détaillée (Fetch):** Pour chaque papier ou source identifiée, utiliser `web_fetch` pour récupérer le contenu complet (abstract, sections clés).
4.  **Analyse et Synthèse (Synthesize):** Analyser les données collectées, extraire les concepts clés, les taxonomies, les défis et les orientations futures.
5.  **Présentation (Report):** Structurer les informations dans un rapport clair, en citant systématiquement les sources académiques.

## Pièges et Bonnes Pratiques
*   **Spécificité:** Toujours inclure des termes de domaine précis (ex: "World Models", "AGI", "Transformers") pour affiner la recherche.
*   **Validation:** Vérifier la date de soumission des papiers (arXiv) pour s'assurer de la pertinence (état de l'art récent).
*   **Raisonnement:** Ne pas se contenter des résumés ; utiliser `web_fetch` pour obtenir le contexte complet.

**Outils Utilisés:** `web_deep_search`, `web_fetch`, `task_complete`
