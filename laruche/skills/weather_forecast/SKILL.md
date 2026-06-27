---
type: skill
name: weather-forecast
description: Obtenir et résumer les prévisions météorologiques détaillées pour une ville donnée.
tools: [web_search, web_fetch]
---

# Procédure de prévision météo

Cette procédure permet d'obtenir et de synthétiser les prévisions météorologiques détaillées pour une ville donnée en utilisant des sources fiables.

## Étapes
1. **Recherche de source (`web_search`)**: Utiliser l'outil `web_search` avec une requête combinant la ville et le terme "météo" (ex: "météo Cannes").
2. **Sélection de la source**: Analyser les résultats de la recherche pour identifier le site le plus fiable et le plus pertinent (ex: Météo-France, sites officiels).
3. **Récupération des données (`web_fetch`)**: Utiliser l'outil `web_fetch` sur l'URL sélectionnée pour récupérer le contenu de la page de prévisions.
4. **Extraction et Synthèse**: Parcourir le contenu récupéré (HTML/texte) pour extraire les informations clés :
    *   Conditions générales (soleil, pluie, etc.).
    *   Températures (min/max par période).
    *   Vent et vigilance.
    *   Période couverte (jours/heures).
5. **Présentation**: Structurer les données extraites dans un résumé clair et lisible pour l'utilisateur, en citant la source.

## Pièges à éviter
*   **Sources non fiables**: Ne pas se fier aux premiers résultats de recherche si leur domaine n'est pas reconnu comme une source météorologique professionnelle.
*   **Parsing complexe**: Le contenu web est souvent complexe (HTML/JavaScript). Il faut s'assurer que l'extraction est robuste aux variations de structure du site.

**Outils utilisés**: `web_search`, `web_fetch`.
