# Durabilite du harness, 14 septembre 2026

Checkpoint avant implementation : `1c18dbb`.

Cette livraison renforce le moteur existant. Elle ne constitue pas une certification d'autonomie multi-jour ni la fermeture de tous les sujets de l'audit.

## Changements livres

- Controle de mission serialise : criteres, consignes, etat, effets incertains, repertoire de travail et empreintes des skills consultes survivent aux reprises.
- Admission exclusive par verrou systeme pour les carnets et iterations de missions. Le verrou est libere lorsque le processus termine.
- Journal des operations sequentielles avec checkpoint avant execution et resultat apres execution. Une operation terminee est reutilisee ; une operation commencee sans resultat est rendue incertaine et n'est pas rejouee automatiquement.
- Une simple lecture ne leve plus l'incertitude d'une mutation. Les mutations suivantes sont bloquees en attendant une reconciliation.
- Les arrets et demandes de clarification ne sont plus assimiles a un succes. Les plans ouverts sont conserves.
- Les appels auxiliaires au modele traversent la meme protection de delai, annulation et reservation de budget que les appels principaux. Les enfants partagent le compteur de budget en cours d'execution.
- Le contexte final inclut le controle de mission et une reserve de sortie. L'historique est archive avant les reductions principales. Les petites fenetres ne recoivent plus un minimum d'observation de 24 000 caracteres.
- Les erreurs de persistance memoire sont propagees. La consolidation ne vide pas l'historique lorsqu'une ecriture echoue ou que la reponse est incomplete.
- Routes de secours explicites avec provider, modele, endpoint, fenetre et protocole d'outils. Les credentials invalides et limites sont signales au pool.
- Les erreurs de lecture des flux sont remontees. Une fin de flux sans marqueur terminal n'est plus une reponse reussie. Ollama ne bascule plus sur generate pour n'importe quelle erreur HTTP.
- Les actions textuelles de secours sont reservees au protocole texte declare. Les schemas d'arguments sont valides recursivement, y compris les contraintes imbriquees et references locales.
- Jobs persistants avec contexte de travail, proprietaire, admission limitee, sortie bornee et annulation. Un job en cours retrouve au redemarrage est signale comme interrompu, sans relance automatique.
- Un changement d'identite d'embedder invalide les anciens vecteurs sans supprimer les faits stockes.
- Launchers regroupes par systeme dans `launcher`.

## Configuration

`mission_budget_tokens` accepte un entier positif pour borner les jetons ; zero conserve le mode sans plafond de jetons. Les limites de passes et delais restent actives.

`fallback_profiles` est une liste de huit routes maximum. Chaque route declare `provider`, `model`, `context_max_tokens`, et eventuellement `api_key`, `api_base`, `ollama_url`, `text_tools`. Une route cloud doit etre configuree explicitement. Les anciens `fallback_models` restent pris en charge.

Les fichiers `events.jsonl` a cote des carnets contiennent des traces et du contexte. Ils peuvent contenir des donnees de mission. Leur retention et leur taille doivent etre surveillees.

## Limites encore ouvertes

- La reconciliation d'un effet inconnu reste manuelle. Il faut un protocole par outil qui verifie l'identite de la ressource et apporte une preuve avant de lever le blocage.
- Le budget est une reservation estimee, pas un plafond de facturation garanti. Les tentatives internes aux routes de secours et la restauration des depenses auxiliaires apres un crash necessitent une comptabilite durable par tentative.
- La fenetre Ollama effective et la limite HTTP des gateways doivent encore etre negociees par endpoint. La reduction actuelle utilise une estimation en caracteres, pas un tokenizer propre a chaque modele.
- L'annulation d'un job ne prouve pas l'annulation de ses effets externes. La terminaison de tous les descendants de processus et la supervision externe apres crash restent a renforcer, notamment sous Windows.
- Les empreintes de skills assurent une trace, pas un environnement reproductible avec dependances verrouillees.
- La rotation des journaux, les delais des backends de memoire eux-memes et les scenarios d'evaluation de plusieurs heures restent a traiter.
- Les launchers Windows et Linux, les providers cloud reels et une mission multi-jour ne sont pas valides par les tests locaux macOS.

## Validation

Tests locaux du moteur, de la compaction, de la memoire et des skills ; suite unitaire essaim sans fonctionnalites optionnelles ; compilation du node avec sa configuration par defaut. Les nouveaux tests couvrent la reprise sans double effet, le verrou exclusif et les contraintes de schema imbriquees. Les resultats exacts figurent dans le compte rendu de livraison.
