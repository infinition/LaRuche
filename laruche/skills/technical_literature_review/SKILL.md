---
type: skill
name: technical_literature_review
description: Mener une recherche académique profonde sur un sujet technique, cibler les sources de pointe (arXiv), et synthétiser les résultats en un rapport structuré (définitions, taxonomies, défis).
tools: [web_deep_search, web_fetch]
---

# Processus de Revue Littéraire Technique Approfondie

Ce skill est conçu pour transformer une requête de recherche technique complexe (ex: "World Models en ML") en un rapport structuré et synthétique, en s'appuyant sur des sources académiques (arXiv, publications de conférence).

## Étape 1 : Identification et Ciblage des Sources
1.  Utiliser `web_deep_search` avec une requête très spécifique (incluant le domaine et les sources académiques cibles, ex: "World Models Machine Learning arXiv").
2.  Filtrer les résultats pour identifier les articles de revue (surveys) ou les papiers les plus récents et complets.

## Étape 2 : Récupération et Analyse du Contenu
1.  Pour les sources sélectionnées, utiliser `web_fetch` pour récupérer le contenu détaillé (Abstract, sections clés, etc.).
2.  Analyser le contenu pour extraire les éléments structurants :
    *   Définitions clés.
    *   Taxonomies ou classifications (dimensions, types).
    *   Problématiques ou lacunes du champ (challenges).
    *   Applications majeures.

## Étape 3 : Synthèse et Rapport
1.  Structurer les informations extraites dans un rapport Markdown clair.
2.  Assurer la traçabilité en citant les sources utilisées.
3.  Le rapport doit être synthétique mais complet, mettant en évidence les concepts fondamentaux et les perspectives futures.

**Pièges à éviter :**
*   Ne pas se contenter du résumé (snippet) ; toujours tenter de récupérer le contenu complet si possible.
*   Ne pas mélanger les concepts de différentes sources sans distinction claire.

**Sortie attendue :** Un rapport Markdown structuré.
