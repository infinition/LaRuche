LaRuche.i18n.add({
  'settings.loading':            {fr:'Chargement...',    en:'Loading...'},
  'settings.codexSubscription':  {fr:'- abonnement (OAuth, sans clé API)', en:'- subscription (OAuth, no API key)'},
  'settings.modelsLabel':        {fr:'Modèles',          en:'Models'},
  'settings.viewSource':         {fr:'Voir source',      en:'View source'},
  'settings.editJson':           {fr:'Éditer JSON',      en:'Edit JSON'},
  'settings.tlDragHint':         {fr:'Glisser horizontalement pour décaler l\'heure (crons à heure fixe)', en:'Drag horizontally to shift the time (fixed-time crons)'},
  'settings.channelLabel':       {fr:'Canal',            en:'Channel'},
  'settings.watcherChannelLabel':{fr:'Canal (déclenchement → notification)', en:'Channel (trigger → notification)'},
  'settings.publicProviderConfirm': {fr:'Public = le mesh utilise ce provider VIA ce node (ta clé reste locale, ce node relaie et exécute les appels). N\'expose jamais une clé que tu ne veux pas voir consommée par le réseau. Continuer ?', en:'Public = the mesh uses this provider VIA this node (your key stays local, this node relays and runs the calls). Never expose a key you don\'t want consumed by the network. Continue?'},
  'settings.confirmDeleteSkill': {fr:'Supprimer {name} ?', en:'Delete {name}?'},
  'settings.collapseHint':       {fr:'Cliquer pour déplier/replier', en:'Click to expand/collapse'},
  'settings.varPlaceholder':     {fr:'Utilise {nom} pour référencer une variable...', en:'Use {nom} to reference a variable...'},
  'settings.optional':           {fr:'optionnel',        en:'optional'},
  'settings.generationTitle':    {fr:'Génération (à chaud, sans redémarrage)', en:'Generation (hot reload, no restart)'},
  'settings.maxPassesTitle':     {fr:'Passes ReAct max par tâche (anti-runaway)', en:'Max ReAct passes per task (anti-runaway)'},
  'settings.temperature':        {fr:'Température',      en:'Temperature'},
  'settings.maxTokensOut':       {fr:'Max tokens (sortie)', en:'Max tokens (output)'},
  'settings.dynToolsLimit':      {fr:'Nb d\'outils injectés en sélection dynamique', en:'Number of tools injected in dynamic selection'},
  'settings.dynToolsLimitLabel': {fr:'Limite outils dyn.', en:'Dyn. tools limit'},
  'settings.narrowCtxThreshold': {fr:'Sous ce n_ctx, outils ET catalogue de skills passent en sélection dynamique (DB sémantique)', en:'Below this n_ctx, tools AND skill catalog switch to dynamic selection (semantic DB)'},
  'settings.narrowCtxLabel':     {fr:'Seuil contexte étroit', en:'Narrow context threshold'},
  'settings.apply':              {fr:'Appliquer',        en:'Apply'},
  'settings.contextCompaction':  {fr:'Contexte & compaction', en:'Context & compaction'},
  'settings.maxMessages':        {fr:'Max Messages',     en:'Max Messages'},
  'settings.compactionThreshold':{fr:'Seuil Compaction', en:'Compaction Threshold'},
  'settings.save':               {fr:'Sauvegarder',      en:'Save'},
  'settings.curateur':           {fr:'Curateur · Butinage', en:'Curateur · Butinage'},
  'settings.autoSkillCreate':    {fr:'Auto-création de skills/outils vérifiés', en:'Auto-creation of verified skills/tools'},
  'settings.dynToolsSelect':     {fr:'Sélection dynamique des outils ', en:'Dynamic tool selection '},
  'settings.dynToolsHint':       {fr:'(prompt léger, recommandé pour petits modèles / llama.cpp)', en:'(light prompt, recommended for small models / llama.cpp)'},
  'settings.curEnvForced':       {fr:'Forcé par RUCHE_CURATEUR=1 (variable d\'env).', en:'Forced by RUCHE_CURATEUR=1 (env variable).'},
  'settings.curDefault':         {fr:'En arrière-plan, conservateur (dédup auto). Off = ne crée rien.', en:'Background, conservative (auto-dedup). Off = creates nothing.'},
  'settings.system':             {fr:'System',           en:'System'},
  'settings.showTransparency':   {fr:'Afficher la transparence (outils/mémoire)', en:'Show transparency (tools/memory)'},
  'settings.codexLoading':       {fr:'Chargement…',     en:'Loading…'},
  'settings.codexConnected':     {fr:'✓ Connecté',      en:'✓ Connected'},
  'settings.codexExpiring':      {fr:'Token expiré: refresh auto au prochain appel.', en:'Token expired: auto-refresh on next call.'},
  'settings.codexDisconnect':    {fr:'Déconnecter',     en:'Disconnect'},
  'settings.codexConnectInstr':  {fr:'Pour vous connecter :', en:'To connect:'},
  'settings.codexStep1':         {fr:'Ouvrez', en:'Open'},
  'settings.codexStep2':         {fr:'Entrez ce code :', en:'Enter this code:'},
  'settings.codexWaiting':       {fr:'⏳ En attente de validation…', en:'⏳ Waiting for validation…'},
  'settings.codexError':         {fr:'Échec : ', en:'Failed: '},
  'settings.codexRetry':         {fr:'Réessayer', en:'Retry'},
  'settings.codexUseSubscription': {fr:'Utilisez votre abonnement ChatGPT (Plus/Pro) au lieu d\'une clé API.', en:'Use your ChatGPT subscription (Plus/Pro) instead of an API key.'},
  'settings.codexSignIn':        {fr:'Se connecter avec ChatGPT', en:'Sign in with ChatGPT'},
  'settings.codexInit':          {fr:'Initialisation…', en:'Initializing…'},
  'settings.codexNetwork':       {fr:'réseau',           en:'network'},
  'settings.codexLogoutConfirm': {fr:'Déconnecter ChatGPT Codex ?', en:'Disconnect ChatGPT Codex?'},
  'settings.sharedReadOnly':     {fr:'🐝 partagé par un pair · lecture seule', en:'🐝 shared by a peer · read-only'},
  'settings.removeFromList':     {fr:'Retirer de ma liste', en:'Remove from my list'},
  'settings.credPool':           {fr:'Pool de Credentials', en:'Credential Pool'},
  'settings.deleteCred':         {fr:'Supprimer',        en:'Delete'},
  'settings.sharedWithMe':       {fr:'🐝 Partagés avec moi (mesh)', en:'🐝 Shared with me (mesh)'},
  'settings.sharedHint':         {fr:'Modèles exposés par d\'autres ruches. Tu peux les utiliser, mais pas les éditer ni les re-partager.', en:'Models exposed by other nodes. You can use them but not edit or re-share.'},
  'settings.accessBtn':          {fr:'🔐 Accès',         en:'🔐 Access'},
  'settings.visPrivate':         {fr:'🔒 Privé',         en:'🔒 Private'},
  'settings.visRestricted':      {fr:'🐝 Restreint',     en:'🐝 Restricted'},
  'settings.enableAll':          {fr:'Tout Activer',     en:'Enable All'},
  'settings.disableAll':         {fr:'Tout Désactiver',  en:'Disable All'},
  'settings.toolsEmpty':         {fr:'Aucune abeille configurée', en:'No tools configured'},
  'settings.toolsConfigErr':     {fr:'Erreur configuration Abeilles', en:'Tools configuration error'},
  'settings.allToolsEnabled':    {fr:'Toutes les abeilles activées', en:'All tools enabled'},
  'settings.allToolsDisabled':   {fr:'Toutes les abeilles désactivées', en:'All tools disabled'},
  'settings.toolsErr':           {fr:'Erreur Abeilles: ', en:'Tools error: '},
  'settings.meshCodeConfigured': {fr:'(configuré)',      en:'(configured)'},
  'settings.meshCodeUnconfigured':{fr:'(non configuré, auth par IP LAN)', en:'(not configured, auth by LAN IP)'},
  'settings.meshCodeHint':       {fr:'Secret partagé entre tes ruches (comme un mot de passe WiFi). Mets le <b>même</b> code sur toutes tes ruches : il authentifie les échanges du mesh (fin des « rejected » / flapping) et servira de base au chiffrement.', en:'Shared secret between your nodes (like a WiFi password). Set the <b>same</b> code on all nodes: it authenticates mesh exchanges (no more "rejected" / flapping) and will be used for encryption.'},
  'settings.meshCodePlaceholderSet':   {fr:'•••• (vide = inchangé)', en:'•••• (empty = unchanged)'},
  'settings.meshCodePlaceholderNew':   {fr:'choisis un code',        en:'choose a code'},
  'settings.meshSave':           {fr:'Enregistrer',     en:'Save'},
  'settings.noNodes':            {fr:'Aucun nœud',      en:'No nodes'},
  'settings.meshCodeUnchanged':  {fr:'Code inchangé.',  en:'Code unchanged.'},
  'settings.meshCodeSaved':      {fr:'Code enregistré. Mets le MÊME sur tes autres ruches, puis relance-les.', en:'Code saved. Set the SAME code on your other nodes, then restart them.'},
  'settings.meshCodeFailed':     {fr:'Échec.',           en:'Failed.'},
  'settings.noCron':             {fr:'Aucun cron planifié.', en:'No scheduled cron.'},
  'settings.recenter':           {fr:'Recentrer',        en:'Recenter'},
  'settings.tlShiftUnsupported': {fr:'Décalage non supporté pour ce planning', en:'Shift not supported for this schedule'},
  'settings.tlShiftFixed':       {fr:'Décalage : crons à heure fixe uniquement', en:'Shift: fixed-hour crons only'},
  'settings.tlHourShifted':      {fr:'Heure décalée → ', en:'Hour shifted → '},
  'settings.tlNoName':           {fr:'(sans nom)',       en:'(unnamed)'},
  'settings.tlLastRun':          {fr:'Dernier : ',       en:'Last run: '},
  'settings.tlNever':            {fr:'jamais',           en:'never'},
  'settings.tlRuns':             {fr:' · Exécutions : ', en:' · Runs: '},
  'settings.tlChannel':          {fr:' · Canal : ',      en:' · Channel: '},
  'settings.tlRunNow':           {fr:'Lancer maintenant', en:'Run now'},
  'settings.tlEdit':             {fr:'Éditer',           en:'Edit'},
  'settings.tlPause':            {fr:'Mettre en pause',  en:'Pause'},
  'settings.tlResume':           {fr:'Réactiver',        en:'Resume'},
  'settings.tlDelete':           {fr:'Supprimer',        en:'Delete'},
  'settings.tlDeleteConfirm':    {fr:'Supprimer ce cron ?', en:'Delete this cron?'},
  'settings.tlRunning':          {fr:'Cron lancé',       en:'Cron started'},
  'settings.tlFailed':           {fr:'Échec',            en:'Failed'},
  'settings.tlPlanLabel':        {fr:'Planning : ',      en:'Schedule: '},
  'settings.tlSaveEdit':         {fr:'Enregistrer',      en:'Save'},
  'settings.tlCancel':           {fr:'Annuler',          en:'Cancel'},
  'settings.tlSaved':            {fr:'Cron mis à jour',  en:'Cron updated'},
  'settings.skillsUnavailable':  {fr:'Skills indisponibles : les associations existantes seront conservées.', en:'Skills unavailable: existing associations will be preserved.'},
  'settings.noSkillsAvailable':  {fr:'Aucun skill disponible. Créez-en dans Settings → Skills.', en:'No skills available. Create some in Settings → Skills.'},
  'settings.skillsInjected':     {fr:'Skills injectés à ce cron', en:'Skills injected into this cron'},
  'settings.skillDisabled':      {fr:'(désactivé : non injecté)', en:'(disabled: not injected)'},
  'settings.defaultModel':       {fr:'Default (modele actif)', en:'Default (active model)'},
  'settings.providerDefault':    {fr:'Défaut du provider', en:'Provider default'},
  'settings.secretsDesc':        {fr:'Les secrets sont <b>chiffrés au repos</b>. Le LLM ne voit JAMAIS leur valeur, seulement leur nom. Dans une commande, un script ou un champ clé d\'API, référence-les par <code>${NOM}</code> : la vraie valeur est substituée à l\'exécution.', en:'Secrets are <b>encrypted at rest</b>. The LLM NEVER sees their value, only their name. In a command, script, or API key field, reference them as <code>${NAME}</code>: the real value is substituted at runtime.'},
  'settings.secretsTitle':       {fr:'Secrets', en:'Secrets'},
  'settings.secretsHint':        {fr:'Ex: API_OPENAI, TOKEN_TELEGRAM, USERID_TELEGRAM…', en:'E.g.: API_OPENAI, TOKEN_TELEGRAM, USERID_TELEGRAM…'},
  'settings.webhooksTitle':      {fr:'Webhooks', en:'Webhooks'},
  'settings.webhooksHint':       {fr:'Nomme-les WEBHOOK_… (ex: WEBHOOK_DISCORD). Référence dans un script : ${WEBHOOK_DISCORD}', en:'Name them WEBHOOK_… (e.g.: WEBHOOK_DISCORD). Reference in a script: ${WEBHOOK_DISCORD}'},
  'settings.addOrUpdate':        {fr:'Ajouter / mettre à jour', en:'Add / update'},
  'settings.secretNameLabel':    {fr:'Nom (A-Z, 0-9, _)', en:'Name (A-Z, 0-9, _)'},
  'settings.secretValLabel':     {fr:'Valeur (jamais ré-affichée)', en:'Value (never shown again)'},
  'settings.secretNamePlaceholder': {fr:'ex: WEBHOOK_DISCORD', en:'e.g.: WEBHOOK_DISCORD'},
  'settings.secretValPlaceholder':  {fr:'collez la valeur ici', en:'paste value here'},
  'settings.secretSave':         {fr:'Enregistrer', en:'Save'},
  'settings.secretDeleteBtn':    {fr:'Suppr', en:'Del'},
  'settings.secretNone':         {fr:'Aucun.', en:'None.'},
  'settings.secretNameRequired': {fr:'Nom et valeur requis', en:'Name and value required'},
  'settings.secretSaved':        {fr:'Secret enregistré', en:'Secret saved'},
  'settings.secretSaveFailed':   {fr:'Échec (nom invalide ? A-Z/0-9/_ uniquement)', en:'Failed (invalid name? A-Z/0-9/_ only)'},
  'settings.secretDeleted':      {fr:'Secret supprimé', en:'Secret deleted'},
  'settings.mcpDesc':            {fr:'Configurez des serveurs MCP locaux. LaRuche les utilisera pour étendre ses capacités via les agents.', en:'Configure local MCP servers. LaRuche will use them to extend its capabilities via agents.'},
  'settings.mcpArgsLabel':       {fr:'Arguments (séparés par un espace)', en:'Arguments (space-separated)'},
  'settings.mcpAddBtn':          {fr:'Ajouter le serveur', en:'Add server'},
  'settings.mcpAdded':           {fr:'Serveur MCP ajouté', en:'MCP server added'},
  'settings.mcpNone':            {fr:'Aucun serveur configuré.', en:'No servers configured.'},
  'settings.mcpDeleteConfirm':   {fr:'Supprimer ce serveur MCP ?', en:'Delete this MCP server?'},
  'settings.mcpDeleted':         {fr:'Serveur MCP supprimé', en:'MCP server deleted'},
  'settings.mcpDeleteBtn':       {fr:'Suppr', en:'Del'},
  'settings.parDefault':         {fr:'(Par défaut)',      en:'(Default)'},
  'settings.notifyLabel':        {fr:'Activer Notifier proactif', en:'Enable proactive notifications'},
  'settings.notifyHint':         {fr:'Envoi proactif des events (AgentCompleted, WatcherFired) via Telegram (le premier Chat ID configuré est utilisé).', en:'Proactive delivery of events (AgentCompleted, WatcherFired) via Telegram (the first configured Chat ID is used).'},
  'settings.chAllowedChats':     {fr:'vide = tous',      en:'empty = all'},
  'settings.chTgLaunch':         {fr:'Lancer: python -m src.telegram', en:'Launch: python -m src.telegram'},
  'settings.chDcLaunch':         {fr:'Lancer: python -m src.discord_bot', en:'Launch: python -m src.discord_bot'},
  'settings.chSlLaunch':         {fr:'Lancer: python -m src.slack_bot', en:'Launch: python -m src.slack_bot'},
  'settings.saveChannels':       {fr:'Sauvegarder',      en:'Save'},
  'settings.chModelTitle':       {fr:'Modèle par canal',  en:'Model per channel'},
  'settings.chModelHint':        {fr:'Choisis un modèle fiable sur les outils par canal (ex : Telegram). Par défaut = modèle actif global.', en:'Pick a tool-reliable model per channel (e.g. Telegram). Default = global active model.'},
  'settings.chModelDefault':     {fr:'Défaut (modèle actif)', en:'Default (active model)'},
  'settings.chModelSaved':       {fr:'Modèle du canal mis à jour', en:'Channel model updated'},
  'settings.startTelegram':      {fr:'Demarrer Telegram', en:'Start Telegram'},
  'settings.stopTelegram':       {fr:'Arreter Telegram', en:'Stop Telegram'},
  'settings.skillsDesc':         {fr:'Connaissances procédurales (OKF). Activées = injectables dans le contexte / attachables aux crons.', en:'Procedural knowledge (OKF). Enabled = injectable in context / attachable to crons.'},
  'settings.newSkillBtn':        {fr:'+ Nouveau skill',  en:'+ New skill'},
  'settings.noSkills':           {fr:'Aucun skill. L\'agent peut en créer (memory_write capacities.skills.*) ou clique « + Nouveau skill ».', en:'No skills. The agent can create them (memory_write capacities.skills.*) or click "+ New skill".'},
  'settings.skillViewEdit':      {fr:'Voir / Éditer',    en:'View / Edit'},
  'settings.skillDelBtn':        {fr:'Suppr',            en:'Del'},
  'settings.skillActivated':     {fr:'activé',           en:'enabled'},
  'settings.skillDeactivated':   {fr:'désactivé',        en:'disabled'},
  'settings.skillToast':         {fr:'Skill ',           en:'Skill '},
  'settings.skillToolsHint':     {fr:'Abeilles / plugins de ce skill (→ <code>tools:</code>) ', en:'Tools / plugins for this skill (→ <code>tools:</code>) '},
  'settings.skillToolsFilter':   {fr:'filtrer…',         en:'filter…'},
  'settings.skillToolsClear':    {fr:'Vider',            en:'Clear'},
  'settings.skillToolsLoading':  {fr:'Chargement…',     en:'Loading…'},
  'settings.skillToolsNone':     {fr:'Aucun résultat.',  en:'No results.'},
  'settings.skillEditorHint':    {fr:'- SKILL.md (frontmatter validé au save)', en:'- SKILL.md (frontmatter validated on save)'},
  'settings.skillNewTitle':      {fr:'Nouveau skill',    en:'New skill'},
  'settings.skillEditPrefix':    {fr:'Éditer : ',        en:'Edit: '},
  'settings.skillSaveBtn':       {fr:'Enregistrer',      en:'Save'},
  'settings.skillCancelBtn':     {fr:'Annuler',          en:'Cancel'},
  'settings.skillSaved':         {fr:' » enregistré',   en:' » saved'},
  'settings.skillFailed':        {fr:'Échec',            en:'Failed'},
  'settings.pluginEditTitle':    {fr:'Éditer Plugin : ', en:'Edit Plugin: '},
  'settings.pluginEditorHint':   {fr:'- JSON (rechargé au save)', en:'- JSON (reloaded on save)'},
  'settings.pluginSaveBtn':      {fr:'Enregistrer',      en:'Save'},
  'settings.pluginCancelBtn':    {fr:'Annuler',          en:'Cancel'},
  'settings.pluginSaved':        {fr:' » enregistré',   en:' » saved'},
  'settings.pluginFailed':       {fr:'Échec',            en:'Failed'},
  'settings.pluginDeleted':      {fr:'Plugin supprimé',  en:'Plugin deleted'},
  'settings.pluginSrcUnavailable': {fr:'Source non disponible', en:'Source unavailable'},
  'settings.pluginJsonNoEdit':   {fr:'JSON non modifiable ici', en:'JSON not editable here'},
  'settings.pluginNotFound':     {fr:'Fichier non trouvé', en:'File not found'},
  'settings.fileNotFound':       {fr:'Watcher introuvable', en:'Watcher not found'},
  'settings.watcherEditTitle':   {fr:'Éditer le watcher', en:'Edit watcher'},
  'settings.watcherNomLabel':    {fr:'Nom',              en:'Name'},
  'settings.watcherTypeLabel':   {fr:'Type',             en:'Type'},
  'settings.watcherTargetLabel': {fr:'Cible (Path/URL)', en:'Target (Path/URL)'},
  'settings.watcherCondLabel':   {fr:'Condition',        en:'Condition'},
  'settings.watcherPromptLabel': {fr:'Prompt',           en:'Prompt'},
  'settings.watcherProviderLabel':{fr:'Provider',        en:'Provider'},
  'settings.watcherModelLabel':  {fr:'Modèle',           en:'Model'},
  'settings.watcherChannelLabel':{fr:'Canal',            en:'Channel'},
  'settings.watcherActiveLabel': {fr:'Actif',            en:'Active'},
  'settings.watcherSave':        {fr:'Enregistrer',      en:'Save'},
  'settings.watcherCancel':      {fr:'Annuler',          en:'Cancel'},
  'settings.watcherSaved':       {fr:'Watcher modifié',  en:'Watcher updated'},
  'settings.watcherSaveFailed':  {fr:'Échec modification', en:'Update failed'},
  'settings.watcherDefaut':      {fr:'Défaut',           en:'Default'},
  'settings.watcherEditBtn':     {fr:'Éditer',           en:'Edit'},
  'settings.watcherHomeChannel': {fr:'Home channel (défaut)', en:'Home channel (default)'},
  'settings.watcherDefChannel':  {fr:'Défaut (modèle actif)', en:'Default (active model)'},
  'settings.newCredKey':         {fr:'Nouvelle cle API pour ', en:'New API key for '},
  'settings.newCredLabel':       {fr:'Label optionnel (ex: Compte Dev) :', en:'Optional label (e.g.: Dev Account):'},
  'settings.credErr':            {fr:'Erreur: ',         en:'Error: '},
  'settings.visibilityUpdated':  {fr:'Visibilité modifiée avec succès', en:'Visibility updated successfully'},
  'settings.accessTitle':        {fr:'🔐 Accès mesh du provider', en:'🔐 Provider mesh access'},
  'settings.accessHint':         {fr:'Qui peut utiliser ce provider/LLM via le mesh ? (la clé API reste toujours locale)', en:'Who can use this provider/LLM via the mesh? (the API key always stays local)'},
  'settings.accessPrivate':      {fr:'🔒 <b>Privé</b> - moi seulement', en:'🔒 <b>Private</b> - me only'},
  'settings.accessPublic':       {fr:'🌐 <b>Public</b> - toutes les ruches du mesh', en:'🌐 <b>Public</b> - all mesh nodes'},
  'settings.accessRestricted':   {fr:'🐝 <b>Restreint</b> - seulement ces ruches :', en:'🐝 <b>Restricted</b> - only these nodes:'},
  'settings.accessNoPeers':      {fr:'Aucune ruche découverte sur le réseau.', en:'No nodes discovered on the network.'},
  'settings.accessSave':         {fr:'Enregistrer',      en:'Save'},
  'settings.accessUpdated':      {fr:'Accès mis à jour', en:'Access updated'},
  'settings.deleteCredConfirm':  {fr:'Supprimer cette cle du pool ?', en:'Delete this key from the pool?'},
  'settings.kanbanTitle':        {fr:'Titre de la tâche', en:'Task title'},
  'settings.kanbanTitlePlaceholder': {fr:'Nouvelle tâche...', en:'New task...'},
  'settings.kanbanDesc':         {fr:'Description',      en:'Description'},
  'settings.kanbanDescPlaceholder': {fr:'Détails...',   en:'Details...'},
  'settings.kanbanModel':        {fr:'Modèle',           en:'Model'},
  'settings.kanbanChannel':      {fr:'Canal',            en:'Channel'},
  'settings.kanbanBoardChannel': {fr:'Défaut du board',  en:'Board default'},
  'settings.kanbanBoardChannelNone': {fr:'Aucun (→ home channel)', en:'None (→ home channel)'},
  'settings.kanbanCreate':       {fr:'Créer',            en:'Create'},
  'settings.kanbanDefaultChannelLabel': {fr:'Canal par défaut du board', en:'Board default channel'},
  'settings.kanbanDefaultUpdated': {fr:'Canal par défaut du board mis à jour', en:'Board default channel updated'},
  'settings.kanbanTaskCreated':  {fr:'Tâche créée',      en:'Task created'},
  'settings.kanbanEditTitle':    {fr:'Éditer la tâche',  en:'Edit task'},
  'settings.kanbanEditTitleLabel': {fr:'Titre',          en:'Title'},
  'settings.kanbanEditDescLabel':  {fr:'Description',    en:'Description'},
  'settings.kanbanEditProviderLabel': {fr:'Provider',    en:'Provider'},
  'settings.kanbanEditModelLabel': {fr:'Modèle',         en:'Model'},
  'settings.kanbanEditChannelLabel': {fr:'Canal',        en:'Channel'},
  'settings.kanbanEditSave':     {fr:'Enregistrer',      en:'Save'},
  'settings.kanbanEditCancel':   {fr:'Annuler',          en:'Cancel'},
  'settings.kanbanTaskUpdated':  {fr:'Tâche modifiée',   en:'Task updated'},
  'settings.kanbanEditBtn':      {fr:'Éditer',           en:'Edit'},
  'settings.kanbanDelBtn':       {fr:'Suppr',            en:'Del'},
  'settings.kanbanResultLabel':  {fr:'Résultat',         en:'Result'},
  'settings.kanbanCols':         {fr:'Colonnes',         en:'Columns'},
  'settings.kanbanHorizontal':   {fr:'Horizontal',       en:'Horizontal'},
  'settings.kanbanDefProvider':  {fr:'Défaut (modèle actif)', en:'Default (active model)'},
  'settings.kanbanParDefault':   {fr:'(Par défaut)',     en:'(Default)'},
  'settings.deleteProfile':      {fr:'Supprimer le profil provider "', en:'Delete provider profile "'},
  'settings.profileDeleted':     {fr:'Profile deleted',  en:'Profile deleted'},
  'settings.cronSaved':          {fr:'Cron mis  jour',   en:'Cron updated'},
  'settings.cronDeleteConfirm':  {fr:'Supprimer ce cron ?', en:'Delete this cron?'},
  'settings.cronDeleteFailed':   {fr:'Suppression impossible', en:'Deletion failed'},
  'settings.cronDeleted':        {fr:'Cron supprimé',    en:'Cron deleted'},
  'settings.contextSaved':       {fr:'Configuration Contexte sauvegardée', en:'Context configuration saved'},
  'settings.contextSaveFailed':  {fr:'Erreur de sauvegarde', en:'Save error'},
  'settings.generationApplied':  {fr:'Génération appliquée (à chaud)', en:'Generation applied (hot reload)'},
  'settings.errorGeneric':       {fr:'Erreur',           en:'Error'},
  'settings.curateEnabled':      {fr:'activé',           en:'enabled'},
  'settings.curateDisabled':     {fr:'désactivé',        en:'disabled'},
  'settings.curateFailed':       {fr:'Échec curateur',   en:'Curateur failed'},
  'settings.dynToolsEnabled':    {fr:'activée',          en:'enabled'},
  'settings.dynToolsDisabled':   {fr:'désactivée',       en:'disabled'},
  'settings.dynToolsSaved':      {fr:'Sélection dynamique des outils ', en:'Dynamic tool selection '},
  'settings.dynToolsFailed':     {fr:'Échec',            en:'Failed'},
  'settings.kbAddLabel':         {fr:'Ajouter une connaissance', en:'Add knowledge'},
  'settings.kbAddPlaceholder':   {fr:'Information a memoriser...', en:'Information to memorize...'},
  'settings.kbSourceLabel':      {fr:'Source',           en:'Source'},
  'settings.kbExportBtn':        {fr:'Export OKF',       en:'Export OKF'},
  'settings.kbImportBtn':        {fr:'Import OKF',       en:'Import OKF'},
  'settings.kbAdded':            {fr:'Connaissance ajoutee (', en:'Knowledge added ('},
  'settings.kbExportLaunched':   {fr:'Telechargement OKF lance (tout)', en:'OKF download started (all)'},
  'settings.kbImported':         {fr:'OKF importe avec succes', en:'OKF imported successfully'},
  'settings.kbEditBtn':          {fr:'Editer',           en:'Edit'},
  'settings.kbDelBtn':           {fr:'Suppr',            en:'Del'},
  'settings.kbEmpty':            {fr:'Base vide. L\'agent peut ajouter des connaissances via l\'outil knowledge_add, ou ajoutez-en manuellement ci-dessus.', en:'Empty base. The agent can add knowledge via the knowledge_add tool, or add it manually above.'},
  'settings.kbUpdated':          {fr:'Mis a jour',       en:'Updated'},
  'settings.kbDeleted':          {fr:'Supprime',         en:'Deleted'},
  'settings.channelStarted':     {fr:' demarre !',       en:' started!'},
  'settings.channelAlreadyRunning': {fr:' deja en marche', en:' already running'},
  'settings.channelStopped':     {fr:' arrete',          en:' stopped'},
  'settings.blueprintsHint':     {fr:'Sélectionnez un blueprint pour l\'instancier en tant que tâche cron.', en:'Select a blueprint to instantiate it as a cron task.'},
  'settings.newBlueprintBtn':    {fr:'+ Nouveau blueprint', en:'+ New blueprint'},
  'settings.bpNone':             {fr:'Aucun blueprint disponible', en:'No blueprints available'},
  'settings.bpDeleteBtn':        {fr:'Supprimer',        en:'Delete'},
  'settings.bpInstanciateBtn':   {fr:'Instancier',       en:'Instantiate'},
  'settings.bpNewTitle':         {fr:'Nouveau blueprint', en:'New blueprint'},
  'settings.bpTitleLabel':       {fr:'Titre',            en:'Title'},
  'settings.bpPromptLabel':      {fr:'Prompt (template)', en:'Prompt (template)'},
  'settings.bpScheduleLabel':    {fr:'Cadence (cron)',   en:'Schedule (cron)'},
  'settings.bpSlotsLabel':       {fr:'Variables (slots) - referencees via <code>{name}</code> dans les templates', en:'Variables (slots) - referenced via <code>{name}</code> in templates'},
  'settings.bpAddSlot':          {fr:'+ Variable',       en:'+ Variable'},
  'settings.bpCreateBtn':        {fr:'Créer le blueprint', en:'Create blueprint'},
  'settings.bpCancelBtn':        {fr:'Annuler',          en:'Cancel'},
  'settings.bpTitleRequired':    {fr:'Titre requis',     en:'Title required'},
  'settings.bpPromptRequired':   {fr:'Prompt requis',    en:'Prompt required'},
  'settings.bpCreated':          {fr:'Blueprint créé',   en:'Blueprint created'},
  'settings.bpCreateError':      {fr:'Erreur création: ', en:'Creation error: '},
  'settings.bpDeleteConfirm':    {fr:'Supprimer le blueprint "', en:'Delete blueprint "'},
  'settings.bpDeleteConfirmSuffix': {fr:'" ? (les blueprints intégrés ne peuvent pas être supprimés)', en:'" ? (built-in blueprints cannot be deleted)'},
  'settings.bpDeleted':          {fr:'Blueprint supprimé', en:'Blueprint deleted'},
  'settings.bpDeleteRefused':    {fr:'Suppression refusée: ', en:'Deletion refused: '},
  'settings.bpDeleteRefusedFallback': {fr:'blueprint intégré ?', en:'built-in blueprint?'},
  'settings.bpInstanciated':     {fr:'Blueprint instancié avec succès', en:'Blueprint instantiated successfully'},
  'settings.bpInstanciateError': {fr:'Erreur d\'instanciation', en:'Instantiation error'},
  'settings.bpDeleteSlotBtn':    {fr:'Supprimer cette variable', en:'Delete this variable'},
  'settings.deleteWatcherBtn':   {fr:'Delete',           en:'Delete'},
  'settings.errorColon':         {fr:'Erreur: ',         en:'Error: '},
  'settings.skillToolsRef':      {fr:'(référence)',      en:'(reference)'},
  'settings.skillToolsChecked':  {fr:'coché(s)',         en:'checked'},
  'settings.maxPassesLabel':     {fr:'Passes max',       en:'Max passes'},
  'settings.inferenceConfig':    {fr:'Config d\'inférence', en:'Inference Config'},
  'settings.fallbackModels':     {fr:'Modèles de repli', en:'Fallback Models'},
  'settings.maxTokensLabel':     {fr:'Max Tokens',       en:'Max Tokens'},
  'settings.reviewModel':        {fr:'Modèle de revue',  en:'Review Model'},
  'settings.modelExample':       {fr:'ex: gpt-4o',       en:'e.g.: gpt-4o'},
  'settings.activeLabel':        {fr:'Actif : ',         en:'Active: '},
  'settings.voice':              {fr:'Voix',             en:'Voice'},
  'settings.statusOk':           {fr:'OK',               en:'OK'},
  'settings.statusOff':          {fr:'Off',              en:'Off'},
  'settings.sttExternal':        {fr:'STT externe',      en:'External STT'},
  'settings.sttExternalHint':    {fr:'Décoché (défaut) : le modèle transcrit lui-même l\'audio. Coché : utiliser le service STT externe (:8421).', en:'Unchecked (default): the model transcribes audio itself. Checked: use the external STT service (:8421).'},
  'settings.sttExternalNote':    {fr:'Par défaut, l\'audio (ex. vocal Telegram) va au modèle. Cochez si votre modèle ne sait pas faire le STT.', en:'By default, audio (e.g. Telegram voice) goes to the model. Check this if your model cannot do STT.'},
  'settings.ttsSpeed':           {fr:'Vitesse TTS',       en:'TTS speed'},
  'settings.ttsVoice':           {fr:'Voix TTS',          en:'TTS voice'},
  'settings.ttsVoiceHint':       {fr:'Identifiant de voix Kokoro (ex. ff_siwis). Vide = voix par défaut du service.', en:'Kokoro voice id (e.g. ff_siwis). Empty = the service default voice.'},
  'settings.security':           {fr:'Sécurité',         en:'Security'},
  'settings.secretsCount':       {fr:'17 motifs',        en:'17 patterns'},
  'settings.protocol':           {fr:'Protocole',        en:'Protocol'},
  'settings.statusLabel':        {fr:'État',             en:'Status'},
  'settings.statusOkValue':      {fr:'OK',               en:'OK'},
  'settings.inferenceCfgSaved':  {fr:'Config d\'inférence sauvegardée', en:'Inference config saved'},
  'settings.saveFailed':         {fr:'Échec de la sauvegarde', en:'Save failed'},
  'settings.addProvider':        {fr:'+ Ajouter un provider', en:'+ Add Provider'},
  'settings.activeBadge':        {fr:'(actif)',          en:'(active)'},
  'settings.visPublic':          {fr:'🌐 Public 📡',     en:'🌐 Public 📡'},
  'settings.visRestrictedN':     {fr:'🐝 Restreint',     en:'🐝 Restricted'},
  'settings.typeLabel':          {fr:'Type',             en:'Type'},
  'settings.apiKeyLabel':        {fr:'Clé API',          en:'API Key'},
  'settings.apiKeySet':          {fr:'***définie***',    en:'***set***'},
  'settings.apiKeyNone':         {fr:'(aucune)',         en:'(none)'},
  'settings.editBtn':            {fr:'Éditer',           en:'Edit'},
  'settings.deleteBtn':          {fr:'Supprimer',        en:'Delete'},
  'settings.testBtn':            {fr:'Tester',           en:'Test'},
  'settings.advancedSection':    {fr:'Avancé',           en:'Advanced'},
  'settings.testRunning':        {fr:'Test en cours...', en:'Testing...'},
  'settings.testOk':             {fr:'Connecté',         en:'Connected'},
  'settings.testFail':           {fr:'Échec',            en:'Failed'},
  'settings.addCredKey':         {fr:'+ Ajouter une clé', en:'+ Add Credential Key'},
  'settings.editProviderTitle':  {fr:'Éditer le provider', en:'Edit Provider'},
  'settings.addProviderTitle':   {fr:'Ajouter un provider', en:'Add Provider'},
  'settings.pfIdLabel':          {fr:'Profile ID',       en:'Profile ID'},
  'settings.pfIdReadonly':       {fr:' (lecture seule)', en:' (read-only)'},
  'settings.pfIdPlaceholder':    {fr:'ex. groq-free',    en:'e.g. groq-free'},
  'settings.pfNameLabel':        {fr:'Nom affiché',      en:'Display Name'},
  'settings.pfNamePlaceholder':  {fr:'ex. Groq Free Tier', en:'e.g. Groq Free Tier'},
  'settings.pfProviderTypeLabel':{fr:'Type de provider', en:'Provider Type'},
  'settings.pfBaseUrlLabel':     {fr:'URL de base',      en:'Base URL'},
  'settings.pfApiKeyLabel':      {fr:'Clé API',          en:'API Key'},
  'settings.pfApiKeyPlaceholder':{fr:'sk-... (vide pour Ollama)', en:'sk-... (leave empty for Ollama)'},
  'settings.pfModelsLabel':      {fr:'Modèles (séparés par des virgules, auto-détectés pour Ollama)', en:'Models (comma-separated, auto-detected for Ollama)'},
  'settings.pfModelsPlaceholder':{fr:'gpt-4o, gpt-4o-mini', en:'gpt-4o, gpt-4o-mini'},
  'settings.pfSave':             {fr:'Enregistrer',      en:'Save'},
  'settings.pfCancel':           {fr:'Annuler',          en:'Cancel'},
  'settings.profileIdRequired':  {fr:'Profile ID requis', en:'Profile ID is required'},
  'settings.profileSavedPrefix': {fr:'Profil « ',        en:'Profile "'},
  'settings.profileSavedSuffix': {fr:' » enregistré',    en:'" saved'},
  'settings.toolViewSource':     {fr:'Voir source',      en:'View source'},
  'settings.toolCustomBadge':    {fr:'Custom',           en:'Custom'},
  'settings.toolNativeBadge':    {fr:'Rust natif',       en:'Native Rust'},
  'settings.toolOn':             {fr:'ON',               en:'ON'},
  'settings.toolOff':            {fr:'OFF',              en:'OFF'},
  'settings.toolDanger':         {fr:'Danger',           en:'Danger'},
  'settings.toolDangerSafe':     {fr:'safe',             en:'safe'},
  'settings.toolEnabled':        {fr:' activée',         en:' enabled'},
  'settings.toolDisabled':       {fr:' désactivée',      en:' disabled'},
  'settings.meshCodeTitle':      {fr:'Code du mesh',     en:'Mesh code'},
  'settings.hostLabel':          {fr:'Hôte',             en:'Host'},
  'settings.cronCount':          {fr:'cron(s)',          en:'cron(s)'},
  'settings.mcpServersTitle':    {fr:'Serveurs MCP (Model Context Protocol)', en:'MCP Servers (Model Context Protocol)'},
  'settings.mcpNameLabel':       {fr:'Nom du serveur',   en:'Server name'},
  'settings.mcpNamePlaceholder': {fr:'ex: local-sqlite', en:'e.g.: local-sqlite'},
  'settings.mcpCmdLabel':        {fr:'Commande',         en:'Command'},
  'settings.mcpCmdPlaceholder':  {fr:'ex: node',         en:'e.g.: node'},
  'settings.mcpArgsPlaceholder': {fr:'ex: src/index.js --db sqlite.db', en:'e.g.: src/index.js --db sqlite.db'},
  'settings.newTaskBtn':         {fr:'+ Nouvelle tâche', en:'+ New Task'},
  'settings.nameLabel':          {fr:'Nom',              en:'Name'},
  'settings.promptLabel':        {fr:'Prompt',           en:'Prompt'},
  'settings.cronChannelNone':    {fr:'Aucun (Journal d\'activité)', en:'None (Activity Log)'},
  'settings.providerLabel':      {fr:'Provider',         en:'Provider'},
  'settings.modelLabel':         {fr:'Modèle',           en:'Model'},
  'settings.createBtn':          {fr:'Créer',            en:'Create'},
  'settings.scheduleLabel':      {fr:'Planning',         en:'Schedule'},
  'settings.runsLabel':          {fr:'Exécutions',       en:'Runs'},
  'settings.channelLabelShort':  {fr:'Canal',            en:'Channel'},
  'settings.channelNone':        {fr:'Aucun',            en:'None'},
  'settings.providerModelLabel': {fr:'Provider/Modèle',  en:'Provider/Model'},
  'settings.cronTaskCreated':    {fr:'Tâche cron créée', en:'Cron task created'},
  'settings.newWatcherBtn':      {fr:'+ Nouvelle vigie', en:'+ New Watcher'},
  'settings.newWatcherTitle':    {fr:'Nouvelle vigie',   en:'New Watcher'},
  'settings.watcherTypeFile':    {fr:'Fichier',          en:'File'},
  'settings.watcherTypeUrl':     {fr:'URL',              en:'URL'},
  'settings.watcherTypeLog':     {fr:'Motif de log',     en:'Log Pattern'},
  'settings.watcherTargetField': {fr:'Cible (Path/URL)', en:'Target (Path/URL)'},
  'settings.watcherCondField':   {fr:'Condition (optionnel)', en:'Condition (optional)'},
  'settings.targetLabel':        {fr:'Cible',            en:'Target'},
  'settings.watcherCreated':     {fr:'Vigie créée',      en:'Watcher created'},
  'settings.comingSoon':         {fr:'Bientôt disponible', en:'Coming soon'},
  'settings.channelsSaved':      {fr:'Config des canaux sauvegardée', en:'Channels config saved'},
  'settings.notificationsTitle': {fr:'Notifications',    en:'Notifications'},
  'settings.botTokenLabel':      {fr:'Token du bot',     en:'Bot Token'},
  'settings.tgAllowedChats':     {fr:'IDs de chat autorisés', en:'Allowed Chat IDs'},
  'settings.dcAllowedChannels':  {fr:'IDs de canaux autorisés', en:'Allowed Channel IDs'},
  'settings.slBotToken':         {fr:'Token du bot (xoxb-)', en:'Bot Token (xoxb-)'},
  'settings.slAppToken':         {fr:'Token de l\'app (xapp-)', en:'App Token (xapp-)'},
  'settings.setupChecklist':     {fr:'Checklist de configuration', en:'Setup Checklist'},
  'settings.kbAddKnowledgeBtn':  {fr:'Ajouter',          en:'Add'},
  'settings.kbEntriesCount':     {fr:' entrée(s) dans la base de connaissances', en:' entry(ies) in the knowledge base'},
  'settings.kbColText':          {fr:'Texte',            en:'Text'},
  'settings.kbColSource':        {fr:'Source',           en:'Source'},
  'settings.kbColActions':       {fr:'Actions',          en:'Actions'},
  'settings.kbImportError':      {fr:'Erreur d\'import : ', en:'Import error: '},
  'settings.tlCronFields':       {fr:'Cron (5 champs) ou vide', en:'Cron (5 fields) or empty'},
  'settings.bpSlotNamePlaceholder':    {fr:'nom',     en:'name'},
  'settings.bpSlotLabelPlaceholder':   {fr:'libellé', en:'label'},
  'settings.bpSlotDefaultPlaceholder': {fr:'défaut',  en:'default'},
  'settings.bpTitlePlaceholder':       {fr:'Ex. : Veille quotidienne', en:'E.g.: Daily watch'},
  'settings.pluginToast':              {fr:'Plugin ', en:'Plugin '}
});

LaRuche.Settings = (function(){
  var currentTab = 'general';

  function init() {
    document.getElementById('settingsTabsBar').addEventListener('click', function(e){
      var btn = e.target.closest('.settings-tab-btn');
      if(!btn) return;
      currentTab = btn.dataset.tab;
      document.querySelectorAll('#settingsTabsBar .settings-tab-btn').forEach(function(b){b.classList.toggle('active',b.dataset.tab===currentTab);});
      loadTab(currentTab);
    });
  }

  function enter() { loadTab(currentTab); }
  function leave() {}

  function loadTab(tab) {
    var host = document.getElementById('settingsContent');
    if(!host) return;
    // Anti-race: give EACH load a fresh canvas. If a slow async loader finishes
    // AFTER the tab has changed, it writes into ITS old `el` (now detached
    // from the DOM) -> invisible. No more "General shows up when I clicked Provider".
    var el = document.createElement('div');
    el.className = 'settings-tab-canvas';
    host.innerHTML = '';
    host.appendChild(el);
    el.innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px">'+LaRuche.i18n.t('settings.loading')+'</div>';
    switch(tab) {
      case 'general': loadGeneral(el); break;
      case 'providers': loadProviders(el); break;
      case 'mcp': loadMcp(el); break;
      case 'secrets': loadSecrets(el); break;
      case 'tools': loadTools(el); break;
      case 'channels': loadChannels(el); break;
      case 'knowledge': loadKnowledge(el); break;
      case 'network': loadNetwork(el); break;
      case 'cron': loadCron(el); break;
      case 'cron-timeline': loadCronTimeline(el); break;
      case 'blueprints': loadBlueprints(el); break;
      case 'watchers': loadWatchers(el); break;
      case 'kanban': loadKanban(el); break;
      case 'skills': loadSkills(el); break;
      case 'onboarding': loadOnboarding(el); break;
    }
  }

  async function loadGeneral(el) {
    // The 6 calls are INDEPENDENT -> run them in PARALLEL (Promise.all) instead of 6 serial awaits
    // (that was the slowness: each fetch waited for the previous one). gj = error-tolerant fetch.
    function gj(u){ return fetch(u).then(function(r){return r.json();}).catch(function(){return {};}); }
    var _r = await Promise.all([
      gj('/api/doctor'), gj('/api/voice/status'), gj('/api/config/provider'),
      gj('/api/context/stats'), gj('/api/config/compaction'), gj('/api/config/curateur'),
      gj('/api/config/runtime'), gj('/api/config/reine'), gj('/api/config/channel-models'),
      gj('/api/config/voice')
    ]);
    var doc=_r[0], voice=_r[1], provCfg=_r[2], ctxStats=_r[3], ctxCfg=_r[4], curCfg=_r[5], rt=_r[6]||{}, reineCfg=_r[7]||{}, chmReine=_r[8]||{options:[]}, voiceCfg=_r[9]||{};
    // Provider dropdown options for LaReine's judge (reuse the channel-models catalog).
    var reineProvOpts = '<option value="">'+LaRuche.i18n.t('reine.providerSame')+'</option>';
    (chmReine.options||[]).forEach(function(o){
      var rpVal = o.profile_id+'|||'+o.model;
      var rpSel = (reineCfg.provider_profile===rpVal) ? ' selected' : '';
      reineProvOpts += '<option value="'+LaRuche.Utils.esc(rpVal)+'"'+rpSel+'>'+LaRuche.Utils.esc((o.name||o.provider)+' / '+o.model)+'</option>';
    });
    // Max reworks: 255 is the unlimited sentinel (rework until the draft passes).
    var reineUnlim = (reineCfg.max_revues===255);
    var reineMaxVal = reineUnlim ? 10 : (reineCfg.max_revues||0);
    el.innerHTML = '<div class="settings-grid">'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.generationTitle')+'</div>'+
      '<div class="settings-row" style="flex-direction:column;align-items:stretch;gap:4px;">'+
      '<div class="settings-row" style="padding:0;"><span class="settings-label" title="'+LaRuche.i18n.t('settings.maxPassesTitle')+'">'+LaRuche.i18n.t('settings.maxPassesLabel')+'</span><input type="number" id="cfgMaxIter" class="form-input" style="width:80px;padding:2px 6px;" value="'+(rt.max_iterations||40)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.temperature')+'</span><input type="number" id="cfgTemp" class="form-input" style="width:80px;padding:2px 6px;" step="0.05" min="0" max="2" value="'+(rt.temperature!=null?rt.temperature:0.7)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.maxTokensOut')+'</span><input type="number" id="cfgMaxTok" class="form-input" style="width:90px;padding:2px 6px;" value="'+(rt.max_tokens||4096)+'"></div>'+
      '<details class="settings-advanced" style="margin-top:6px;"><summary style="cursor:pointer;font-size:11px;color:var(--text-dim);user-select:none;">'+LaRuche.i18n.t('settings.advancedSection')+'</summary>'+
      '<div class="settings-row" style="padding:0;margin-top:6px;"><span class="settings-label" title="'+LaRuche.i18n.t('settings.dynToolsLimit')+'">'+LaRuche.i18n.t('settings.dynToolsLimitLabel')+'</span><input type="number" id="cfgToolLim" class="form-input" style="width:80px;padding:2px 6px;" value="'+(rt.tool_selection_limit||24)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label" title="'+LaRuche.i18n.t('settings.narrowCtxThreshold')+'">'+LaRuche.i18n.t('settings.narrowCtxLabel')+'</span><input type="number" id="cfgCtxThreshold" class="form-input" style="width:90px;padding:2px 6px;" value="'+(rt.dynamic_context_threshold||40000)+'"></div>'+
      '</details>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveRuntimeCfg()" style="margin-top:8px;">'+LaRuche.i18n.t('settings.apply')+'</button></div></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.contextCompaction')+'</div>'+
      '<div class="settings-row" style="flex-direction:column;align-items:stretch;gap:4px;">'+
      '<div class="settings-row" style="padding:0;"><span class="settings-label">'+LaRuche.i18n.t('settings.maxMessages')+'</span><input type="number" id="cfgCtxMax" class="form-input" style="width:80px;padding:2px 6px;" value="'+(ctxCfg.context_max_messages||50)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.compactionThreshold')+'</span><input type="number" id="cfgCtxThresh" class="form-input" style="width:80px;padding:2px 6px;" step="0.05" value="'+(ctxCfg.compaction_threshold||0.75)+'"></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveContextCfg()" style="margin-top:8px;">'+LaRuche.i18n.t('settings.save')+'</button></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.inferenceConfig')+'</div>'+
      '<div class="settings-row" style="padding:0;"><span class="settings-label">'+LaRuche.i18n.t('settings.fallbackModels')+'</span><input type="text" id="cfgProvFallback" class="form-input" style="width:120px;padding:2px 6px;" value="'+(provCfg.fallback_models||'')+'" placeholder="claude-3-haiku, ..."></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.maxTokensLabel')+'</span><input type="number" id="cfgProvMaxTokens" class="form-input" style="width:80px;padding:2px 6px;" value="'+(provCfg.max_tokens||4096)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.temperature')+'</span><input type="number" id="cfgProvTemp" class="form-input" style="width:80px;padding:2px 6px;" step="0.1" value="'+(provCfg.temperature||0.7)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.reviewModel')+'</span><input type="text" id="cfgProvReview" class="form-input" style="width:120px;padding:2px 6px;" value="'+(provCfg.review_model||'')+'" placeholder="'+LaRuche.i18n.t('settings.modelExample')+'"></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveProviderCfg()" style="margin-top:8px;">'+LaRuche.i18n.t('settings.save')+'</button>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:8px">'+LaRuche.i18n.t('settings.activeLabel')+(provCfg.provider||'ollama')+' / '+(provCfg.model||'-')+'</div></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.voice')+'</div>'+
      '<div class="settings-row"><span class="settings-label">STT</span><span style="color:'+(voice.stt&&voice.stt.available?'var(--green)':'var(--red)')+'">'+(voice.stt&&voice.stt.available?LaRuche.i18n.t('settings.statusOk'):LaRuche.i18n.t('settings.statusOff'))+'</span></div>'+
      '<div class="settings-row"><span class="settings-label">TTS</span><span style="color:'+(voice.tts&&voice.tts.available?'var(--green)':'var(--red)')+'">'+(voice.tts&&voice.tts.available?LaRuche.i18n.t('settings.statusOk'):LaRuche.i18n.t('settings.statusOff'))+'</span></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('settings.sttExternalHint')+'"><span class="settings-label">'+LaRuche.i18n.t('settings.sttExternal')+'</span><input type="checkbox" id="cfgSttExternal" onchange="LaRuche.Settings.saveVoiceCfg()"'+(voiceCfg.stt_external?' checked':'')+'></div>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:4px">'+LaRuche.i18n.t('settings.sttExternalNote')+'</div>'+
      '<div class="settings-row" style="margin-top:6px"><span class="settings-label">'+LaRuche.i18n.t('settings.ttsSpeed')+'</span><span style="display:flex;align-items:center;gap:6px"><input type="range" id="cfgTtsSpeed" min="0.5" max="2" step="0.05" value="'+(voiceCfg.tts_speed||1)+'" oninput="document.getElementById(\'cfgTtsSpeedVal\').textContent=parseFloat(this.value).toFixed(2)+\'x\'"><span id="cfgTtsSpeedVal" style="font-size:11px;width:38px">'+(parseFloat(voiceCfg.tts_speed||1).toFixed(2))+'x</span></span></div>'+
      '<div class="settings-row"><span class="settings-label" title="'+LaRuche.i18n.t('settings.ttsVoiceHint')+'">'+LaRuche.i18n.t('settings.ttsVoice')+'</span><input type="text" id="cfgTtsVoice" class="form-input" style="width:120px;padding:2px 6px" value="'+LaRuche.Utils.esc(voiceCfg.tts_voice||'')+'" placeholder="ff_siwis"></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveVoiceCfg()" style="margin-top:8px">'+LaRuche.i18n.t('settings.save')+'</button></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.security')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.secretsTitle')+'</span><span class="settings-value">'+LaRuche.i18n.t('settings.secretsCount')+'</span></div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.protocol')+'</span><span class="settings-value">Miel v'+(doc.version||'0.2.0')+'</span></div></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.curateur')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.autoSkillCreate')+'</span><label class="lr-switch"><input type="checkbox" id="cfgCurateur" '+(curCfg.enabled?'checked':'')+' '+(curCfg.env_forced?'disabled':'')+' onchange="LaRuche.Settings.toggleCurateur(this.checked)"><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.dynToolsSelect')+'<span style="color:var(--text-dim);font-size:10px">'+LaRuche.i18n.t('settings.dynToolsHint')+'</span></span><label class="lr-switch"><input type="checkbox" id="cfgDynTools" '+(curCfg.dynamic_tools?'checked':'')+' onchange="LaRuche.Settings.toggleDynamicTools(this.checked)"><span class="lr-slider"></span></label></div>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:6px">'+(curCfg.env_forced?LaRuche.i18n.t('settings.curEnvForced'):LaRuche.i18n.t('settings.curDefault'))+'</div></div>'+
      '<div class="settings-card"><div class="settings-card-title">👑 '+LaRuche.i18n.t('reine.settingsTitle')+'</div>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-bottom:6px">'+LaRuche.i18n.t('reine.settingsDesc')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('reine.modeLabel')+'</span><select id="cfgReineMode" class="form-input" style="width:150px;padding:2px 6px;">'+
        '<option value="off"'+(reineCfg.mode==='off'?' selected':'')+'>'+LaRuche.i18n.t('reine.modeOff')+'</option>'+
        '<option value="auto"'+(reineCfg.mode==='auto'?' selected':'')+'>'+LaRuche.i18n.t('reine.modeAuto')+'</option>'+
        '<option value="hybride"'+(reineCfg.mode==='hybride'?' selected':'')+'>'+LaRuche.i18n.t('reine.modeHybride')+'</option>'+
        '<option value="humaine"'+(reineCfg.mode==='humaine'?' selected':'')+'>'+LaRuche.i18n.t('reine.modeHumaine')+'</option>'+
      '</select></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('reine.maxReviewsHint')+'"><span class="settings-label">'+LaRuche.i18n.t('reine.maxReviews')+'</span><input type="range" id="cfgReineMax" min="0" max="10" value="'+reineMaxVal+'"'+(reineUnlim?' disabled':'')+' oninput="document.getElementById(\'cfgReineMaxVal\').textContent=this.value" style="width:100px"> <span id="cfgReineMaxVal" style="min-width:18px;text-align:right;color:var(--text-muted)">'+(reineUnlim?'∞':reineMaxVal)+'</span>'+
        '<label style="margin-left:10px;font-size:10px;color:var(--text-dim);cursor:pointer;display:inline-flex;align-items:center;gap:3px"><input type="checkbox" id="cfgReineUnlim"'+(reineUnlim?' checked':'')+' onchange="LaRuche.Settings.reineToggleUnlim()"> '+LaRuche.i18n.t('reine.unlimited')+'</label></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('reine.confidenceHint')+'"><span class="settings-label">'+LaRuche.i18n.t('reine.confidenceThreshold')+'</span><input type="number" id="cfgReineSeuil" class="form-input" style="width:70px;padding:2px 6px;" min="0" max="100" value="'+(reineCfg.seuil_confiance!=null?reineCfg.seuil_confiance:60)+'"></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('reine.contextMessagesHint')+'"><span class="settings-label">'+LaRuche.i18n.t('reine.contextMessages')+'</span><input type="range" id="cfgReineCtx" min="0" max="20" value="'+(reineCfg.contexte_messages!=null?reineCfg.contexte_messages:4)+'" oninput="document.getElementById(\'cfgReineCtxVal\').textContent=this.value" style="width:100px"> <span id="cfgReineCtxVal" style="min-width:18px;text-align:right;color:var(--text-muted)">'+(reineCfg.contexte_messages!=null?reineCfg.contexte_messages:4)+'</span></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('reine.providerHint')+'"><span class="settings-label">'+LaRuche.i18n.t('reine.providerLabel')+'</span><select id="cfgReineProvider" class="form-input" style="width:160px;padding:2px 6px;">'+reineProvOpts+'</select></div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('reine.tier1')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineTier1" '+(reineCfg.tier_reponse?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('reine.tier2')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineTier2" '+(reineCfg.tier_artefacts?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('reine.tier3Warn')+'"><span class="settings-label">'+LaRuche.i18n.t('reine.tier3')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineTier3" '+(reineCfg.tier_supervision?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('reine.queueGateHint')+'"><span class="settings-label">'+LaRuche.i18n.t('reine.queueGate')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineQueue" '+(reineCfg.queue_gate?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveReineCfg()" style="margin-top:8px;">'+LaRuche.i18n.t('settings.save')+'</button>'+
      '<div style="margin-top:10px;border-top:1px solid rgba(245,158,11,.2);padding-top:8px">'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('reine.queueTitle')+'</span><span style="font-size:10px;color:var(--text-dim);text-align:right">'+LaRuche.i18n.t('reine.queueInMemory')+'</span></div>'+
      '</div></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.system')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.showTransparency')+'</span><label class="lr-switch"><input type="checkbox" id="cfgTransparence" onchange="window.localStorage.setItem(\'laruche_hide_transparency\', this.checked ? \'false\' : \'true\')" \'+(window.localStorage.getItem(\'laruche_hide_transparency\') !== \'true\' ? \'checked\' : \'\')+\'><span class="lr-slider"></span></label></div>'+
      ((doc.checks||[]).map(function(c){return '<div class="settings-row"><span class="settings-label">'+c.name+'</span><span style="color:'+(c.status==='ok'?'var(--green)':'var(--red)')+'">'+c.status+'</span></div>';}).join('')||'<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.statusLabel')+'</span><span class="settings-value">'+LaRuche.i18n.t('settings.statusOkValue')+'</span></div>')+
      '</div></div>';
  }

  // ── Providers Tab ─────────────────────────────────────────────

  async function loadProviders(el) {
    var data = {};
    try { data = await fetch('/api/profiles').then(function(r){return r.json();}); } catch(e) {}
    var profiles = data.profiles || {};
    var active = data.active_model || {};
    var ids = Object.keys(profiles).sort();

    var credsData = {};
    try { credsData = await fetch('/api/credentials').then(function(r){return r.json();}); } catch(e) {}
    var allCreds = credsData.credentials || [];

    // Dedicated card: ChatGPT Codex connection via subscription (OAuth).
    var html = '<div class="settings-card" id="codexAuthCard" style="margin-bottom:16px;border:1px solid var(--amber)">'+
      '<div class="settings-card-title">ChatGPT Codex <span style="color:var(--text-dim);font-size:10px;font-weight:normal">'+LaRuche.i18n.t('settings.codexSubscription')+'</span></div>'+
      '<div id="codexAuthBox" style="font-size:12px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.codexLoading')+'</div>'+
      '</div>';

    html += '<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="LaRuche.Settings.showProfileForm()">'+LaRuche.i18n.t('settings.addProvider')+'</button></div>';
    html += '<div id="profileFormContainer" style="display:none"></div>';

    // (MCP servers now have their own "MCP" tab, see loadMcp.)

    html += '<div class="settings-grid">';
    var sharedHtml = '';

    ids.forEach(function(id) {
      var p = profiles[id];
      var isActive = (id === active.profile_id);
      var modelCount = (p.models || []).length;
      var provLabel = p.provider === 'ollama' ? 'Ollama' : p.provider === 'anthropic' ? 'Anthropic' : p.provider === 'codex' ? 'ChatGPT Codex' : 'OpenAI-compat';
      // SHARED BY A PEER: base_url = private LAN IP (not loopback) -> separate card, read-only
      // (we don't re-share / edit someone else's provider).
      var _bu = (p.base_url||'').toLowerCase();
      var _shared = /(^|\/\/)(10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.)/.test(_bu) && !/127\.0\.0\.1|localhost/.test(_bu);
      if(_shared){
        sharedHtml += '<div class="settings-card">'+
          '<div class="settings-card-title" style="display:flex;align-items:center;gap:6px;flex-wrap:wrap"><span>'+LaRuche.Utils.esc(p.name)+'</span>'+
          '<span style="color:var(--cyan);font-size:10px;font-weight:normal">'+LaRuche.i18n.t('settings.sharedReadOnly')+'</span></div>'+
          '<div class="settings-row"><span class="settings-label">URL</span><span class="settings-value" style="font-size:10px;word-break:break-all">'+LaRuche.Utils.esc(p.base_url)+'</span></div>'+
          '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.modelsLabel')+'</span><span class="settings-value">'+modelCount+'</span></div>'+
          '<div style="margin-top:10px"><button onclick="LaRuche.Settings.deleteProfile(\''+id+'\')" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.removeFromList')+'</button></div>'+
          '</div>';
        return; // no normal card: neither "Make Public" nor "Edit"
      }

      var pCreds = allCreds.filter(function(c){ return c.provider.toLowerCase() === p.provider.toLowerCase(); });
      var credsHtml = '';
      if(pCreds.length > 0) {
        credsHtml += '<div style="margin-top:10px;padding-top:10px;border-top:1px dashed var(--border)">';
        credsHtml += '<div style="font-size:10px;color:var(--text-dim);margin-bottom:6px;font-weight:bold">'+LaRuche.i18n.t('settings.credPool')+'</div>';
        pCreds.forEach(function(c){
           var maskedKey = c.api_key ? (c.api_key.substring(0,6) + '...' + c.api_key.substring(c.api_key.length-4)) : '';
           var cdText = c.cooldown_until ? (' <span style="color:var(--red)">(cooldown)</span>') : '';
           var lbl = c.label ? ('<span style="color:var(--amber);margin-right:6px">['+LaRuche.Utils.esc(c.label)+']</span>') : '';
           credsHtml += '<div style="font-size:10px;display:flex;justify-content:space-between;align-items:center;margin-bottom:4px;background:var(--bg-lighter);padding:4px;border-radius:4px">'+
             '<div>'+lbl+'<span style="font-family:monospace">'+LaRuche.Utils.esc(maskedKey)+'</span> '+cdText+' <span style="color:var(--text-dim)">['+c.request_count+' reqs]</span></div>'+
             '<button onclick="LaRuche.Settings.deleteCredential(\''+p.provider+'\', \''+c.api_key+'\')" style="background:none;border:none;color:var(--red);cursor:pointer;font-size:12px;padding:0 4px" title="'+LaRuche.i18n.t('settings.deleteCred')+'">&times;</button>'+
             '</div>';
        });
        credsHtml += '</div>';
      }
      var addCredBtn = '<button onclick="LaRuche.Settings.addCredential(\''+p.provider+'\')" style="margin-top:8px;background:none;border:1px dashed var(--border);color:var(--text-dim);border-radius:4px;padding:4px 10px;cursor:pointer;font-size:10px;width:100%">'+LaRuche.i18n.t('settings.addCredKey')+'</button>';

      var _vis = p.visibility || 'prive';
      var _nAllowed = (p.allowed_peers||[]).length;
      var visBadge = _vis==='public_proxy'
        ? '<span style="color:var(--blue);font-size:10px;font-weight:bold;margin-left:8px;">'+LaRuche.i18n.t('settings.visPublic')+'</span>'
        : _vis==='restricted'
        ? '<span style="color:var(--cyan);font-size:10px;font-weight:bold;margin-left:8px;">'+LaRuche.i18n.t('settings.visRestrictedN')+' ('+_nAllowed+')</span>'
        : '<span style="color:var(--text-dim);font-size:10px;font-weight:bold;margin-left:8px;">'+LaRuche.i18n.t('settings.visPrivate')+'</span>';
      var visToggleBtn = '<button onclick="LaRuche.Settings.openAccess(\''+id+'\', \''+_vis+'\', \''+encodeURIComponent(JSON.stringify(p.allowed_peers||[]))+'\')" style="margin-left:auto;background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;">'+LaRuche.i18n.t('settings.accessBtn')+'</button>';
      html += '<div class="settings-card" style="'+(isActive?'border:1px solid var(--amber);':'')+'">'+
        '<div class="settings-card-title" style="display:flex;align-items:center;"><span>'+LaRuche.Utils.esc(p.name)+'</span>'+
        (isActive?' <span style="color:var(--amber);font-size:10px;font-weight:normal;margin-left:4px;">'+LaRuche.i18n.t('settings.activeBadge')+'</span>':'')+
        visBadge+visToggleBtn+
        '</div>'+
        '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.typeLabel')+'</span><span class="settings-value">'+provLabel+'</span></div>'+
        '<div class="settings-row"><span class="settings-label">URL</span><span class="settings-value" style="font-size:10px;word-break:break-all">'+LaRuche.Utils.esc(p.base_url)+'</span></div>'+
        '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.apiKeyLabel')+'</span><span class="settings-value">'+(p.api_key?LaRuche.i18n.t('settings.apiKeySet'):LaRuche.i18n.t('settings.apiKeyNone'))+'</span></div>'+
        '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.modelsLabel')+'</span><span class="settings-value">'+modelCount+'</span></div>'+
        credsHtml + addCredBtn +
        '<div style="margin-top:12px;display:flex;gap:6px">'+
        '<button id="pftestbtn-'+id+'" onclick="LaRuche.Settings.testProfile(\''+id+'\')" style="background:none;border:1px solid var(--cyan);color:var(--cyan);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.testBtn')+'</button>'+
        '<button onclick="LaRuche.Settings.editProfile(\''+id+'\')" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.editBtn')+'</button>'+
        '<button onclick="LaRuche.Settings.deleteProfile(\''+id+'\')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.deleteBtn')+'</button>'+
        '</div>'+
        '<div id="pftest-'+id+'" style="margin-top:8px;font-size:10px;line-height:1.4;word-break:break-word"></div>'+
        '</div>';
    });

    html += '</div>';
    if(sharedHtml){
      html += '<div class="settings-card-title" style="margin:18px 0 8px;color:var(--cyan)">'+LaRuche.i18n.t('settings.sharedWithMe')+'</div>'+
        '<div style="color:var(--text-dim);font-size:11px;margin-bottom:10px">'+LaRuche.i18n.t('settings.sharedHint')+'</div>'+
        '<div class="settings-grid">'+sharedHtml+'</div>';
    }
    el.innerHTML = html;
    refreshCodexStatus();
  }

  // ── ChatGPT Codex (OAuth subscription) ──────────────────────────
  var _codexPoll = null;

  function renderCodexBox(s) {
    var box = document.getElementById('codexAuthBox');
    if(!box) return;
    s = s || {};
    if(s.phase === 'connected') {
      box.innerHTML = '<div style="color:var(--green)">'+LaRuche.i18n.t('settings.codexConnected')+
        (s.account_id?(' <span style="color:var(--text-dim)">('+LaRuche.Utils.esc(s.account_id)+')</span>'):'')+'</div>'+
        (s.expiring?'<div style="color:var(--amber);font-size:10px;margin-top:4px">'+LaRuche.i18n.t('settings.codexExpiring')+'</div>':'')+
        '<div style="margin-top:8px"><button onclick="LaRuche.Settings.logoutCodex()" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.codexDisconnect')+'</button></div>';
    } else if(s.phase === 'pending' && s.user_code) {
      box.innerHTML = '<div>'+LaRuche.i18n.t('settings.codexConnectInstr')+'</div>'+
        '<ol style="margin:6px 0 6px 16px;padding:0;line-height:1.7">'+
        '<li>'+LaRuche.i18n.t('settings.codexStep1')+' <a href="'+LaRuche.Utils.esc(s.verification_url)+'" target="_blank" rel="noopener" style="color:var(--amber)">'+LaRuche.Utils.esc(s.verification_url)+'</a></li>'+
        '<li>'+LaRuche.i18n.t('settings.codexStep2')+' <span style="font-size:16px;font-weight:bold;color:var(--amber);letter-spacing:2px">'+LaRuche.Utils.esc(s.user_code)+'</span></li>'+
        '</ol>'+
        '<div style="color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('settings.codexWaiting')+'</div>';
    } else if(s.phase === 'error') {
      box.innerHTML = '<div style="color:var(--red)">'+LaRuche.i18n.t('settings.codexError')+LaRuche.Utils.esc(s.message||'error')+'</div>'+
        '<div style="margin-top:8px"><button onclick="LaRuche.Settings.startCodexLogin()" style="background:var(--amber);border:none;color:#000;border-radius:4px;padding:4px 12px;cursor:pointer;font-size:11px">'+LaRuche.i18n.t('settings.codexRetry')+'</button></div>';
    } else {
      box.innerHTML = '<div>'+LaRuche.i18n.t('settings.codexUseSubscription')+'</div>'+
        '<div style="margin-top:8px"><button onclick="LaRuche.Settings.startCodexLogin()" style="background:var(--amber);border:none;color:#000;border-radius:4px;padding:4px 12px;cursor:pointer;font-size:11px;font-weight:bold">'+LaRuche.i18n.t('settings.codexSignIn')+'</button></div>';
    }
  }

  function refreshCodexStatus() {
    fetch('/api/auth/codex/status').then(function(r){return r.json();})
      .then(renderCodexBox).catch(function(){});
  }

  function startCodexLogin() {
    var box = document.getElementById('codexAuthBox');
    if(box) box.innerHTML = '<div style="color:var(--text-dim)">'+LaRuche.i18n.t('settings.codexInit')+'</div>';
    fetch('/api/auth/codex/start',{method:'POST'}).then(function(r){return r.json();})
      .then(function(s){
        renderCodexBox(s);
        if(s.phase === 'pending' && s.user_code) startCodexPoll();
      }).catch(function(){
        renderCodexBox({phase:'error',message:LaRuche.i18n.t('settings.codexNetwork')});
      });
  }

  function startCodexPoll() {
    if(_codexPoll) clearInterval(_codexPoll);
    _codexPoll = setInterval(function(){
      fetch('/api/auth/codex/status').then(function(r){return r.json();}).then(function(s){
        if(s.phase === 'connected' || s.phase === 'error') {
          clearInterval(_codexPoll); _codexPoll = null;
          renderCodexBox(s);
          if(s.phase === 'connected' && LaRuche.Models && LaRuche.Models.loadModels) LaRuche.Models.loadModels();
        } else {
          renderCodexBox(s);
        }
      }).catch(function(){});
    }, 3000);
  }

  function logoutCodex() {
    if(!confirm(LaRuche.i18n.t('settings.codexLogoutConfirm'))) return;
    fetch('/api/auth/codex/logout',{method:'POST'}).then(function(){ refreshCodexStatus(); });
  }

  function showProfileForm(editId) {
    var container = document.getElementById('profileFormContainer');
    if(!container) return;
    // If editing, fetch current data
    if(editId) {
      fetch('/api/profiles').then(function(r){return r.json();}).then(function(data){
        var p = (data.profiles||{})[editId];
        if(p) renderProfileForm(container, editId, p);
      });
    } else {
      renderProfileForm(container, '', null);
    }
  }

  function renderProfileForm(container, editId, existing) {
    var p = existing || {};
    var provType = p.provider || 'ollama';
    var defaultUrls = {ollama:'http://127.0.0.1:11434', openai:'https://api.openai.com', anthropic:'https://api.anthropic.com'};
    container.style.display = 'block';
    container.innerHTML = '<div class="settings-card" style="margin-bottom:16px">'+
      '<div class="settings-card-title">'+(editId?LaRuche.i18n.t('settings.editProviderTitle'):LaRuche.i18n.t('settings.addProviderTitle'))+'</div>'+
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfIdLabel')+(editId?LaRuche.i18n.t('settings.pfIdReadonly'):'')+'</label>'+
      '<input class="form-input" id="pfId" value="'+LaRuche.Utils.esc(editId)+'" '+(editId?'readonly':'')+' placeholder="'+LaRuche.i18n.t('settings.pfIdPlaceholder')+'"></div>'+
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfNameLabel')+'</label>'+
      '<input class="form-input" id="pfName" value="'+LaRuche.Utils.esc(p.name||'')+'" placeholder="'+LaRuche.i18n.t('settings.pfNamePlaceholder')+'"></div>'+
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfProviderTypeLabel')+'</label>'+
      '<select class="form-select" id="pfProvider" onchange="LaRuche.Settings.onProfileProviderChange()">'+
      '<option value="ollama"'+(provType==='ollama'?' selected':'')+'>Ollama</option>'+
      '<option value="openai"'+(provType==='openai'?' selected':'')+'>OpenAI-compatible</option>'+
      '<option value="anthropic"'+(provType==='anthropic'?' selected':'')+'>Anthropic</option>'+
      '</select></div>'+
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfBaseUrlLabel')+'</label>'+
      '<input class="form-input" id="pfBaseUrl" value="'+LaRuche.Utils.esc(p.base_url||defaultUrls[provType]||'')+'" placeholder="'+defaultUrls[provType]+'"></div>'+
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfApiKeyLabel')+'</label>'+
      '<input class="form-input" id="pfApiKey" type="password" value="'+LaRuche.Utils.esc(p.api_key||'')+'" placeholder="'+LaRuche.i18n.t('settings.pfApiKeyPlaceholder')+'" autocomplete="off"></div>'+
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfModelsLabel')+'</label>'+
      '<input class="form-input" id="pfModels" value="'+LaRuche.Utils.esc((p.models||[]).join(', '))+'" placeholder="'+LaRuche.i18n.t('settings.pfModelsPlaceholder')+'"></div>'+
      '<div style="display:flex;gap:8px;margin-top:8px">'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.saveProfile()">'+LaRuche.i18n.t('settings.pfSave')+'</button>'+
      '<button style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:6px;padding:6px 16px;cursor:pointer" onclick="document.getElementById(\'profileFormContainer\').style.display=\'none\'">'+LaRuche.i18n.t('settings.pfCancel')+'</button>'+
      '</div></div>';
  }

  function onProfileProviderChange() {
    var prov = document.getElementById('pfProvider').value;
    var urlField = document.getElementById('pfBaseUrl');
    var defaultUrls = {ollama:'http://127.0.0.1:11434', openai:'https://api.openai.com', anthropic:'https://api.anthropic.com'};
    if(urlField && !urlField.value || urlField.value.indexOf('127.0.0.1') !== -1 || urlField.value.indexOf('api.openai.com') !== -1 || urlField.value.indexOf('api.anthropic.com') !== -1) {
      urlField.value = defaultUrls[prov] || '';
    }
  }

  function saveProfile() {
    var id = (document.getElementById('pfId').value||'').trim();
    if(!id) { LaRuche.Toast.show(LaRuche.i18n.t('settings.profileIdRequired'),'err'); return; }
    var name = document.getElementById('pfName').value || id;
    var provider = document.getElementById('pfProvider').value;
    var baseUrl = document.getElementById('pfBaseUrl').value;
    var apiKey = document.getElementById('pfApiKey').value;
    var modelsRaw = document.getElementById('pfModels').value;
    var models = modelsRaw ? modelsRaw.split(',').map(function(s){return s.trim();}).filter(function(s){return s;}) : [];

    fetch('/api/profiles',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({
      id:id, name:name, provider:provider, base_url:baseUrl, api_key:apiKey, models:models
    })}).then(function(r){return r.json();}).then(function(d){
      if(d.status==='ok') {
        LaRuche.Toast.show(LaRuche.i18n.t('settings.profileSavedPrefix')+d.name+LaRuche.i18n.t('settings.profileSavedSuffix'),'ok');
        document.getElementById('profileFormContainer').style.display = 'none';
        loadTab('providers');
        // Refresh dropdown after a short delay to ensure server has processed
        setTimeout(function(){ LaRuche.Header.loadModels(); }, 300);
      } else {
        LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+(d.error||'?'),'err');
      }
    }).catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err');});
  }

  function editProfile(id) {
    showProfileForm(id);
  }

  function deleteProfile(id) {
    if(!confirm(LaRuche.i18n.t('settings.deleteProfile')+id+'"?')) return;
    fetch('/api/profiles/'+id,{method:'DELETE'}).then(function(r){return r.json();}).then(function(d){
      if(d.status==='ok') {
        LaRuche.Toast.show(LaRuche.i18n.t('settings.profileDeleted'),'ok');
        loadTab('providers');
        LaRuche.Header.loadModels();
      } else {
        LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+(d.error||'?'),'err');
      }
    });
  }

  function testProfile(id) {
    var out = document.getElementById('pftest-'+id);
    var btn = document.getElementById('pftestbtn-'+id);
    if(out){ out.innerHTML = '<span style="color:var(--text-dim)">'+LaRuche.i18n.t('settings.testRunning')+'</span>'; }
    if(btn){ btn.disabled = true; }
    fetch('/api/profiles/'+id+'/test',{method:'POST'}).then(function(r){return r.json();}).then(function(d){
      if(btn){ btn.disabled = false; }
      if(!out) return;
      if(d.ok){
        out.innerHTML = '<span style="color:var(--green)">✔ '+LaRuche.i18n.t('settings.testOk')+(d.model?' ('+LaRuche.Utils.esc(d.model)+')':'')+'</span>';
      } else {
        var st = d.status?(' [HTTP '+d.status+']'):'';
        out.innerHTML = '<span style="color:var(--red)">✗ '+LaRuche.i18n.t('settings.testFail')+st+'</span>'+
          (d.message?'<div style="color:var(--text-dim);margin-top:2px">'+LaRuche.Utils.esc(d.message)+'</div>':'');
      }
    }).catch(function(){
      if(btn){ btn.disabled = false; }
      if(out){ out.innerHTML = '<span style="color:var(--red)">✗ '+LaRuche.i18n.t('settings.testFail')+'</span>'; }
    });
  }

  async function loadTools(el) {
    var tools=[];try{tools=await fetch('/api/tools').then(function(r){return r.json();});}catch(e){}
    tools.sort(function(a,b){return String(a.name).localeCompare(String(b.name));});
    window._allTools = tools;
    
    var html = '<div style="display:flex;justify-content:flex-end;gap:10px;margin-bottom:20px;">';
    html += '<button onclick="LaRuche.Settings.toggleAllTools(true)" style="background:rgba(16,185,129,0.15);color:var(--green);border:1px solid var(--green);padding:6px 14px;border-radius:6px;cursor:pointer;font-size:12px;font-weight:600;transition:all 0.2s;" onmouseover="this.style.background=\'var(--green)\';this.style.color=\'#000\'" onmouseout="this.style.background=\'rgba(16,185,129,0.15)\';this.style.color=\'var(--green)\'">'+LaRuche.i18n.t('settings.enableAll')+'</button>';
    html += '<button onclick="LaRuche.Settings.toggleAllTools(false)" style="background:rgba(239,68,68,0.15);color:var(--red);border:1px solid var(--red);padding:6px 14px;border-radius:6px;cursor:pointer;font-size:12px;font-weight:600;transition:all 0.2s;" onmouseover="this.style.background=\'var(--red)\';this.style.color=\'#000\'" onmouseout="this.style.background=\'rgba(239,68,68,0.15)\';this.style.color=\'var(--red)\'">'+LaRuche.i18n.t('settings.disableAll')+'</button>';
    html += '</div>';

    html += '<div class="settings-grid">'+tools.map(function(t, idx){
      var enabled = t.enabled !== false;
      var originBadge = (t.origin === 'Custom') ? '<span style="margin-left:8px;font-size:9px;color:var(--purple);border:1px solid var(--purple-dim);background:var(--purple-dim);padding:2px 4px;border-radius:4px;">'+LaRuche.i18n.t('settings.toolCustomBadge')+'</span>' : '<span style="margin-left:8px;font-size:9px;color:var(--text-dim);border:1px solid var(--border);padding:2px 4px;border-radius:4px;">'+LaRuche.i18n.t('settings.toolNativeBadge')+'</span>';
      var customActions = (t.origin === 'Custom') ? '<div style="margin-top:10px;display:flex;gap:8px;border-top:1px solid rgba(255,255,255,0.05);padding-top:8px;"><button style="background:none;border:1px solid var(--border);color:var(--text-muted);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();LaRuche.Toast.show(LaRuche.i18n.t(\'settings.pluginSrcUnavailable\'),\'err\')">'+LaRuche.i18n.t('settings.viewSource')+'</button><button style="background:none;border:1px solid var(--border);color:var(--text-muted);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();LaRuche.Toast.show(LaRuche.i18n.t(\'settings.pluginJsonNoEdit\'),\'err\')">'+LaRuche.i18n.t('settings.editJson')+'</button><button style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();fetch(\'/api/tools/\'+LaRuche.Utils.esc(t.name),{method:\'DELETE\'}).then(function(){LaRuche.Settings.refreshTab()})">'+LaRuche.i18n.t('settings.tlDelete')+'</button></div>' : '';
      return '<div class="settings-card" style="cursor:pointer; transition:transform 0.2s, box-shadow 0.2s; position:relative;" onmouseover="this.style.transform=\'translateY(-2px)\';this.style.boxShadow=\'0 4px 12px rgba(0,0,0,0.3)\';" onmouseout="this.style.transform=\'\';this.style.boxShadow=\'\';" onclick="LaRuche.Utils.openMediaModal(\'text\', JSON.stringify(window._allTools['+idx+'], null, 2))">'+
        '<div class="settings-card-title" style="display:flex;justify-content:space-between;gap:8px;align-items:center">'+
          '<span style="color:var(--cyan);font-weight:600;">'+LaRuche.Utils.esc(t.name)+originBadge+'</span>'+
          '<label onclick="event.stopPropagation()" style="display:flex;align-items:center;gap:6px;color:'+(enabled?'var(--green)':'var(--red)')+';font-size:10px;text-transform:none;letter-spacing:0;background:'+(enabled?'rgba(16,185,129,0.1)':'rgba(239,68,68,0.1)')+';padding:3px 8px;border-radius:12px;font-weight:bold;">'+
            '<input type="checkbox" '+(enabled?'checked':'')+' onchange="LaRuche.Settings.toggleTool(\''+LaRuche.Utils.esc(t.name)+'\',this.checked)"> '+(enabled?LaRuche.i18n.t('settings.toolOn'):LaRuche.i18n.t('settings.toolOff'))+
          '</label>'+
        '</div>'+
        '<div class="settings-row" style="margin-top:8px;"><span class="settings-label">'+LaRuche.i18n.t('settings.toolDanger')+'</span><span class="settings-value" style="color:'+(t.danger==='high'?'var(--red)':(t.danger==='medium'?'var(--orange)':'var(--text-dim)'))+';font-weight:bold;">'+LaRuche.Utils.esc(t.danger||LaRuche.i18n.t('settings.toolDangerSafe'))+'</span></div>'+
        '<div style="font-size:12px;color:var(--text-dim);line-height:1.5;margin-top:10px;border-top:1px solid rgba(255,255,255,0.05);padding-top:10px;">'+LaRuche.Utils.esc((t.description||'').substring(0,180))+'</div>'+
        customActions+
      '</div>';
    }).join('')+'</div>';
    
    el.innerHTML = html;
    if(!tools.length) el.innerHTML='<div style="text-align:center;color:var(--text-muted);padding:20px">'+LaRuche.i18n.t('settings.toolsEmpty')+'</div>';
  }

  async function toggleAllTools(enable) {
    var disabled = enable ? [] : (window._allTools || []).map(function(t){return t.name;});
    fetch('/api/tools/config',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({disabled_tools:disabled})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d.status !== 'ok') LaRuche.Toast.show(LaRuche.i18n.t('settings.toolsConfigErr'),'err');
        else { LaRuche.Toast.show(enable ? LaRuche.i18n.t('settings.allToolsEnabled') : LaRuche.i18n.t('settings.allToolsDisabled'),'ok'); loadTab('tools'); }
      });
  }

  async function toggleTool(name, enabled) {
    var tools=[];try{tools=await fetch('/api/tools').then(function(r){return r.json();});}catch(e){}
    var disabled = tools.filter(function(t){return t.enabled === false;}).map(function(t){return t.name;});
    var idx = disabled.indexOf(name);
    if(enabled && idx !== -1) disabled.splice(idx,1);
    if(!enabled && idx === -1) disabled.push(name);
    fetch('/api/tools/config',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({disabled_tools:disabled})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d.status !== 'ok') LaRuche.Toast.show(LaRuche.i18n.t('settings.toolsConfigErr'),'err');
        else { LaRuche.Toast.show(name+(enabled?LaRuche.i18n.t('settings.toolEnabled'):LaRuche.i18n.t('settings.toolDisabled')),'ok'); loadTab('tools'); }
      })
      .catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('settings.toolsErr')+e,'err');});
  }

  async function loadNetwork(el) {
    var codeSet=false; try{ codeSet=(await fetch('/api/mesh/code').then(function(r){return r.json();})).set; }catch(e){}
    var codeCard='<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.meshCodeTitle')+' '+
      (codeSet?'<span style="color:var(--green);font-size:11px">'+LaRuche.i18n.t('settings.meshCodeConfigured')+'</span>':'<span style="color:var(--text-muted);font-size:11px">'+LaRuche.i18n.t('settings.meshCodeUnconfigured')+'</span>')+'</div>'+
      '<p style="color:var(--text-dim);font-size:12px;margin:4px 0 8px">'+LaRuche.i18n.t('settings.meshCodeHint')+'</p>'+
      '<div style="display:flex;gap:8px"><input id="meshCodeInput" type="password" placeholder="'+(codeSet?LaRuche.i18n.t('settings.meshCodePlaceholderSet'):LaRuche.i18n.t('settings.meshCodePlaceholderNew'))+'" style="flex:1;background:var(--bg-input);color:var(--text);border:1px solid var(--border);border-radius:8px;padding:8px 10px;font-size:14px"><button class="send-btn" id="meshCodeSave"><span>'+LaRuche.i18n.t('settings.meshSave')+'</span></button></div></div>';
    var d={nodes:[]};try{d=await fetch('/swarm').then(function(r){return r.json();});}catch(e){}
    var nodesHtml=(d.nodes||[]).map(function(n){
      var caps=(n.capabilities||[]).map(function(c){return '<span style="background:rgba(6,182,212,.15);color:var(--cyan);padding:1px 6px;border-radius:8px;font-size:10px">'+c+'</span>';}).join(' ');
      return '<div class="settings-card"><div class="settings-card-title">'+(n.name||'?')+'</div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.hostLabel')+'</span><span class="settings-value">'+n.host+':'+(n.port||'?')+'</span></div><div style="margin-top:4px">'+caps+'</div></div>';
    }).join('')||'<div style="text-align:center;color:var(--text-muted);padding:20px">'+LaRuche.i18n.t('settings.noNodes')+'</div>';
    el.innerHTML=codeCard+nodesHtml;
    var btn=document.getElementById('meshCodeSave');
    if(btn) btn.onclick=async function(){
      var v=(document.getElementById('meshCodeInput').value||'');
      if(!v.trim()){ LaRuche.Toast.show(LaRuche.i18n.t('settings.meshCodeUnchanged'),'info'); return; }
      try{ await fetch('/api/mesh/code',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({code:v})});
        LaRuche.Toast.show(LaRuche.i18n.t('settings.meshCodeSaved'),'ok'); loadNetwork(el);
      }catch(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.meshCodeFailed'),'err'); }
    };
  }

  // ── Cron timeline (ported from third-party PR #47944, in vanilla JS) ──────
  var _tlSpanH = 24;            // window: 24 / 48 / 168 h
  var _tlFromMs = 0;           // left edge
  var _tlJobs = [];
  var _tlTimer = null;
  var _tlHost = null;          // render container element
  var _tlPxPerH = 64;          // px per hour (depends on zoom)
  function ensureTimelineStyle(){
    if(document.getElementById('lr-tl-style'))return;
    var s=document.createElement('style'); s.id='lr-tl-style';
    s.textContent=
      '.tl-ctrls{display:flex;gap:8px;align-items:center;margin-bottom:10px;flex-wrap:wrap}'+
      '.tl-seg{display:flex;border:1px solid var(--border);border-radius:6px;overflow:hidden}'+
      '.tl-seg button{background:none;border:none;color:var(--text-dim);padding:4px 12px;cursor:pointer;font-size:11px}'+
      '.tl-seg button.on{background:var(--amber);color:#000;font-weight:600}'+
      '.tl-btn{background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:6px;padding:4px 12px;cursor:pointer;font-size:11px}'+
      '.tl-wrap{display:flex;border:1px solid var(--border);border-radius:8px;overflow:hidden;background:rgba(20,20,24,.5)}'+
      '.tl-gutter{flex:0 0 130px;border-right:1px solid var(--border);background:rgba(30,30,34,.7);position:sticky;left:0;z-index:2}'+
      '.tl-scroll{flex:1;overflow-x:auto;overflow-y:hidden;touch-action:pan-x pan-y;position:relative}'+
      '.tl-strip{position:relative}'+
      '.tl-row{height:44px;border-bottom:1px solid rgba(255,255,255,.04);position:relative}'+
      '.tl-name{height:44px;display:flex;flex-direction:column;justify-content:center;padding:0 10px;border-bottom:1px solid rgba(255,255,255,.04);font-size:11px;overflow:hidden}'+
      '.tl-name .n{color:var(--text);white-space:nowrap;text-overflow:ellipsis;overflow:hidden;font-weight:600}'+
      '.tl-name .s{color:var(--text-dim);font-size:9px}'+
      '.tl-head{height:24px;border-bottom:1px solid var(--border);position:relative}'+
      '.tl-tick{position:absolute;top:0;bottom:0;border-left:1px solid rgba(255,255,255,.06);font-size:9px;color:var(--text-dim);padding-left:3px}'+
      '.tl-now{position:absolute;top:0;bottom:0;width:2px;background:var(--amber);box-shadow:0 0 8px 1px var(--amber);z-index:3;pointer-events:none}'+
      '.tl-mk{position:absolute;top:50%;transform:translate(-50%,-50%);width:10px;height:10px;border-radius:50%;background:var(--text-dim);cursor:pointer;transition:transform .1s}'+
      '.tl-mk:hover{transform:translate(-50%,-50%) scale(1.4)}'+
      '.tl-mk.next{width:13px;height:13px;border-radius:2px;transform:translate(-50%,-50%) rotate(45deg);background:var(--amber);box-shadow:0 0 8px 1px var(--amber)}'+
      '.tl-mk.past{opacity:.4}.tl-mk.future{opacity:.65}.tl-mk.err{background:var(--red)!important}'+
      '.tl-row.paused{opacity:.45}'+
      '.tl-detail{margin-top:12px;border:1px solid var(--amber);border-radius:8px;padding:12px;font-size:12px}';
    document.head.appendChild(s);
  }
  function tlMatches(expr, d){
    var p=(expr||'').trim().split(/\s+/); if(p.length<5)return false;
    function f(field,val,min,max){
      if(field==='*'||field==='?')return true;
      return field.split(',').some(function(tok){
        var step=1,range=tok,sl=tok.split('/'); if(sl.length===2){range=sl[0];step=parseInt(sl[1])||1;}
        var lo,hi;
        if(range==='*'){lo=min;hi=max;}
        else if(range.indexOf('-')>=0){var r=range.split('-');lo=parseInt(r[0]);hi=parseInt(r[1]);}
        else {lo=hi=parseInt(range);}
        if(isNaN(lo))return false; if(val<lo||val>hi)return false; return ((val-lo)%step)===0;
      });
    }
    return f(p[0],d.getMinutes(),0,59)&&f(p[1],d.getHours(),0,23)&&f(p[2],d.getDate(),1,31)&&f(p[3],d.getMonth()+1,1,12)&&f(p[4],d.getDay(),0,6);
  }
  function tlOccurrences(job, fromMs, toMs){
    var occ=[];
    if(job.fire_at){var t=Date.parse(job.fire_at); if(t>=fromMs&&t<=toMs)occ.push(t); return occ;}
    if(!job.cron_expr)return occ;
    var start=Math.ceil(fromMs/60000)*60000;
    for(var t=start;t<=toMs&&occ.length<600;t+=60000){ if(tlMatches(job.cron_expr,new Date(t)))occ.push(t); }
    return occ;
  }
  async function loadCronTimeline(el){
    ensureTimelineStyle(); _tlHost=el;
    try{_tlJobs=await fetch('/api/cron').then(function(r){return r.json();});}catch(e){_tlJobs=[];}
    var spanMs=_tlSpanH*3600000; _tlFromMs=Date.now()-0.28*spanMs;
    renderTimeline(el);
    if(_tlTimer)clearInterval(_tlTimer);
    _tlTimer=setInterval(function(){ var nowLine=document.getElementById('tlNow'); if(!nowLine){clearInterval(_tlTimer);return;} positionNow(); },1000);
  }
  function positionNow(){
    var strip=document.getElementById('tlStrip'); var nowEl=document.getElementById('tlNow'); if(!strip||!nowEl)return;
    var spanMs=_tlSpanH*3600000; var w=strip.offsetWidth;
    nowEl.style.left=((Date.now()-_tlFromMs)/spanMs*w)+'px';
  }
  function renderTimeline(el){
    var spanMs=_tlSpanH*3600000, toMs=_tlFromMs+spanMs;
    var pxPerH=_tlSpanH<=24?64:(_tlSpanH<=48?34:12); _tlPxPerH=pxPerH; var width=_tlSpanH*pxPerH;
    function seg(h,lbl){return '<button class="'+(_tlSpanH===h?'on':'')+'" onclick="LaRuche.Settings.tlZoom('+h+')">'+lbl+'</button>';}
    var html='<div class="tl-ctrls"><div class="tl-seg">'+seg(24,'24h')+seg(48,'48h')+seg(168,LaRuche.i18n.t('common.range7d'))+'</div>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlRecenter()">'+LaRuche.i18n.t('settings.recenter')+'</button>'+
      '<span style="color:var(--text-dim);font-size:10px">'+_tlJobs.length+' '+LaRuche.i18n.t('settings.cronCount')+'</span></div>';
    if(!_tlJobs.length){ el.innerHTML=html+'<div style="color:var(--text-dim);padding:20px">'+LaRuche.i18n.t('settings.noCron')+'</div>'; return; }
    // axe (ticks)
    var ticks=''; var stepH=_tlSpanH<=24?2:(_tlSpanH<=48?4:24);
    for(var h=0;h<=_tlSpanH;h+=stepH){ var d=new Date(_tlFromMs+h*3600000);
      var lbl=_tlSpanH<=48?(('0'+d.getHours()).slice(-2)+'h'):((d.getDate())+'/'+(d.getMonth()+1));
      ticks+='<div class="tl-tick" style="left:'+(h*pxPerH)+'px">'+lbl+'</div>'; }
    var gutter='<div class="tl-head"></div>', lanes='';
    _tlJobs.forEach(function(job,i){
      var paused=job.enabled===false;
      gutter+='<div class="tl-name'+(paused?' tl-row paused':'')+'" onclick="LaRuche.Settings.tlDetail('+i+')"><span class="n">'+LaRuche.Utils.esc(job.name||LaRuche.i18n.t('settings.tlNoName'))+'</span><span class="s">'+LaRuche.Utils.esc(job.cron_expr||job.fire_at||'')+'</span></div>';
      var occ=tlOccurrences(job,_tlFromMs,toMs); var now=Date.now();
      var nextT=occ.find(function(t){return t>=now;});
      var mk='';
      occ.forEach(function(t){
        var cls=t<now?'past':(t===nextT?'next':'future');
        var err=(job.last_status==='error');
        mk+='<span class="tl-mk '+cls+(err&&cls==='next'?' err':'')+'" style="left:'+((t-_tlFromMs)/spanMs*width)+'px" title="'+new Date(t).toLocaleString('fr-FR')+'" onclick="LaRuche.Settings.tlDetail('+i+')"></span>';
      });
      lanes+='<div class="tl-row'+(paused?' paused':'')+'" data-i="'+i+'" title="'+LaRuche.i18n.t('settings.tlDragHint')+'">'+mk+'</div>';
    });
    html+='<div class="tl-wrap"><div class="tl-gutter">'+gutter+'</div><div class="tl-scroll"><div class="tl-strip" id="tlStrip" style="width:'+width+'px">'+
      '<div class="tl-head" style="width:'+width+'px">'+ticks+'</div>'+lanes+
      '<div class="tl-now" id="tlNow"></div></div></div></div><div id="tlDetail"></div>';
    el.innerHTML=html; positionNow();
    // auto-scroll to place "now" ~28% from the left edge
    var sc=el.querySelector('.tl-scroll'); if(sc){ var nowX=(Date.now()-_tlFromMs)/spanMs*width; sc.scrollLeft=Math.max(0,nowX-sc.offsetWidth*0.28); }
    wireTlDrag(el);
  }
  // Horizontal drag of a lane -> shifts the hour of a fixed-time cron ("m h * * ...").
  function wireTlDrag(el){
    el.querySelectorAll('.tl-row[data-i]').forEach(function(row){
      var startX=0, dragging=false, moved=0;
      row.style.cursor='grab';
      row.addEventListener('pointerdown',function(e){ startX=e.clientX; dragging=true; moved=0; row.setPointerCapture(e.pointerId); row.style.cursor='grabbing'; });
      row.addEventListener('pointermove',function(e){ if(!dragging)return; moved=e.clientX-startX; row.style.transform='translateX('+(moved*0.15)+'px)'; });
      row.addEventListener('pointerup',function(e){
        if(!dragging)return; dragging=false; row.style.cursor='grab'; row.style.transform='';
        if(Math.abs(moved)<8) return; // simple click -> handled by the marker
        var job=_tlJobs[parseInt(row.getAttribute('data-i'))]; if(!job||!job.cron_expr){ LaRuche.Toast.show(LaRuche.i18n.t('settings.tlShiftUnsupported'),'warn'); return; }
        var p=job.cron_expr.trim().split(/\s+/); if(p.length<5||isNaN(parseInt(p[1]))){ LaRuche.Toast.show(LaRuche.i18n.t('settings.tlShiftFixed'),'warn'); return; }
        var dh=Math.round(moved/_tlPxPerH); if(dh===0)return;
        var nh=((parseInt(p[1])+dh)%24+24)%24; p[1]=String(nh);
        var expr=p.join(' ');
        fetch('/api/cron/'+job.id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({cron_expr:expr})})
          .then(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.tlHourShifted')+expr,'ok'); tlReload(); });
      });
    });
  }
  function tlZoom(h){ _tlSpanH=h; var spanMs=h*3600000; _tlFromMs=Date.now()-0.28*spanMs; if(_tlHost)renderTimeline(_tlHost); }
  function tlRecenter(){ tlZoom(_tlSpanH); }
  function tlReload(){ if(_tlHost) loadCronTimeline(_tlHost); }
  function tlDetail(i){
    var job=_tlJobs[i]; if(!job)return; var d=document.getElementById('tlDetail'); if(!d)return;
    d.innerHTML='<div style="font-weight:600;color:var(--amber);margin-bottom:6px">'+LaRuche.Utils.esc(job.name||LaRuche.i18n.t('settings.tlNoName'))+'</div>'+
      '<div>'+LaRuche.i18n.t('settings.tlPlanLabel')+'<code>'+LaRuche.Utils.esc(job.cron_expr||job.fire_at||'-')+'</code></div>'+
      '<div style="color:var(--text-dim)">'+LaRuche.i18n.t('settings.tlLastRun')+(job.last_run||LaRuche.i18n.t('settings.tlNever'))+LaRuche.i18n.t('settings.tlRuns')+(job.run_count||0)+(job.channel?(LaRuche.i18n.t('settings.tlChannel')+LaRuche.Utils.esc(job.channel)):'')+'</div>'+
      '<div style="margin-top:8px;display:flex;gap:6px;flex-wrap:wrap">'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlRun('+i+')">'+LaRuche.i18n.t('settings.tlRunNow')+'</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlEdit('+i+')">'+LaRuche.i18n.t('settings.tlEdit')+'</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlToggle('+i+')">'+(job.enabled===false?LaRuche.i18n.t('settings.tlResume'):LaRuche.i18n.t('settings.tlPause'))+'</button>'+
      '<button class="tl-btn" onclick="if(confirm(LaRuche.i18n.t(\'settings.tlDeleteConfirm\')))fetch(\'/api/cron/'+job.id+'\',{method:\'DELETE\'}).then(function(){LaRuche.Settings.tlReload&&LaRuche.Settings.tlReload();})">'+LaRuche.i18n.t('settings.tlDelete')+'</button>'+
      '</div>';
  }
  function tlRun(i){ var job=_tlJobs[i]; if(!job)return; fetch('/api/cron/'+job.id+'/run',{method:'POST'}).then(function(r){return r.json();}).then(function(d){ LaRuche.Toast.show(d.status==='started'?LaRuche.i18n.t('settings.tlRunning'):LaRuche.i18n.t('settings.tlFailed'), d.status==='started'?'ok':'err'); }).catch(function(){LaRuche.Toast.show(LaRuche.i18n.t('settings.tlFailed'),'err');}); }
  function tlToggle(i){ var job=_tlJobs[i]; if(!job)return; fetch('/api/cron/'+job.id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({enabled: job.enabled===false})}).then(function(){tlReload();}); }
  async function tlEdit(i){
    var job=_tlJobs[i]; if(!job)return; var d=document.getElementById('tlDetail'); if(!d)return;
    var skillsLoaded=true, skills=[];
    try{ skills=await fetch(LaRuche.API.base+'/api/skills').then(function(r){ if(!r.ok)throw new Error('skills'); return r.json(); }); }
    catch(e){ skillsLoaded=false; }
    var selected=Array.isArray(job.skills)?job.skills:[];
    var skillHtml;
    if(!skillsLoaded){
      skillHtml='<div data-skills-unavailable style="margin-top:10px;color:var(--red);font-size:11px">'+LaRuche.i18n.t('settings.skillsUnavailable')+'</div>';
    }else if(!skills.length){
      skillHtml='<div style="margin-top:10px;color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('settings.noSkillsAvailable')+'</div>';
    }else{
      skillHtml='<fieldset style="margin:10px 0 0;padding:8px;border:1px solid var(--border);border-radius:6px"><legend style="padding:0 4px;color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('settings.skillsInjected')+'</legend>'+
        skills.map(function(skill){
          var name=String(skill.name||''), enabled=skill.enabled!==false, checked=selected.indexOf(name)!==-1;
          return '<label style="display:flex;align-items:flex-start;gap:7px;margin:5px 0;cursor:'+(enabled?'pointer':'not-allowed')+';opacity:'+(enabled?'1':'0.55')+'">'+
            '<input class="tlf-skill" type="checkbox" value="'+LaRuche.Utils.esc(name)+'" '+(checked?'checked ':'')+(enabled?'':'disabled ')+'>'+
            '<span><strong>'+LaRuche.Utils.esc(name)+'</strong>'+(skill.description?' <span style="color:var(--text-dim)">- '+LaRuche.Utils.esc(skill.description)+'</span>':'')+(enabled?'':' <span style="color:var(--red)">'+LaRuche.i18n.t('settings.skillDisabled')+'</span>')+'</span></label>';
        }).join('')+'</fieldset>';
    }
        var profiles = window._lastProfiles || {};
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.defaultModel')+'</option>';
    Object.keys(profiles).forEach(function(k){
        profOpts += '<option value="'+k+'" '+(job.profile_id===k?'selected':'')+'>'+LaRuche.Utils.esc(profiles[k].name)+'</option>';
    });
    var modOpts = '<option value="">'+LaRuche.i18n.t('settings.providerDefault')+'</option>';
    if(job.profile_id && profiles[job.profile_id]) {
        var models = profiles[job.profile_id].models || [];
        models.forEach(function(m){
            modOpts += '<option value="'+LaRuche.Utils.esc(m)+'" '+(job.model===m?'selected':'')+'>'+LaRuche.Utils.esc(m)+'</option>';
        });
    } else if (!job.profile_id && job.model) {
        modOpts += '<option value="'+LaRuche.Utils.esc(job.model)+'" selected>'+LaRuche.Utils.esc(job.model)+'</option>';
    }

    d.innerHTML='<div class="tl-detail"><div style="font-weight:600;color:var(--amber);margin-bottom:8px">'+LaRuche.i18n.t('settings.tlEdit')+' : '+LaRuche.Utils.esc(job.name||'')+'</div>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.nameLabel')+'</label><input class="form-input" id="tlfName" value="'+LaRuche.Utils.esc(job.name||'')+'">'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.promptLabel')+'</label><textarea class="form-input" id="tlfPrompt" rows="3">'+LaRuche.Utils.esc(job.prompt||'')+'</textarea>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.tlCronFields')+'</label><input class="form-input" id="tlfCron" value="'+LaRuche.Utils.esc(job.cron_expr||'')+'" placeholder="*/30 * * * *">'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.channelLabel')+'</label><input class="form-input" id="tlfChannel" value="'+LaRuche.Utils.esc(job.channel||'')+'" placeholder="telegram / empty">'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.providerLabel')+'</label><select class="form-input" id="tlfProfileId" onchange="LaRuche.Settings.updateCronEditModelSelect()">'+profOpts+'</select>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.modelLabel')+'</label><select class="form-input" id="tlfModel">'+modOpts+'</select>'+
      skillHtml+
      '<div style="margin-top:10px;display:flex;gap:6px">'+
      '<button class="tl-btn" style="background:var(--amber);color:#000" onclick="LaRuche.Settings.tlSaveEdit('+i+')">'+LaRuche.i18n.t('settings.tlSaveEdit')+'</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlDetail('+i+')">'+LaRuche.i18n.t('settings.tlCancel')+'</button></div></div>';
  }
  function updateCronEditModelSelect() {
      var profSel = document.getElementById('tlfProfileId');
      var modSel = document.getElementById('tlfModel');
      if(!profSel || !modSel) return;
      var pid = profSel.value;
      modSel.innerHTML = '<option value="">'+LaRuche.i18n.t('settings.providerDefault')+'</option>';
      if(pid && window._lastProfiles && window._lastProfiles[pid]) {
          var models = window._lastProfiles[pid].models || [];
          models.forEach(function(m) {
              modSel.innerHTML += '<option value="'+LaRuche.Utils.esc(m)+'">'+LaRuche.Utils.esc(m)+'</option>';
          });
      }
  }

  function tlSaveEdit(i){
    var job=_tlJobs[i]; if(!job)return;
    var skillBox=document.querySelector('#tlDetail [data-skills-unavailable]');
    var skills=skillBox ? (Array.isArray(job.skills)?job.skills:[]) : Array.prototype.map.call(document.querySelectorAll('#tlDetail .tlf-skill:checked'),function(input){return input.value;});
    
    var profile_id = document.getElementById('tlfProfileId').value || null;
    var model = document.getElementById('tlfModel').value || null;

    var body={ name:(document.getElementById('tlfName').value||''), prompt:(document.getElementById('tlfPrompt').value||''),
      cron_expr:(document.getElementById('tlfCron').value||''), channel:(document.getElementById('tlfChannel').value||''),
      profile_id: profile_id, model: model, skills:skills };
      
    fetch('/api/cron/'+job.id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})
      .then(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.tlSaved'),'ok'); tlReload(); });
  }

  // MCP logic
  // Secrets tab: encrypted vault. The UI NEVER receives the values, only the names.
  async function loadSecrets(el){
    var data={names:[]};
    try{ data=await fetch('/api/secrets').then(function(r){return r.json();}); }catch(e){}
    var names=(data.names||[]);
    var hooks=names.filter(function(n){return n.indexOf('WEBHOOK')===0;});
    var others=names.filter(function(n){return n.indexOf('WEBHOOK')!==0;});
    function card(list,title,hint){
      var rows = list.length ? list.map(function(n){
        return '<div class="settings-row"><span class="settings-value" style="font-family:var(--mono,monospace)">'+LaRuche.Utils.esc(n)+' <span style="color:var(--text-dim);font-size:10px">= ••••••••</span></span><button onclick="LaRuche.Settings.secretDelete(\''+LaRuche.Utils.esc(n)+'\')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:1px 8px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.secretDeleteBtn')+'</button></div>';
      }).join('') : '<div style="color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('settings.secretNone')+'</div>';
      return '<div class="settings-card"><div class="settings-card-title">'+title+'</div><div style="color:var(--text-dim);font-size:11px;margin-bottom:8px">'+hint+'</div>'+rows+'</div>';
    }
    el.innerHTML =
      '<div style="color:var(--text-dim);font-size:12px;margin-bottom:12px">'+LaRuche.i18n.t('settings.secretsDesc')+'</div>'+
      card(others,LaRuche.i18n.t('settings.secretsTitle'),LaRuche.i18n.t('settings.secretsHint'))+
      card(hooks,LaRuche.i18n.t('settings.webhooksTitle'),LaRuche.i18n.t('settings.webhooksHint'))+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.addOrUpdate')+'</div>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.secretNameLabel')+'</label><input class="form-input" id="secName" placeholder="'+LaRuche.i18n.t('settings.secretNamePlaceholder')+'">'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.secretValLabel')+'</label><input class="form-input" id="secVal" type="password" placeholder="'+LaRuche.i18n.t('settings.secretValPlaceholder')+'">'+
      '<button class="form-btn" style="margin-top:8px" onclick="LaRuche.Settings.secretSet()">'+LaRuche.i18n.t('settings.secretSave')+'</button></div>';
  }
  function secretSet(){
    var name=(document.getElementById('secName').value||'').trim();
    var value=document.getElementById('secVal').value||'';
    if(!name||!value){ LaRuche.Toast.show(LaRuche.i18n.t('settings.secretNameRequired'),'warn'); return; }
    fetch(LaRuche.API.base+'/api/secrets',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify({name:name,value:value})})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.secretSaved'),'ok'); if(LaRuche.Secrets)LaRuche.Secrets.refresh(); refreshTab(); } else { LaRuche.Toast.show(LaRuche.i18n.t('settings.secretSaveFailed'),'err'); } });
  }
  function secretDelete(name){
    fetch(LaRuche.API.base+'/api/secrets/'+encodeURIComponent(name),{method:'DELETE',credentials:'include'})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.secretDeleted'),'ok'); if(LaRuche.Secrets)LaRuche.Secrets.refresh(); refreshTab(); } });
  }

  // Dedicated MCP tab (moved out of Providers).
  function loadMcp(el){
    var html = '<div class="settings-card" style="margin-bottom:16px">';
    html += '  <div class="settings-card-title">'+LaRuche.i18n.t('settings.mcpServersTitle')+'</div>';
    html += '  <div style="color:var(--text-dim);font-size:12px;margin-bottom:12px">'+LaRuche.i18n.t('settings.mcpDesc')+'</div>';
    html += '  <div id="mcp-list" style="margin-bottom:12px"></div>';
    html += '  <div style="border:1px solid var(--border);border-radius:6px;padding:8px;background:var(--bg-panel)">';
    html += '     <div style="margin-bottom:8px"><label class="form-label">'+LaRuche.i18n.t('settings.mcpNameLabel')+'</label><input id="mcp-new-name" class="form-input" placeholder="'+LaRuche.i18n.t('settings.mcpNamePlaceholder')+'"></div>';
    html += '     <div style="margin-bottom:8px"><label class="form-label">'+LaRuche.i18n.t('settings.mcpCmdLabel')+'</label><input id="mcp-new-cmd" class="form-input" placeholder="'+LaRuche.i18n.t('settings.mcpCmdPlaceholder')+'"></div>';
    html += '     <div style="margin-bottom:8px"><label class="form-label">'+LaRuche.i18n.t('settings.mcpArgsLabel')+'</label><input id="mcp-new-args" class="form-input" placeholder="'+LaRuche.i18n.t('settings.mcpArgsPlaceholder')+'"></div>';
    html += '     <button class="settings-save-btn" onclick="LaRuche.Settings.createMcpServer()">'+LaRuche.i18n.t('settings.mcpAddBtn')+'</button>';
    html += '  </div>';
    html += '</div>';
    el.innerHTML = html;
    loadMcpServers();
  }

  async function loadMcpServers() {
    try {
      var r = await fetch('/api/mcp/servers');
      var d = await r.json();
      var el = document.getElementById('mcp-list');
      if(!el) return;
      var html = '';
      for(var k in d.mcpServers) {
        var s = d.mcpServers[k];
        html += '<div class="settings-row" style="margin-bottom:6px;padding-bottom:6px;border-bottom:1px solid rgba(42,42,46,0.3)"><span class="settings-label" style="flex:1">'+k+' <span style="font-size:10px;color:var(--text-dim)">('+s.command+' '+(s.args?s.args.join(' '):'')+')</span></span><button onclick="LaRuche.Settings.deleteMcpServer(\''+k+'\')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.mcpDeleteBtn')+'</button></div>';
      }
      if(!html) html = '<div style="color:var(--text-dim);font-size:12px;padding:8px">'+LaRuche.i18n.t('settings.mcpNone')+'</div>';
      el.innerHTML = html;
    } catch(e) {}
  }

  function createMcpServer() {
    var n = document.getElementById('mcp-new-name').value.trim();
    var c = document.getElementById('mcp-new-cmd').value.trim();
    var a = document.getElementById('mcp-new-args').value.trim();
    if(!n || !c) return;
    var args = a ? a.split(' ') : [];
    fetch('/api/mcp/servers/'+encodeURIComponent(n), {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({command: c, args: args})
    }).then(function(r){
      if(r.ok) {
         LaRuche.Toast.show(LaRuche.i18n.t('settings.mcpAdded'),'ok');
         document.getElementById('mcp-new-name').value = '';
         document.getElementById('mcp-new-cmd').value = '';
         document.getElementById('mcp-new-args').value = '';
         loadMcpServers();
      }
    });
  }

  function deleteMcpServer(n) {
    if(!confirm(LaRuche.i18n.t('settings.mcpDeleteConfirm'))) return;
    fetch('/api/mcp/servers/'+encodeURIComponent(n), {method:'DELETE'}).then(function(r){
      if(r.ok) { loadMcpServers(); LaRuche.Toast.show(LaRuche.i18n.t('settings.mcpDeleted'),'ok'); }
    });
  }

  // Provider/Model selectors logic for Kanban/Watcher
  function updateKanbanModelSelect() {
    var pId = document.getElementById('kanban-profile').value;
    var modelSel = document.getElementById('kanban-model');
    if(!modelSel) return;
    modelSel.innerHTML = '<option value="">'+LaRuche.i18n.t('settings.parDefault')+'</option>';
    if(pId && _profiles[pId] && _profiles[pId].models) {
      _profiles[pId].models.forEach(function(m){
        modelSel.innerHTML += '<option value="'+LaRuche.Utils.esc(m)+'">'+LaRuche.Utils.esc(m)+'</option>';
      });
    }
  }

  function updateWatcherModelSelect() {
    var pId = document.getElementById('watcher-profile').value;
    var modelSel = document.getElementById('watcher-model');
    if(!modelSel) return;
    modelSel.innerHTML = '<option value="">'+LaRuche.i18n.t('settings.parDefault')+'</option>';
    if(pId && _profiles[pId] && _profiles[pId].models) {
      _profiles[pId].models.forEach(function(m){
        modelSel.innerHTML += '<option value="'+LaRuche.Utils.esc(m)+'">'+LaRuche.Utils.esc(m)+'</option>';
      });
    }
  }

  var _ncCronBuilderId = null;
  async function loadCron(el) {
    var tasks=[];try{tasks=await fetch('/api/cron').then(function(r){return r.json();});}catch(e){}
    var profilesResp={profiles:{}};try{profilesResp=await fetch('/api/profiles').then(function(r){return r.json();});}catch(e){}
    var profiles = profilesResp.profiles || {};
    window._lastProfiles = profiles;
    
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.defaultModel')+'</option>';
    Object.keys(profiles).forEach(function(k){
        profOpts += '<option value="'+k+'">'+LaRuche.Utils.esc(profiles[k].name)+'</option>';
    });

    el.innerHTML='<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="document.getElementById(\'newCronForm\').style.display=\'block\'">'+LaRuche.i18n.t('settings.newTaskBtn')+'</button></div>'+
      '<div id="newCronForm" style="display:none" class="settings-card">'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.nameLabel')+'</label><input id="ncName" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.promptLabel')+'</label><input id="ncPrompt" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.bpScheduleLabel')+'</label><div id="ncCronBuilder"></div></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherChannelLabel')+'</label><select id="ncChannel" class="form-input"><option value="">'+LaRuche.i18n.t('settings.cronChannelNone')+'</option><option value="telegram">Telegram</option><option value="discord">Discord</option></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.providerLabel')+'</label><select id="ncProfileId" class="form-input" onchange="LaRuche.Settings.updateCronModelSelect()">'+profOpts+'</select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.modelLabel')+'</label><select id="ncModel" class="form-input"><option value="">'+LaRuche.i18n.t('settings.providerDefault')+'</option></select></div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.createCron()">'+LaRuche.i18n.t('settings.createBtn')+'</button></div>'+
      tasks.map(function(t){
          var effProv = LaRuche.i18n.t('settings.watcherDefaut');
          if(t.profile_id && profiles[t.profile_id]) effProv = profiles[t.profile_id].name;
          else if(t.profile_id) effProv = t.profile_id;
          else if(t.provider) effProv = t.provider + (t.model ? " / " + t.model : "");
          else if(t.model) effProv = t.model;
          if(t.profile_id && t.model) effProv += " (" + t.model + ")";
          return '<div class="settings-card"><div class="settings-card-title">'+LaRuche.Utils.esc(t.name)+'</div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.scheduleLabel')+'</span><span class="settings-value">'+(t.cron_expr||t.fire_at||'-')+'</span></div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.runsLabel')+'</span><span class="settings-value">'+(t.run_count||0)+'</span></div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.channelLabelShort')+'</span><span class="settings-value">'+LaRuche.Utils.esc(t.channel||LaRuche.i18n.t('settings.channelNone'))+'</span></div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.providerModelLabel')+'</span><span class="settings-value">'+LaRuche.Utils.esc(effProv)+'</span></div><button onclick="LaRuche.Settings.deleteCronTask(\''+t.id+'\',this)" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;margin-top:6px">'+LaRuche.i18n.t('settings.deleteBtn')+'</button></div>';
      }).join('');
    // Human-friendly cron builder for the creation form.
    if(LaRuche.CronBuilder){ _ncCronBuilderId = LaRuche.CronBuilder.mount('ncCronBuilder', { value:'' }); }
  }
  // OPTIMISTIC cron deletion: removes the card from the DOM as soon as the DELETE succeeds. Works
  // in any container (Cron page OR Missions hub), no more F5 (refreshTab
  // reloaded the wrong tab depending on context).
  function deleteCronTask(id, btn){
    if(!confirm(LaRuche.i18n.t('settings.cronDeleteConfirm'))) return;
    fetch('/api/cron/'+id,{method:'DELETE'}).then(function(r){
      if(!r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.cronDeleteFailed'),'err'); return; }
      var card = btn && btn.closest('.settings-card'); if(card) card.remove();
      LaRuche.Toast.show(LaRuche.i18n.t('settings.cronDeleted'),'ok');
    }).catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.cronDeleteFailed'),'err'); });
  }
  
  function updateCronModelSelect() {
      var profSel = document.getElementById('ncProfileId');
      var modSel = document.getElementById('ncModel');
      if(!profSel || !modSel) return;
      var pid = profSel.value;
      modSel.innerHTML = '<option value="">'+LaRuche.i18n.t('settings.providerDefault')+'</option>';
      if(pid && window._lastProfiles && window._lastProfiles[pid]) {
          var models = window._lastProfiles[pid].models || [];
          models.forEach(function(m) {
              modSel.innerHTML += '<option value="'+LaRuche.Utils.esc(m)+'">'+LaRuche.Utils.esc(m)+'</option>';
          });
      }
  }
  function createCron() {
    var name=document.getElementById('ncName').value;
    var prompt=document.getElementById('ncPrompt').value;
    var cron=(_ncCronBuilderId && LaRuche.CronBuilder) ? LaRuche.CronBuilder.getValue(_ncCronBuilderId) : '';
    var channel=document.getElementById('ncChannel').value;
    var profile_id=document.getElementById('ncProfileId').value;
    var model=document.getElementById('ncModel').value;
    
    var payload = {name:name,prompt:prompt,cron_expr:cron,channel:channel||null};
    if(profile_id) payload.profile_id = profile_id;
    if(model) payload.model = model;
    
    fetch('/api/cron',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)}).then(function(){loadTab('cron');LaRuche.Toast.show(LaRuche.i18n.t('settings.cronTaskCreated'),'ok');});
  }

  async function loadWatchers(el) {
    var watchers=[];try{watchers=await fetch('/api/watchers').then(function(r){return r.json();});}catch(e){}
    _watchersLast = JSON.stringify(watchers);
    // P1: profiles for the watcher's Provider selector.
    var profilesResp={profiles:{}};try{profilesResp=await fetch('/api/profiles').then(function(r){return r.json();});}catch(e){}
    var profiles = profilesResp.profiles || {};
    _profiles = profiles;
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.watcherDefChannel')+'</option>';
    Object.keys(profiles).forEach(function(k){
        profOpts += '<option value="'+k+'">'+LaRuche.Utils.esc(profiles[k].name||k)+'</option>';
    });
    el.innerHTML='<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="document.getElementById(\'newWatcherForm\').style.display=\'block\'">'+LaRuche.i18n.t('settings.newWatcherBtn')+'</button></div>'+
      '<div id="newWatcherForm" style="display:none" class="settings-card">'+
      '<div style="font-weight:600;margin-bottom:8px">'+LaRuche.i18n.t('settings.newWatcherTitle')+'</div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.nameLabel')+'</label><input id="nwName" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherTypeLabel')+'</label><select id="nwType" class="form-input"><option value="file">'+LaRuche.i18n.t('settings.watcherTypeFile')+'</option><option value="url">'+LaRuche.i18n.t('settings.watcherTypeUrl')+'</option><option value="log">'+LaRuche.i18n.t('settings.watcherTypeLog')+'</option></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherTargetField')+'</label><input id="nwTarget" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherCondField')+'</label><input id="nwCondition" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.promptLabel')+'</label><input id="nwPrompt" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.providerLabel')+'</label><select id="watcher-profile" class="form-input" onchange="LaRuche.Settings.updateWatcherModelSelect()">'+profOpts+'</select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.modelLabel')+'</label><select id="watcher-model" class="form-input"><option value="">'+LaRuche.i18n.t('settings.parDefault')+'</option></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherChannelLabel')+'</label><select id="nwChannel" class="form-input"><option value="">'+LaRuche.i18n.t('settings.watcherHomeChannel')+'</option></select></div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.createWatcher()">'+LaRuche.i18n.t('settings.createBtn')+'</button></div>'+
      watchers.map(function(w){
        var effProv = LaRuche.i18n.t('settings.watcherDefaut');
        if(w.profile_id && profiles[w.profile_id]) effProv = profiles[w.profile_id].name || w.profile_id;
        else if(w.profile_id) effProv = w.profile_id;
        else if(w.model) effProv = w.model;
        if(w.profile_id && w.model) effProv += " (" + w.model + ")";
        return '<div class="settings-card"><div class="settings-card-title">'+LaRuche.Utils.esc(w.name)+'</div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.typeLabel')+'</span><span class="settings-value">'+LaRuche.Utils.esc(w.watcher_type)+'</span></div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.targetLabel')+'</span><span class="settings-value">'+LaRuche.Utils.esc(w.target)+'</span></div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.providerModelLabel')+'</span><span class="settings-value">'+LaRuche.Utils.esc(effProv)+'</span></div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.runsLabel')+'</span><span class="settings-value">'+(w.run_count||0)+'</span></div><div style="margin-top:6px;display:flex;gap:6px"><button onclick="LaRuche.Settings.editWatcher(\''+w.id+'\')" style="background:none;border:1px solid var(--amber);color:var(--amber);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.watcherEditBtn')+'</button><button onclick="fetch(\'/api/watchers/'+w.id+'\',{method:\'DELETE\'}).then(function(){LaRuche.Settings.refreshTab()})" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.deleteWatcherBtn')+'</button></div></div>';}).join('');
    window.__fillChannels(document.getElementById('nwChannel'), '', LaRuche.i18n.t('settings.watcherHomeChannel'));
  }

  function createWatcher() {
    var name=document.getElementById('nwName').value;
    var type=document.getElementById('nwType').value;
    var target=document.getElementById('nwTarget').value;
    var cond=document.getElementById('nwCondition').value;
    var prompt=document.getElementById('nwPrompt').value;
    var profEl=document.getElementById('watcher-profile');
    var modEl=document.getElementById('watcher-model');
    var profile_id = profEl ? profEl.value : '';
    var model = modEl ? modEl.value : '';
    var chEl=document.getElementById('nwChannel'); var channel = chEl ? chEl.value : '';
    var body={name:name,watcher_type:type,target:target,condition:cond,prompt:prompt};
    if(profile_id) body.profile_id = profile_id;
    if(model) body.model = model;
    if(channel) body.channel = channel;
    fetch('/api/watchers',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(function(){loadTab('watchers');LaRuche.Toast.show(LaRuche.i18n.t('settings.watcherCreated'),'ok');});
  }

  // Inline watcher editing (parity with cron/kanban).
  function editWatcher(id) {
    var w=null; try{ w=JSON.parse(_watchersLast).find(function(x){return x.id===id;}); }catch(e){}
    if(!w){ LaRuche.Toast.show(LaRuche.i18n.t('settings.fileNotFound'),'err'); return; }
    function opt(v,label,cur){ return '<option value="'+v+'" '+(cur===v?'selected':'')+'>'+label+'</option>'; }
    var typeSel = opt('file',LaRuche.i18n.t('settings.watcherTypeFile'),w.watcher_type)+opt('url',LaRuche.i18n.t('settings.watcherTypeUrl'),w.watcher_type)+opt('log',LaRuche.i18n.t('settings.watcherTypeLog'),w.watcher_type);
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.watcherDefChannel')+'</option>';
    Object.keys(_profiles).forEach(function(k){ profOpts += '<option value="'+k+'" '+((w.profile_id===k)?'selected':'')+'>'+LaRuche.Utils.esc(_profiles[k].name||k)+'</option>'; });
    var modOpts = '<option value="">'+LaRuche.i18n.t('settings.parDefault')+'</option>';
    if(w.profile_id && _profiles[w.profile_id] && _profiles[w.profile_id].models){
      _profiles[w.profile_id].models.forEach(function(mm){ modOpts += '<option value="'+LaRuche.Utils.esc(mm)+'" '+((w.model===mm)?'selected':'')+'>'+LaRuche.Utils.esc(mm)+'</option>'; });
    }
    var ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div style="width:480px;max-width:92vw;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;padding:16px;max-height:90vh;overflow:auto">'+
      '<div style="font-weight:600;color:var(--amber);margin-bottom:10px">'+LaRuche.i18n.t('settings.watcherEditTitle')+'</div>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.watcherNomLabel')+'</label><input class="form-input" id="weName" value="'+LaRuche.Utils.esc(w.name||'')+'">'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.watcherTypeLabel')+'</label><select class="form-input" id="weType">'+typeSel+'</select>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.watcherTargetLabel')+'</label><input class="form-input" id="weTarget" value="'+LaRuche.Utils.esc(w.target||'')+'">'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.watcherCondLabel')+'</label><input class="form-input" id="weCondition" value="'+LaRuche.Utils.esc(w.condition||'')+'">'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.watcherPromptLabel')+'</label><textarea class="form-input" id="wePrompt" rows="3">'+LaRuche.Utils.esc(w.prompt||'')+'</textarea>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.watcherProviderLabel')+'</label><select class="form-input" id="weProfile" onchange="LaRuche.Settings.updateWatcherEditModelSelect()">'+profOpts+'</select>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.watcherModelLabel')+'</label><select class="form-input" id="weModel">'+modOpts+'</select>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.watcherChannelLabel')+'</label><select class="form-input" id="weChannel"><option value="">'+LaRuche.i18n.t('settings.watcherHomeChannel')+'</option></select>'+
      '<label class="form-label" style="display:flex;align-items:center;gap:8px;margin-top:8px"><input type="checkbox" id="weActive" '+(w.active?'checked':'')+'> '+LaRuche.i18n.t('settings.watcherActiveLabel')+'</label>'+
      '<div style="margin-top:12px;display:flex;gap:8px"><button class="form-btn" onclick="LaRuche.Settings.saveWatcherEdit(\''+id+'\',this)">'+LaRuche.i18n.t('settings.watcherSave')+'</button>'+
      '<button class="form-btn" style="background:none;border:1px solid var(--border);color:var(--text-dim)" onclick="this.closest(\'div[style*=fixed]\')&&this.closest(\'div[style*=fixed]\').remove()">'+LaRuche.i18n.t('settings.watcherCancel')+'</button></div></div>';
    document.body.appendChild(ov);
    window.__fillChannels(document.getElementById('weChannel'), (w&&w.channel)||'', LaRuche.i18n.t('settings.watcherHomeChannel'));
  }

  function updateWatcherEditModelSelect() {
    var pId=document.getElementById('weProfile').value, sel=document.getElementById('weModel');
    if(!sel) return;
    sel.innerHTML='<option value="">'+LaRuche.i18n.t('settings.parDefault')+'</option>';
    if(pId && _profiles[pId] && _profiles[pId].models){ _profiles[pId].models.forEach(function(m){ sel.innerHTML+='<option value="'+LaRuche.Utils.esc(m)+'">'+LaRuche.Utils.esc(m)+'</option>'; }); }
  }

  function saveWatcherEdit(id, btn) {
    var body={
      name: document.getElementById('weName').value,
      watcher_type: document.getElementById('weType').value,
      target: document.getElementById('weTarget').value,
      condition: document.getElementById('weCondition').value,
      prompt: document.getElementById('wePrompt').value,
      active: document.getElementById('weActive').checked,
      profile_id: document.getElementById('weProfile').value,
      model: document.getElementById('weModel').value,
      channel: document.getElementById('weChannel')?document.getElementById('weChannel').value:''
    };
    fetch(LaRuche.API.base+'/api/watchers/'+id,{method:'PATCH',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.watcherSaved'),'ok'); var ov=btn.closest('div[style*=fixed]'); if(ov)ov.remove(); refreshTab(); } else { LaRuche.Toast.show(LaRuche.i18n.t('settings.watcherSaveFailed'),'err'); } });
  }

  function addCredential(provider) {
    var key = prompt(LaRuche.i18n.t('settings.newCredKey') + provider + ' :');
    if(!key) return;
    var label = prompt(LaRuche.i18n.t('settings.newCredLabel')) || '';
    fetch('/api/credentials', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({provider: provider, api_key: key, label: label})
    }).then(function(){
      loadProviders(document.getElementById('settingsContent'));
    }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.credErr')+e, 'err'); });
  }


  function toggleVisibility(id, providerType, currentVis) {
    var newVis = currentVis === 'public_proxy' ? 'prive' : 'public_proxy';
    if(newVis === 'public_proxy' && (providerType === 'openai' || providerType === 'anthropic' || providerType === 'codex')) {
      if(!confirm(LaRuche.i18n.t('settings.publicProviderConfirm'))) {
        return;
      }
    }
    fetch('/api/profiles/'+id+'/visibility', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({visibility: newVis})})
    .then(function(r){return r.json();})
    .then(function(d){
      if(d.status==='ok') {
        LaRuche.Toast.show(LaRuche.i18n.t('settings.visibilityUpdated'),'ok'); window.LaRuche.forceReactivityUpdate();
        loadTab('providers');
      } else {
        LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+(d.error||'?'),'err');
      }
    }).catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err');});
  }

  // Permissions menu: Private / Public / Restricted (checkbox per node) -> grants layer.
  async function openAccess(id, currentVis, allowedEnc){
    var esc = LaRuche.Utils.esc;
    var allowed=[]; try{ allowed=JSON.parse(decodeURIComponent(allowedEnc||'%5B%5D')); }catch(e){}
    var peers=[]; try{ peers=(await fetch('/api/mesh/peers').then(function(r){return r.json();})).peers||[]; }catch(e){}
    function peersHtml(){
      if(!peers.length) return '<div style="color:var(--text-dim);font-size:12px">'+LaRuche.i18n.t('settings.accessNoPeers')+'</div>';
      return peers.map(function(pr){ var ck=allowed.indexOf(pr.id)!==-1?'checked':''; return '<label style="display:flex;gap:8px;align-items:center;padding:3px 0;font-size:13px"><input type="checkbox" class="acc-peer" value="'+esc(pr.id)+'" '+ck+'> 🐝 '+esc(pr.name||pr.id)+'</label>'; }).join('');
    }
    var ov=document.createElement('div'); ov.className='profile-modal-overlay open';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div class="profile-modal"><div class="profile-modal-head"><span class="profile-modal-name">'+LaRuche.i18n.t('settings.accessTitle')+'</span><button class="fd-btn" id="accClose">&#x2716;</button></div>'+
      '<p class="profile-modal-hint">'+LaRuche.i18n.t('settings.accessHint')+'</p>'+
      '<div style="display:flex;flex-direction:column;gap:8px">'+
        '<label><input type="radio" name="accvis" value="prive" '+(currentVis==='prive'?'checked':'')+'>'+LaRuche.i18n.t('settings.accessPrivate')+'</label>'+
        '<label><input type="radio" name="accvis" value="public_proxy" '+(currentVis==='public_proxy'?'checked':'')+'>'+LaRuche.i18n.t('settings.accessPublic')+'</label>'+
        '<label><input type="radio" name="accvis" value="restricted" '+(currentVis==='restricted'?'checked':'')+'>'+LaRuche.i18n.t('settings.accessRestricted')+'</label>'+
        '<div id="accPeers" style="margin-left:24px;'+(currentVis==='restricted'?'':'opacity:.45;pointer-events:none')+'">'+peersHtml()+'</div>'+
      '</div>'+
      '<div class="profile-modal-actions"><button class="send-btn" id="accSave"><span>'+LaRuche.i18n.t('settings.accessSave')+'</span></button></div></div>';
    document.body.appendChild(ov);
    ov.querySelector('#accClose').onclick=function(){ ov.remove(); };
    ov.querySelectorAll('input[name=accvis]').forEach(function(r){ r.onchange=function(){
      var isR=ov.querySelector('input[name=accvis]:checked').value==='restricted';
      var ap=ov.querySelector('#accPeers'); ap.style.opacity=isR?'1':'.45'; ap.style.pointerEvents=isR?'auto':'none';
    };});
    ov.querySelector('#accSave').onclick=function(){
      var vis=ov.querySelector('input[name=accvis]:checked').value;
      var aps=Array.prototype.map.call(ov.querySelectorAll('.acc-peer:checked'),function(c){return c.value;});
      fetch('/api/profiles/'+id+'/visibility',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({visibility:vis,allowed_peers:aps})})
        .then(function(r){return r.json();}).then(function(d){
          if(d.status==='ok'){ LaRuche.Toast.show(LaRuche.i18n.t('settings.accessUpdated'),'ok'); ov.remove(); if(window.LaRuche.forceReactivityUpdate)window.LaRuche.forceReactivityUpdate(); loadTab('providers'); }
          else LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+(d.error||'?'),'err');
        }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); });
    };
  }

  function deleteCredential(provider, apiKey) {
    if(!confirm(LaRuche.i18n.t('settings.deleteCredConfirm'))) return;
    fetch('/api/credentials', {
      method: 'DELETE',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({provider: provider, api_key: apiKey})
    }).then(function(){
      loadProviders(document.getElementById('settingsContent'));
    }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.credErr')+e, 'err'); });
  }

  async function loadChannels(el) {
    var config = await fetch(LaRuche.API.base+'/api/config/channels').then(function(r){return r.json();}).catch(function(){return {};});
    var notify = await fetch(LaRuche.API.base+'/api/config/notify').then(function(r){return r.json();}).catch(function(){return {};});
    var tg = config.telegram || {};
    var dc = config.discord || {};
    var sl = config.slack || {};
    var chmodels = await fetch(LaRuche.API.base+'/api/config/channel-models').then(function(r){return r.json();}).catch(function(){return {options:[],overrides:{}};});
    function chModelSel(channel){
      var cur = (chmodels.overrides||{})[channel] || null;
      var opts = '<option value="">'+LaRuche.i18n.t('settings.chModelDefault')+'</option>';
      (chmodels.options||[]).forEach(function(o){
        var val = o.profile_id+'|||'+o.model;
        var sel = (cur && cur.profile_id===o.profile_id && cur.model===o.model) ? ' selected' : '';
        opts += '<option value="'+LaRuche.Utils.esc(val)+'"'+sel+'>'+LaRuche.Utils.esc((o.name||o.provider)+' / '+o.model)+'</option>';
      });
      return '<select class="form-input" style="font-size:11px" onchange="LaRuche.Settings.setChannelModel(\''+channel+'\',this.value)">'+opts+'</select>';
    }
    el.innerHTML = '<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:16px">' +
      '<div class="settings-card"><div class="card-title" style="color:var(--amber)">'+LaRuche.i18n.t('settings.notificationsTitle')+'</div>' +
        '<div style="font-size:11px;color:var(--text-dim);margin-bottom:8px">'+LaRuche.i18n.t('settings.notifyHint')+'</div>' +
        '<label style="display:flex;align-items:center;gap:8px;cursor:pointer"><input type="checkbox" id="ch-notify-en" '+(notify.enabled?'checked':'')+'> <span>'+LaRuche.i18n.t('settings.notifyLabel')+'</span></label></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--blue)">Telegram</div>' +
        '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.botTokenLabel')+'</label><input class="form-input" id="ch-tg-token" value="'+LaRuche.Utils.esc(tg.bot_token||'')+'" placeholder="7123456789:AAH..."></div>' +
        '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.tgAllowedChats')+'</label><input class="form-input" id="ch-tg-chats" value="'+LaRuche.Utils.esc(tg.allowed_chats||'')+'" placeholder="'+LaRuche.i18n.t('settings.chAllowedChats')+'"></div>' +
        '<div style="font-size:10px;color:var(--text-muted);margin-top:4px">'+LaRuche.i18n.t('settings.chTgLaunch')+'</div></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--purple)">Discord</div>' +
        '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.botTokenLabel')+'</label><input class="form-input" id="ch-dc-token" value="'+LaRuche.Utils.esc(dc.bot_token||'')+'" placeholder="MTIxxx..."></div>' +
        '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.dcAllowedChannels')+'</label><input class="form-input" id="ch-dc-channels" value="'+LaRuche.Utils.esc(dc.allowed_channels||'')+'" placeholder="'+LaRuche.i18n.t('settings.chAllowedChats')+'"></div>' +
        '<div style="font-size:10px;color:var(--text-muted);margin-top:4px">'+LaRuche.i18n.t('settings.chDcLaunch')+'</div></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--green)">Slack</div>' +
        '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.slBotToken')+'</label><input class="form-input" id="ch-sl-bot" value="'+LaRuche.Utils.esc(sl.bot_token||'')+'" placeholder="xoxb-..."></div>' +
        '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.slAppToken')+'</label><input class="form-input" id="ch-sl-app" value="'+LaRuche.Utils.esc(sl.app_token||'')+'" placeholder="xapp-..."></div>' +
        '<div style="font-size:10px;color:var(--text-muted);margin-top:4px">'+LaRuche.i18n.t('settings.chSlLaunch')+'</div></div>' +
      '<div class="settings-card" style="opacity:0.5;border-style:dashed"><div class="card-title" style="color:#25D366">WhatsApp</div>' +
        '<div style="color:var(--text-muted);font-size:12px;padding:12px 0">'+LaRuche.i18n.t('settings.comingSoon')+'</div></div>' +
      '<div class="settings-card" style="opacity:0.5;border-style:dashed"><div class="card-title" style="color:#3A76F0">Signal</div>' +
        '<div style="color:var(--text-muted);font-size:12px;padding:12px 0">'+LaRuche.i18n.t('settings.comingSoon')+'</div></div>' +
      '<div class="settings-card" style="opacity:0.5;border-style:dashed"><div class="card-title" style="color:#0DBD8B">Matrix</div>' +
        '<div style="color:var(--text-muted);font-size:12px;padding:12px 0">'+LaRuche.i18n.t('settings.comingSoon')+'</div></div>' +
    '</div>' +
    '<div class="settings-card" style="margin-top:16px">' +
      '<div class="card-title">'+LaRuche.i18n.t('settings.chModelTitle')+'</div>' +
      '<div style="font-size:11px;color:var(--text-dim);margin-bottom:8px">'+LaRuche.i18n.t('settings.chModelHint')+'</div>' +
      '<div class="settings-row"><span class="settings-label">Telegram</span>'+chModelSel('telegram')+'</div>' +
      '<div class="settings-row"><span class="settings-label">Discord</span>'+chModelSel('discord')+'</div>' +
      '<div class="settings-row"><span class="settings-label">Slack</span>'+chModelSel('slack')+'</div>' +
      '<div class="settings-row"><span class="settings-label">Web</span>'+chModelSel('web')+'</div>' +
    '</div>' +
    '<div style="margin-top:16px;display:flex;gap:8px">' +
      '<button class="form-btn" onclick="LaRuche.Settings.saveChannels()">'+LaRuche.i18n.t('settings.saveChannels')+'</button>' +
      '<button class="form-btn" style="background:var(--green)" onclick="LaRuche.Settings.startChannel(\'telegram\')" id="ch-tg-start">'+LaRuche.i18n.t('settings.startTelegram')+'</button>' +
      '<button class="form-btn" style="background:var(--red);color:#fff" onclick="LaRuche.Settings.stopChannel(\'telegram\')" id="ch-tg-stop" style="display:none">'+LaRuche.i18n.t('settings.stopTelegram')+'</button>' +
    '</div>';
    // Check running status
    fetch(LaRuche.API.base+'/api/channels/status').then(function(r){return r.json();}).then(function(d){
      var running = d.running || [];
      if(running.indexOf('telegram')!==-1) {
        var startBtn=document.getElementById('ch-tg-start'); if(startBtn) startBtn.style.display='none';
        var stopBtn=document.getElementById('ch-tg-stop'); if(stopBtn) stopBtn.style.display='';
      }
    }).catch(function(){});
  }
  // ── Skills page (OKF in memory, capacities.skills.*) ──────────────────
  var SKILL_TEMPLATE='---\ntype: skill\nname: my-skill\ndescription: "What this skill teaches how to do."\nallowed-tools: []\n---\n\n# My Skill\n\n## When to use it\n- ...\n\n## Procedure\n1. ...\n';
  async function loadSkills(el){
    var skills=await fetch(LaRuche.API.base+'/api/skills').then(function(r){return r.json();}).catch(function(){return [];});
    var html='<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">'+
      '<div style="color:var(--text-dim);font-size:12px">'+LaRuche.i18n.t('settings.skillsDesc')+'</div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.newSkill()">'+LaRuche.i18n.t('settings.newSkillBtn')+'</button></div>';
    if(!skills.length){ html+='<div style="color:var(--text-dim);padding:20px">'+LaRuche.i18n.t('settings.noSkills')+'</div>'; }
    html+='<div class="settings-grid">';
    skills.forEach(function(s){
      html+='<div class="settings-card">'+
        '<div style="display:flex;justify-content:space-between;align-items:center;gap:8px">'+
        '<div class="settings-card-title" style="margin:0">'+LaRuche.Utils.esc(s.name)+'</div>'+
        '<label class="lr-switch"><input type="checkbox" '+(s.enabled?'checked':'')+' onchange="LaRuche.Settings.toggleSkill(\''+LaRuche.Utils.esc(s.name)+'\')"><span class="lr-slider"></span></label>'+
        '</div>'+
        '<div style="font-size:11px;color:var(--text-dim);margin:6px 0;min-height:28px">'+LaRuche.Utils.esc(s.description||'')+'</div>'+
        '<div style="display:flex;gap:6px">'+
        '<button class="tl-btn" onclick="LaRuche.Settings.viewSkill(\''+LaRuche.Utils.esc(s.name)+'\')">'+LaRuche.i18n.t('settings.skillViewEdit')+'</button>'+
        '<button class="tl-btn" style="border-color:var(--red);color:var(--red)" onclick="if(confirm(LaRuche.i18n.t(\'settings.confirmDeleteSkill\',{name:LaRuche.Utils.esc(s.name)})))LaRuche.Settings.deleteSkill(\''+LaRuche.Utils.esc(s.name)+'\')">'+LaRuche.i18n.t('settings.skillDelBtn')+'</button>'+
        '</div></div>';
    });
    html+='</div>';
    el.innerHTML=html;
    if(!document.getElementById('lr-switch-style')){
      var st=document.createElement('style'); st.id='lr-switch-style';
      st.textContent='.lr-switch{position:relative;display:inline-block;width:38px;height:20px;flex:0 0 auto}.lr-switch input{display:none}'+
        '.lr-slider{position:absolute;inset:0;background:#444;border-radius:20px;transition:.2s;cursor:pointer}'+
        '.lr-slider:before{content:"";position:absolute;height:14px;width:14px;left:3px;top:3px;background:#fff;border-radius:50%;transition:.2s}'+
        '.lr-switch input:checked+.lr-slider{background:var(--amber)}.lr-switch input:checked+.lr-slider:before{transform:translateX(18px)}';
      document.head.appendChild(st);
    }
  }
  function toggleSkill(name){ fetch(LaRuche.API.base+'/api/skills/'+encodeURIComponent(name)+'/toggle',{method:'POST'}).then(function(r){return r.json();}).then(function(d){ LaRuche.Toast.show(LaRuche.i18n.t('settings.skillToast')+(d.enabled?LaRuche.i18n.t('settings.skillActivated'):LaRuche.i18n.t('settings.skillDeactivated')),'ok'); }); }
  function deleteSkill(name){ fetch(LaRuche.API.base+'/api/skills/'+encodeURIComponent(name),{method:'DELETE'}).then(function(){ LaRuche.Settings.refreshTab&&LaRuche.Settings.refreshTab(); }); }
  var PLUGIN_TEMPLATE = '{\n  "name": "my_plugin",\n  "description": "Description of my plugin",\n  "danger": "safe",\n  "parameters": {\n    "type": "object",\n    "properties": {},\n    "required": []\n  },\n  "command": "echo {{arg}}"\n}';
  function newPlugin(){ pluginEditor('new_plugin', PLUGIN_TEMPLATE); }
  function newSkill(){ skillEditor('', SKILL_TEMPLATE); }
  function viewSkill(name){ fetch(LaRuche.API.base+'/api/skills/'+encodeURIComponent(name)).then(function(r){return r.json();}).then(function(d){ skillEditor(name, d.content||''); }); }
  function skillEditor(name, content){
    var ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div style="width:680px;max-width:94vw;height:80vh;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;display:flex;flex-direction:column">'+
      '<div style="padding:10px 14px;border-bottom:1px solid var(--border);font-weight:600;color:var(--amber)">'+(name?(LaRuche.i18n.t('settings.skillEditPrefix')+LaRuche.Utils.esc(name)):LaRuche.i18n.t('settings.skillNewTitle'))+' <span style="color:var(--text-dim);font-size:10px;font-weight:normal">'+LaRuche.i18n.t('settings.skillEditorHint')+'</span></div>'+
      '<textarea id="skEditor" class="form-input" style="flex:1;margin:12px 12px 6px;font-family:var(--mono);font-size:12px;resize:none">'+LaRuche.Utils.esc(content)+'</textarea>'+
      '<div style="margin:0 12px 6px">'+
        '<div style="display:flex;align-items:center;gap:8px;margin-bottom:4px">'+
          '<span style="font-size:10px;color:var(--text-dim);flex:1">'+LaRuche.i18n.t('settings.skillToolsHint')+'<span id="skToolsCount" style="color:var(--amber)"></span></span>'+
          '<input id="skToolsSearch" placeholder="'+LaRuche.i18n.t('settings.skillToolsFilter')+'" oninput="LaRuche.Settings.filterSkillTools()" style="font-size:11px;padding:2px 6px;width:120px;background:#16161a;border:1px solid var(--border);border-radius:4px;color:var(--text)">'+
          '<button class="tl-btn" style="font-size:10px;padding:2px 6px" onclick="LaRuche.Settings.clearSkillTools()">'+LaRuche.i18n.t('settings.skillToolsClear')+'</button>'+
        '</div>'+
        '<div id="skToolsBox" style="max-height:200px;overflow:auto;border:1px solid var(--border);border-radius:6px;padding:4px"><span style="color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('settings.skillToolsLoading')+'</span></div></div>'+
      '<div style="padding:10px 14px;border-top:1px solid var(--border);display:flex;gap:8px;justify-content:flex-end">'+
      '<button class="tl-btn" onclick="this.closest(\'div[style*=fixed]\').remove()">'+LaRuche.i18n.t('settings.skillCancelBtn')+'</button>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.saveSkill(this)">'+LaRuche.i18n.t('settings.skillSaveBtn')+'</button></div></div>';
    document.body.appendChild(ov);
    mountSkillTools(content);
  }
  // Builds the skill's tool checklist (grouped Tools/Plugins, searchable,
  // selected ones first) and syncs the frontmatter `tools:` line.
  async function mountSkillTools(content){
    var box=document.getElementById('skToolsBox'); if(!box) return;
    var tools = window._allTools;
    if(!tools){ try{ tools=await fetch('/api/tools').then(function(r){return r.json();}); window._allTools=tools; }catch(e){ tools=[]; } }
    var plugins = [];
    try{ plugins=await fetch('/api/plugins').then(function(r){return r.json();}); }catch(e){}
    var pluginNames = (plugins||[]).map(function(p){return p.name||p;});
    // Unified model: {name, group, desc}. group = Plugins | Abeilles | Autres.
    var items = [];
    var seen = {};
    (tools||[]).forEach(function(t){
      var n=t.name||t; if(seen[n])return; seen[n]=1;
      items.push({name:n, group:(pluginNames.indexOf(n)>=0?'Plugins':'Abeilles'), desc:(t.description||'')});
    });
    pluginNames.forEach(function(n){ if(!seen[n]){ seen[n]=1; items.push({name:n, group:'Plugins', desc:''}); } });
    var m = content.match(/^\s*(?:allowed-)?tools:\s*\[([^\]]*)\]/m);
    var current = m ? m[1].split(',').map(function(s){return s.trim().replace(/['"]/g,'');}).filter(Boolean) : [];
    current.forEach(function(n){ if(!seen[n]){ seen[n]=1; items.push({name:n, group:'Autres', desc:LaRuche.i18n.t('settings.skillToolsRef')}); } });
    window._skItems = items;
    window._skChecked = {}; current.forEach(function(n){ window._skChecked[n]=1; });
    renderSkillTools();
  }
  // (Re)renders the list per filter + current checked state. Selected ones at the top of each group.
  function renderSkillTools(){
    var box=document.getElementById('skToolsBox'); if(!box) return;
    var items=window._skItems||[]; var checked=window._skChecked||{};
    var f=(document.getElementById('skToolsSearch')||{}).value||''; f=f.toLowerCase();
    function row(it){
      var on=!!checked[it.name];
      return '<label title="'+LaRuche.Utils.esc(it.desc||'')+'" style="display:flex;align-items:center;gap:7px;padding:4px 7px;border-radius:5px;cursor:pointer;'+(on?'background:rgba(245,158,11,.13)':'')+'" onmouseover="this.style.background=\''+(on?'rgba(245,158,11,.2)':'rgba(255,255,255,.05)')+'\'" onmouseout="this.style.background=\''+(on?'rgba(245,158,11,.13)':'transparent')+'\'">'+
        '<input type="checkbox" value="'+LaRuche.Utils.esc(it.name)+'" '+(on?'checked':'')+' onchange="LaRuche.Settings.toggleSkillTool(this.value,this.checked)" style="accent-color:var(--amber)">'+
        '<span style="font-size:12px;'+(on?'color:var(--amber)':'')+'">'+LaRuche.Utils.esc(it.name)+'</span>'+
        (it.desc?'<span style="font-size:10px;color:var(--text-dim);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1">'+LaRuche.Utils.esc(it.desc)+'</span>':'')+
      '</label>';
    }
    var groups=['Abeilles','Plugins','Autres']; var html='';
    groups.forEach(function(g){
      var list=items.filter(function(it){return it.group===g && (!f || it.name.toLowerCase().indexOf(f)>=0);});
      if(!list.length) return;
      // selected first, then alpha
      list.sort(function(a,b){ var ca=checked[a.name]?0:1, cb=checked[b.name]?0:1; return ca-cb || a.name.localeCompare(b.name); });
      html+='<div style="font-size:9px;text-transform:uppercase;letter-spacing:.5px;color:var(--text-dim);padding:6px 7px 2px">'+g+' ('+list.filter(function(i){return checked[i.name];}).length+'/'+list.length+')</div>';
      html+='<div style="display:grid;grid-template-columns:1fr 1fr;gap:1px">'+list.map(row).join('')+'</div>';
    });
    box.innerHTML = html || '<span style="color:var(--text-dim);font-size:11px;padding:6px;display:block">'+LaRuche.i18n.t('settings.skillToolsNone')+'</span>';
    var cnt=document.getElementById('skToolsCount');
    if(cnt){ var n=Object.keys(checked).filter(function(k){return checked[k];}).length; cnt.textContent=n?('- '+n+' '+LaRuche.i18n.t('settings.skillToolsChecked')):''; }
  }
  function toggleSkillTool(name, on){ window._skChecked=window._skChecked||{}; if(on) window._skChecked[name]=1; else delete window._skChecked[name]; applySkillTools(); renderSkillTools(); }
  function filterSkillTools(){ renderSkillTools(); }
  function clearSkillTools(){ window._skChecked={}; applySkillTools(); renderSkillTools(); }
  function applySkillTools(){
    // Reads the MODEL (_skChecked), not the DOM: otherwise an active filter would hide checked items
    // and we'd lose them on save.
    var checked = Object.keys(window._skChecked||{}).filter(function(k){return window._skChecked[k];});
    var line = 'tools: ['+checked.join(', ')+']';
    var ta=document.getElementById('skEditor'); if(!ta) return;
    var c=ta.value;
    if(/^\s*(?:allowed-)?tools:.*$/m.test(c)){
      c = c.replace(/^\s*(?:allowed-)?tools:.*$/m, line);
    } else {
      var parts=c.split('---');
      if(parts.length>=3){ parts[1]=parts[1].replace(/\n*$/,'\n')+line+'\n'; c=parts.join('---'); }
      else { c='---\n'+line+'\n---\n'+c; }
    }
    ta.value=c;
  }
  function saveSkill(btn){
    var content=document.getElementById('skEditor').value;
    fetch(LaRuche.API.base+'/api/skills',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:content})})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show(LaRuche.i18n.t('settings.skillToast')+'"'+d.name+'"'+LaRuche.i18n.t('settings.skillSaved'),'ok');
        var ov=btn.closest('div[style*=fixed]'); if(ov)ov.remove();
        LaRuche.Settings.refreshTab&&LaRuche.Settings.refreshTab();
      }).catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.skillFailed'),'err'); });
  }

  function viewPlugin(name){ fetch(LaRuche.API.base+'/api/plugins/'+encodeURIComponent(name)).then(function(r){return r.json();}).then(function(d){ pluginEditor(name, d.content||''); }).catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.pluginNotFound'),'err'); }); }
  function pluginEditor(name, content){
    var ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div style="width:680px;max-width:94vw;height:80vh;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;display:flex;flex-direction:column">'+
      '<div style="padding:10px 14px;border-bottom:1px solid var(--border);font-weight:600;color:var(--amber)">'+LaRuche.i18n.t('settings.pluginEditTitle')+LaRuche.Utils.esc(name)+' <span style="color:var(--text-dim);font-size:10px;font-weight:normal">'+LaRuche.i18n.t('settings.pluginEditorHint')+'</span></div>'+
      '<textarea id="plEditor" data-name="'+LaRuche.Utils.esc(name)+'" class="form-input" style="flex:1;margin:12px;font-family:var(--mono);font-size:12px;resize:none" spellcheck="false">'+LaRuche.Utils.esc(content)+'</textarea>'+
      '<div style="padding:10px 14px;border-top:1px solid var(--border);display:flex;gap:8px;justify-content:flex-end">'+
      '<button class="tl-btn" onclick="this.closest(\'div[style*=fixed]\').remove()">'+LaRuche.i18n.t('settings.pluginCancelBtn')+'</button>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.savePlugin(this)">'+LaRuche.i18n.t('settings.pluginSaveBtn')+'</button></div></div>';
    document.body.appendChild(ov);
  }
  function savePlugin(btn){
    var ta=document.getElementById('plEditor');
    var content=ta.value; var name=ta.dataset.name;
    fetch(LaRuche.API.base+'/api/plugins/'+encodeURIComponent(name),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:content})})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show(LaRuche.i18n.t('settings.pluginToast')+'"'+d.name+'"'+LaRuche.i18n.t('settings.pluginSaved'),'ok');
        var ov=btn.closest('div[style*=fixed]'); if(ov)ov.remove();
        LaRuche.Settings.refreshTab&&LaRuche.Settings.refreshTab();
      }).catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.pluginFailed'),'err'); });
  }
  function deletePlugin(name){
    fetch(LaRuche.API.base+'/api/plugins/'+encodeURIComponent(name),{method:'DELETE'})
      .then(function(r){return r.json();}).then(function(d){
        LaRuche.Toast.show(LaRuche.i18n.t('settings.pluginDeleted'),'ok');
        LaRuche.Settings.refreshTab&&LaRuche.Settings.refreshTab();
      }).catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.pluginFailed'),'err'); });
  }

  var _kanbanTimer=null, _kanbanLast='';
  var _kanbanView=(function(){ try{ return localStorage.getItem('lr_kanban_view')||'cols'; }catch(e){ return 'cols'; } })();
  var _profiles={}; // P1: profiles cache for the Provider selectors (kanban/watcher)
  var _watchersLast='[]'; // watchers cache for inline editing

  function setKanbanView(mode){
    _kanbanView = mode;
    try{ localStorage.setItem('lr_kanban_view', mode); }catch(e){}
    var tg=document.getElementById('kanbanViewToggle'); if(tg) tg.innerHTML = kanbanToggleInner();
    _kanbanLast=''; refreshKanbanCols();
  }
  function kanbanToggleInner(){
    return '<button class="tl-btn" style="border-radius:0'+(_kanbanView==='cols'?';background:var(--amber);color:#000':'')+'" onclick="LaRuche.Settings.setKanbanView(\'cols\')">'+LaRuche.i18n.t('settings.kanbanCols')+'</button>'+
      '<button class="tl-btn" style="border-radius:0'+(_kanbanView==='rows'?';background:var(--amber);color:#000':'')+'" onclick="LaRuche.Settings.setKanbanView(\'rows\')">'+LaRuche.i18n.t('settings.kanbanHorizontal')+'</button>';
  }
  // Kanban card (HTML), shared between column mode and horizontal mode.
  function kanbanCardHtml(t){
    var h='<div draggable="true" ondragstart="LaRuche.Settings.kanbanDragStart(event,\''+t.id+'\')" style="background:#2a2a2e;border:1px solid var(--border);border-radius:4px;padding:8px;cursor:grab">';
    h+='<div style="font-size:13px;font-weight:600;color:#fff;margin-bottom:4px">'+LaRuche.Utils.esc(t.title)+'</div>';
    h+='<div style="font-size:11px;color:var(--text-dim);margin-bottom:6px">'+LaRuche.Utils.esc(t.description||'')+'</div>';
    if(t.profile_id || t.model){
      var kProv = (t.profile_id && _profiles[t.profile_id]) ? (_profiles[t.profile_id].name||t.profile_id) : (t.profile_id||'');
      if(t.model) kProv += (kProv?' ':'') + '(' + t.model + ')';
      if(kProv) h+='<div style="font-size:10px;color:var(--amber);margin-bottom:6px">⚙ '+LaRuche.Utils.esc(kProv)+'</div>';
    }
    if(t.result){
      var _full = String(t.result||'');
      var _trunc = _full.length>60;
      var _short = _trunc ? (_full.substring(0,60)+'…') : _full;
      // Accordion: click to expand/collapse the LLM comment (fully readable, mobile-friendly).
      // stopPropagation avoids interfering with the card's drag & drop.
      h+='<div class="kb-result" onclick="event.stopPropagation();LaRuche.Settings.toggleKanbanResult(this)" '+
         'data-collapsed="1" style="font-size:10px;color:var(--green);margin-bottom:6px;cursor:pointer" '+
         'title="'+(_trunc?LaRuche.i18n.t('settings.collapseHint'):'')+'">'+
         '<span class="kb-result-label">'+LaRuche.i18n.t('settings.kanbanResultLabel')+(_trunc?' ▸':'')+': </span>'+
         '<span class="kb-result-short" style="white-space:pre-wrap;word-break:break-word">'+LaRuche.Utils.esc(_short)+'</span>'+
         '<span class="kb-result-full" style="display:none;white-space:pre-wrap;word-break:break-word">'+LaRuche.Utils.esc(_full)+'</span>'+
         '</div>';
    }
    h+='<div style="display:flex;justify-content:space-between;align-items:center">';
    h+='<span style="font-size:9px;color:var(--text-muted);font-family:var(--mono)">'+t.id.split('-')[0]+'</span>';
    h+='<span><button onclick="LaRuche.Settings.editKanbanTask(\''+t.id+'\')" style="background:none;border:none;color:var(--amber);cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.kanbanEditBtn')+'</button> <button onclick="LaRuche.Settings.deleteKanbanTask(\''+t.id+'\')" style="background:none;border:none;color:var(--red);cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.kanbanDelBtn')+'</button></span>';
    h+='</div></div>';
    return h;
  }
  // Expands/collapses the LLM comment (result field) of a kanban card.
  function toggleKanbanResult(elDiv){
    if(!elDiv) return;
    var collapsed = elDiv.dataset.collapsed === '1';
    var shortEl = elDiv.querySelector('.kb-result-short');
    var fullEl = elDiv.querySelector('.kb-result-full');
    var labelEl = elDiv.querySelector('.kb-result-label');
    if(!shortEl || !fullEl) return;
    if(collapsed){
      shortEl.style.display='none'; fullEl.style.display='';
      elDiv.dataset.collapsed='0';
      if(labelEl && /▸/.test(labelEl.textContent)) labelEl.textContent = labelEl.textContent.replace('▸','▾');
    } else {
      shortEl.style.display=''; fullEl.style.display='none';
      elDiv.dataset.collapsed='1';
      if(labelEl && /▾/.test(labelEl.textContent)) labelEl.textContent = labelEl.textContent.replace('▾','▸');
    }
  }
  async function loadKanban(el) {
    // P1: profiles for the kanban task's Provider selector.
    var profilesResp={profiles:{}};try{profilesResp=await fetch('/api/profiles').then(function(r){return r.json();});}catch(e){}
    _profiles = profilesResp.profiles || {};
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.kanbanDefProvider')+'</option>';
    Object.keys(_profiles).forEach(function(k){
        profOpts += '<option value="'+k+'">'+LaRuche.Utils.esc(_profiles[k].name||k)+'</option>';
    });
    el.innerHTML = '<div style="margin-bottom:16px;display:flex;gap:8px;align-items:end;flex-wrap:wrap">' +
      '<div style="flex:1;min-width:140px"><label class="form-label">'+LaRuche.i18n.t('settings.kanbanTitle')+'</label><input class="form-input" id="kanban-title" placeholder="'+LaRuche.i18n.t('settings.kanbanTitlePlaceholder')+'"></div>' +
      '<div style="flex:2;min-width:160px"><label class="form-label">'+LaRuche.i18n.t('settings.kanbanDesc')+'</label><input class="form-input" id="kanban-desc" placeholder="'+LaRuche.i18n.t('settings.kanbanDescPlaceholder')+'"></div>' +
      '<div style="flex:1;min-width:130px"><label class="form-label">'+LaRuche.i18n.t('settings.providerLabel')+'</label><select class="form-input" id="kanban-profile" onchange="LaRuche.Settings.updateKanbanModelSelect()">'+profOpts+'</select></div>' +
      '<div style="flex:1;min-width:130px"><label class="form-label">'+LaRuche.i18n.t('settings.modelLabel')+'</label><select class="form-input" id="kanban-model"><option value="">'+LaRuche.i18n.t('settings.kanbanParDefault')+'</option></select></div>' +
      '<div style="flex:1;min-width:150px"><label class="form-label">'+LaRuche.i18n.t('settings.kanbanChannel')+'</label><select class="form-input" id="kanban-channel"><option value="">'+LaRuche.i18n.t('settings.kanbanBoardChannel')+'</option></select></div>' +
      '<button class="form-btn" onclick="LaRuche.Settings.createKanbanTask()">'+LaRuche.i18n.t('settings.kanbanCreate')+'</button></div>' +
      '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px;flex-wrap:wrap;gap:8px">' +
        '<div style="display:flex;align-items:center;gap:6px"><label class="form-label" style="margin:0">'+LaRuche.i18n.t('settings.kanbanDefaultChannelLabel')+'</label>' +
        '<select class="form-input" id="kanban-default-channel" style="width:auto" onchange="LaRuche.Settings.setKanbanDefaultChannel(this.value)"><option value="">'+LaRuche.i18n.t('settings.kanbanBoardChannelNone')+'</option></select></div>' +
        '<div id="kanbanViewToggle" style="display:inline-flex;border:1px solid var(--border);border-radius:6px;overflow:hidden">'+kanbanToggleInner()+'</div></div>' +
      '<div id="kanbanCols"></div>';
    _kanbanLast='';
    window.__fillChannels(document.getElementById('kanban-channel'), '', LaRuche.i18n.t('settings.kanbanBoardChannel'));
    try{ var dc=await fetch('/api/kanban/default_channel').then(function(r){return r.json();}); window.__fillChannels(document.getElementById('kanban-default-channel'), (dc&&dc.channel)||'', LaRuche.i18n.t('settings.kanbanBoardChannelNone')); }catch(e){}
    await refreshKanbanCols();
    if(_kanbanTimer) clearInterval(_kanbanTimer);
    // Auto-refresh (the agent/daemon can modify the board): re-render
    // only if the content changed -> doesn't break in-progress input.
    _kanbanTimer=setInterval(function(){
      if(!document.getElementById('kanbanCols')){ clearInterval(_kanbanTimer); _kanbanTimer=null; return; }
      refreshKanbanCols();
    }, 4000);
  }

  async function refreshKanbanCols(){
    var host=document.getElementById('kanbanCols'); if(!host)return;
    var tasks=await fetch(LaRuche.API.base+'/api/kanban').then(function(r){return r.json();}).catch(function(){return [];});
    var sig=_kanbanView+'|'+JSON.stringify(tasks); if(sig===_kanbanLast) return; _kanbanLast=sig;
    var cols=['Triage','Todo','Ready','Running','Blocked','Done','Archived'];
    // Display label for a status. The value `c` itself stays the contract code (drag/drop, t.status===c).
    function kanbanColLabel(c){ return LaRuche.i18n.t('kanban.col.'+c.toLowerCase()); }
    var html;
    if(_kanbanView==='rows'){
      // Condensed horizontal mode: each status = a band, cards in flex-wrap, height = content.
      html='<div style="display:flex;flex-direction:column;gap:10px">';
      cols.forEach(function(c){
        var colTasks=tasks.filter(function(t){return t.status===c;});
        html+='<div style="background:rgba(30,30,32,0.8);border:1px solid var(--amber-dim);border-radius:6px;overflow:hidden" ondragover="LaRuche.Settings.kanbanDragOver(event)" ondrop="LaRuche.Settings.kanbanDrop(event,\''+c+'\')">';
        html+='<div style="padding:6px 10px;font-weight:600;color:var(--amber);border-bottom:1px solid var(--border);display:flex;justify-content:space-between;align-items:center"><span>'+kanbanColLabel(c)+'</span><span style="font-size:10px;color:var(--text-dim)">'+colTasks.length+'</span></div>';
        html+='<div style="padding:8px;display:flex;flex-wrap:wrap;gap:8px;min-height:36px">';
        if(!colTasks.length){ html+='<span style="font-size:10px;color:var(--text-muted);align-self:center">-</span>'; }
        colTasks.forEach(function(t){ html+='<div style="flex:0 0 230px;max-width:230px">'+kanbanCardHtml(t)+'</div>'; });
        html+='</div></div>';
      });
      html+='</div>';
    } else {
      // Column mode (existing).
      html='<div style="display:flex;gap:12px;overflow-x:auto;padding-bottom:10px;min-height:400px">';
      cols.forEach(function(c){
        html+='<div style="flex:0 0 250px;background:rgba(30,30,32,0.8);border:1px solid var(--amber-dim);border-radius:6px;display:flex;flex-direction:column" ondragover="LaRuche.Settings.kanbanDragOver(event)" ondrop="LaRuche.Settings.kanbanDrop(event,\''+c+'\')">';
        var colTasks=tasks.filter(function(t){return t.status===c;});
        html+='<div style="padding:10px;font-weight:600;color:var(--amber);border-bottom:1px solid var(--border);text-align:center">'+kanbanColLabel(c)+(colTasks.length?(' ('+colTasks.length+')'):'')+'</div>';
        html+='<div style="flex:1;padding:8px;display:flex;flex-direction:column;gap:8px">';
        colTasks.forEach(function(t){ html+=kanbanCardHtml(t); });
        html+='</div></div>';
      });
      html+='</div>';
    }
    host.innerHTML=html;
  }

  function setKanbanDefaultChannel(ch){
    fetch('/api/kanban/default_channel',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({channel: ch||null})})
      .then(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.kanbanDefaultUpdated'),'ok'); });
  }
  function createKanbanTask() {
    var title = document.getElementById('kanban-title').value;
    var desc = document.getElementById('kanban-desc').value;
var pId = document.getElementById('kanban-profile')?document.getElementById('kanban-profile').value:'';
var m = document.getElementById('kanban-model')?document.getElementById('kanban-model').value:'';
var ch = document.getElementById('kanban-channel')?document.getElementById('kanban-channel').value:'';
    if(!title) return;
    fetch(LaRuche.API.base+'/api/kanban',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({title: title, description: desc, profile_id: pId||null, model: m||null, channel: ch||null})})
      .then(function(r){if(r.ok) { LaRuche.Toast.show(LaRuche.i18n.t('settings.kanbanTaskCreated'),'ok'); document.getElementById('kanban-title').value=''; document.getElementById('kanban-desc').value=''; _kanbanLast=''; refreshKanbanCols(); }});
  }

  function deleteKanbanTask(id) {
    fetch(LaRuche.API.base+'/api/kanban/'+id,{method:'DELETE'})
      .then(function(r){if(r.ok) { _kanbanLast=''; refreshKanbanCols(); }});
  }

  function editKanbanTask(id) {
    var t=null; try{ t=JSON.parse(_kanbanLast).find(function(x){return x.id===id;}); }catch(e){}
    // P1: Provider selector in the kanban editor.
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.kanbanDefProvider')+'</option>';
    Object.keys(_profiles).forEach(function(k){
        profOpts += '<option value="'+k+'" '+((t&&t.profile_id===k)?'selected':'')+'>'+LaRuche.Utils.esc(_profiles[k].name||k)+'</option>';
    });
    var modOpts = '<option value="">'+LaRuche.i18n.t('settings.kanbanParDefault')+'</option>';
    if(t && t.profile_id && _profiles[t.profile_id] && _profiles[t.profile_id].models){
      _profiles[t.profile_id].models.forEach(function(mm){
        modOpts += '<option value="'+LaRuche.Utils.esc(mm)+'" '+((t.model===mm)?'selected':'')+'>'+LaRuche.Utils.esc(mm)+'</option>';
      });
    }
    var ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div style="width:480px;max-width:92vw;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;padding:16px">'+
      '<div style="font-weight:600;color:var(--amber);margin-bottom:10px">'+LaRuche.i18n.t('settings.kanbanEditTitle')+'</div>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.kanbanEditTitleLabel')+'</label><input class="form-input" id="kbeTitle" value="'+LaRuche.Utils.esc(t?t.title:'')+'">'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.kanbanEditDescLabel')+'</label><textarea class="form-input" id="kbeDesc" rows="4">'+LaRuche.Utils.esc(t?(t.description||''):'')+'</textarea>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.kanbanEditProviderLabel')+'</label><select class="form-input" id="kbeProfile" onchange="LaRuche.Settings.updateKanbanEditModelSelect()">'+profOpts+'</select>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.modelLabel')+'</label><select class="form-input" id="kbeModel">'+modOpts+'</select>'+
      '<label class="form-label">'+LaRuche.i18n.t('settings.kanbanEditChannelLabel')+'</label><select class="form-input" id="kbeChannel"><option value="">'+LaRuche.i18n.t('settings.kanbanBoardChannel')+'</option></select>'+
      '<div style="margin-top:12px;display:flex;gap:8px"><button class="form-btn" onclick="LaRuche.Settings.saveKanbanEdit(\''+id+'\',this)">'+LaRuche.i18n.t('settings.kanbanEditSave')+'</button>'+
      '<button class="form-btn" style="background:none;border:1px solid var(--border);color:var(--text-dim)" onclick="this.closest(\'div[style*=fixed]\')&&this.closest(\'div[style*=fixed]\').remove()">'+LaRuche.i18n.t('settings.kanbanEditCancel')+'</button></div></div>';
    document.body.appendChild(ov);
    window.__fillChannels(document.getElementById('kbeChannel'), (t&&t.channel)||'', LaRuche.i18n.t('settings.kanbanBoardChannel'));
  }

  // P1: repopulates the kanban editor's model selector when the provider changes.
  function updateKanbanEditModelSelect() {
    var pId = document.getElementById('kbeProfile').value;
    var modelSel = document.getElementById('kbeModel');
    if(!modelSel) return;
    modelSel.innerHTML = '<option value="">'+LaRuche.i18n.t('settings.kanbanParDefault')+'</option>';
    if(pId && _profiles[pId] && _profiles[pId].models) {
      _profiles[pId].models.forEach(function(m){
        modelSel.innerHTML += '<option value="'+LaRuche.Utils.esc(m)+'">'+LaRuche.Utils.esc(m)+'</option>';
      });
    }
  }

  function saveKanbanEdit(id, btn) {
    var title=document.getElementById('kbeTitle').value, desc=document.getElementById('kbeDesc').value;
    var pEl=document.getElementById('kbeProfile'), mEl=document.getElementById('kbeModel');
    var pId = pEl ? pEl.value : '';
    var m = mEl ? mEl.value : '';
    var chEl=document.getElementById('kbeChannel'); var ch = chEl ? chEl.value : '';
    fetch(LaRuche.API.base+'/api/kanban/'+id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({title: title, description: desc, profile_id: pId||null, model: m||null, channel: ch})})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.kanbanTaskUpdated'),'ok'); _kanbanLast=''; refreshKanbanCols(); var ov=btn.closest('div[style*=fixed]'); if(ov)ov.remove(); } });
  }

  function kanbanDragStart(e, id) {
    e.dataTransfer.setData('text/plain', id);
  }

  function kanbanDragOver(e) {
    e.preventDefault();
  }

  function kanbanDrop(e, status) {
    e.preventDefault();
    var id = e.dataTransfer.getData('text/plain');
    if(id) {
       fetch(LaRuche.API.base+'/api/kanban/'+id+'/status',{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({status:status})})
         .then(function(r){if(r.ok) { _kanbanLast=''; refreshKanbanCols(); }});
    }
  }

  async function saveVoiceCfg() {
    var stt_external = !!document.getElementById('cfgSttExternal').checked;
    var speedEl = document.getElementById('cfgTtsSpeed');
    var voiceEl = document.getElementById('cfgTtsVoice');
    var body = { stt_external: stt_external };
    if(speedEl) body.tts_speed = parseFloat(speedEl.value);
    if(voiceEl) body.tts_voice = voiceEl.value.trim();
    try {
      var res = await fetch(LaRuche.API.base+'/api/config/voice', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify(body)
      });
      if(res.ok) LaRuche.Toast.show(LaRuche.i18n.t('settings.save'),'ok');
      else LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err');
    } catch(e) { LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); }
  }

  async function saveProviderCfg() {
    var fallback_models = document.getElementById('cfgProvFallback').value;
    var max_tokens = parseInt(document.getElementById('cfgProvMaxTokens').value, 10);
    var temperature = parseFloat(document.getElementById('cfgProvTemp').value);
    try {
      var res = await fetch(LaRuche.API.base+'/api/config/provider', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({
          fallback_models: fallback_models,
          max_tokens: max_tokens,
          temperature: temperature
        })
      });
      if(res.ok) LaRuche.Toast.show(LaRuche.i18n.t('settings.inferenceCfgSaved'),'ok');
      else LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err');
    } catch(e) { LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); }
  }

  async function loadOnboarding(el) {
    var data = await fetch(LaRuche.API.base+'/api/onboarding').then(function(r){return r.json();}).catch(function(){return {steps:[],progress:'0/0',complete:false};});
    var html = '<div style="margin-bottom:16px"><span style="font-size:18px;font-weight:600">'+LaRuche.i18n.t('settings.setupChecklist')+'</span>' +
      '<span style="margin-left:12px;padding:2px 10px;border-radius:10px;font-size:12px;background:'+(data.complete?'var(--green)':'var(--amber)')+';color:#000">'+LaRuche.Utils.esc(data.progress)+'</span></div>';
    html += '<div style="display:flex;flex-direction:column;gap:12px">';
    (data.steps||[]).forEach(function(s){
      var icon = s.done ? '<span style="color:var(--green);font-size:18px;margin-right:8px"><svg width="1.2em" height="1.2em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: middle;"><polyline points="20 6 9 17 4 12"></polyline></svg></span>' : '<span style="color:var(--red);font-size:18px;margin-right:8px">&#x2717;</span>';
      html += '<div class="settings-card" style="display:flex;align-items:center">' + icon +
        '<div><div style="font-weight:600">'+LaRuche.Utils.esc(s.title)+'</div>' +
        '<div style="font-size:11px;color:var(--text-muted);margin-top:2px">'+LaRuche.Utils.esc(s.instruction)+'</div></div></div>';
    });
    html += '</div>';
    el.innerHTML = html;
  }

  function saveContextCfg() {
    var max = parseInt(document.getElementById('cfgCtxMax').value, 10);
    var th = parseFloat(document.getElementById('cfgCtxThresh').value);
    fetch(LaRuche.API.base+'/api/config/compaction',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({context_max_messages:max,compaction_threshold:th})})
      .then(function(r){if(r.ok) LaRuche.Toast.show(LaRuche.i18n.t('settings.contextSaved'), 'ok'); else LaRuche.Toast.show(LaRuche.i18n.t('settings.contextSaveFailed'), 'err');})
      .catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err');});
  }

  function saveRuntimeCfg() {
    var body = {
      max_iterations: parseInt(document.getElementById('cfgMaxIter').value,10),
      temperature: parseFloat(document.getElementById('cfgTemp').value),
      max_tokens: parseInt(document.getElementById('cfgMaxTok').value,10),
      tool_selection_limit: parseInt(document.getElementById('cfgToolLim').value,10),
      dynamic_context_threshold: parseInt(document.getElementById('cfgCtxThreshold').value,10)
    };
    fetch(LaRuche.API.base+'/api/config/runtime',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})
      .then(function(r){ if(r.ok) LaRuche.Toast.show(LaRuche.i18n.t('settings.generationApplied'),'ok'); else LaRuche.Toast.show(LaRuche.i18n.t('settings.errorGeneric'),'err'); })
      .catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); });
  }

  function toggleCurateur(on) {
    fetch(LaRuche.API.base+'/api/config/curateur',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify({enabled:!!on})})
      .then(function(r){return r.json();})
      .then(function(d){ if(d && d.status==='ok') LaRuche.Toast.show('Curateur '+(on?LaRuche.i18n.t('settings.curateEnabled'):LaRuche.i18n.t('settings.curateDisabled')),'ok'); else LaRuche.Toast.show(LaRuche.i18n.t('settings.curateFailed'),'err'); })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.curateFailed'),'err'); });
  }
  function toggleDynamicTools(on) {
    fetch(LaRuche.API.base+'/api/config/curateur',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify({dynamic_tools:!!on})})
      .then(function(r){return r.json();})
      .then(function(d){ if(d && d.status==='ok') LaRuche.Toast.show(LaRuche.i18n.t('settings.dynToolsSaved')+(on?LaRuche.i18n.t('settings.dynToolsEnabled'):LaRuche.i18n.t('settings.dynToolsDisabled')),'ok'); else LaRuche.Toast.show(LaRuche.i18n.t('settings.dynToolsFailed'),'err'); })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.dynToolsFailed'),'err'); });
  }

  // Toggle the unlimited-reworks sentinel: disable the slider and show the infinity
  // marker when on, restore the slider value when off.
  function reineToggleUnlim() {
    var chk=document.getElementById('cfgReineUnlim');
    var sl=document.getElementById('cfgReineMax');
    var val=document.getElementById('cfgReineMaxVal');
    if(!chk||!sl||!val) return;
    if(chk.checked){ sl.disabled=true; val.textContent='∞'; }
    else { sl.disabled=false; val.textContent=sl.value; }
  }

  function saveReineCfg() {
    var unlim = !!(document.getElementById('cfgReineUnlim') && document.getElementById('cfgReineUnlim').checked);
    var body = {
      mode: document.getElementById('cfgReineMode').value,
      max_revues: unlim ? 255 : parseInt(document.getElementById('cfgReineMax').value,10),
      seuil_confiance: parseInt(document.getElementById('cfgReineSeuil').value,10),
      provider_profile: document.getElementById('cfgReineProvider').value.trim() || null,
      tier_reponse: !!document.getElementById('cfgReineTier1').checked,
      tier_artefacts: !!document.getElementById('cfgReineTier2').checked,
      tier_supervision: !!document.getElementById('cfgReineTier3').checked,
      queue_gate: !!document.getElementById('cfgReineQueue').checked,
      contexte_messages: parseInt(document.getElementById('cfgReineCtx').value,10)
    };
    fetch(LaRuche.API.base+'/api/config/reine',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})
      .then(function(r){ if(r.ok) LaRuche.Toast.show(LaRuche.i18n.t('settings.save'),'ok'); else LaRuche.Toast.show(LaRuche.i18n.t('settings.errorGeneric'),'err'); })
      .catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); });
  }

  // LaReine proposals backlog (Tier 2): list pending memory proposals, approve/reject.
  function renderReineProposals() {
    var box = document.getElementById('reineProposals');
    if(!box) return;
    fetch(LaRuche.API.base+'/api/reine/proposals').then(function(r){return r.json();}).catch(function(){return {proposals:[]};}).then(function(d){
      var pend = (d.proposals||[]).filter(function(p){ return p.status==='EnAttente'; });
      if(!pend.length){ box.innerHTML='<div style="color:var(--text-dim)">'+LaRuche.i18n.t('reine.queueEmpty')+'</div>'; return; }
      box.innerHTML = pend.map(function(p){
        var rc = p.risk==='Critique'?'var(--red)':(p.risk==='Sensible'?'var(--amber)':'var(--green)');
        return '<div style="display:flex;align-items:flex-start;gap:6px;padding:4px 0;border-bottom:1px solid rgba(255,255,255,.05)">'+
          '<span style="color:'+rc+';font-size:9px;margin-top:3px">●</span>'+
          '<div style="flex:1;min-width:0">'+
            '<div style="color:var(--text);font-size:11px">'+LaRuche.Utils.esc(p.target||p.type)+'</div>'+
            '<div style="color:var(--text-dim);font-size:10px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">'+LaRuche.Utils.esc(p.preview||'')+'</div>'+
          '</div>'+
          '<button class="form-btn" style="font-size:10px;padding:1px 6px" onclick="LaRuche.Settings.reineApprove(\''+p.id+'\')">'+LaRuche.i18n.t('reine.queueApprove')+'</button>'+
          '<button class="form-btn" style="font-size:10px;padding:1px 6px" onclick="LaRuche.Settings.reineReject(\''+p.id+'\')">'+LaRuche.i18n.t('reine.queueReject')+'</button>'+
        '</div>';
      }).join('');
    });
  }
  function reineApprove(id){
    fetch(LaRuche.API.base+'/api/reine/proposals/'+encodeURIComponent(id)+'/approve',{method:'POST',credentials:'include'}).then(function(){ renderReineProposals(); });
  }
  function reineReject(id){
    fetch(LaRuche.API.base+'/api/reine/proposals/'+encodeURIComponent(id)+'/reject',{method:'POST',credentials:'include'}).then(function(){ renderReineProposals(); });
  }
  function reineApplySafe(){
    fetch(LaRuche.API.base+'/api/reine/proposals/apply-safe',{method:'POST',credentials:'include'}).then(function(r){return r.json();}).then(function(d){ if(LaRuche.Toast) LaRuche.Toast.show((d.applied||0)+' OK','ok'); renderReineProposals(); }).catch(function(){ renderReineProposals(); });
  }

  function saveChannels() {
    var config = {
      telegram: { bot_token: document.getElementById('ch-tg-token').value, allowed_chats: document.getElementById('ch-tg-chats').value, enabled: !!document.getElementById('ch-tg-token').value },
      discord: { bot_token: document.getElementById('ch-dc-token').value, allowed_channels: document.getElementById('ch-dc-channels').value, enabled: !!document.getElementById('ch-dc-token').value },
      slack: { bot_token: document.getElementById('ch-sl-bot').value, app_token: document.getElementById('ch-sl-app').value, enabled: !!document.getElementById('ch-sl-bot').value },
    };
    var notifyEnabled = document.getElementById('ch-notify-en') ? document.getElementById('ch-notify-en').checked : false;
    Promise.all([
      fetch(LaRuche.API.base+'/api/config/channels',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(config)}),
      fetch(LaRuche.API.base+'/api/config/notify',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({enabled:notifyEnabled})})
    ])
      .then(function(){LaRuche.Toast.show(LaRuche.i18n.t('settings.channelsSaved'),'ok');})
      .catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err');});
  }

  async function loadKnowledge(el) {
    var data = await fetch(LaRuche.API.base+'/api/knowledge').then(function(r){return r.json();}).catch(function(){return {entries:[],count:0};});
    var html = '<div style="margin-bottom:16px;display:flex;gap:8px;align-items:end">' +
      '<div style="flex:1"><label class="form-label">'+LaRuche.i18n.t('settings.kbAddLabel')+'</label><input class="form-input" id="kb-text" placeholder="'+LaRuche.i18n.t('settings.kbAddPlaceholder')+'"></div>' +
      '<div><label class="form-label">'+LaRuche.i18n.t('settings.kbSourceLabel')+'</label><input class="form-input" id="kb-source" placeholder="'+LaRuche.i18n.t('settings.optional')+'" style="width:150px"></div>' +
      '<button class="form-btn" onclick="LaRuche.Settings.addKnowledge()">'+LaRuche.i18n.t('settings.kbAddKnowledgeBtn')+'</button></div>';
    html += '<div style="margin-bottom:16px;display:flex;gap:8px;">' +
      '<button class="form-btn" onclick="LaRuche.Settings.exportOkf()">'+LaRuche.i18n.t('settings.kbExportBtn')+'</button>' +
      '<button class="form-btn" onclick="LaRuche.Settings.importOkf()">'+LaRuche.i18n.t('settings.kbImportBtn')+'</button>' +
      '</div>';
    html += '<div style="font-size:12px;color:var(--text-dim);margin-bottom:12px">'+data.count+LaRuche.i18n.t('settings.kbEntriesCount')+'</div>';
    if(data.entries && data.entries.length > 0) {
      html += '<table style="width:100%;border-collapse:collapse;font-size:12px">';
      html += '<tr><th style="text-align:left;padding:6px;color:var(--text-dim);border-bottom:1px solid var(--border)">ID</th>';
      html += '<th style="text-align:left;padding:6px;color:var(--text-dim);border-bottom:1px solid var(--border)">'+LaRuche.i18n.t('settings.kbColText')+'</th>';
      html += '<th style="padding:6px;color:var(--text-dim);border-bottom:1px solid var(--border)">'+LaRuche.i18n.t('settings.kbColSource')+'</th>';
      html += '<th style="padding:6px;color:var(--text-dim);border-bottom:1px solid var(--border)">'+LaRuche.i18n.t('settings.kbColActions')+'</th></tr>';
      data.entries.forEach(function(e) {
        html += '<tr><td style="padding:6px;border-bottom:1px solid rgba(42,42,46,.3);font-family:var(--mono);font-size:10px;color:var(--text-muted)">'+LaRuche.Utils.esc(e.id)+'</td>';
        html += '<td style="padding:6px;border-bottom:1px solid rgba(42,42,46,.3)">'+LaRuche.Utils.esc((e.text||'').substring(0,100))+'</td>';
        html += '<td style="padding:6px;border-bottom:1px solid rgba(42,42,46,.3);color:var(--text-dim)">'+LaRuche.Utils.esc(e.source||'-')+'</td>';
        html += '<td style="padding:6px;border-bottom:1px solid rgba(42,42,46,.3);text-align:center">' +
          '<button onclick="LaRuche.Settings.editKnowledge(\''+e.id+'\',this)" style="background:none;border:1px solid var(--amber);color:var(--amber);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;margin-right:4px">'+LaRuche.i18n.t('settings.kbEditBtn')+'</button>' +
          '<button onclick="LaRuche.Settings.deleteKnowledge(\''+e.id+'\')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.kbDelBtn')+'</button></td></tr>';
      });
      html += '</table>';
    } else {
      html += '<div style="text-align:center;color:var(--text-muted);padding:30px">'+LaRuche.i18n.t('settings.kbEmpty')+'</div>';
    }
    el.innerHTML = html;
  }

  function addKnowledge() {
    var text = document.getElementById('kb-text').value;
    var source = document.getElementById('kb-source').value;
    if(!text) return;
    fetch(LaRuche.API.base+'/api/knowledge',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({text:text,source:source||'manual'})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d.error) LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+d.error,'err');
        else { LaRuche.Toast.show(LaRuche.i18n.t('settings.kbAdded')+d.id+')','ok'); loadTab('knowledge'); }
      })
      .catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err');});
  }

  function exportOkf() {
    // Browser .zip download (the whole memory) instead of a server folder.
    var a = document.createElement('a');
    a.href = LaRuche.API.base+'/api/memory/export.zip';
    a.download = ''; a.style.display = 'none';
    document.body.appendChild(a); a.click();
    setTimeout(function(){ a.remove(); }, 0);
    LaRuche.Toast.show(LaRuche.i18n.t('settings.kbExportLaunched'), 'ok');
  }

  function importOkf() {
    fetch(LaRuche.API.base+'/api/memory/import_okf?dir=okf-export', {method:'POST'})
      .then(function(r){return r.json();})
      .then(function(res){
        if(res.ok) {
            LaRuche.Toast.show(LaRuche.i18n.t('settings.kbImported'), 'ok');
            loadKnowledge(document.getElementById('settings-content'));
        }
        else LaRuche.Toast.show(LaRuche.i18n.t('settings.kbImportError') + res.error, 'err');
      });
  }

  function editKnowledge(id, btn) {
    var row = btn.closest('tr');
    var textCell = row.cells[1];
    var sourceCell = row.cells[2];
    var currentText = textCell.textContent;
    var currentSource = sourceCell.textContent === '-' ? '' : sourceCell.textContent;

    // Replace cells with inputs
    textCell.innerHTML = '<textarea style="width:100%;background:var(--bg-input);border:1px solid var(--amber);border-radius:4px;color:var(--text);padding:4px;font-size:11px;min-height:50px;resize:vertical">'+LaRuche.Utils.esc(currentText)+'</textarea>';
    sourceCell.innerHTML = '<input style="width:100%;background:var(--bg-input);border:1px solid var(--border);border-radius:4px;color:var(--text);padding:4px;font-size:11px" value="'+LaRuche.Utils.esc(currentSource)+'">';

    // Replace buttons with Save/Cancel
    var actionsCell = row.cells[3];
    actionsCell.innerHTML = '<button onclick="LaRuche.Settings.saveKnowledgeEdit(\''+id+'\',this)" style="background:var(--green);color:#000;border:none;border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;margin-right:4px">'+LaRuche.i18n.t('settings.statusOk')+'</button>' +
      '<button onclick="LaRuche.Settings.refreshTab()" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.tlCancel')+'</button>';
  }

  function saveKnowledgeEdit(id, btn) {
    var row = btn.closest('tr');
    var newText = row.cells[1].querySelector('textarea').value;
    var newSource = row.cells[2].querySelector('input').value;
    fetch(LaRuche.API.base+'/api/knowledge/'+id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({text:newText,source:newSource||'manual'})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d.error) LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+d.error,'err');
        else { LaRuche.Toast.show(LaRuche.i18n.t('settings.kbUpdated'),'ok'); loadTab('knowledge'); }
      })
      .catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err');});
  }

  function deleteKnowledge(id) {
    fetch(LaRuche.API.base+'/api/knowledge/'+id,{method:'DELETE'})
      .then(function(){LaRuche.Toast.show(LaRuche.i18n.t('settings.kbDeleted'),'ok'); loadTab('knowledge');})
      .catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err');});
  }

  function refreshTab() { loadTab(currentTab); }

  function startChannel(name) {
    fetch(LaRuche.API.base+'/api/channels/start',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({channel:name})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d.status==='started') LaRuche.Toast.show(name+LaRuche.i18n.t('settings.channelStarted'),'ok');
        else if(d.status==='already_running') LaRuche.Toast.show(name+LaRuche.i18n.t('settings.channelAlreadyRunning'),'info');
        else LaRuche.Toast.show(d.message||LaRuche.i18n.t('settings.errorGeneric'),'err');
        loadTab('channels');
      });
  }

  function stopChannel(name) {
    fetch(LaRuche.API.base+'/api/channels/stop',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({channel:name})})
      .then(function(r){return r.json();})
      .then(function(d){
        LaRuche.Toast.show(name+LaRuche.i18n.t('settings.channelStopped'),'ok');
        loadTab('channels');
      });
  }

    var _bpCronBuilderId = null; // CronBuilder instance of the creation form

    async function loadBlueprints(el) {
    var bps=[];try{bps=await fetch('/api/blueprints').then(function(r){return r.json();});}catch(e){}
    window._blueprints = bps || [];
    var head = '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px;gap:8px;flex-wrap:wrap">' +
      '<span style="color:var(--amber);font-size:12px;">'+LaRuche.i18n.t('settings.blueprintsHint')+'</span>' +
      '<button class="settings-save-btn" onclick="LaRuche.Settings.openNewBlueprintForm()">'+LaRuche.i18n.t('settings.newBlueprintBtn')+'</button>' +
      '</div>';
    var creationSlot = '<div id="bpNewFormWrap"></div>';
    var cards = (!window._blueprints.length)
      ? '<div style="text-align:center;color:var(--text-muted);padding:20px">'+LaRuche.i18n.t('settings.bpNone')+'</div>'
      : window._blueprints.map(function(b, idx) {
        return '<div class="settings-card" style="margin-bottom:12px;cursor:pointer;" onclick="LaRuche.Settings.openBlueprintForm('+idx+')">' +
          '<div style="display:flex;justify-content:space-between;align-items:flex-start;gap:8px">' +
            '<div style="flex:1">' +
              '<div class="settings-card-title">'+LaRuche.Utils.esc(b.title||b.id)+'</div>' +
              '<div style="font-size:12px;color:var(--text-dim);margin-top:4px;">'+LaRuche.Utils.esc(b.description||'')+'</div>' +
            '</div>' +
            '<button onclick="event.stopPropagation();LaRuche.Settings.deleteBlueprint('+idx+')" title="'+LaRuche.i18n.t('settings.bpDeleteBtn')+'" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;flex:0 0 auto">'+LaRuche.i18n.t('settings.bpDeleteBtn')+'</button>' +
          '</div>' +
          '<div id="bpForm_'+idx+'" style="display:none;margin-top:12px;padding-top:12px;border-top:1px solid var(--border);" onclick="event.stopPropagation()">' +
            (b.slots||[]).map(function(slot){
              return '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.Utils.esc(slot.label||slot.name)+'</label><input id="bpInput_'+idx+'_'+slot.name+'" class="form-input" placeholder="'+LaRuche.Utils.esc(slot.placeholder||slot.default||'')+'" value="'+LaRuche.Utils.esc(slot.default||'')+'"></div>';
            }).join('') +
            '<button class="settings-save-btn" style="margin-top:8px" onclick="LaRuche.Settings.instanciateBlueprint('+idx+')">'+LaRuche.i18n.t('settings.bpInstanciateBtn')+'</button>' +
          '</div>' +
        '</div>';
      }).join('');
    el.innerHTML = head + creationSlot + cards;
  }

  // --- Custom blueprint creation form ---
  function bpSlotRowHtml(){
    return '<div class="bp-slot-row" style="display:flex;gap:6px;margin-bottom:6px;align-items:center">' +
      '<input class="form-input bp-slot-name" placeholder="'+LaRuche.i18n.t('settings.bpSlotNamePlaceholder')+'" style="flex:1">' +
      '<input class="form-input bp-slot-label" placeholder="'+LaRuche.i18n.t('settings.bpSlotLabelPlaceholder')+'" style="flex:1">' +
      '<input class="form-input bp-slot-default" placeholder="'+LaRuche.i18n.t('settings.bpSlotDefaultPlaceholder')+'" style="flex:1">' +
      '<button onclick="this.parentNode.remove()" title="'+LaRuche.i18n.t('settings.bpDeleteSlotBtn')+'" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:4px 8px;cursor:pointer;font-size:11px;flex:0 0 auto">×</button>' +
      '</div>';
  }

  function addBlueprintSlotRow(){
    var box = document.getElementById('bpSlotsList');
    if(!box) return;
    var tmp = document.createElement('div'); tmp.innerHTML = bpSlotRowHtml();
    box.appendChild(tmp.firstChild);
  }

  function openNewBlueprintForm(){
    var wrap = document.getElementById('bpNewFormWrap');
    if(!wrap) return;
    if(wrap.dataset.open === '1'){ wrap.innerHTML=''; wrap.dataset.open='0'; _bpCronBuilderId=null; return; }
    wrap.dataset.open = '1';
    wrap.innerHTML =
      '<div class="settings-card" style="margin-bottom:12px;border:1px solid var(--amber)">' +
        '<div class="settings-card-title">'+LaRuche.i18n.t('settings.bpNewTitle')+'</div>' +
        '<div style="margin-top:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.bpTitleLabel')+'</label>' +
          '<input id="bpNewTitle" class="form-input" placeholder="'+LaRuche.i18n.t('settings.bpTitlePlaceholder')+'"></div>' +
        '<div style="margin-top:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.bpPromptLabel')+'</label>' +
          '<textarea id="bpNewPrompt" class="form-input" style="min-height:90px;resize:vertical" placeholder="'+LaRuche.i18n.t('settings.varPlaceholder')+'"></textarea></div>' +
        '<div style="margin-top:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.bpScheduleLabel')+'</label><div id="bpNewCron"></div></div>' +
        '<div style="margin-top:10px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.bpSlotsLabel')+'</label>' +
          '<div id="bpSlotsList" style="margin-top:6px"></div>' +
          '<button onclick="LaRuche.Settings.addBlueprintSlotRow()" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:4px 10px;cursor:pointer;font-size:11px;margin-top:2px">'+LaRuche.i18n.t('settings.bpAddSlot')+'</button>' +
        '</div>' +
        '<div style="margin-top:12px;display:flex;gap:8px">' +
          '<button class="settings-save-btn" onclick="LaRuche.Settings.saveNewBlueprint()">'+LaRuche.i18n.t('settings.bpCreateBtn')+'</button>' +
          '<button onclick="LaRuche.Settings.openNewBlueprintForm()" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:6px 12px;cursor:pointer;font-size:12px">'+LaRuche.i18n.t('settings.bpCancelBtn')+'</button>' +
        '</div>' +
      '</div>';
    _bpCronBuilderId = (LaRuche.CronBuilder) ? LaRuche.CronBuilder.mount('bpNewCron', { value:'' }) : null;
    addBlueprintSlotRow();
  }

  function saveNewBlueprint(){
    var title = (document.getElementById('bpNewTitle')||{}).value || '';
    var prompt = (document.getElementById('bpNewPrompt')||{}).value || '';
    var cron = (_bpCronBuilderId && LaRuche.CronBuilder) ? LaRuche.CronBuilder.getValue(_bpCronBuilderId) : '';
    title = title.trim();
    if(!title){ LaRuche.Toast.show(LaRuche.i18n.t('settings.bpTitleRequired'),'warn'); return; }
    if(!prompt.trim()){ LaRuche.Toast.show(LaRuche.i18n.t('settings.bpPromptRequired'),'warn'); return; }
    var slots = [];
    document.querySelectorAll('#bpSlotsList .bp-slot-row').forEach(function(row){
      var name = (row.querySelector('.bp-slot-name')||{}).value || '';
      name = name.trim();
      if(!name) return;
      slots.push({
        name: name,
        label: ((row.querySelector('.bp-slot-label')||{}).value || '').trim() || name,
        default: ((row.querySelector('.bp-slot-default')||{}).value || '').trim()
      });
    });
    var body = { title:title, prompt_template:prompt, schedule_template:cron, slots:slots };
    fetch('/api/blueprints', {
      method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
    }).then(function(r){ return r.json().catch(function(){ return {}; }).then(function(d){ return {ok:r.ok, d:d}; }); })
      .then(function(res){
        if(res.ok && !(res.d && res.d.error)){
          LaRuche.Toast.show(LaRuche.i18n.t('settings.bpCreated'),'ok');
          var el = document.getElementById('autoContent'); if(el) loadBlueprints(el);
        } else {
          LaRuche.Toast.show(LaRuche.i18n.t('settings.bpCreateError')+((res.d&&res.d.error)||'?'),'err');
        }
      }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); });
  }

  function deleteBlueprint(idx){
    var b = window._blueprints[idx]; if(!b) return;
    if(!window.confirm(LaRuche.i18n.t('settings.bpDeleteConfirm')+(b.title||b.id)+LaRuche.i18n.t('settings.bpDeleteConfirmSuffix'))) return;
    fetch('/api/blueprints/'+encodeURIComponent(b.id), { method:'DELETE' })
      .then(function(r){ return r.json().catch(function(){ return {}; }).then(function(d){ return {ok:r.ok, d:d}; }); })
      .then(function(res){
        if(res.ok && !(res.d && res.d.error)){
          LaRuche.Toast.show(LaRuche.i18n.t('settings.bpDeleted'),'ok');
          var el = document.getElementById('autoContent'); if(el) loadBlueprints(el);
        } else {
          LaRuche.Toast.show(LaRuche.i18n.t('settings.bpDeleteRefused')+((res.d&&res.d.error)||LaRuche.i18n.t('settings.bpDeleteRefusedFallback')),'err');
        }
      }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); });
  }

  function openBlueprintForm(idx) {
    var form = document.getElementById('bpForm_'+idx);
    if(!form) return;
    if (form.style.display === 'none') {
      form.style.display = 'block';
    } else {
      form.style.display = 'none';
    }
  }

  function instanciateBlueprint(idx) {
    var b = window._blueprints[idx];
    var slotsData = {};
    (b.slots||[]).forEach(function(slot){
      var inp = document.getElementById('bpInput_'+idx+'_'+slot.name);
      slotsData[slot.name] = inp ? inp.value : '';
    });
    fetch('/api/blueprints/'+encodeURIComponent(b.id)+'/instancier', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(slotsData)
    }).then(function(res) {
      if(res.ok) {
        LaRuche.Toast.show(LaRuche.i18n.t('settings.bpInstanciated'), 'ok');
        document.getElementById('bpForm_'+idx).style.display = 'none';
      } else {
        LaRuche.Toast.show(LaRuche.i18n.t('settings.bpInstanciateError'), 'err');
      }
    }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e, 'err'); });
  }

  function setChannelModel(channel, val){
    var parts = val ? val.split('|||') : ['',''];
    fetch(LaRuche.API.base+'/api/config/channel-models', {
      method:'POST', headers:{'Content-Type':'application/json'},
      body: JSON.stringify({channel:channel, profile_id:parts[0], model:parts[1]})
    }).then(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.chModelSaved'),'ok'); })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.codexError'),'err'); });
  }

  return { init:init, openBlueprintForm:openBlueprintForm, instanciateBlueprint:instanciateBlueprint, openNewBlueprintForm:openNewBlueprintForm, saveNewBlueprint:saveNewBlueprint, addBlueprintSlotRow:addBlueprintSlotRow, deleteBlueprint:deleteBlueprint, enter:enter, leave:leave, createCron:createCron, deleteCronTask:deleteCronTask, createWatcher:createWatcher, editWatcher:editWatcher, saveWatcherEdit:saveWatcherEdit, updateWatcherEditModelSelect:updateWatcherEditModelSelect, refreshTab:refreshTab,
    loadCron:loadCron, loadWatchers:loadWatchers, loadKanban:loadKanban, loadBlueprints:loadBlueprints, loadCronTimeline:loadCronTimeline, saveChannels:saveChannels, setChannelModel:setChannelModel, saveContextCfg:saveContextCfg, saveRuntimeCfg:saveRuntimeCfg, saveReineCfg:saveReineCfg, reineToggleUnlim:reineToggleUnlim, renderReineProposals:renderReineProposals, reineApprove:reineApprove, reineReject:reineReject, reineApplySafe:reineApplySafe, toggleCurateur:toggleCurateur, toggleDynamicTools:toggleDynamicTools, saveProviderCfg:saveProviderCfg, saveVoiceCfg:saveVoiceCfg, addKnowledge:addKnowledge, exportOkf:exportOkf, importOkf:importOkf, deleteKnowledge:deleteKnowledge, editKnowledge:editKnowledge, saveKnowledgeEdit:saveKnowledgeEdit, startChannel:startChannel, stopChannel:stopChannel, showProfileForm:showProfileForm, editProfile:editProfile, deleteProfile:deleteProfile, testProfile:testProfile, saveProfile:saveProfile, onProfileProviderChange:onProfileProviderChange, startCodexLogin:startCodexLogin, logoutCodex:logoutCodex, toggleTool:toggleTool, toggleAllTools:toggleAllTools, loadSkills:loadSkills, toggleSkill:toggleSkill, deleteSkill:deleteSkill, newSkill:newSkill, viewSkill:viewSkill, saveSkill:saveSkill, applySkillTools:applySkillTools, toggleSkillTool:toggleSkillTool, filterSkillTools:filterSkillTools, clearSkillTools:clearSkillTools, newPlugin:newPlugin, viewPlugin:viewPlugin, savePlugin:savePlugin, deletePlugin:deletePlugin, createKanbanTask:createKanbanTask, setKanbanDefaultChannel:setKanbanDefaultChannel, loadSecrets: loadSecrets, secretSet: secretSet, secretDelete: secretDelete, loadMcp: loadMcp, loadMcpServers: loadMcpServers, createMcpServer: createMcpServer, deleteMcpServer: deleteMcpServer, updateKanbanModelSelect: updateKanbanModelSelect, updateKanbanEditModelSelect: updateKanbanEditModelSelect, updateWatcherModelSelect: updateWatcherModelSelect, deleteKanbanTask:deleteKanbanTask, editKanbanTask:editKanbanTask, saveKanbanEdit:saveKanbanEdit, toggleKanbanResult:toggleKanbanResult, setKanbanView:setKanbanView, kanbanDragStart:kanbanDragStart, kanbanDragOver:kanbanDragOver, kanbanDrop:kanbanDrop, addCredential:addCredential, deleteCredential:deleteCredential, updateCronModelSelect:updateCronModelSelect, updateCronEditModelSelect:updateCronEditModelSelect, toggleVisibility:toggleVisibility, openAccess:openAccess, tlZoom:tlZoom, tlRecenter:tlRecenter, tlDetail:tlDetail, tlReload:tlReload, tlRun:tlRun, tlEdit:tlEdit, tlSaveEdit:tlSaveEdit, tlToggle:tlToggle };
})();

/* ── CronBuilder: reusable "human-friendly" component (missions + cron) ── */
