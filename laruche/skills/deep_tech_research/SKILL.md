---
type: skill
name: deep_tech_research
description: Mener une recherche approfondie et synthétisée sur les tendances futures d'un domaine technologique (IA, ML, Quantique, etc.) en utilisant des sources d'autorité.
tools: [web_deep_search, web_fetch]
---

# Processus de Recherche de Tendances Technologiques Profondes

Ce skill est conçu pour mener une analyse exhaustive et synthétisée sur l'évolution future d'un domaine technologique (IA, ML, Quantique, etc.).

## 🔬 Étapes de la Recherche
1.  **Identification des Sources (web_deep_search):** Utiliser `web_deep_search` avec des requêtes ciblées (ex: "tendances IA 2026", "avenir ML"). L'objectif est d'identifier des articles de haute autorité (Microsoft, IBM, Gartner, etc.).
2.  **Extraction du Contenu (web_fetch):** Pour les sources jugées les plus pertinentes, utiliser `web_fetch` avec l'argument `render: true` pour garantir l'extraction du contenu principal, en ignorant les éléments de navigation ou les bannières.
3.  **Analyse et Synthèse:** Examiner les résultats pour identifier les thèmes récurrents (ex: collaboration homme-IA, avancées d'infrastructure, éthique).
4.  **Rapport Final:** Synthétiser les informations en un rapport structuré, en distinguant les tendances majeures et les preuves tirées des sources.

## ⚠️ Pièges à éviter
*   Ne pas se fier uniquement aux snippets : toujours utiliser `web_fetch` pour obtenir le contexte complet.
*   Éviter les sources trop commerciales ou promotionnelles ; privilégier les rapports de recherche ou les analyses de grands acteurs.

## 🛠️ Outils utilisés
*   web_deep_search
*   web_fetch
