---
type: skill
name: cron_manager
description: gestion des crons
tools: [cron_manager]
---

Skill : cron_manager

Description : Protocole de gestion des tâches planifiées.

Procédures :
1. add_cron(nom, cron_expr, prompt) : Appelle cron_create.
2. update_cron(nom, cron_expr, prompt) : Trouve l'ID via cron_list, supprime avec cron_delete, puis crée avec cron_create.
3. run_now(nom) : Trouve le prompt via cron_list et l'exécute immédiatement.
4. list_summary() : Affiche une liste formatée des crons.
5. delete_cron(nom) : Trouve l'ID via cron_list et supprime avec cron_delete.