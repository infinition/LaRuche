---
type: skill
name: telegram_encoding_fix
description: Procédure pour diagnostiquer et corriger les problèmes d'encodage UTF-8 dans les scripts d'envoi de messages Telegram, en forçant l'encodage correct sur le corps de la requête.
tools: [file_list, file_read, file_edit, reload_plugins]
---

1. Identifier le script cible (ex: send_telegram.py) en utilisant file_list. 2. Lire le contenu du script avec file_read pour localiser le point de défaillance (généralement l'envoi de données JSON ou l'écriture de fichiers). 3. Modifier le script avec file_edit pour forcer l'encodage UTF-8 (ex: en utilisant json.dumps(..., ensure_ascii=False) ou en configurant l'envoi HTTP). 4. Recharger les plugins avec reload_plugins pour que la correction soit immédiatement effective.
