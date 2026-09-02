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
  'settings.mcpServerToggle':    {fr:'Serveur MCP (exposer mes outils)', en:'MCP server (expose my tools)'},
  'settings.mcpServerHint':      {fr:' un client externe pourra appeler TOUS les outils, shell compris', en:' an external client will be able to call EVERY tool, shell included'},
  'settings.dynToolsSelect':     {fr:'Sélection dynamique des outils ', en:'Dynamic tool selection '},
  'settings.dynToolsHint':       {fr:'(prompt léger, recommandé pour petits modèles / llama.cpp)', en:'(light prompt, recommended for small models / llama.cpp)'},
  'settings.curEnvForced':       {fr:'Forcé par RUCHE_CURATEUR=1 (variable d\'env).', en:'Forced by RUCHE_CURATEUR=1 (env variable).'},
  'settings.curDefault':         {fr:'En arrière-plan, conservateur (dédup auto). Off = ne crée rien.', en:'Background, conservative (auto-dedup). Off = creates nothing.'},
  'settings.system':             {fr:'System',           en:'System'},
  'settings.showTransparency':   {fr:'Outils et mémoires dans le fil', en:'Tools and memories in the thread'},
  'settings.showTransparencyHint': {fr:"Dans le fil du chat, montre les outils que l'agent a choisis et le nombre de mémoires utilisées. Affichage seulement - ça ne change rien au fonctionnement de LaRuche.", en:'In the chat thread, show which tools the agent picked and how many memories it used. Display only - it changes nothing to how LaRuche works.'},
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
  'settings.toolsEmpty':         {fr:'Aucun outil configuré', en:'No tools configured'},
  'settings.toolsConfigErr':     {fr:'Erreur de configuration des outils', en:'Tools configuration error'},
  'settings.allToolsEnabled':    {fr:'Tous les outils activés', en:'All tools enabled'},
  'settings.allToolsDisabled':   {fr:'Tous les outils désactivés', en:'All tools disabled'},
  'settings.toolsErr':           {fr:'Erreur outils: ', en:'Tools error: '},
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
  'settings.visionReessayer':    {fr:'Reessayer',         en:'Try again'},
  'settings.visionRendue':       {fr:'Les images repartent vers ce modele',
                                  en:'Images are sent to this model again'},
  'settings.visionDejaOk':       {fr:'Ce modele recevait deja les images',
                                  en:'This model was already getting images'},
  'settings.tlRunStarting':      {fr:'Lancement...', en:'Starting...'},
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
  'settings.mcpNone':            {fr:'Aucun serveur configuré.', en:'No servers configured.'},
  'settings.mcpDeleteConfirm':   {fr:'Supprimer ce serveur MCP ?', en:'Delete this MCP server?'},
  'settings.mcpDeleted':         {fr:'Serveur MCP supprimé', en:'MCP server deleted'},
  'settings.parDefault':         {fr:'(Par défaut)',      en:'(Default)'},
  'settings.notifyLabel':        {fr:'Activer Notifier proactif', en:'Enable proactive notifications'},
  'settings.notifyHint':         {fr:"LaRuche t'écrit sur Telegram d'elle-même, sans que tu aies rien demandé, dans deux cas : une tâche lancée en arrière-plan vient de se terminer, ou une sentinelle a détecté ce qu'elle surveillait. Le message part vers le premier Chat ID de la liste ci-dessus. Désactivé, rien ne part : tu retrouves ces événements dans le fil d'activité.", en:"LaRuche messages you on Telegram on its own, unprompted, in two cases: a task started in the background has just finished, or a watcher spotted what it was watching for. The message goes to the first Chat ID in the list above. Turned off, nothing is sent: you still find these events in the activity feed."},
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
  'settings.skillToolsHint':     {fr:'Outils / plugins recommandés par ce skill (→ <code>tools:</code>). Pas une permission : ceux qui manquent au tour sont rappelés à l\'agent. ', en:'Tools / plugins this skill recommends (→ <code>tools:</code>). Not a permission: the ones missing from the turn are pointed out to the agent. '},
  'settings.skillGroupTools':    {fr:'Outils',           en:'Tools'},
  'settings.skillGroupPlugins':  {fr:'Plugins',          en:'Plugins'},
  'settings.skillGroupOther':    {fr:'Autres',           en:'Other'},
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
  'settings.kanbanInterval':       {fr:'Relève', en:'Poll'},
  'settings.kanbanIntervalDesc':   {fr:"secondes entre deux relèves de la colonne Ready",
                                    en:'seconds between two sweeps of the Ready column'},
  'settings.kanbanIntervalUpdated':{fr:'Relève réglée sur {n} s', en:'Poll set to {n} s'},
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
  'settings.kanbanTaskIntrouvable': {fr:"Cette tâche n'est plus sur le tableau.",
                                     en:'That task is no longer on the board.'},
  'settings.kanbanDelBtn':       {fr:'Suppr',            en:'Del'},
  'settings.kanbanLancerBtn':    {fr:'Lancer',           en:'Run'},
  'settings.kanbanColonne':      {fr:'Colonne',          en:'Column'},
  'settings.profilePhotoAide':   {fr:'Cliquez sur la photo pour la remplacer. Carree, redimensionnee en 128 px.',
                                  en:'Click the photo to replace it. Square, resized to 128 px.'},
  'settings.adminPhoto':         {fr:'Photo',              en:'Photo'},
  'settings.adminPhotoFaite':    {fr:'Photo mise a jour',  en:'Photo updated'},
  'settings.kanbanTodoTitre':    {fr:'Relever la colonne A faire', en:'Sweep the Todo column'},
  'settings.kanbanTodoAide':     {fr:"A l'echeance, les taches de A faire passent dans Pret, les plus anciennes d'abord. C'est la releve de Pret qui les execute ensuite, une par une, chacune avec son propre fournisseur.",
                                  en:'On schedule, Todo tasks move to Ready, oldest first. The Ready poll then runs them one at a time, each with its own provider.'},
  'settings.kanbanTodoPeriode':  {fr:'Tous les',           en:'Every'},
  'settings.kanbanTodoMaintenant': {fr:'Relever maintenant', en:'Sweep now'},
  'settings.kanbanTodoFait':     {fr:'{n} tache(s) passee(s) dans Pret', en:'{n} task(s) moved to Ready'},
  'settings.kanbanTodoRien':     {fr:'La colonne A faire est vide', en:'The Todo column is empty'},
  'settings.kanbanTodoRegle':    {fr:'Releve reglee',      en:'Sweep updated'},
  'settings.kanbanTodoJamais':   {fr:'jamais relevee',     en:'never swept'},
  'settings.kanbanTodoDernier':  {fr:'derniere releve {q}', en:'last sweep {q}'},
  'settings.uniteHeures':        {fr:'heures',             en:'hours'},
  'settings.uniteJours':         {fr:'jours',              en:'days'},
  'settings.uniteSemaines':      {fr:'semaines',           en:'weeks'},
  'settings.kanbanLancerHint':   {fr:'Envoyer cette tache dans Pret: la releve la prendra au prochain passage.',
                                  en:'Send this task to Ready: the next poll picks it up.'},
  'settings.kanbanLancee':       {fr:'Tache envoyee dans Pret', en:'Task moved to Ready'},
  'settings.kanbanFluxAide':     {fr:"Une tache creee arrive dans A faire et y reste. Seule la colonne Pret est relevee: glissez-la dedans, ou cliquez Lancer.",
                                  en:'A new task lands in Todo and stays there. Only Ready is polled: drag it there, or click Run.'},
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
  'settings.cronUpdated':        {fr:'Tache mise a jour', en:'Task updated'},
  'settings.cronNamePromptRequired': {fr:'Nom et prompt requis', en:'Name and prompt required'},
  'settings.cronListTitle':      {fr:'Taches planifiees', en:'Scheduled tasks'},
  'settings.bpKindCron':         {fr:'Cron', en:'Cron'},
  'settings.bpKindWatcher':      {fr:'Watcher', en:'Watcher'},
  'settings.bpKindRecherche':    {fr:'Recherche', en:'Research'},
  'settings.bpCreateWatcher':    {fr:'Creer le watcher', en:'Create watcher'},
  'settings.bpCreateRecherche':  {fr:'Lancer la recherche', en:'Start research'},
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
  'settings.mixtureModels':      {fr:'Candidats Mixture', en:'Mixture candidates'},
  'settings.mixtureModelsHint':  {fr:'Utilisés uniquement par l’outil Mixture quand aucun candidat n’est fourni.', en:'Used only by the Mixture tool when no candidates are supplied.'},
  'settings.memoryReviewModel':  {fr:'Modèle d’enrichissement mémoire', en:'Memory enrichment model'},
  'settings.memoryReviewHint':   {fr:'Optionnel · utilisé seulement pour enrichir un nœud mémoire.', en:'Optional · used only when enriching a memory node.'},
  'settings.codexRuntimeHint':   {fr:'Codex gère actuellement ses propres paramètres : Température et Max tokens ne lui sont pas envoyés. Ces valeurs restent actives pour les autres providers.', en:'Codex currently manages its own parameters: Temperature and Max tokens are not sent to it. These values still apply to other providers.'},
  'settings.modelExample':       {fr:'ex: gpt-4o',       en:'e.g.: gpt-4o'},
  'settings.activeLabel':        {fr:'Actif : ',         en:'Active: '},
  'settings.voice':              {fr:'Voix',             en:'Voice'},
  'settings.statusOk':           {fr:'OK',               en:'OK'},
  'settings.statusOff':          {fr:'Off',              en:'Off'},
  'settings.sttExternal':        {fr:'STT externe',      en:'External STT'},
  'settings.sttExternalHint':    {fr:'Décoché (défaut) : le modèle transcrit lui-même l\'audio. Coché : utiliser le service STT externe (:8421).', en:'Unchecked (default): the model transcribes audio itself. Checked: use the external STT service (:8421).'},
  'settings.sttExternalNote':    {fr:'Par défaut, l\'audio (ex. vocal Telegram) va au modèle. Cochez si votre modèle ne sait pas faire le STT.', en:'By default, audio (e.g. Telegram voice) goes to the model. Check this if your model cannot do STT.'},
  'settings.ttsSpeed':           {fr:'Vitesse TTS',       en:'TTS speed'},
  'settings.ttsBackend':         {fr:'Moteur TTS',        en:'TTS backend'},
  'settings.ttsBackendAuto':     {fr:'Auto (défaut)',     en:'Auto (default)'},
  'settings.ttsBackendHint':     {fr:'Moteur de synthese. Voicebox = ta voix clonee (service voicebox.sh lance). openai-tts = tout serveur parlant le format OpenAI /v1/audio/speech, local ou distant, regle par TTS_OPENAI_URL. Auto = celui detecte par le service.', en:'Synthesis engine. Voicebox = your cloned voice (voicebox.sh running). openai-tts = any server speaking the OpenAI /v1/audio/speech format, local or remote, set with TTS_OPENAI_URL. Auto = whatever the service detected.'},
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
  'settings.navAppearance':     {fr:'Apparence',            en:'Appearance'},
  'settings.dockDetacher':      {fr:'Detacher',               en:'Detach'},
  'settings.dockAide':          {fr:"Accrocher ce panneau sur le cote, pour s'en servir depuis n'importe quelle page", en:'Dock this panel to the side, to use it from any page'},
  'settings.dockAilleurs':      {fr:"Ce panneau est ouvert dans le dock, sur le cote. Fermez-le pour le retrouver ici.", en:'This panel is open in the side dock. Close it to bring it back here.'},
  'settings.dockVersPage':      {fr:'Rouvrir dans la page',    en:'Reopen in the page'},
  'settings.themeSaveAs':       {fr:'Enregistrer comme nouveau theme', en:'Save as a new theme'},
  'settings.themeRename':       {fr:'Renommer',               en:'Rename'},
  'settings.themeDuplicate':    {fr:'Dupliquer',              en:'Duplicate'},
  'settings.themeRevert':       {fr:"Revenir aux valeurs d'origine", en:'Back to original values'},
  'settings.themeAutoSaved':    {fr:"Theme a vous: chaque changement est enregistre tout seul.", en:'Your own theme: every change is saved on its own.'},
  'settings.themeUnsaved':      {fr:'non enregistre',         en:'unsaved'},
  'settings.themeSaving':       {fr:'enregistrement...',      en:'saving...'},
  'settings.themeSavedOk':      {fr:'enregistre',             en:'saved'},
  'settings.themeSaveFail':     {fr:"echec de l'enregistrement", en:'save failed'},
  'settings.themeAskName':      {fr:'Nom du nouveau theme',   en:'Name of the new theme'},
  'settings.themeCopyOf':       {fr:'Copie de',               en:'Copy of'},
  'settings.tokenReset':        {fr:"Revenir a la valeur d'origine", en:'Back to the original value'},
  'settings.brandTitle':        {fr:'Marque',                 en:'Brand'},
  'settings.brandHint':         {fr:"Le nom et le logo en haut a gauche. Ils suivent le theme: chaque theme porte les siens.", en:'The name and logo in the top left. They travel with the theme: each theme carries its own.'},
  'settings.brandName':         {fr:'Nom affiche',            en:'Displayed name'},
  'settings.brandLogo':         {fr:'Logo',                   en:'Logo'},
  'settings.brandPick':         {fr:'Choisir un fichier',     en:'Pick a file'},
  'settings.brandClear':        {fr:'Retirer',                en:'Remove'},
  'settings.brandSvgHint':      {fr:"SVG de preference: il herite de la couleur d'accent, donc il suit le theme. Un PNG garde la sienne. Le SVG est lave par le serveur avant d'etre enregistre.", en:'Prefer SVG: it inherits the accent colour, so it follows the theme. A PNG keeps its own. SVG is sanitised by the server before being stored.'},
  'settings.brandTooBig':       {fr:'Logo trop lourd (512 Ko maximum).', en:'Logo too heavy (512 KB max).'},
  'settings.iconTitle':         {fr:'Icones',                 en:'Icons'},
  'settings.iconHint':          {fr:"Remplacez une icone par un SVG a vous. Elle herite de la couleur d'accent, donc elle suit le theme. Douze emplacements, ceux que l'oeil reconnait.", en:'Replace an icon with your own SVG. It inherits the accent colour, so it follows the theme. Twelve slots, the ones the eye recognises.'},
  'settings.bgTitle':           {fr:"Image de fond",         en:'Background image'},
  'settings.bgHint':            {fr:"Une seule image, derriere tout, et vous choisissez les zones qui la laissent voir.", en:'One image, behind everything, and you choose which zones let it show.'},
  'settings.bgPick':            {fr:'Choisir une image',      en:'Pick an image'},
  'settings.bgClear':           {fr:'Retirer',                en:'Remove'},
  'settings.bgOpacity':         {fr:'Opacite',                en:'Opacity'},
  'settings.bgFit':             {fr:'Cadrage',                en:'Fit'},
  'settings.bgZones':           {fr:'Zones concernees',       en:'Zones affected'},
  'settings.bgTooBig':          {fr:'Image trop lourde (3 Mo maximum).', en:'Image too heavy (3 MB max).'},
  'settings.themeTitle':        {fr:'Thème de l’interface', en:'Interface theme'},
  'settings.themeHint':         {fr:'Le changement est immédiat. Survolez un thème pour le voir avant de choisir.', en:'Applied immediately. Hover a theme to see it before choosing.'},
  'settings.themeNew':          {fr:'Nouveau thème',        en:'New theme'},
  'settings.themeEditTitle':    {fr:'Couleurs du thème',    en:'Theme colours'},
  'settings.themeEditHint':     {fr:'Chaque couleur s’applique en direct. Enregistrez pour la garder.', en:'Every colour applies live. Save to keep it.'},
  'settings.themeName':         {fr:'Nom du thème',         en:'Theme name'},
  'settings.themeSave':         {fr:'Enregistrer le thème', en:'Save theme'},
  'settings.themeDelete':       {fr:'Supprimer',            en:'Delete'},
  'settings.themeReset':        {fr:'Revenir au thème LaRuche', en:'Back to the LaRuche theme'},
  'settings.themeSaved':        {fr:'Thème enregistré',     en:'Theme saved'},
  'settings.themeBuiltinHint':  {fr:'Ce thème est intégré. Modifiez une couleur et enregistrez : une copie sera créée, l’original reste intact.', en:'This theme is built in. Change a colour and save: a copy is created, the original stays untouched.'},
  'settings.themePreview':      {fr:'Aperçu',               en:'Preview'},
  'settings.qrTitle':           {fr:'Ouvrir sur un téléphone', en:'Open on a phone'},
  'settings.qrHint':            {fr:"Scanne le code, ou tape l'adresse. Le téléphone doit être sur le même réseau que cette machine.", en:'Scan the code, or type the address. The phone must be on the same network as this machine.'},
  'settings.qrNoLan':           {fr:'Aucune adresse réseau utilisable sur cette machine.', en:'No usable network address on this machine.'},
  'settings.qrNoLanHint':       {fr:"L'adresse locale ne veut rien dire pour un téléphone : il ouvrirait son propre navigateur sur lui-même.", en:'The local address means nothing to a phone: it would open its own browser on itself.'},
  'settings.lanTitre':          {fr:'Rendre la ruche joignable sur le réseau', en:'Make the hive reachable on the network'},
  'settings.lanHint':           {fr:'Sans cela, LaRuche n’écoute que sur cette machine : le code ci-dessus mène à une adresse qui ne répond pas.', en:'Without this, LaRuche only listens on this machine: the code above points at an address that does not answer.'},
  'settings.lanRedemarrage':    {fr:'Le changement prend effet au prochain démarrage de LaRuche : le port s’ouvre une fois, au lancement, et ne se déplace pas en cours de route.', en:'The change takes effect the next time LaRuche starts: the port opens once, at launch, and cannot move afterwards.'},
  'settings.lanEnv':            {fr:'Réglage imposé par la variable d’environnement LARUCHE_BIND_LAN. Retirez-la du lanceur pour reprendre la main ici.', en:'Setting forced by the LARUCHE_BIND_LAN environment variable. Remove it from the launcher to control it here.'},
  'settings.lanActifMaintenant':{fr:'Actif',                   en:'On'},
  'settings.lanInactifMaintenant':{fr:'Inactif',               en:'Off'},
  'settings.qrBindTitle':       {fr:"Le code ne marchera pas encore", en:'The code will not work yet'},
  'settings.qrBindWarn':        {fr:'La ruche n\'écoute que sur cette machine. Démarre-la avec LARUCHE_BIND_LAN=1, sinon elle s\'annonce sur le réseau sans y répondre.', en:'The hive only listens on this machine. Start it with LARUCHE_BIND_LAN=1, otherwise it announces itself on the network without answering.'},
  'settings.qrCopy':            {fr:"Copier l'adresse", en:'Copy the address'},
  'settings.qrCopied':          {fr:'Adresse copiée', en:'Address copied'},
  'settings.meshCodeTitle':      {fr:'Code du mesh',     en:'Mesh code'},
  'settings.hostLabel':          {fr:'Hôte',             en:'Host'},
  'settings.cronCount':          {fr:'cron(s)',          en:'cron(s)'},
  'settings.mcpManageHint':      {fr:"Les serveurs MCP s'ajoutent, se modifient et s'activent dans Capacites, qui gere aussi les serveurs distants.", en:'MCP servers are added, edited and enabled in Capabilities, which also handles remote servers.'},
  'settings.mcpManageBtn':       {fr:'Gerer dans Capacites', en:'Manage in Capabilities'},
  'settings.mcpLocal':           {fr:'local',  en:'local'},
  'settings.mcpRemote':          {fr:'distant', en:'remote'},
  'settings.mcpOn':              {fr:'actif',  en:'enabled'},
  'settings.mcpOff':             {fr:'inactif', en:'disabled'},
  'settings.tlTasks':            {fr:'Taches', en:'Tasks'},
  'settings.tlAllHint':          {fr:'Afficher toutes les fiches', en:'Show every card'},
  'settings.usageModelsTitle':   {fr:'Modeles par usage', en:'Models per usage'},
  'settings.usageModelsHint':    {fr:"Ces trois-la font tourner un LLM sans passer par un canal. Laisse par defaut pour suivre le modele actif.", en:'These three run an LLM without going through a channel. Leave on default to follow the active model.'},
  'settings.usageConsolidation': {fr:'Consolidation de la memoire', en:'Memory consolidation'},
  'settings.usageConsolidationHint': {fr:'Fusionne les items en masse : un modele local bon marche suffit.', en:'Bulk item merging: a cheap local model is enough.'},
  'settings.usageMemoryEnrich':  {fr:'@LaRuche dans la memoire', en:'@LaRuche in memory'},
  'settings.usageMemoryEnrichHint': {fr:'Le sous-agent qui enrichit un noeud. A defaut, le modele de revue.', en:'The subagent that enriches a node. Falls back to the review model.'},
  'settings.usageFeed':          {fr:'Demander a LaRuche (fil)', en:'Ask LaRuche (feed)'},
  'settings.usageFeedHint':      {fr:'La session dediee du fil, separee du chat principal.', en:"The feed's own session, separate from the main chat."},
  'settings.mcpDoorSaved':       {fr:'Porte MCP enregistree', en:'MCP door saved'},
  'settings.mcpDoorTitle':       {fr:'LaRuche comme serveur MCP', en:'LaRuche as an MCP server'},
  'settings.mcpDoorHint':        {fr:"Cette surface execute tout le registre d'outils, shell compris. Ferme par defaut.", en:'This surface executes the whole tool registry, shell included. Closed by default.'},
  'settings.mcpFirewall':        {fr:'Pare-feu par IP', en:'IP firewall'},
  'settings.mcpFirewallHint':    {fr:'Une adresse absente de la liste est refusee avant tout appel d outil.', en:'An address not on the list is refused before any tool is looked up.'},
  'settings.mcpAllowlist':       {fr:'Adresses autorisees', en:'Allowed addresses'},
  'settings.mcpAllowlistHint':   {fr:"Une par ligne. Adresse simple (192.168.1.10) ou bloc CIDR (192.168.1.0/24). Le mot localhost couvre 127.0.0.1 et ::1. Liste vide = personne.", en:'One per line. Plain address (192.168.1.10) or CIDR block (192.168.1.0/24). The word localhost covers 127.0.0.1 and ::1. Empty list means nobody.'},
  'settings.mcpBans':            {fr:'Adresses bannies', en:'Banned addresses'},
  'settings.mcpBansNone':        {fr:'Aucune. Cinq refus en une minute bannissent une adresse, et la duree double a chaque recidive.', en:'None. Five refusals in a minute ban an address, and the wait doubles on each repeat.'},
  'settings.mcpBanLeft':         {fr:'encore', en:'another'},
  'settings.mcpUnban':           {fr:'Liberer', en:'Lift'},
  'settings.mcpUnbanned':        {fr:'Ban leve', en:'Ban lifted'},
  'settings.mcpServersTitle':    {fr:'Serveurs MCP (Model Context Protocol)', en:'MCP Servers (Model Context Protocol)'},
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
  'settings.watcherTypeCommand': {fr:'Commande',        en:'Command'},
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
  'settings.pluginToast':              {fr:'Plugin ', en:'Plugin '},
  // ── New Settings sections (left vertical nav) ──────────────────────────
  'settings.navGeneral':       {fr:'Général',            en:'General'},
  'settings.navGeneration':    {fr:'Génération',         en:'Generation'},
  'settings.navModels':        {fr:'Modèles & Providers', en:'Models & Providers'},
  'settings.navVoice':         {fr:'Voix',               en:'Voice'},
  'settings.navReine':         {fr:'LaReine',            en:'LaReine'},
  'settings.navChannels':      {fr:'Canaux',             en:'Channels'},
  'settings.navSecrets':       {fr:'Secrets & Webhooks', en:'Secrets & Webhooks'},
  'settings.navNetwork':       {fr:'Réseau / Mesh',      en:'Network / Mesh'},
  'settings.navCapabilities':  {fr:'Capacités',          en:'Capabilities'},
  'settings.navProfile':       {fr:'Profil',             en:'Profile'},
  'settings.navAdmin':         {fr:'Admin',              en:'Admin'},
  'settings.navHelp':          {fr:'Aide',               en:'Help'},
  'help.aproposTitre':   {fr:'À propos de LaRuche',  en:'About LaRuche'},
  'help.aproposDesc':    {fr:'La version qui tourne sur cette machine, et le protocole qu\'elle parle.',
                          en:'The build running on this machine, and the protocol it speaks.'},
  'help.depot':          {fr:'Dépôt GitHub',         en:'GitHub repository'},
  'help.depotDesc':      {fr:'Le code source, les tickets et les releases.',
                          en:'Source code, issues and releases.'},
  'help.wiki':           {fr:'Wiki',                 en:'Wiki'},
  'help.wikiDesc':       {fr:'Comprendre comment LaRuche fonctionne, et comment la régler.',
                          en:'Understand how LaRuche works, and how to tune it.'},
  'help.soutien':        {fr:'Ça vous plaît ? Soutenez le projet',
                          en:'Like it? Support the project'},
  'help.soutienDesc':    {fr:'LaRuche est gratuite. Un café aide à la faire avancer.',
                          en:'LaRuche is free. A coffee helps it move forward.'},
  'help.majTitre':       {fr:'Mise à jour',          en:'Update'},
  'help.majDesc':        {fr:'Compare cette version avec la dernière release publiée sur GitHub.',
                          en:'Compares this build with the latest release published on GitHub.'},
  'help.majBouton':      {fr:'Vérifier les mises à jour', en:'Check for updates'},
  'help.majEnCours':     {fr:'Vérification...',      en:'Checking...'},
  'help.majAJour':       {fr:'À jour.',              en:'Up to date.'},
  'help.majDispo':       {fr:'Une version plus récente est disponible',
                          en:'A newer version is available'},
  'help.majAvance':      {fr:'Cette version est en avance sur la dernière release.',
                          en:'This build is ahead of the latest release.'},
  'help.majEchecNoeud':  {fr:'Le nœud a répondu {code} sur /api/maj. Recompilez-le si vous venez de mettre à jour les sources.',
                          en:'The node answered {code} on /api/maj. Rebuild it if you have just updated the sources.'},
  'help.majEchec':       {fr:"Impossible de joindre GitHub. Vérifiez la connexion, ou réessayez plus tard: l'API limite le nombre d'appels.",
                          en:'Could not reach GitHub. Check the connection, or try again later: the API rate-limits requests.'},
  'help.majVersionLocale':{fr:'Version installée',   en:'Installed version'},
  'help.majTelecharger': {fr:'Voir la release',      en:'View the release'},
  'help.ressourcesTitre':{fr:'Documentation et code', en:'Documentation and code'},
  'help.cafeBouton':     {fr:'Offrir un café (ou une bière)', en:'Buy a coffee (or a beer)'},
  'settings.profileAccount':   {fr:'Compte',             en:'Account'},
  'settings.profileAvatar':    {fr:'Photo',              en:'Photo'},
  'settings.profileChangePhoto':{fr:'Changer',           en:'Change'},
  'settings.profileName':      {fr:'Nom affiché',        en:'Display name'},
  'settings.profilePassword':  {fr:'Mot de passe',       en:'Password'},
  'settings.profilePwSet':     {fr:'Un mot de passe est défini.', en:'A password is set.'},
  'settings.profilePwNone':    {fr:'Aucun mot de passe : définis-en un pour pouvoir te reconnecter.', en:'No password yet: set one so you can log back in.'},
  'settings.profileNewPw':     {fr:'Nouveau mot de passe', en:'New password'},
  'settings.profilePwChanged': {fr:'Mot de passe mis à jour.', en:'Password updated.'},
  'settings.profile2fa':       {fr:'Double authentification (2FA)', en:'Two-factor (2FA)'},
  'settings.profile2faOn':     {fr:'2FA activée (TOTP).', en:'2FA enabled (TOTP).'},
  'settings.profile2faOff':    {fr:'2FA désactivée. (Enrôlement TOTP bientôt.)', en:'2FA disabled. (TOTP enrollment coming.)'},
  'settings.profileFiche':     {fr:'Ce que LaRuche sait de toi', en:'What LaRuche knows about you'},
  'settings.profileFicheDesc': {fr:'Injecté dans le contexte de LaRuche pour personnaliser ses réponses.', en:'Injected into LaRuche context to personalize its answers.'},
  'settings.profileSaved':     {fr:'Profil enregistré.',  en:'Profile saved.'},
  'settings.totpEnable':       {fr:'Activer la 2FA',      en:'Enable 2FA'},
  'settings.totpDisable':      {fr:'Désactiver',          en:'Disable'},
  'settings.totpScan':         {fr:'Scanne ce QR dans ton app d\'authentification (Google Authenticator, etc.), puis entre le code.', en:'Scan this QR in your authenticator app (Google Authenticator, etc.), then enter the code.'},
  'settings.totpVerify':       {fr:'Vérifier & activer',  en:'Verify & enable'},
  'settings.totpEnabled':      {fr:'2FA activée.',        en:'2FA enabled.'},
  'settings.totpDisabled':     {fr:'2FA désactivée.',     en:'2FA disabled.'},
  'settings.totpBadCode':      {fr:'Code invalide.',      en:'Invalid code.'},
  'settings.searchPlaceholder':{fr:'Rechercher un réglage...', en:'Search a setting...'},
  'settings.adminDesc':        {fr:'Gérer les comptes utilisateur de cette ruche.', en:'Manage the user accounts on this hive.'},
  'settings.adminNoUsers':     {fr:'Aucun compte.',      en:'No accounts.'},
  'settings.adminLoadError':   {fr:'Impossible de charger les comptes.', en:'Could not load the accounts.'},
  'settings.adminPromote':     {fr:'Promouvoir admin',   en:'Make admin'},
  'settings.adminDemote':      {fr:'Rétrograder',        en:'Demote'},
  'settings.adminYou':         {fr:'(toi)',              en:'(you)'},
  'settings.adminNoPw':        {fr:'sans mdp',           en:'no password'},
  'settings.adminConfirmDelete':{fr:'Supprimer le compte "{name}" ? Action irréversible.', en:'Delete the account "{name}"? This cannot be undone.'},
  'settings.adminDeleted':     {fr:'Compte supprimé.',   en:'Account deleted.'},
  'settings.searchNoResults':  {fr:'Aucun réglage ne correspond.', en:'No setting matches.'},
  'settings.language':         {fr:'Langue',             en:'Language'},
  'settings.languageHint':     {fr:'Recharge l\'interface dans la langue choisie.', en:'Reloads the interface in the chosen language.'},
  'settings.onboardingTitle':  {fr:'Onboarding',         en:'Onboarding'}
});

LaRuche.Settings = (function(){
  var currentTab = 'general';

  // ── New Settings sections (left vertical nav on desktop, scrollable bar on mobile) ──
  // Each entry: { id, i18n (label key), icon (inline SVG) }. The wiring (data-tab ->
  // loader) is preserved: switching a section still calls the matching load*().
  function _ic(path){ return '<svg class="settings-nav-ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'+path+'</svg>'; }
  var SECTIONS = [
    { id:'profile',      i18n:'settings.navProfile',      icon:_ic('<circle cx="12" cy="8" r="4"/><path d="M5 20a7 7 0 0 1 14 0"/>') },
    { id:'appearance',   i18n:'settings.navAppearance',   icon:_ic('<circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/><circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/><circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/><circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>') },
    { id:'general',      i18n:'settings.navGeneral',      icon:_ic('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>') },
    { id:'chat',         i18n:'settings.navChat',         icon:_ic('<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>') },
    { id:'generation',   i18n:'settings.navGeneration',   icon:_ic('<path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9z"/><path d="M19 14l.8 2.2L22 17l-2.2.8L19 20l-.8-2.2L16 17l2.2-.8z"/>') },
    { id:'providers',    i18n:'settings.navModels',       icon:_ic('<path d="M9.5 2a4.5 4.5 0 0 0-4.4 5.6A4.5 4.5 0 0 0 4 16.5 4.5 4.5 0 0 0 12 19V4.5A2.5 2.5 0 0 0 9.5 2z"/><path d="M14.5 2A2.5 2.5 0 0 0 12 4.5V19a4.5 4.5 0 0 0 8-2.5 4.5 4.5 0 0 0-1.1-8.9A4.5 4.5 0 0 0 14.5 2z"/>') },
    { id:'voice',        i18n:'settings.navVoice',        icon:_ic('<rect x="9" y="2" width="6" height="12" rx="3"/><path d="M5 10a7 7 0 0 0 14 0"/><line x1="12" y1="17" x2="12" y2="22"/><line x1="8" y1="22" x2="16" y2="22"/>') },
    { id:'reine',        i18n:'settings.navReine',        icon:_ic('<path d="M3 8l4 4 5-7 5 7 4-4v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>') },
    { id:'channels',     i18n:'settings.navChannels',     icon:_ic('<path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8z"/>') },
    { id:'secrets',      i18n:'settings.navSecrets',      icon:_ic('<rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>') },
    { id:'network',      i18n:'settings.navNetwork',      icon:_ic('<circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>') },
    { id:'capabilities', i18n:'settings.navCapabilities', icon:_ic('<path d="M19.4 13a2.4 2.4 0 0 1 0-4.8h.6V6a2 2 0 0 0-2-2h-2.2v-.6a2.4 2.4 0 0 0-4.8 0V4H8a2 2 0 0 0-2 2v2.2H5.4a2.4 2.4 0 0 0 0 4.8H6V17a2 2 0 0 0 2 2h2.2v.6a2.4 2.4 0 0 0 4.8 0V19H18a2 2 0 0 0 2-2v-4z"/>') },
    { id:'admin', i18n:'settings.navAdmin', adminOnly:true, icon:_ic('<path d="M12 2l8 4v5c0 5-3.4 8.5-8 10-4.6-1.5-8-5-8-10V6z"/><circle cx="12" cy="10" r="2.2"/><path d="M8.5 16a3.5 3.5 0 0 1 7 0"/>') },
    { id:'help', i18n:'settings.navHelp', icon:_ic('<circle cx="12" cy="12" r="10"/><path d="M9.1 9a3 3 0 0 1 5.8 1c0 2-3 2.5-3 4"/><line x1="12" y1="17" x2="12" y2="17.01"/>') }
  ];
  function _visibleSections(){
    var admin = !!(LaRuche.Auth && LaRuche.Auth.isAdmin && LaRuche.Auth.isAdmin());
    return SECTIONS.filter(function(s){ return !s.adminOnly || admin; });
  }
  // Legacy alias: old code/links that referenced 'mcp' now resolve to the Capabilities section.
  var _ALIASES = { mcp:'capabilities', models:'providers' };

  function _renderNav(){
    var bar = document.getElementById('settingsTabsBar');
    if(!bar) return;
    bar.innerHTML = _visibleSections().map(function(s){
      var on = (s.id===currentTab) ? ' active' : '';
      return '<button class="settings-tab-btn settings-nav-btn'+on+'" data-tab="'+s.id+'">'+s.icon+'<span class="settings-nav-label">'+LaRuche.i18n.t(s.i18n)+'</span></button>';
    }).join('');
  }

  function init() {
    _renderNav();
    var bar = document.getElementById('settingsTabsBar');
    if(bar){
      bar.classList.add('settings-nav-vertical');
      bar.addEventListener('click', function(e){
        var btn = e.target.closest('.settings-tab-btn');
        if(!btn) return;
        currentTab = btn.dataset.tab;
        document.querySelectorAll('#settingsTabsBar .settings-tab-btn').forEach(function(b){b.classList.toggle('active',b.dataset.tab===currentTab);});
        loadTab(currentTab);
      });
    }
    // Search box: lives above the content, filters visible rows/cards by label text.
    var layout = bar ? bar.closest('.settings-page-layout') : null;
    var host = document.getElementById('settingsContent');
    if(layout && host && !document.getElementById('settingsSearch')){
      var sw = document.createElement('div');
      sw.className = 'settings-search';
      sw.innerHTML = '<input type="search" id="settingsSearch" autocomplete="off" placeholder="'+LaRuche.i18n.t('settings.searchPlaceholder')+'">';
      layout.insertBefore(sw, host);
      sw.querySelector('input').addEventListener('input', function(){ _applySearch(this.value); });
    }
  }

  // Filter the rendered cards/rows by case-insensitive label text. Empty query
  // restores everything. Shows a "no results" hint when nothing matches.
  function _applySearch(q){
    var host = document.getElementById('settingsContent');
    if(!host) return;
    q = (q||'').trim().toLowerCase();
    var cards = host.querySelectorAll('.settings-card');
    var anyVisible = false;
    if(!q){
      host.querySelectorAll('.settings-row, .settings-card').forEach(function(n){ n.style.display=''; });
      anyVisible = true;
    } else {
      cards.forEach(function(card){
        var title = (card.querySelector('.settings-card-title, .card-title') || {}).textContent || '';
        var titleHit = title.toLowerCase().indexOf(q) !== -1;
        var rows = card.querySelectorAll('.settings-row');
        var cardHit = titleHit;
        if(rows.length){
          rows.forEach(function(r){
            var hit = titleHit || (r.textContent||'').toLowerCase().indexOf(q) !== -1;
            r.style.display = hit ? '' : 'none';
            if(hit) cardHit = true;
          });
        } else {
          cardHit = titleHit || (card.textContent||'').toLowerCase().indexOf(q) !== -1;
        }
        card.style.display = cardHit ? '' : 'none';
        if(cardHit) anyVisible = true;
      });
      // Cards living outside a .settings-card (free rows) are matched directly.
      host.querySelectorAll('.settings-tab-canvas > .settings-row').forEach(function(r){
        var hit = (r.textContent||'').toLowerCase().indexOf(q) !== -1;
        r.style.display = hit ? '' : 'none';
        if(hit) anyVisible = true;
      });
    }
    var hint = host.querySelector('.settings-no-results');
    if(!anyVisible){
      if(!hint){
        hint = document.createElement('div');
        hint.className = 'settings-no-results';
        hint.textContent = LaRuche.i18n.t('settings.searchNoResults');
        host.appendChild(hint);
      }
      hint.style.display = '';
    } else if(hint){ hint.style.display = 'none'; }
  }

  function _clearSearch(){
    var s = document.getElementById('settingsSearch');
    if(s) s.value = '';
  }

  function enter() { _renderNav(); loadTab(currentTab); }
  function leave() {}

  // Deep-link into a section from outside (welcome modal, CLI-printed links, agent).
  // Everything goes through the router so the address bar ends up showing
  // '#settings/providers' - the URL the CLI prints and a person can bookmark.
  // Returns false on an unknown or admin-only section, so the caller can fall back
  // instead of silently landing on the wrong tab.
  function ouvrirSection(id){
    id = _ALIASES[id] || id;
    if(!_visibleSections().some(function(s){ return s.id === id; })) return false;
    // Set before navigating: enter() paints `currentTab`, and leaving it stale would
    // flash the previous section for one frame.
    currentTab = id;
    LaRuche.Router.go('settings/' + id);
    return true;
  }
  // Called by the router with whatever followed 'settings/'. Repaints directly rather
  // than bouncing back through go(), which at this point is already inside this route.
  function deepLink(id){
    id = _ALIASES[id] || id;
    if(!_visibleSections().some(function(s){ return s.id === id; })) return;
    currentTab = id;
    _renderNav();
    loadTab(id);
  }

  /* ------------------------------------------------------------------------
     LE DOCK. Un onglet des reglages accroche sur le cote, pendant qu'on se sert
     du reste de l'application.

     Un seul a la fois, et c'est un choix, pas une limite technique: deux
     panneaux flottants se recouvrent, et surtout chaque onglet s'adresse a ses
     controles par identifiant. Deux exemplaires du meme onglet dans la page
     rendraient le second inerte sans le dire. Ouvrir un autre onglet remplace
     donc le contenu du dock, et la page des reglages affiche un renvoi quand
     l'onglet qu'elle devrait montrer est justement celui qui est accroche.
     ------------------------------------------------------------------------ */
  var _dockTab = null;

  function _dockTitre(tab){
    var s = _visibleSections().filter(function(x){ return x.id === tab; })[0];
    return s ? LaRuche.i18n.t(s.i18n || ('settings.nav' + tab)) : tab;
  }

  function dock(tab){
    tab = _ALIASES[tab] || tab;
    if(!_visibleSections().some(function(s){ return s.id === tab; })) return false;
    var d = document.getElementById('lrDock');
    if(!d){
      d = document.createElement('aside');
      d.id = 'lrDock';
      d.className = 'lr-dock';
      d.setAttribute('role', 'complementary');
      d.innerHTML =
        '<div class="lr-dock-poignee" id="lrDockPoignee"></div>'+
        '<div class="lr-dock-head">'+
          '<span class="lr-dock-titre" id="lrDockTitre"></span>'+
          '<button class="lr-dock-btn" id="lrDockPage" title="'+LaRuche.i18n.t('settings.dockVersPage')+'">&#8599;</button>'+
          '<button class="lr-dock-btn" id="lrDockClose" title="'+LaRuche.i18n.t('common.close')+'">&times;</button>'+
        '</div>'+
        '<div class="lr-dock-corps" id="lrDockCorps"></div>';
      document.body.appendChild(d);
      document.getElementById('lrDockClose').onclick = fermerDock;
      document.getElementById('lrDockPage').onclick = function(){
        var t = _dockTab; fermerDock(); ouvrirSection(t);
      };
      _brancherPoignee(d);
      document.body.classList.add('lr-dock-ouvert');
    }
    _dockTab = tab;
    document.getElementById('lrDockTitre').textContent = _dockTitre(tab);
    var corps = document.getElementById('lrDockCorps');
    corps.innerHTML = '';
    var el = document.createElement('div');
    el.className = 'settings-tab-canvas';
    corps.appendChild(el);
    _monterOnglet(tab, el);
    _majLargeurDock();
    // La page des reglages, si elle affiche le meme onglet, doit lacher la main.
    if(currentTab === tab && document.getElementById('settingsContent')) loadTab(tab);
    try { localStorage.setItem('laruche_dock', tab); } catch(e){}
    return true;
  }

  function fermerDock(){
    var d = document.getElementById('lrDock');
    if(d) d.parentNode.removeChild(d);
    document.body.classList.remove('lr-dock-ouvert');
    document.documentElement.style.removeProperty('--lr-dock-largeur');
    document.documentElement.style.removeProperty('--lr-dock-hauteur');
    var ancien = _dockTab;
    _dockTab = null;
    try { localStorage.removeItem('laruche_dock'); } catch(e){}
    // Rendre son contenu a la page des reglages si elle attendait dessus.
    if(ancien && currentTab === ancien && document.getElementById('settingsContent')) loadTab(ancien);
  }

  function _majLargeurDock(){
    var d = document.getElementById('lrDock');
    if(!d) return;
    var r = d.getBoundingClientRect();
    document.documentElement.style.setProperty('--lr-dock-largeur', Math.round(r.width) + 'px');
    document.documentElement.style.setProperty('--lr-dock-hauteur', Math.round(r.height) + 'px');
  }

  /* Redimensionnement a la poignee. `pointer` plutot que `mouse`: le meme code
     sert alors au doigt sur la feuille du bas, sans deuxieme jeu d'evenements. */
  function _brancherPoignee(d){
    var p = d.querySelector('.lr-dock-poignee');
    if(!p) return;
    p.addEventListener('pointerdown', function(e){
      e.preventDefault();
      p.setPointerCapture(e.pointerId);
      var vertical = window.innerWidth <= 720;
      function bouger(ev){
        if(vertical){
          var h = Math.min(window.innerHeight * 0.92, Math.max(160, window.innerHeight - ev.clientY));
          d.style.height = h + 'px';
        } else {
          var w = Math.min(window.innerWidth * 0.8, Math.max(280, window.innerWidth - ev.clientX));
          d.style.width = w + 'px';
        }
        _majLargeurDock();
      }
      function lacher(ev){
        p.releasePointerCapture(e.pointerId);
        document.removeEventListener('pointermove', bouger);
        document.removeEventListener('pointerup', lacher);
      }
      document.addEventListener('pointermove', bouger);
      document.addEventListener('pointerup', lacher);
    });
  }

  function loadTab(tab) {
    tab = _ALIASES[tab] || tab;
    currentTab = tab;
    var host = document.getElementById('settingsContent');
    if(!host) return;
    document.querySelectorAll('#settingsTabsBar .settings-tab-btn').forEach(function(b){b.classList.toggle('active',b.dataset.tab===tab);});
    _clearSearch();
    // Anti-race: give EACH load a fresh canvas. If a slow async loader finishes
    // AFTER the tab has changed, it writes into ITS old `el` (now detached
    // from the DOM) -> invisible. No more "General shows up when I clicked Provider".
    var el = document.createElement('div');
    el.className = 'settings-tab-canvas';
    host.innerHTML = '';
    // Un seul bouton, au-dessus du contenu: il vaut pour l'onglet affiche, quel
    // qu'il soit, plutot qu'un bouton par onglet a maintenir dans la barre.
    var barre = document.createElement('div');
    barre.style.cssText = 'display:flex;justify-content:flex-end;margin-bottom:6px';
    var bd = document.createElement('button');
    bd.className = 'cwd-btn';
    bd.style.cssText = 'opacity:1;font-size:12px;padding:5px 10px';
    bd.innerHTML = '&#11026; ' + LaRuche.i18n.t('settings.dockDetacher');
    bd.title = LaRuche.i18n.t('settings.dockAide');
    bd.onclick = function(){ dock(tab); };
    barre.appendChild(bd);
    host.appendChild(barre);
    host.appendChild(el);
    el.innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px">'+LaRuche.i18n.t('settings.loading')+'</div>';
    // Le dock affiche deja cet onglet: on ne le monte pas deux fois. Les panneaux
    // s'adressent a leurs controles par identifiant, et deux exemplaires du meme
    // onglet dans la page rendraient le second inerte, sans le dire.
    if(_dockTab === tab){
      el.innerHTML = '<div class="settings-card" style="color:var(--text-dim);font-size:12.5px">'+
        LaRuche.i18n.t('settings.dockAilleurs')+'</div>';
      return;
    }
    _monterOnglet(tab, el);
  }

  /* Monter un onglet dans le conteneur qu'on lui donne. C'est le seul endroit qui
     connait la correspondance onglet -> chargeur, et c'est ce qui rend le dock
     possible: la page des reglages et le dock lui passent simplement un `el`
     different. */
  function _monterOnglet(tab, el) {
    switch(tab) {
      case 'appearance': loadApparence(el); break;
      case 'profile': loadProfile(el); break;
      case 'general': loadGeneral(el); break;
      case 'chat': loadChat(el); break;
      case 'generation': loadGeneration(el); break;
      case 'providers': loadProviders(el); break;
      case 'voice': loadVoice(el); break;
      case 'reine': loadReine(el); break;
      case 'capabilities': loadMcp(el); break;
      case 'mcp': loadMcp(el); break;
      case 'secrets': loadSecrets(el); break;
      case 'tools': loadTools(el); break;
      case 'channels': loadChannels(el); break;
      case 'knowledge': loadKnowledge(el); break;
      case 'network': loadNetwork(el); break;
      case 'admin': loadAdmin(el); break;
      case 'cron': loadCron(el); break;
      case 'cron-timeline': loadCronTimeline(el); break;
      case 'blueprints': loadBlueprints(el); break;
      case 'watchers': loadWatchers(el); break;
      case 'kanban': loadKanban(el); break;
      case 'skills': loadSkills(el); break;
      case 'onboarding': loadOnboarding(el); break;
      case 'help': loadHelp(el); break;
    }
  }

  // Shared data fetch for the General/Generation/Voice/LaReine sections. These used to
  // be one monolithic "general" tab; the fetch is kept whole so each split section gets
  // the same data shape (gj = error-tolerant fetch). Returns a normalized bag.
  /* Six sections all call this, and it fetches TEN endpoints every time. Two of them
   * probe services over the network (`/api/doctor` reaches for Ollama, `/api/voice/status`
   * for STT and TTS), so with any of those down every tab switch waited on a timeout.
   * A short cache makes moving between tabs instant, which is what the panel should feel
   * like; 15 seconds is long enough to cover browsing the sections and short enough that
   * a value changed elsewhere shows up on the next visit. `_invalidateGeneral` drops it
   * after a save, so what you just wrote is never read back from a stale copy. */
  var _generalCache = null, _generalAt = 0, _generalInflight = null;
  var GENERAL_TTL_MS = 15000;
  function _invalidateGeneral(){ _generalCache = null; _generalInflight = null; }

  async function _loadGeneralData(force) {
    var now = Date.now();
    if(!force && _generalCache && (now - _generalAt) < GENERAL_TTL_MS) return _generalCache;
    // Sections rendered back to back must share ONE round of requests rather than
    // firing ten more each while the first is still in flight.
    if(!force && _generalInflight) return _generalInflight;
    _generalInflight = _loadGeneralDataFresh().then(function(d){
      _generalCache = d; _generalAt = Date.now(); _generalInflight = null;
      return d;
    }).catch(function(e){ _generalInflight = null; throw e; });
    return _generalInflight;
  }

  function _gj(u){ return fetch(u).then(function(r){return r.json();}).catch(function(){return {};}); }

  /* The ten endpoints are NOT equal. Eight read a local config file and answer instantly;
   * `/api/doctor` and `/api/voice/status` PROBE THE NETWORK (Ollama, the provider, STT,
   * TTS) with timeouts. Bundling them in one Promise.all made every section wait on the
   * probes - including Generation and LaReine, which never look at that data. Split, so a
   * tab waits only for what it actually renders. */
  function _loadConfigs() {
    return Promise.all([
      _gj('/api/config/provider'), _gj('/api/context/stats'), _gj('/api/config/compaction'),
      _gj('/api/config/curateur'), _gj('/api/config/runtime'), _gj('/api/config/reine'),
      _gj('/api/config/channel-models'), _gj('/api/config/voice'),
      _gj('/api/memory/episodes')
    ]).then(function(r){
      return {
        provCfg:r[0], ctxStats:r[1], ctxCfg:r[2], curCfg:r[3],
        rt:r[4]||{}, reineCfg:r[5]||{}, chmReine:r[6]||{options:[]}, voiceCfg:r[7]||{},
        epCfg:r[8]||{}
      };
    });
  }
  function _loadSondes() {
    return Promise.all([_gj('/api/doctor'), _gj('/api/voice/status')])
      .then(function(r){ return { doc:r[0], voice:r[1] }; });
  }

  async function _loadGeneralDataFresh() {
    var r = await Promise.all([_loadConfigs(), _loadSondes()]);
    return Object.assign({}, r[0], r[1]);
  }

  // ── Voice card (reused by the Voice section) ──────────────────────────
  function _voiceCardHtml(voice, voiceCfg){
    return '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.voice')+'</div>'+
      '<div class="settings-row"><span class="settings-label">STT</span><span style="color:'+(voice.stt&&voice.stt.available?'var(--green)':'var(--red)')+'">'+(voice.stt&&voice.stt.available?LaRuche.i18n.t('settings.statusOk'):LaRuche.i18n.t('settings.statusOff'))+'</span></div>'+
      '<div class="settings-row"><span class="settings-label">TTS</span><span style="color:'+(voice.tts&&voice.tts.available?'var(--green)':'var(--red)')+'">'+(voice.tts&&voice.tts.available?LaRuche.i18n.t('settings.statusOk'):LaRuche.i18n.t('settings.statusOff'))+'</span></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('settings.sttExternalHint')+'"><span class="settings-label">'+LaRuche.i18n.t('settings.sttExternal')+'</span><input type="checkbox" id="cfgSttExternal" onchange="LaRuche.Settings.saveVoiceCfg()"'+(voiceCfg.stt_external?' checked':'')+'></div>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:4px">'+LaRuche.i18n.t('settings.sttExternalNote')+'</div>'+
      '<div class="settings-row" style="margin-top:6px"><span class="settings-label" title="'+LaRuche.i18n.t('settings.ttsBackendHint')+'">'+LaRuche.i18n.t('settings.ttsBackend')+'</span><select id="cfgTtsBackend" class="form-input" style="width:130px;padding:2px 6px" onchange="LaRuche.Settings.saveVoiceCfg()">'+
      ['','kokoro','voicebox','edge-tts','voxtral','openai-tts'].map(function(b){ var lbl=b===''?LaRuche.i18n.t('settings.ttsBackendAuto'):b; return '<option value="'+b+'"'+((voiceCfg.tts_backend||'')===b?' selected':'')+'>'+lbl+'</option>'; }).join('')+'</select></div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.ttsSpeed')+'</span><span style="display:flex;align-items:center;gap:6px"><input type="range" id="cfgTtsSpeed" min="0.5" max="2" step="0.05" value="'+(voiceCfg.tts_speed||1)+'" oninput="document.getElementById(\'cfgTtsSpeedVal\').textContent=parseFloat(this.value).toFixed(2)+\'x\'"><span id="cfgTtsSpeedVal" style="font-size:11px;width:38px">'+(parseFloat(voiceCfg.tts_speed||1).toFixed(2))+'x</span></span></div>'+
      '<div class="settings-row"><span class="settings-label" title="'+LaRuche.i18n.t('settings.ttsVoiceHint')+'">'+LaRuche.i18n.t('settings.ttsVoice')+'</span><input type="text" id="cfgTtsVoice" class="form-input" style="width:120px;padding:2px 6px" value="'+LaRuche.Utils.esc(voiceCfg.tts_voice||'')+'" placeholder="ff_siwis"></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveVoiceCfg()" style="margin-top:8px">'+LaRuche.i18n.t('settings.save')+'</button></div>';
  }

  /* ── Render now, fill later ────────────────────────────────────────────────
   * A section used to await EVERY endpoint before painting a single pixel, so a tab
   * whose data comes from a network probe (/api/doctor reaches Ollama, the provider,
   * STT, TTS...) stayed on "Loading…" for as long as the slowest timeout. The layout
   * is drawn immediately and each slow card arrives in its own slot.
   * `el.isConnected` is the guard: loadTab hands every load a fresh canvas and detaches
   * the old one, so a late answer for a tab you already left writes nowhere. */
  function _slot(id){
    return '<div class="settings-slot" id="'+id+'"><span class="settings-spin"></span>'+
           LaRuche.i18n.t('settings.loading')+'</div>';
  }
  function _fillSlot(el, id, promesse, rendu){
    promesse.then(function(d){
      if(!el.isConnected) return;              // tab changed while we waited
      var slot = el.querySelector('#'+id);
      if(slot) slot.outerHTML = rendu(d);
    }).catch(function(){
      if(!el.isConnected) return;
      var slot = el.querySelector('#'+id);
      if(slot) slot.innerHTML = '<span style="color:var(--text-muted)">'+LaRuche.i18n.t('settings.errorGeneric')+'</span>';
    });
  }

  function _onboardingHtml(onboarding){
    return ((onboarding.steps||[]).map(function(s){
      // An unmet OPTIONAL step is not a failure: amber circle, not a red cross. The
      // instruction is shown in both states, because "why is this red" was the question
      // people actually had, and the answer was only ever in the unmet branch.
      var icon = s.done
        ? '<span style="color:var(--green);margin-right:8px"><svg width="1.1em" height="1.1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:middle"><polyline points="20 6 9 17 4 12"></polyline></svg></span>'
        : (s.optional ? '<span style="color:var(--amber);margin-right:8px" title="'+LaRuche.i18n.t('settings.onbOptional')+'">&#x25CB;</span>'
                      : '<span style="color:var(--red);margin-right:8px">&#x2717;</span>');
      return '<div class="settings-row" style="align-items:flex-start"><span class="settings-label">'+icon+LaRuche.Utils.esc(s.title)+
        (s.instruction ? '<span style="display:block;font-size:10px;color:var(--text-muted);font-weight:400;margin:1px 0 0 22px;line-height:1.4">'+LaRuche.Utils.esc(s.instruction)+'</span>' : '')+
        '</span></div>';
    }).join('') || '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.statusLabel')+'</span><span class="settings-value">'+LaRuche.i18n.t('settings.statusOkValue')+'</span></div>')
      // Same list, richer form: the welcome modal adds a jump button per unmet step
      // and the "ask the agent" route. Reachable here because the modal only opens
      // by itself once, and someone who dismissed it still needs a way back in.
      + '<div class="settings-row"><span class="settings-label"></span>'+
        '<button class="form-btn" onclick="LaRuche.Accueil.ouvrir()">'+LaRuche.i18n.t('accueil.reopen')+'</button></div>';
  }

  /* Rendre sa vue au modele actif, sans attendre le repit ni redemarrer. */
  function visionReessayer(){
    fetch(LaRuche.API.base+'/api/vision/reset', {
      method:'POST', headers:{'Content-Type':'application/json'}, body:'{}'
    })
      .then(function(r){ return r.json().catch(function(){ return {}; }); })
      .then(function(d){
        LaRuche.Toast.show(
          (d && d.reactive) ? LaRuche.i18n.t('settings.visionRendue') : LaRuche.i18n.t('settings.visionDejaOk'),
          'ok');
        // Le panneau se relit: le verdict doit changer sous les yeux, sinon on
        // ne sait pas si le clic a servi.
        var el = document.getElementById('settingsContent');
        if(el && LaRuche.Settings.loadGeneral) LaRuche.Settings.loadGeneral(el);
      })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'), 'err'); });
  }

  function _securiteHtml(doc){
    return '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.protocol')+'</span><span class="settings-value">Miel v'+(doc.version||'0.2.0')+'</span></div>'+
      ((doc.checks||[]).map(function(c){
        // /api/doctor already explains every verdict in `detail` ("Cannot reach
        // http://.../v1/models"). Showing only the word left "OpenAI-compatible / error"
        // unreadable. And a warning is not a failure: it gets amber, not red.
        var couleur = c.status==='ok' ? 'var(--green)' : (c.status==='warning' ? 'var(--amber)' : 'var(--red)');
        return '<div class="settings-row" style="align-items:flex-start">'+
          '<span class="settings-label">'+LaRuche.Utils.esc(c.name)+
            (c.detail ? '<span style="display:block;font-size:10px;color:var(--text-muted);font-weight:400;margin-top:1px">'+LaRuche.Utils.esc(c.detail)+'</span>' : '')+
          '</span>'+
          // Un verdict qui se repare d'un clic porte le bouton qui le repare.
          // Constater qu'un modele est ecarte sans pouvoir rien y faire ne vaut
          // guere mieux que de ne pas le savoir.
          (c.action==='vision_reset'
            ? '<span style="display:flex;gap:8px;align-items:center;white-space:nowrap">'+
              '<button class="tl-btn" onclick="LaRuche.Settings.visionReessayer()">'+LaRuche.i18n.t('settings.visionReessayer')+'</button>'+
              '<span style="color:'+couleur+'">'+LaRuche.Utils.esc(c.status)+'</span></span>'
            : '<span style="color:'+couleur+';white-space:nowrap">'+LaRuche.Utils.esc(c.status)+'</span>')+
          '</div>';
      }).join('')||'<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.statusLabel')+'</span><span class="settings-value">'+LaRuche.i18n.t('settings.statusOkValue')+'</span></div>');
  }

  // ── GENERAL section: language, transparency, security/system, onboarding ──
  // Nothing is awaited before painting. The language card needs no data at all, and the
  // two probe-backed cards fill themselves in parallel.
  function loadGeneral(el) {
    var curLang = (LaRuche.i18n && LaRuche.i18n.get) ? LaRuche.i18n.get() : 'fr';
    el.innerHTML = '<div class="settings-grid">'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.navGeneral')+'</div>'+
      '<div class="settings-card-desc">'+LaRuche.i18n.t('settings.languageHint')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.language')+'</span><select id="cfgLang" class="form-input" style="width:120px;padding:2px 6px" onchange="LaRuche.i18n.setLang(this.value)">'+
        '<option value="fr"'+(curLang==='fr'?' selected':'')+'>Français</option>'+
        '<option value="en"'+(curLang==='en'?' selected':'')+'>English</option>'+
      '</select></div>'+
      '</div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.security')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.secretsTitle')+'</span><span class="settings-value">'+LaRuche.i18n.t('settings.secretsCount')+'</span></div>'+
      _slot('genSecurite')+
      '</div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.onboardingTitle')+
        '<span id="genOnbProgress"></span></div>'+
      _slot('genOnboarding')+
      '</div>'+
      '</div>';

    _fillSlot(el, 'genSecurite', _loadSondes().then(function(d){ return d.doc||{}; }), _securiteHtml);
    _fillSlot(el, 'genOnboarding',
      fetch(LaRuche.API.base+'/api/onboarding').then(function(r){return r.json();}),
      function(onb){
        var pastille = el.querySelector('#genOnbProgress');
        if(pastille) pastille.outerHTML = ' <span style="margin-left:8px;padding:1px 8px;border-radius:10px;font-size:10px;background:'+(onb.complete?'var(--green)':'var(--amber)')+';color:#000">'+LaRuche.Utils.esc(onb.progress||'')+'</span>';
        return _onboardingHtml(onb);
      });
  }

  /* ── CHAT section: what shows up in the thread ────────────────────────────
   * Both settings govern the conversation surface, and both used to sit in sections
   * about something else - transparency under General (a system panel) and agent
   * reactions under Generation (model parameters), where nobody looked for them. */
  async function loadChat(el) {
    var transpOn = window.localStorage.getItem('laruche_hide_transparency') !== 'true';
    el.innerHTML = '<div class="settings-grid">'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.navChat')+'</div>'+
      '<div class="settings-card-desc">'+LaRuche.i18n.t('settings.chatSectionHint')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.showTransparency')+'</span><label class="lr-switch"><input type="checkbox" id="cfgTransparence"'+(transpOn?' checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<div class="settings-card-desc">'+LaRuche.i18n.t('settings.showTransparencyHint')+'</div>'+
      _slot('chatReactions')+
      '</div></div>';
    // Wired here rather than inline: this one is browser-local, it never touches the
    // server, so it takes effect the moment it is clicked.
    var tr = el.querySelector('#cfgTransparence');
    if(tr) tr.onchange = function(){
      window.localStorage.setItem('laruche_hide_transparency', this.checked ? 'false' : 'true');
    };
    // Server-side setting: paint the local one first, fill this in when it lands.
    _fillSlot(el, 'chatReactions', _loadConfigs().then(function(d){ return d.rt||{}; }), function(rt){
      return '<div class="settings-row" style="margin-top:6px"><span class="settings-label" title="'+LaRuche.Utils.esc(LaRuche.i18n.t('settings.agentReactionsTitle'))+'">'+LaRuche.i18n.t('settings.agentReactionsLabel')+'</span>'+
        '<label class="lr-switch"><input type="checkbox" id="cfgAgentReactions"'+(rt.reactions_agent?' checked':'')+' onchange="LaRuche.Settings.saveChatCfg()"><span class="lr-slider"></span></label></div>'+
        '<div class="settings-card-desc">'+LaRuche.i18n.t('settings.agentReactionsHint')+'</div>';
    });
  }
  /* Saves ONLY `reactions_agent`. The runtime endpoint merges, so the generation
   * parameters this form does not show are left exactly as they are. */
  function saveChatCfg(){
    var on = !!(document.getElementById('cfgAgentReactions')||{}).checked;
    _invalidateGeneral();
    fetch(LaRuche.API.base+'/api/config/runtime',{method:'POST',credentials:'include',
      headers:{'Content-Type':'application/json'},body:JSON.stringify({reactions_agent:on})})
      .then(function(r){ LaRuche.Toast.show(LaRuche.i18n.t(r.ok?'settings.save':'settings.errorGeneric'), r.ok?'ok':'err'); })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorGeneric'),'err'); });
  }

  // ── GENERATION section: generation params (+ advanced), context/compaction, curateur ──
  async function loadGeneration(el) {
    // Config only: this section renders nothing that comes from a network probe.
    var data = await _loadConfigs();
    var rt = data.rt, ctxCfg = data.ctxCfg, curCfg = data.curCfg, provCfg = data.provCfg || {};
    var epCfg = data.epCfg || {};
    var codexRuntimeHint = provCfg.provider === 'codex'
      ? '<div class="provider-runtime-note">'+LaRuche.i18n.t('settings.codexRuntimeHint')+'</div>'
      : '';
    el.innerHTML = '<div class="settings-grid">'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.generationTitle')+'</div>'+
      '<div class="settings-row" style="flex-direction:column;align-items:stretch;gap:4px;">'+
      '<div class="settings-row" style="padding:0;"><span class="settings-label" title="'+LaRuche.i18n.t('settings.maxPassesTitle')+'">'+LaRuche.i18n.t('settings.maxPassesLabel')+'</span><input type="number" id="cfgMaxIter" class="form-input" style="width:80px;padding:2px 6px;" value="'+(rt.max_iterations||40)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.temperature')+'</span><input type="number" id="cfgTemp" class="form-input" style="width:80px;padding:2px 6px;" step="0.05" min="0" max="2" value="'+(rt.temperature!=null?rt.temperature:0.7)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.maxTokensOut')+'</span><input type="number" id="cfgMaxTok" class="form-input" style="width:90px;padding:2px 6px;" value="'+(rt.max_tokens||4096)+'"></div>'+
      codexRuntimeHint+
      '<details class="settings-advanced" style="margin-top:6px;"><summary style="cursor:pointer;font-size:11px;color:var(--text-dim);user-select:none;">'+LaRuche.i18n.t('settings.advancedSection')+'</summary>'+
      '<div class="settings-row" style="padding:0;margin-top:6px;"><span class="settings-label" title="'+LaRuche.i18n.t('settings.dynToolsLimit')+'">'+LaRuche.i18n.t('settings.dynToolsLimitLabel')+'</span><input type="number" id="cfgToolLim" class="form-input" style="width:80px;padding:2px 6px;" value="'+(rt.tool_selection_limit||24)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label" title="'+LaRuche.i18n.t('settings.narrowCtxThreshold')+'">'+LaRuche.i18n.t('settings.narrowCtxLabel')+'</span><input type="number" id="cfgCtxThreshold" class="form-input" style="width:90px;padding:2px 6px;" value="'+(rt.dynamic_context_threshold||40000)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label" title="'+LaRuche.i18n.t('settings.mixtureModelsHint')+'">'+LaRuche.i18n.t('settings.mixtureModels')+'</span><input type="text" id="cfgProvFallback" class="form-input" style="width:180px;padding:2px 6px;" value="'+LaRuche.Utils.esc(provCfg.fallback_models||'')+'" placeholder="model-a, model-b"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label" title="'+LaRuche.i18n.t('settings.memoryReviewHint')+'">'+LaRuche.i18n.t('settings.memoryReviewModel')+'</span><input type="text" id="cfgProvReview" class="form-input" style="width:180px;padding:2px 6px;" value="'+LaRuche.Utils.esc(provCfg.review_model||'')+'" placeholder="'+LaRuche.i18n.t('settings.optional')+'"></div>'+
      '</details>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveRuntimeCfg()" style="margin-top:8px;">'+LaRuche.i18n.t('settings.apply')+'</button></div></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.contextCompaction')+'</div>'+
      '<div class="settings-row" style="flex-direction:column;align-items:stretch;gap:4px;">'+
      '<div class="settings-row" style="padding:0;"><span class="settings-label">'+LaRuche.i18n.t('settings.maxMessages')+'</span><input type="number" id="cfgCtxMax" class="form-input" style="width:80px;padding:2px 6px;" value="'+(ctxCfg.context_max_messages||50)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">'+LaRuche.i18n.t('settings.compactionThreshold')+'</span><input type="number" id="cfgCtxThresh" class="form-input" style="width:80px;padding:2px 6px;" step="0.05" value="'+(ctxCfg.compaction_threshold||0.75)+'"></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveContextCfg()" style="margin-top:8px;">'+LaRuche.i18n.t('settings.save')+'</button></div></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.curateur')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.autoSkillCreate')+'</span><label class="lr-switch"><input type="checkbox" id="cfgCurateur" '+(curCfg.enabled?'checked':'')+' '+(curCfg.env_forced?'disabled':'')+' onchange="LaRuche.Settings.toggleCurateur(this.checked)"><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.halo')+'<span style="color:var(--text-dim);font-size:10px;display:block;max-width:340px;line-height:1.4">'+LaRuche.i18n.t('settings.haloHint')+'</span></span><label class="lr-switch"><input type="checkbox" id="cfgHalo" '+(curCfg.halo_actif!==false?'checked':'')+' onchange="LaRuche.Settings.toggleHalo(this.checked)"><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.dynToolsSelect')+'<span style="color:var(--text-dim);font-size:10px">'+LaRuche.i18n.t('settings.dynToolsHint')+'</span></span><label class="lr-switch"><input type="checkbox" id="cfgDynTools" '+(curCfg.dynamic_tools?'checked':'')+' onchange="LaRuche.Settings.toggleDynamicTools(this.checked)"><span class="lr-slider"></span></label></div>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:6px">'+(curCfg.env_forced?LaRuche.i18n.t('settings.curEnvForced'):LaRuche.i18n.t('settings.curDefault'))+'</div></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.episodes')+'</div>'+
      '<div style="font-size:11px;color:var(--text-dim);line-height:1.5;margin-bottom:8px">'+LaRuche.i18n.t('settings.episodesWhat')+'</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.episodesRetention')+'</span>'+
      '<select id="cfgEpisodes" class="form-input" style="width:150px;padding:2px 6px;" onchange="LaRuche.Settings.saveEpisodesCfg(this.value)">'+
      _episodesOptions(epCfg.retention_days||0)+'</select></div>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:4px">'+_episodesEtat(epCfg)+'</div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.clearEpisodes()" style="margin-top:8px;border-color:var(--red);color:var(--red)">'+LaRuche.i18n.t('settings.episodesClear')+'</button>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:6px">'+LaRuche.i18n.t('settings.episodesClearHint')+'</div></div>'+
      '</div>';
  }

  // Les paliers proposes. Pas de champ libre: le reglage se lit en un coup d'oeil
  // et personne n'a besoin de "43 jours".
  function _episodesOptions(actuel){
    var paliers = [[0,'settings.episodesKeepAll'],[7,'settings.episodes7'],[30,'settings.episodes30'],
                   [90,'settings.episodes90'],[180,'settings.episodes180'],[365,'settings.episodes365']];
    return paliers.map(function(p){
      return '<option value="'+p[0]+'"'+(Number(actuel)===p[0]?' selected':'')+'>'+LaRuche.i18n.t(p[1])+'</option>';
    }).join('');
  }

  // Ce qu'il y a reellement en memoire, pour que le reglage ne soit pas abstrait.
  function _episodesEtat(ep){
    if(!ep || !ep.days) return LaRuche.i18n.t('settings.episodesNone');
    var s = LaRuche.i18n.t('settings.episodesCount', { n: ep.days });
    if(ep.oldest) s += ' · ' + LaRuche.i18n.t('settings.episodesSince', { d: ep.oldest });
    return s;
  }

  // ── VOICE section ──────────────────────────────────────────────────────
  async function loadVoice(el) {
    // The card needs `voice`, which is a live probe of the STT/TTS services. Paint from
    // the config immediately and let the two status lines arrive on their own.
    var data = await _loadConfigs();
    el.innerHTML = '<div class="settings-grid">'+_voiceCardHtml({}, data.voiceCfg)+'</div>';
    _loadSondes().then(function(p){
      if(!el.isConnected) return;
      el.innerHTML = '<div class="settings-grid">'+_voiceCardHtml(p.voice||{}, data.voiceCfg)+'</div>';
    });
  }

  // ── LAREINE section (supervisor config) ────────────────────────────────
  async function loadReine(el) {
    // Config only: this section renders nothing that comes from a network probe.
    var data = await _loadConfigs();
    var reineCfg = data.reineCfg, chmReine = data.chmReine;
    var reineProvOpts = '<option value="">'+LaRuche.i18n.t('reine.providerSame')+'</option>';
    (chmReine.options||[]).forEach(function(o){
      var rpVal = o.profile_id+'|||'+o.model;
      var rpSel = (reineCfg.provider_profile===rpVal) ? ' selected' : '';
      reineProvOpts += '<option value="'+LaRuche.Utils.esc(rpVal)+'"'+rpSel+'>'+LaRuche.Utils.esc((o.name||o.provider)+' / '+o.model)+'</option>';
    });
    var reineUnlim = (reineCfg.max_revues===255);
    var reineMaxVal = reineUnlim ? 10 : (reineCfg.max_revues||0);
    el.innerHTML = '<div class="settings-grid">'+
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
      '<div style="font-size:10px;color:var(--text-dim);margin:2px 0 6px">'+LaRuche.i18n.t('reine.judgeDistinctHint')+'</div>'+
      // Every review already produces a request, a refused draft, an accepted one and a
      // reason. Off by default because switching it on starts keeping the full text.
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('reine.dataset')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineDataset" '+(reineCfg.dataset?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<div class="settings-card-desc">'+LaRuche.i18n.t('reine.datasetHint')+'</div>'+
      '<div style="display:flex;gap:6px;flex-wrap:wrap;margin:2px 0 8px">'+
        '<button class="tl-btn" onclick="LaRuche.Settings.reineDataset(\'dpo\')">'+LaRuche.i18n.t('reine.datasetDpo')+'</button>'+
        '<button class="tl-btn" onclick="LaRuche.Settings.reineDataset(\'sft\')">'+LaRuche.i18n.t('reine.datasetSft')+'</button>'+
        '<button class="tl-btn" onclick="LaRuche.Settings.reineDataset(\'judge\')">'+LaRuche.i18n.t('reine.datasetJudge')+'</button>'+
      '</div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('reine.tier1')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineTier1" '+(reineCfg.tier_reponse?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('reine.tier2')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineTier2" '+(reineCfg.tier_artefacts?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('reine.tier3Warn')+'"><span class="settings-label">'+LaRuche.i18n.t('reine.tier3')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineTier3" '+(reineCfg.tier_supervision?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row" title="'+LaRuche.i18n.t('reine.queueGateHint')+'"><span class="settings-label">'+LaRuche.i18n.t('reine.queueGate')+'</span><label class="lr-switch"><input type="checkbox" id="cfgReineQueue" '+(reineCfg.queue_gate?'checked':'')+'><span class="lr-slider"></span></label></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveReineCfg()" style="margin-top:8px;">'+LaRuche.i18n.t('settings.save')+'</button>'+
      '<div style="margin-top:10px;border-top:1px solid rgba(245,158,11,.2);padding-top:8px">'+
      '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('reine.queueTitle')+'</span><span style="font-size:10px;color:var(--text-dim);text-align:right">'+LaRuche.i18n.t('reine.queueInMemory')+'</span></div>'+
      '</div>'+
      '<div id="reineScorecards" style="margin-top:10px;border-top:1px solid rgba(245,158,11,.2);padding-top:8px"></div>'+
      '</div>'+
      '</div>';
    renderReineScorecards();
  }

  // Scorecard dashboard: aggregates of every completed review (the JSONL journal
  // finally has a face). Silent when no review has run yet.
  function renderReineScorecards(){
    fetch(LaRuche.API.base+'/api/reine/scorecards').then(function(r){return r.json();}).then(function(d){
      var host=document.getElementById('reineScorecards');
      if(!host) return;
      var t=LaRuche.i18n.t;
      if(!d || !d.total){ host.innerHTML='<div style="font-size:10px;color:var(--text-dim)">'+t('reine.scoreAucune')+'</div>'; return; }
      var pct=function(n){ return Math.round(n*100/d.total)+'%'; };
      var barre=function(label,val){
        return '<div style="display:flex;align-items:center;gap:6px;font-size:10px;margin:2px 0">'+
          '<span style="width:86px;color:var(--text-dim)">'+label+'</span>'+
          '<div style="flex:1;height:6px;background:rgba(255,255,255,.06);border-radius:3px;overflow:hidden"><div style="width:'+Math.min(100,val)+'%;height:100%;background:var(--amber)"></div></div>'+
          '<span style="width:30px;text-align:right;color:var(--text)">'+val+'</span></div>';
      };
      host.innerHTML=
        '<div class="settings-card-title" style="margin-bottom:4px">👑 '+t('reine.scoreTitre')+'</div>'+
        '<div style="display:flex;gap:12px;font-size:11px;margin-bottom:6px">'+
          '<span>'+d.total+' '+t('reine.scoreRevues')+'</span>'+
          '<span style="color:var(--green)">✓ '+pct(d.approve)+'</span>'+
          '<span style="color:var(--amber)">↻ '+pct(d.revise)+'</span>'+
          '<span style="color:var(--red)">⚑ '+pct(d.escalate)+'</span>'+
          '<span style="color:var(--text-dim)">'+t('reine.scoreRefaits').replace('{n}', d.revised)+'</span>'+
        '</div>'+
        barre(t('reine.scorePertinence'), d.avg.relevance)+
        barre(t('reine.scoreMethodo'), d.avg.methodology)+
        barre(t('reine.scoreObjectif'), d.avg.objective)+
        barre(t('reine.scoreMarque'), d.avg.brand)+
        barre(t('reine.scoreConfiance'), d.avg.confidence)+
        '<div style="font-size:9px;color:var(--text-dim);margin-top:4px">'+t('reine.scoreTours').replace('{n}', d.avg.rounds)+'</div>';
    }).catch(function(){});
  }

  // ── Providers Tab ─────────────────────────────────────────────

  function _providerFaviconUrl(baseUrl) {
    try {
      var parsed = new URL(baseUrl || '', window.location.origin);
      if(parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return '';
      return parsed.origin + '/favicon.ico';
    } catch(e) { return ''; }
  }

  function _providerIconHtml(p) {
    p = p || {};
    var provider = (p.provider || '').toLowerCase();
    var fallback = provider === 'codex' ? 'AI' : provider === 'anthropic' ? 'A' : provider === 'ollama' ? 'O' : 'LL';
    var favicon = _providerFaviconUrl(p.base_url);
    return '<span class="provider-favicon" aria-hidden="true">'+
      '<span class="provider-favicon-fallback">'+fallback+'</span>'+
      (favicon ? '<img src="'+LaRuche.Utils.esc(favicon)+'" alt="" loading="lazy" referrerpolicy="no-referrer" onerror="this.remove()">' : '')+
      '</span>';
  }

  async function loadProviders(el) {
    // Status refresh also repairs/updates the built-in Codex profile server-side.
    var codexStatus = {};
    try { codexStatus = await fetch('/api/auth/codex/status').then(function(r){return r.json();}); } catch(e) {}
    var data = {};
    try { data = await fetch('/api/profiles').then(function(r){return r.json();}); } catch(e) {}
    var profiles = data.profiles || {};
    var active = data.active_model || {};
    var ids = Object.keys(profiles).sort();

    var credsData = {};
    try { credsData = await fetch('/api/credentials').then(function(r){return r.json();}); } catch(e) {}
    var allCreds = credsData.credentials || [];

    var html = '<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="LaRuche.Settings.showProfileForm()">'+LaRuche.i18n.t('settings.addProvider')+'</button></div>';
    html += '<div id="profileFormContainer" style="display:none"></div>';

    // (MCP servers now have their own "MCP" tab, see loadMcp.)

    html += '<div class="settings-grid">';
    var sharedHtml = '';
    var hasCodexProfile = ids.some(function(id){ return profiles[id] && profiles[id].provider === 'codex'; });
    if(!hasCodexProfile) {
      html += '<div class="settings-card codex-provider-card" style="border:1px solid var(--amber)">'+
        '<div class="settings-card-title provider-card-title">'+_providerIconHtml({provider:'codex',base_url:'https://chatgpt.com'})+
        '<span>ChatGPT Codex <span class="provider-subtitle">'+LaRuche.i18n.t('settings.codexSubscription')+'</span></span></div>'+
        '<div id="codexAuthBox" class="codex-auth-inline">'+LaRuche.i18n.t('settings.codexLoading')+'</div></div>';
    }

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
          '<div class="settings-card-title provider-card-title" style="flex-wrap:wrap">'+_providerIconHtml(p)+'<span>'+LaRuche.Utils.esc(p.name)+'</span>'+
          '<span style="color:var(--cyan);font-size:10px;font-weight:normal">'+LaRuche.i18n.t('settings.sharedReadOnly')+'</span></div>'+
          '<div class="settings-row"><span class="settings-label">URL</span><span class="settings-value" style="font-size:10px;word-break:break-all">'+LaRuche.Utils.esc(p.base_url)+'</span></div>'+
          '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.modelsLabel')+'</span><span class="settings-value">'+modelCount+'</span></div>'+
          '<div style="margin-top:10px"><button onclick="LaRuche.Settings.deleteProfile(\''+id+'\')" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.removeFromList')+'</button></div>'+
          '</div>';
        return; // no normal card: neither "Make Public" nor "Edit"
      }

      var pCreds = allCreds.filter(function(c){ return c.provider.toLowerCase() === p.provider.toLowerCase(); });
      var credsHtml = '';
      if(p.provider !== 'codex' && pCreds.length > 0) {
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
      var addCredBtn = p.provider === 'codex' ? '' : '<button onclick="LaRuche.Settings.addCredential(\''+p.provider+'\')" style="margin-top:8px;background:none;border:1px dashed var(--border);color:var(--text-dim);border-radius:4px;padding:4px 10px;cursor:pointer;font-size:10px;width:100%">'+LaRuche.i18n.t('settings.addCredKey')+'</button>';
      var codexAuthHtml = p.provider === 'codex'
        ? '<div id="codexAuthBox" class="codex-auth-inline">'+LaRuche.i18n.t('settings.codexLoading')+'</div>'
        : '';
      var apiKeyHtml = p.provider === 'codex' ? '' :
        '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.apiKeyLabel')+'</span><span class="settings-value">'+(p.api_key?LaRuche.i18n.t('settings.apiKeySet'):LaRuche.i18n.t('settings.apiKeyNone'))+'</span></div>';

      var _vis = p.visibility || 'prive';
      var _nAllowed = (p.allowed_peers||[]).length;
      var visBadge = _vis==='public_proxy'
        ? '<span style="color:var(--blue);font-size:10px;font-weight:bold;margin-left:8px;">'+LaRuche.i18n.t('settings.visPublic')+'</span>'
        : _vis==='restricted'
        ? '<span style="color:var(--cyan);font-size:10px;font-weight:bold;margin-left:8px;">'+LaRuche.i18n.t('settings.visRestrictedN')+' ('+_nAllowed+')</span>'
        : '<span style="color:var(--text-dim);font-size:10px;font-weight:bold;margin-left:8px;">'+LaRuche.i18n.t('settings.visPrivate')+'</span>';
      var visToggleBtn = '<button onclick="LaRuche.Settings.openAccess(\''+id+'\', \''+_vis+'\', \''+encodeURIComponent(JSON.stringify(p.allowed_peers||[]))+'\')" style="margin-left:auto;background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;">'+LaRuche.i18n.t('settings.accessBtn')+'</button>';
      html += '<div class="settings-card '+(p.provider==='codex'?'codex-provider-card':'')+'" style="'+(isActive?'border:1px solid var(--amber);':'')+'">'+
        '<div class="settings-card-title provider-card-title">'+_providerIconHtml(p)+'<span>'+LaRuche.Utils.esc(p.name)+
        (p.provider==='codex'?' <span class="provider-subtitle">'+LaRuche.i18n.t('settings.codexSubscription')+'</span>':'')+'</span>'+
        (isActive?' <span style="color:var(--amber);font-size:10px;font-weight:normal;margin-left:4px;">'+LaRuche.i18n.t('settings.activeBadge')+'</span>':'')+
        visBadge+visToggleBtn+
        '</div>'+
        codexAuthHtml+
        '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.typeLabel')+'</span><span class="settings-value">'+provLabel+'</span></div>'+
        '<div class="settings-row"><span class="settings-label">URL</span><span class="settings-value" style="font-size:10px;word-break:break-all">'+LaRuche.Utils.esc(p.base_url)+'</span></div>'+
        apiKeyHtml+
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
    if(codexStatus.phase) renderCodexBox(codexStatus); else refreshCodexStatus();
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
          if(s.phase === 'connected') {
            if(LaRuche.Models && LaRuche.Models.loadModels) LaRuche.Models.loadModels();
            var content = document.getElementById('settingsContent');
            if(content) loadProviders(content);
          }
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
    // Named presets: llama.cpp, LM Studio and vLLM all speak the OpenAI wire format, so
    // they need no client of their own, only their usual port. Listing them by name
    // spares the user from guessing which entry to pick and which port to type.
    var defaultUrls = {ollama:'http://127.0.0.1:11434', openai:'https://api.openai.com',
      anthropic:'https://api.anthropic.com', llamacpp:'http://127.0.0.1:8001',
      lmstudio:'http://127.0.0.1:1234', vllm:'http://127.0.0.1:8000'};
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
      '<option value="llamacpp"'+(provType==='llamacpp'?' selected':'')+'>llama.cpp (llama-server)</option>'+
      '<option value="lmstudio"'+(provType==='lmstudio'?' selected':'')+'>LM Studio</option>'+
      '<option value="vllm"'+(provType==='vllm'?' selected':'')+'>vLLM</option>'+
      '<option value="anthropic"'+(provType==='anthropic'?' selected':'')+'>Anthropic</option>'+
      '</select></div>'+
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfBaseUrlLabel')+'</label>'+
      '<input class="form-input" id="pfBaseUrl" value="'+LaRuche.Utils.esc(p.base_url||defaultUrls[provType]||'')+'" placeholder="'+defaultUrls[provType]+'"></div>'+
      /* Two ways to give the key. Typing it stores the secret inside the profile file;
         picking one from the vault stores a `${NAME}` reference that secrets::substituer
         expands at call time, so the value stays in one place and never sits in a second
         file. An existing `${...}` value opens the form already in vault mode. */
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfApiKeyLabel')+'</label>'+
      '<div style="display:flex;gap:6px;margin-bottom:6px">'+
        '<select class="form-input" id="pfKeyMode" style="flex:0 0 auto;width:auto;padding:4px 8px" onchange="LaRuche.Settings.secretPick()">'+
          '<option value="manual"'+(_estRefSecret(p.api_key)?'':' selected')+'>'+LaRuche.i18n.t('settings.pfKeyModeManual')+'</option>'+
          '<option value="vault"'+(_estRefSecret(p.api_key)?' selected':'')+'>'+LaRuche.i18n.t('settings.pfKeyModeVault')+'</option>'+
        '</select>'+
      '</div>'+
      '<input class="form-input" id="pfApiKey" type="password" value="'+LaRuche.Utils.esc(p.api_key||'')+'" placeholder="'+LaRuche.i18n.t('settings.pfApiKeyPlaceholder')+'" autocomplete="off"'+(_estRefSecret(p.api_key)?' style="display:none"':'')+'>'+
      '<select class="form-input" id="pfSecretRef" onchange="LaRuche.Settings.secretPickCreate()"'+(_estRefSecret(p.api_key)?'':' style="display:none"')+'></select>'+
      '<div class="settings-card-desc" id="pfKeyHint" style="margin:4px 0 0">'+(_estRefSecret(p.api_key)?LaRuche.i18n.t('settings.pfKeyVaultHint'):'')+'</div></div>'+
      '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.pfModelsLabel')+'</label>'+
      '<input class="form-input" id="pfModels" value="'+LaRuche.Utils.esc((p.models||[]).join(', '))+'" placeholder="'+LaRuche.i18n.t('settings.pfModelsPlaceholder')+'"></div>'+
      '<div style="display:flex;gap:8px;margin-top:8px">'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.saveProfile()">'+LaRuche.i18n.t('settings.pfSave')+'</button>'+
      '<button style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:6px;padding:6px 16px;cursor:pointer" onclick="document.getElementById(\'profileFormContainer\').style.display=\'none\'">'+LaRuche.i18n.t('settings.pfCancel')+'</button>'+
      '</div></div>';
    // Reopening a profile that already references the vault: fill the picker so the
    // chosen entry shows up selected instead of an empty select.
    if(_estRefSecret(p.api_key)) secretPick();
  }

  function onProfileProviderChange() {
    var prov = document.getElementById('pfProvider').value;
    var urlField = document.getElementById('pfBaseUrl');
    var defaultUrls = {ollama:'http://127.0.0.1:11434', openai:'https://api.openai.com', anthropic:'https://api.anthropic.com', llamacpp:'http://127.0.0.1:8001', lmstudio:'http://127.0.0.1:1234', vllm:'http://127.0.0.1:8000'};
    if(!urlField) return;
    // Only overwrite a value that is itself a preset, so a port typed by hand survives.
    var estPreset = Object.keys(defaultUrls).some(function(k){ return urlField.value === defaultUrls[k]; });
    if(!urlField.value || estPreset) {
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
      var customActions = (t.origin === 'Custom') ? '<div style="margin-top:10px;display:flex;gap:8px;border-top:1px solid rgba(255,255,255,0.05);padding-top:8px;"><button style="background:none;border:1px solid var(--border);color:var(--text-muted);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();LaRuche.Toast.show(LaRuche.i18n.t(\'settings.pluginSrcUnavailable\'),\'err\')">'+LaRuche.i18n.t('settings.viewSource')+'</button><button style="background:none;border:1px solid var(--border);color:var(--text-muted);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();LaRuche.Toast.show(LaRuche.i18n.t(\'settings.pluginJsonNoEdit\'),\'err\')">'+LaRuche.i18n.t('settings.editJson')+'</button><button style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();fetch(\'/api/tools/\'+encodeURIComponent(t.name),{method:\'DELETE\'}).then(function(){LaRuche.Settings.refreshTab()})">'+LaRuche.i18n.t('settings.tlDelete')+'</button></div>' : '';
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

  /* Le QR d'acces telephone.
     Il existait dans le noeud depuis toujours (imprime au demarrage, puis efface
     par le TUI) et n'avait jamais eu de route ni de place dans l'interface: qui
     lance l'application de bureau n'avait aucun moyen d'atteindre sa ruche depuis
     son telephone. En tete de la section reseau, parce que c'est la question
     reseau qu'on se pose le plus souvent. */
  /* L'interrupteur reseau, avec la distinction qui evite la question suivante.

     L'etat EN COURS et l'etat VOULU peuvent differer, et c'est normal: le port
     s'ouvre une fois, au lancement, et ne se deplace pas ensuite. Montrer les deux
     vaut mieux qu'un interrupteur qui semble n'avoir servi a rien. */
  function _interrupteurLan(lan){
    if(!lan) return '';
    var t = LaRuche.i18n.t;
    var voulu = !!lan.voulu, enCours = !!lan.en_cours, fige = !!lan.impose_par_env;
    var pastille = '<span style="font-size:10.5px;padding:1px 7px;border-radius:9px;'+
      (enCours ? 'background:var(--green-dim);color:var(--green)' : 'background:var(--bg-input);color:var(--text-muted)')+
      '">'+t(enCours?'settings.lanActifMaintenant':'settings.lanInactifMaintenant')+'</span>';
    return '<div style="margin:14px 0 0;padding-top:12px;border-top:1px solid var(--border)">'+
      '<div style="display:flex;align-items:center;gap:9px;flex-wrap:wrap">'+
        '<label style="display:flex;align-items:center;gap:7px;cursor:'+(fige?'not-allowed':'pointer')+';font-size:13px">'+
          '<input type="checkbox" id="lanToggle"'+(voulu?' checked':'')+(fige?' disabled':'')+'>'+
          '<span>'+t('settings.lanTitre')+'</span>'+
        '</label>'+
        pastille+
      '</div>'+
      '<p style="color:var(--text-dim);font-size:11.5px;margin:6px 0 0">'+t('settings.lanHint')+'</p>'+
      (fige
        ? '<p style="color:var(--amber);font-size:11px;margin:6px 0 0">'+t('settings.lanEnv')+'</p>'
        // Le rappel du redemarrage n'apparait QUE quand le voulu et l'en-cours
        // divergent: le repeter en permanence le rendrait invisible le jour ou il
        // compte.
        : (voulu !== enCours
            ? '<p style="color:var(--amber);font-size:11px;margin:6px 0 0">'+t('settings.lanRedemarrage')+'</p>'
            : ''))+
    '</div>';
  }

  async function carteQr() {
    var d = null, lan = null;
    try {
      var r2 = await Promise.all([
        fetch('/api/reseau/qr').then(function(r){ return r.json(); }),
        fetch('/api/reseau/bind-lan').then(function(r){ return r.json(); })
      ]);
      d = r2[0]; lan = r2[1];
    } catch(e) {}
    if (!d) return '';
    var t = LaRuche.i18n.t;
    var corps;
    if (d.disponible) {
      // L'adresse est ecrite EN CLAIR et en entier: c'est elle qu'on recopie a la
      // main quand la camera du telephone ne veut pas cooperer.
      corps = '<div style="display:flex;gap:18px;align-items:center;flex-wrap:wrap">'+
        '<div style="background:#1a1a2e;border-radius:10px;padding:8px;line-height:0;flex:0 0 auto">'+d.qr_svg+'</div>'+
        '<div style="flex:1;min-width:190px">'+
          '<p style="color:var(--text-dim);font-size:12px;margin:0 0 10px">'+t('settings.qrHint')+'</p>'+
          '<div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap">'+
            '<a href="'+LaRuche.Utils.esc(d.url)+'" target="_blank" rel="noopener" style="font-family:var(--mono);color:var(--cyan);font-size:14px;word-break:break-all">'+LaRuche.Utils.esc(d.url)+'</a>'+
            '<button class="send-btn" id="qrCopier" data-url="'+LaRuche.Utils.esc(d.url)+'"><span>'+t('settings.qrCopy')+'</span></button>'+
          '</div>'+
          // Le code peut etre parfait et la ruche muette: le dire ici evite de
          // chercher du cote du telephone un probleme qui est cote serveur.
          // Un INTERRUPTEUR et non une consigne. Dire "démarre-la avec
          // LARUCHE_BIND_LAN=1" revient a demander de sortir de l'application, de
          // trouver le bon lanceur et de l'editer, pour une question qu'on se pose
          // ici, telephone en main.
          _interrupteurLan(lan)+
        '</div>'+
      '</div>';
    } else {
      corps = '<p style="color:var(--text-muted);font-size:13px;margin:0">'+t('settings.qrNoLan')+'</p>'+
        '<p style="color:var(--text-dim);font-size:12px;margin:6px 0 0">'+t('settings.qrNoLanHint')+'</p>';
    }
    return '<div class="settings-card"><div class="settings-card-title">'+t('settings.qrTitle')+'</div>'+corps+'</div>';
  }

  /* Apparence: le catalogue, l'editeur genere depuis LaRuche.Themes.GROUPES, et
     un apercu qui montre du VRAI contenu.

     L'editeur n'est pas ecrit a la main. Il se construit a partir des groupes
     declares dans themes.js, donc un jeton ajoute la-bas apparait ici sans qu'on
     y touche, et il ne peut pas exister de champ qui ne corresponde a rien. Les
     valeurs affichees sont lues sur le theme COURANT, telles que le navigateur
     les calcule, plutot que recopiees dans une liste qui aurait divergé. */
  var _themeBrouillon = null;

  function _hexDe(v){
    var t = String(v||'').trim();
    var m = t.match(/^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i);
    if(m){
      var h = m[1];
      if(h.length === 3) h = h.split('').map(function(c){ return c+c; }).join('');
      return '#'+h.slice(0,6);
    }
    m = t.match(/^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)/i);
    if(m){
      return '#'+[m[1],m[2],m[3]].map(function(n){
        return Math.max(0,Math.min(255,Math.round(Number(n)))).toString(16).padStart(2,'0');
      }).join('');
    }
    return null;
  }

  function _apercuMarkdown(){
    var t = LaRuche.i18n.t;
    return '<div class="settings-card"><div class="settings-card-title">'+t('settings.themePreview')+'</div>'+
      '<div style="background:var(--bg);border:1px solid var(--border);border-radius:10px;padding:16px">'+
        '<h3 style="margin:0 0 4px;font-size:19px;color:var(--text)">Titre de document</h3>'+
        '<p style="margin:0 0 12px;font-size:12px;color:var(--text-dim)">Sous-titre, en texte atténué</p>'+
        '<p style="margin:0 0 8px;color:var(--text);font-size:13.5px">Du texte courant avec du <b>gras</b>, de l’<i>italique</i>, '+
          '<code style="background:var(--bg-input);padding:1px 5px;border-radius:4px;font-family:var(--mono);font-size:12px">du code</code> '+
          'et un <a href="#" onclick="return false" style="color:var(--amber)">lien</a>.</p>'+
        '<blockquote style="margin:0 0 10px;padding:4px 0 4px 11px;border-left:2px solid var(--amber);color:var(--text-dim);font-style:italic;font-size:13px">Une citation, pour voir la bordure d’accent.</blockquote>'+
        '<div style="display:flex;gap:6px;flex-wrap:wrap;margin-bottom:10px">'+
          '<span style="background:var(--green-dim);color:var(--green);padding:2px 8px;border-radius:10px;font-size:11px">succès</span>'+
          '<span style="background:var(--red-dim);color:var(--red);padding:2px 8px;border-radius:10px;font-size:11px">erreur</span>'+
          '<span style="background:var(--blue-dim);color:var(--blue);padding:2px 8px;border-radius:10px;font-size:11px">info</span>'+
          '<span style="background:var(--cyan-dim);color:var(--cyan);padding:2px 8px;border-radius:10px;font-size:11px">cyan</span>'+
        '</div>'+
        '<div style="background:var(--bg-card);border:1px solid var(--border);border-radius:8px;padding:9px 11px">'+
          '<div style="font-size:12px;color:var(--text-dim)">Une carte, sur le fond des panneaux</div>'+
        '</div>'+
      '</div></div>';
  }

  var _marqueBrouillon = {};
  var _fondBrouillon = {};
  var _iconesBrouillon = {};
  var _sauveMinuteur = null;
  var _apparenceHote = null;

  /* Reduire l'image de fond avant de la ranger dans le theme.

     Un theme se partage en un fichier: une photo d'appareil de huit mebioctets y
     entrerait telle quelle, encodee en base64, donc un tiers plus lourde encore.
     Elle depassait la limite de corps de la requete, et l'echec etait muet.
     Deux mille deux cents pixels sur le grand cote suffisent a un fond d'ecran,
     et le JPEG a 85 pour cent ramene le tout a quelques centaines de kilooctets.
     On garde le PNG d'origine quand il est deja leger: lui, il peut etre
     transparent, et le reencoder en JPEG lui poserait un fond noir. */
  function _reduireImage(dataUri, cb){
    var MAX = 2200, SEUIL_PNG = 400 * 1024;
    var img = new Image();
    img.onload = function(){
      var ech = Math.min(1, MAX / Math.max(img.width, img.height));
      var estPng = /^data:image\/png/i.test(dataUri);
      if(ech === 1 && estPng && dataUri.length < SEUIL_PNG){ cb(dataUri); return; }
      var c = document.createElement('canvas');
      c.width = Math.round(img.width * ech);
      c.height = Math.round(img.height * ech);
      var g = c.getContext('2d');
      g.drawImage(img, 0, 0, c.width, c.height);
      var out = c.toDataURL('image/jpeg', 0.85);
      cb(out.length < dataUri.length ? out : dataUri);
    };
    img.onerror = function(){ cb(dataUri); };
    img.src = dataUri;
  }

  /* Une ligne de reglage, choisie par le TYPE du jeton.

     Une pipette pour tout, c'etait le defaut d'avant: les jetons qui ne sont pas
     des couleurs restaient hors d'atteinte, et ceux qui portent une transparence
     voyaient leur pipette desactivee faute de savoir lire un `rgba`. Chaque type
     recoit donc le controle qui lui convient, et la couleur en recoit DEUX, la
     teinte et l'opacite, parce que l'une sans l'autre ne decrit pas un fond. */
  function _ligneJeton(j, valeur, esc){
    var T = LaRuche.Themes;
    var nom = esc(j[LaRuche.i18n.get()] || j.fr);
    var etiquette = '<span style="flex:1;font-size:12.5px;color:var(--text-dim)">'+nom+'</span>'+
      '<button type="button" data-defaut="'+j.cle+'" title="'+esc(LaRuche.i18n.t('settings.tokenReset'))+'" '+
        'style="background:none;border:none;color:var(--text-muted);cursor:pointer;font-size:13px;'+
        'padding:0 2px;line-height:1;visibility:hidden">&#8634;</button>';
    var champ = '<input type="text" data-jeton-txt="'+j.cle+'" value="'+esc(valeur)+'" spellcheck="false" '+
      'style="width:190px;background:var(--bg-input);color:var(--text-dim);border:1px solid var(--border);'+
      'border-radius:5px;padding:3px 7px;font-family:var(--mono);font-size:11px">';

    if(j.type === 'taille'){
      var n = parseFloat(valeur);
      if(isNaN(n)) n = j.min;
      return '<div style="display:flex;align-items:center;gap:9px;padding:3px 0">'+
        etiquette+
        '<input type="range" data-jeton-taille="'+j.cle+'" min="'+j.min+'" max="'+j.max+'" step="'+j.pas+'" '+
          'value="'+n+'" data-unite="'+j.unite+'" style="width:130px;accent-color:var(--amber)">'+
        champ+'</div>';
    }
    if(j.type === 'police'){
      var piles = j.mono ? T.PILES_MONO : T.PILES;
      var opts = '<option value="">—</option>'+piles.map(function(p){
        return '<option value="'+esc(p.v)+'"'+(p.v===valeur.trim()?' selected':'')+'>'+esc(p.nom)+'</option>';
      }).join('');
      return '<div style="display:flex;align-items:center;gap:9px;padding:3px 0">'+
        etiquette+
        '<select data-jeton-pile="'+j.cle+'" style="width:130px;background:var(--bg-input);color:var(--text);'+
          'border:1px solid var(--border);border-radius:5px;padding:3px 5px;font-size:11.5px">'+opts+'</select>'+
        champ+'</div>';
    }
    // Couleur: pipette + opacite. `resoudreCouleur` lit toutes les syntaxes, donc
    // la pipette n'est plus desactivee sur les valeurs transparentes.
    var c = T.resoudreCouleur(valeur);
    var hex = c ? T.versHex(c) : '#000000';
    var a = c ? c.a : 1;
    return '<div style="display:flex;align-items:center;gap:9px;padding:3px 0">'+
      '<input type="color" data-jeton="'+j.cle+'" value="'+hex+'" '+
        'style="width:30px;height:24px;padding:0;border:1px solid var(--border);border-radius:5px;background:none;cursor:pointer">'+
      etiquette+
      '<input type="range" data-jeton-alpha="'+j.cle+'" min="0" max="1" step="0.01" value="'+a+'" '+
        'title="opacité" style="width:82px;accent-color:var(--amber)">'+
      champ+'</div>';
  }

  /* Un seul ecouteur pour toute la vie de la page, qui redessine l'onglet
     Apparence quand le theme change AILLEURS: dans le menu de la barre du haut,
     ou depuis le dock. Sans lui, chaque surface gardait son idee du theme actif,
     celle du moment ou elle avait ete dessinee, et choisir dans l'une laissait
     l'autre en arriere. */
  document.addEventListener('laruche:theme', function(){
    if(_apparenceHote && _apparenceHote.isConnected) loadApparence(_apparenceHote);
  });

  function loadApparence(el){
    if(!window.LaRuche || !LaRuche.Themes){ el.innerHTML=''; return; }
    _apparenceHote = el;
    var t = LaRuche.i18n.t, esc = LaRuche.Utils.esc, T = LaRuche.Themes;
    var actif = T.actif();
    var perso = T.estPerso(actif);
    var lg = LaRuche.i18n.get();

    var vignettes = T.catalogue().map(function(x){
      var apercuFond = x.image
        ? 'background-image:url('+JSON.stringify(x.image)+');background-size:cover;background-position:center'
        : 'background:'+x.fond;
      return '<button class="theme-vignette" data-id="'+esc(x.id)+'" style="'+
        'display:flex;flex-direction:column;gap:6px;align-items:center;background:none;cursor:pointer;'+
        'border:1px solid '+(x.id===actif?'var(--amber)':'var(--border)')+';border-radius:10px;padding:9px 7px;min-width:96px">'+
        '<span style="width:100%;height:36px;border-radius:6px;'+apercuFond+';border:1px solid var(--border);'+
          'display:flex;align-items:center;justify-content:center">'+
          '<span style="width:11px;height:11px;border-radius:50%;background:'+x.point+'"></span></span>'+
        '<span style="font-size:11.5px;color:var(--text-dim);max-width:88px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">'+esc(x.nom)+'</span>'+
      '</button>';
    }).join('');

    var jetons = T.jetonsCourants();
    _themeBrouillon = Object.assign({}, jetons);
    var h = T.habillageDe(actif);
    _marqueBrouillon = Object.assign({}, h.marque);
    _fondBrouillon = Object.assign({ opacite: 0.35, cadrage: 'cover', zones: {} }, h.fond);
    _fondBrouillon.zones = Object.assign({}, _fondBrouillon.zones);
    _iconesBrouillon = Object.assign({}, h.icones);

    var champs = T.GROUPES.map(function(g){
      var lignes = g.jetons.map(function(j){ return _ligneJeton(j, jetons[j.cle] || '', esc); }).join('');
      return '<div style="margin-bottom:12px"><div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;'+
        'color:var(--text-muted);margin-bottom:5px">'+esc(g.titre[lg]||g.titre.fr)+'</div>'+lignes+'</div>';
    }).join('');

    var basculesZone = T.ZONES.map(function(z){
      var on = !!_fondBrouillon.zones[z.cle];
      return '<label style="display:inline-flex;align-items:center;gap:6px;font-size:12px;color:var(--text-dim);'+
        'border:1px solid var(--border);border-radius:999px;padding:4px 11px;cursor:pointer">'+
        '<input type="checkbox" data-zone="'+z.cle+'"'+(on?' checked':'')+' style="accent-color:var(--amber)">'+
        esc(z[lg]||z.fr)+'</label>';
    }).join(' ');

    var nomActuel = '';
    if(perso){ T.catalogue().forEach(function(x){ if(x.id===actif) nomActuel = x.nom; }); }

    el.innerHTML =
      '<div class="settings-card"><div class="settings-card-title">'+t('settings.themeTitle')+'</div>'+
        '<p style="color:var(--text-dim);font-size:12px;margin:2px 0 10px">'+t('settings.themeHint')+'</p>'+
        '<div id="themeVignettes" style="display:flex;gap:9px;flex-wrap:wrap">'+vignettes+'</div>'+
      '</div>'+
      _apercuMarkdown()+

      '<div class="settings-card"><div class="settings-card-title">'+t('settings.brandTitle')+'</div>'+
        '<p style="color:var(--text-dim);font-size:12px;margin:2px 0 10px">'+t('settings.brandHint')+'</p>'+
        '<div style="display:flex;align-items:center;gap:9px;padding:3px 0">'+
          '<span style="flex:1;font-size:12.5px;color:var(--text-dim)">'+t('settings.brandName')+'</span>'+
          '<input type="text" id="marqueNom" value="'+esc(_marqueBrouillon.nom||'')+'" placeholder="LaRuche" '+
            'style="width:250px;background:var(--bg-input);color:var(--text);border:1px solid var(--border);'+
            'border-radius:5px;padding:4px 8px;font-size:12px"></div>'+
        '<div style="display:flex;align-items:center;gap:9px;padding:6px 0;flex-wrap:wrap">'+
          '<span style="flex:1;min-width:140px;font-size:12.5px;color:var(--text-dim)">'+t('settings.brandLogo')+'</span>'+
          '<span id="marqueApercu" class="lr-logo" style="width:34px;height:34px;border:1px solid var(--border);border-radius:6px"></span>'+
          '<button class="cwd-btn" id="marqueChoisir" style="opacity:1;font-size:12px;padding:6px 10px">'+t('settings.brandPick')+'</button>'+
          '<button class="cwd-btn" id="marqueVider" style="opacity:1;font-size:12px;padding:6px 10px">'+t('settings.brandClear')+'</button>'+
          '<input type="file" id="marqueFichier" accept=".svg,image/svg+xml,image/png,image/webp" style="display:none"></div>'+
        '<p style="color:var(--text-muted);font-size:11px;margin:6px 0 0">'+t('settings.brandSvgHint')+'</p>'+
      '</div>'+

      '<div class="settings-card"><div class="settings-card-title">'+t('settings.iconTitle')+'</div>'+
        '<p style="color:var(--text-dim);font-size:12px;margin:2px 0 10px">'+t('settings.iconHint')+'</p>'+
        T.ICONES.map(function(ic){
          return '<div style="display:flex;align-items:center;gap:9px;padding:4px 0">'+
            '<span class="lr-icone-apercu" data-icone-apercu="'+ic.cle+'"></span>'+
            '<span style="flex:1;font-size:12.5px;color:var(--text-dim)">'+esc(ic[lg]||ic.fr)+'</span>'+
            '<button class="cwd-btn" data-icone-choisir="'+ic.cle+'" style="opacity:1;font-size:11.5px;padding:4px 9px">'+t('settings.brandPick')+'</button>'+
            '<button class="cwd-btn" data-icone-vider="'+ic.cle+'" style="opacity:1;font-size:11.5px;padding:4px 9px">'+t('settings.brandClear')+'</button>'+
          '</div>';
        }).join('')+
        '<input type="file" id="iconeFichier" accept=".svg,image/svg+xml,image/png,image/webp" style="display:none">'+
      '</div>'+
      '<div class="settings-card"><div class="settings-card-title">'+t('settings.bgTitle')+'</div>'+
        '<p style="color:var(--text-dim);font-size:12px;margin:2px 0 10px">'+t('settings.bgHint')+'</p>'+
        '<div style="display:flex;align-items:center;gap:9px;padding:3px 0;flex-wrap:wrap">'+
          '<span id="fondApercu" style="width:72px;height:44px;border-radius:6px;border:1px solid var(--border);'+
            'background-size:cover;background-position:center;flex-shrink:0"></span>'+
          '<button class="cwd-btn" id="fondChoisir" style="opacity:1;font-size:12px;padding:6px 10px">'+t('settings.bgPick')+'</button>'+
          '<button class="cwd-btn" id="fondVider" style="opacity:1;font-size:12px;padding:6px 10px">'+t('settings.bgClear')+'</button>'+
          '<input type="file" id="fondFichier" accept="image/*" style="display:none"></div>'+
        '<div style="display:flex;align-items:center;gap:9px;padding:8px 0">'+
          '<span style="flex:1;font-size:12.5px;color:var(--text-dim)">'+t('settings.bgOpacity')+'</span>'+
          '<input type="range" id="fondOpacite" min="0" max="1" step="0.01" value="'+_fondBrouillon.opacite+'" '+
            'style="width:180px;accent-color:var(--amber)">'+
          '<span id="fondOpaciteVal" style="width:38px;text-align:right;font-family:var(--mono);font-size:11px;color:var(--text-dim)"></span></div>'+
        '<div style="display:flex;align-items:center;gap:9px;padding:3px 0">'+
          '<span style="flex:1;font-size:12.5px;color:var(--text-dim)">'+t('settings.bgFit')+'</span>'+
          '<select id="fondCadrage" style="width:130px;background:var(--bg-input);color:var(--text);'+
            'border:1px solid var(--border);border-radius:5px;padding:3px 5px;font-size:11.5px">'+
            ['cover','contain','auto','100% 100%'].map(function(v){
              return '<option value="'+v+'"'+(v===_fondBrouillon.cadrage?' selected':'')+'>'+v+'</option>';
            }).join('')+'</select></div>'+
        '<div style="margin-top:8px;font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--text-muted)">'+
          t('settings.bgZones')+'</div>'+
        '<div style="display:flex;gap:7px;flex-wrap:wrap;margin-top:6px">'+basculesZone+'</div>'+
      '</div>'+

      '<div class="settings-card"><div class="settings-card-title">'+t('settings.themeEditTitle')+'</div>'+
        '<p style="color:var(--text-dim);font-size:12px;margin:2px 0 10px">'+t('settings.themeEditHint')+'</p>'+
        '<div style="display:flex;gap:8px;align-items:flex-start;border:1px solid '+
          (perso?'var(--green)':'var(--amber)')+';border-radius:8px;padding:8px 10px;margin-bottom:12px;'+
          'background:'+(perso?'var(--green-dim)':'var(--amber-glow)')+';font-size:12px;color:var(--text-dim)">'+
          '<span>'+(perso?'&#10003;':'&#9888;')+'</span><span>'+
          (perso?t('settings.themeAutoSaved'):t('settings.themeBuiltinHint'))+'</span></div>'+
        champs+
        '<div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-top:10px">'+
          '<input id="themeNom" class="form-input" placeholder="'+esc(t('settings.themeName'))+'" value="'+esc(nomActuel)+'" style="flex:1;min-width:170px">'+
          (perso
            ? '<button class="cwd-btn" id="themeRenommer" style="opacity:1;font-size:12px;padding:7px 11px">'+t('settings.themeRename')+'</button>'
            : '<button class="send-btn" id="themeSave"><span>'+t('settings.themeSaveAs')+'</span></button>')+
          '<button class="cwd-btn" id="themeDup" style="opacity:1;font-size:12px;padding:7px 11px">'+t('settings.themeDuplicate')+'</button>'+
          (perso?'<button class="cwd-btn" id="themeDel" style="opacity:1;font-size:12px;padding:7px 11px;color:var(--red)">'+t('settings.themeDelete')+'</button>':'')+
          '<button class="cwd-btn" id="themeReset" style="opacity:1;font-size:12px;padding:7px 11px">'+t('settings.themeRevert')+'</button>'+
          '<span id="themeEtat" style="font-size:11.5px;color:var(--text-muted)"></span>'+
        '</div>'+
      '</div>';

    el.querySelectorAll('.theme-vignette').forEach(function(b){
      b.onmouseenter = function(){ T.apercuSur(b.dataset.id); };
      b.onmouseleave = function(){ T.apercuFin(); };
      b.onclick = function(){ T.appliquer(b.dataset.id); loadApparence(el); };
    });

    /* Chaque champ peint IMMEDIATEMENT, sans passer par un enregistrement: c'est
       la seule facon de choisir une couleur, en la voyant sur l'interface reelle
       et non sur un carre de 30 pixels. Le brouillon retient ce qui a change,
       l'enregistrement le fige dans un fichier. */
    /* Le brouillon est DECLARE au moteur a chaque changement.

       Sans cela il n'existait qu'en memoire de ce panneau, et la moindre
       repeinture le detruisait: survoler une vignette appelle `apercuFin`, qui
       repeint le theme actif, et le repeignait depuis le fichier enregistre. Une
       image de fond posee disparaissait donc au premier mouvement de souris. */
    function declarer(){
      T.definirBrouillon({ id: actif, jetons: _themeBrouillon, marque: _marqueBrouillon,
                           fond: _fondBrouillon, icones: _iconesBrouillon });
      majEtat();
      if(perso) planifierSauvegarde();
    }

    function majEtat(txt){
      var e = document.getElementById('themeEtat');
      if(e) e.textContent = txt || (perso ? '' : t('settings.themeUnsaved'));
      el.querySelectorAll('[data-defaut]').forEach(function(b){
        b.style.visibility = _differe(b.dataset.defaut) ? 'visible' : 'hidden';
      });
    }

    /* La valeur d'origine d'un jeton. Un theme livre la tient de sa feuille de
       style: il suffit de retirer la valeur en ligne pour la retrouver. Une copie
       la tient de la `base` capturee au moment ou elle a ete faite. */
    var base = T.baseDe(actif);
    function valeurOrigine(cle){ return base ? base[cle] : null; }
    function _differe(cle){
      var o = valeurOrigine(cle);
      if(o === null || o === undefined){
        // Integre: la difference se lit a la presence d'une valeur en ligne.
        return !!document.documentElement.style.getPropertyValue(cle);
      }
      return (_themeBrouillon[cle] || '').trim() !== (o || '').trim();
    }

    /* Auto-enregistrement, temporise. Un curseur d'opacite emet des dizaines
       d'evenements par seconde; ecrire un fichier a chacun serait absurde. */
    function planifierSauvegarde(){
      clearTimeout(_sauveMinuteur);
      majEtat(t('settings.themeSaving'));
      _sauveMinuteur = setTimeout(async function(){
        var nom = (document.getElementById('themeNom')||{}).value || nomActuel;
        var r = await T.enregistrer(nom.trim()||nomActuel, _themeBrouillon,
                                    actif.slice('perso:'.length), _marqueBrouillon, _fondBrouillon, _iconesBrouillon);
        majEtat(r && r.status === 'ok' ? t('settings.themeSavedOk') : t('settings.themeSaveFail'));
      }, 700);
    }

    function poser(cle, valeur){
      _themeBrouillon[cle] = valeur;
      document.documentElement.style.setProperty(cle, valeur);
      var txt = el.querySelector('[data-jeton-txt="'+cle+'"]');
      if(txt && txt.value !== valeur) txt.value = valeur;
      declarer();
    }
    function couleurDe(cle){
      var pip = el.querySelector('[data-jeton="'+cle+'"]');
      var alp = el.querySelector('[data-jeton-alpha="'+cle+'"]');
      if(!pip) return;
      poser(cle, T.composerCouleur(pip.value, alp ? parseFloat(alp.value) : 1));
    }
    el.querySelectorAll('[data-jeton]').forEach(function(inp){
      inp.oninput = function(){ couleurDe(inp.dataset.jeton); };
    });
    el.querySelectorAll('[data-jeton-alpha]').forEach(function(inp){
      inp.oninput = function(){ couleurDe(inp.dataset.jetonAlpha); };
    });
    el.querySelectorAll('[data-jeton-taille]').forEach(function(inp){
      inp.oninput = function(){ poser(inp.dataset.jetonTaille, inp.value + inp.dataset.unite); };
    });
    el.querySelectorAll('[data-jeton-pile]').forEach(function(sel){
      sel.onchange = function(){ if(sel.value) poser(sel.dataset.jetonPile, sel.value); };
    });
    el.querySelectorAll('[data-jeton-txt]').forEach(function(inp){
      inp.oninput = function(){
        var cle = inp.dataset.jetonTxt;
        _themeBrouillon[cle] = inp.value;
        document.documentElement.style.setProperty(cle, inp.value);
        declarer();
        var pip = el.querySelector('[data-jeton="'+cle+'"]');
        if(pip){
          var c = T.resoudreCouleur(inp.value);
          if(c){
            pip.value = T.versHex(c);
            var alp = el.querySelector('[data-jeton-alpha="'+cle+'"]');
            if(alp) alp.value = c.a;
          }
        }
      };
    });

    /* Retour a la valeur d'origine, jeton par jeton. Sur un theme livre, il
       suffit de retirer la valeur en ligne pour retrouver celle de la feuille de
       style; sur une copie, on repose la valeur capturee a sa creation. */
    el.querySelectorAll('[data-defaut]').forEach(function(b){
      b.onclick = function(){
        var cle = b.dataset.defaut;
        var o = valeurOrigine(cle);
        if(o === null || o === undefined){
          delete _themeBrouillon[cle];
          document.documentElement.style.removeProperty(cle);
        } else {
          _themeBrouillon[cle] = o;
          document.documentElement.style.setProperty(cle, o);
        }
        var v = getComputedStyle(document.documentElement).getPropertyValue(cle).trim();
        var txt = el.querySelector('[data-jeton-txt="'+cle+'"]'); if(txt) txt.value = v;
        var pip = el.querySelector('[data-jeton="'+cle+'"]');
        if(pip){
          var c = T.resoudreCouleur(v);
          if(c){
            pip.value = T.versHex(c);
            var al = el.querySelector('[data-jeton-alpha="'+cle+'"]'); if(al) al.value = c.a;
          }
        }
        var tl = el.querySelector('[data-jeton-taille="'+cle+'"]'); if(tl) tl.value = parseFloat(v);
        declarer();
      };
    });

    /* ---- La marque ---- */
    function rendreMarque(){
      var ap = document.getElementById('marqueApercu');
      if(ap) ap.innerHTML = _marqueBrouillon.logo
        ? (String(_marqueBrouillon.logo).slice(0,4) === 'data'
            ? '<img src="'+esc(_marqueBrouillon.logo)+'" alt="">'
            : _marqueBrouillon.logo)
        : '';
      T.peindreMarque(_marqueBrouillon);
      declarer();
    }
    majEtat(perso ? t('settings.themeSavedOk') : t('settings.themeUnsaved'));
    rendreMarque();
    var mn = document.getElementById('marqueNom');
    if(mn) mn.oninput = function(){ _marqueBrouillon.nom = mn.value; rendreMarque(); };
    var mc = document.getElementById('marqueChoisir'), mf = document.getElementById('marqueFichier');
    if(mc && mf){
      mc.onclick = function(){ mf.click(); };
      mf.onchange = function(){
        var f = mf.files && mf.files[0]; if(!f) return;
        if(f.size > 512*1024){ LaRuche.Toast.show(t('settings.brandTooBig'),'warn'); return; }
        var fr = new FileReader();
        // Un SVG voyage en TEXTE, pas en data URI: le serveur doit pouvoir le
        // laver, et on ne lave pas ce qu'on ne peut pas lire. Le reste est une
        // image matricielle, opaque par nature, donc encodee telle quelle.
        fr.onload = function(){ _marqueBrouillon.logo = String(fr.result); rendreMarque(); };
        if(/svg/i.test(f.type) || /\.svg$/i.test(f.name)) fr.readAsText(f);
        else fr.readAsDataURL(f);
      };
    }
    var mv = document.getElementById('marqueVider');
    if(mv) mv.onclick = function(){ _marqueBrouillon.logo = ''; rendreMarque(); };

    /* ---- Les icones ---- */
    function rendreIcones(){
      T.ICONES.forEach(function(ic){
        var ap = el.querySelector('[data-icone-apercu="'+ic.cle+'"]');
        if(!ap) return;
        var v = _iconesBrouillon[ic.cle];
        ap.innerHTML = v
          ? (String(v).slice(0,4) === 'data' ? '<img src="'+esc(v)+'" alt="">' : v)
          : '<span style="color:var(--text-muted);font-size:15px">&#8722;</span>';
      });
      T.peindreIcones(_iconesBrouillon);
      declarer();
    }
    var _icAttente = null;
    var icf = document.getElementById('iconeFichier');
    el.querySelectorAll('[data-icone-choisir]').forEach(function(b){
      b.onclick = function(){ _icAttente = b.dataset.iconeChoisir; if(icf){ icf.value=''; icf.click(); } };
    });
    el.querySelectorAll('[data-icone-vider]').forEach(function(b){
      b.onclick = function(){ delete _iconesBrouillon[b.dataset.iconeVider]; rendreIcones(); };
    });
    if(icf) icf.onchange = function(){
      var f = icf.files && icf.files[0];
      if(!f || !_icAttente) return;
      if(f.size > 256*1024){ LaRuche.Toast.show(t('settings.brandTooBig'),'warn'); return; }
      var fr = new FileReader();
      // Un SVG voyage en TEXTE: le serveur doit pouvoir le laver, et on ne lave
      // pas ce qu'on ne peut pas lire.
      fr.onload = function(){ _iconesBrouillon[_icAttente] = String(fr.result); _icAttente = null; rendreIcones(); };
      if(/svg/i.test(f.type) || /\.svg$/i.test(f.name)) fr.readAsText(f);
      else fr.readAsDataURL(f);
    };
    rendreIcones();

    /* ---- Le fond ---- */
    function rendreFond(){
      var ap = document.getElementById('fondApercu');
      if(ap) ap.style.backgroundImage = _fondBrouillon.image ? 'url('+JSON.stringify(_fondBrouillon.image)+')' : '';
      var v = document.getElementById('fondOpaciteVal');
      if(v) v.textContent = Math.round(_fondBrouillon.opacite*100)+'%';
      T.peindreFond(_fondBrouillon);
      declarer();
    }
    rendreFond();
    var fc = document.getElementById('fondChoisir'), ff = document.getElementById('fondFichier');
    if(fc && ff){
      fc.onclick = function(){ ff.click(); };
      ff.onchange = function(){
        var f = ff.files && ff.files[0]; if(!f) return;
        if(f.size > 3*1024*1024){ LaRuche.Toast.show(t('settings.bgTooBig'),'warn'); return; }
        var fr = new FileReader();
        fr.onload = function(){
          _reduireImage(String(fr.result), function(reduite){
            _fondBrouillon.image = reduite;
            // Poser une image sans allumer une zone ne montrerait rien: la zone
            // centrale s'allume d'office, les autres restent au choix.
            if(!Object.keys(_fondBrouillon.zones).some(function(k){ return _fondBrouillon.zones[k]; })){
              _fondBrouillon.zones.app = true;
              var b = el.querySelector('[data-zone="app"]'); if(b) b.checked = true;
            }
            rendreFond();
          });
        };
        fr.readAsDataURL(f);
      };
    }
    var fv = document.getElementById('fondVider');
    if(fv) fv.onclick = function(){ _fondBrouillon.image = ''; rendreFond(); };
    var fo = document.getElementById('fondOpacite');
    if(fo) fo.oninput = function(){ _fondBrouillon.opacite = parseFloat(fo.value); rendreFond(); };
    var fcad = document.getElementById('fondCadrage');
    if(fcad) fcad.onchange = function(){ _fondBrouillon.cadrage = fcad.value; rendreFond(); };
    el.querySelectorAll('[data-zone]').forEach(function(cb){
      cb.onchange = function(){ _fondBrouillon.zones[cb.dataset.zone] = cb.checked; rendreFond(); };
    });

    function nomDemande(defaut){
      var n = (document.getElementById('themeNom')||{}).value || '';
      n = n.trim();
      if(n && n !== nomActuel) return n;
      return window.prompt(t('settings.themeAskName'), defaut) || '';
    }

    /* Enregistrer un theme LIVRE modifie: cela fabrique une copie, jamais une
       reecriture. Un integre vit dans la feuille de style de l'application; le
       reecrire demanderait de reinstaller pour revenir en arriere, alors que le
       garder intact rend la remise a zero gratuite: il suffit de le reselectionner. */
    var save = document.getElementById('themeSave');
    if(save) save.onclick = async function(){
      var nom = nomDemande(nomActuel || (t('settings.themeCopyOf') + ' ' + (T.catalogue().filter(function(x){return x.id===actif;})[0]||{}).nom));
      if(!nom.trim()){ return; }
      var r = await T.dupliquer(nom.trim(), _themeBrouillon, _marqueBrouillon, _fondBrouillon, _iconesBrouillon);
      if(r && r.status === 'ok'){ LaRuche.Toast.show(t('settings.themeSaved'),'ok'); loadApparence(el); }
      else LaRuche.Toast.show((r&&r.error)||'erreur','warn');
    };

    var ren = document.getElementById('themeRenommer');
    if(ren) ren.onclick = async function(){
      var nom = (document.getElementById('themeNom')||{}).value || '';
      if(!nom.trim()){ LaRuche.Toast.show(t('settings.themeName'),'warn'); return; }
      var r = await T.enregistrer(nom.trim(), _themeBrouillon, actif.slice('perso:'.length),
                                  _marqueBrouillon, _fondBrouillon, _iconesBrouillon);
      if(r && r.status === 'ok'){ LaRuche.Toast.show(t('settings.themeSaved'),'ok'); loadApparence(el); }
    };

    var dup = document.getElementById('themeDup');
    if(dup) dup.onclick = async function(){
      var courant = (T.catalogue().filter(function(x){return x.id===actif;})[0]||{}).nom || '';
      var nom = window.prompt(t('settings.themeAskName'), t('settings.themeCopyOf') + ' ' + courant);
      if(!nom || !nom.trim()) return;
      var r = await T.dupliquer(nom.trim(), _themeBrouillon, _marqueBrouillon, _fondBrouillon, _iconesBrouillon);
      if(r && r.status === 'ok'){ LaRuche.Toast.show(t('settings.themeSaved'),'ok'); loadApparence(el); }
      else LaRuche.Toast.show((r&&r.error)||'erreur','warn');
    };

    var del = document.getElementById('themeDel');
    if(del) del.onclick = async function(){
      T.definirBrouillon(null);
      await T.supprimer(actif);
      loadApparence(el);
    };

    /* Revenir aux valeurs d'origine de CE theme, pas au theme par defaut. Le
       bouton disait `themeReset` et sautait sur `defaut`, ce qui faisait perdre
       le theme choisi en plus des retouches. */
    var reset = document.getElementById('themeReset');
    if(reset) reset.onclick = async function(){
      T.definirBrouillon(null);
      if(perso && base){
        _themeBrouillon = Object.assign({}, base);
        await T.enregistrer(nomActuel, _themeBrouillon, actif.slice('perso:'.length),
                            _marqueBrouillon, _fondBrouillon, _iconesBrouillon);
      }
      T.appliquer(actif);
      loadApparence(el);
    };
  }

  async function loadNetwork(el) {
    var qrCard = await carteQr();
    var codeSet=false; try{ codeSet=(await fetch('/api/mesh/code').then(function(r){return r.json();})).set; }catch(e){}
    var codeCard='<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.meshCodeTitle')+' '+
      (codeSet?'<span style="color:var(--green);font-size:11px">'+LaRuche.i18n.t('settings.meshCodeConfigured')+'</span>':'<span style="color:var(--text-muted);font-size:11px">'+LaRuche.i18n.t('settings.meshCodeUnconfigured')+'</span>')+'</div>'+
      '<p style="color:var(--text-dim);font-size:12px;margin:4px 0 8px">'+LaRuche.i18n.t('settings.meshCodeHint')+'</p>'+
      '<div style="display:flex;gap:8px"><input id="meshCodeInput" type="password" placeholder="'+(codeSet?LaRuche.i18n.t('settings.meshCodePlaceholderSet'):LaRuche.i18n.t('settings.meshCodePlaceholderNew'))+'" style="flex:1;background:var(--bg-input);color:var(--text);border:1px solid var(--border);border-radius:8px;padding:8px 10px;font-size:14px"><button class="send-btn" id="meshCodeSave"><span>'+LaRuche.i18n.t('settings.meshSave')+'</span></button></div></div>';
    var d={nodes:[]};try{d=await fetch('/swarm').then(function(r){return r.json();});}catch(e){}
    var nodesHtml=(d.nodes||[]).map(function(n){
      var caps=(n.capabilities||[]).map(function(c){return '<span style="background:rgba(6,182,212,.15);color:var(--cyan);padding:1px 6px;border-radius:8px;font-size:10px">'+LaRuche.Utils.esc(c)+'</span>';}).join(' ');
      return '<div class="settings-card"><div class="settings-card-title">'+LaRuche.Utils.esc(n.name||'?')+'</div><div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.hostLabel')+'</span><span class="settings-value">'+LaRuche.Utils.esc(n.host||'')+':'+LaRuche.Utils.esc(n.port||'?')+'</span></div><div style="margin-top:4px">'+caps+'</div></div>';
    }).join('')||'<div style="text-align:center;color:var(--text-muted);padding:20px">'+LaRuche.i18n.t('settings.noNodes')+'</div>';
    el.innerHTML=qrCard+codeCard+nodesHtml;
    var lanT=document.getElementById('lanToggle');
    if(lanT) lanT.onchange=async function(){
      await fetch('/api/reseau/bind-lan',{method:'POST',headers:{'Content-Type':'application/json'},
        body:JSON.stringify({actif:lanT.checked})}).catch(function(){});
      loadNetwork(el); // relit l'etat: le voulu a change, l'en-cours non
    };
    var copier=document.getElementById('qrCopier');
    if(copier) copier.onclick=function(){
      var u=copier.dataset.url||'';
      // `writeText` echoue hors contexte securise (http sur une IP de LAN, ce qui
      // est exactement notre cas): on retombe sur la selection, plutot que de ne
      // rien faire du tout.
      var fini=function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.qrCopied'),'ok'); };
      if(navigator.clipboard && navigator.clipboard.writeText){
        navigator.clipboard.writeText(u).then(fini).catch(function(){ secours(u); });
      } else { secours(u); }
      function secours(txt){
        var z=document.createElement('textarea');
        z.value=txt; z.style.position='fixed'; z.style.opacity='0';
        document.body.appendChild(z); z.select();
        try{ document.execCommand('copy'); fini(); }catch(e){}
        document.body.removeChild(z);
      }
    };
    var btn=document.getElementById('meshCodeSave');
    if(btn) btn.onclick=async function(){
      var v=(document.getElementById('meshCodeInput').value||'');
      if(!v.trim()){ LaRuche.Toast.show(LaRuche.i18n.t('settings.meshCodeUnchanged'),'info'); return; }
      try{ await fetch('/api/mesh/code',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({code:v})});
        LaRuche.Toast.show(LaRuche.i18n.t('settings.meshCodeSaved'),'ok'); loadNetwork(el);
      }catch(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.meshCodeFailed'),'err'); }
    };
  }

  // ── Cron timeline (vanilla JS) ────────────────────────────────────────
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
      '.tl-row{cursor:pointer}'+
      '.tl-head-all{display:flex;align-items:center;padding:0 10px;font-size:10px;font-weight:600;letter-spacing:.04em;text-transform:uppercase;color:var(--text-dim);cursor:pointer}'+
      '.tl-head-all:hover{color:var(--amber)}'+
      // The read card used to be written straight into the container, which carries an id
      // and no class, so only the edit form ever picked up this frame. Both share it now,
      // and the read card repeats once per job when every card is asked for.
      '.tl-card,.tl-detail{margin-top:12px;border:1px solid var(--amber);border-radius:8px;padding:12px;font-size:12px}';
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
    if(_tlTimer)LaRuche.Poll.stop(_tlTimer);
    _tlTimer=LaRuche.Poll.every(function(){ var nowLine=document.getElementById('tlNow'); if(!nowLine){LaRuche.Poll.stop(_tlTimer);return;} positionNow(); },1000);
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
    // The gutter head was an empty spacer. It is the natural place to ask for every card
    // at once, so it is now labelled and clickable.
    var gutter='<div class="tl-head tl-head-all" onclick="LaRuche.Settings.tlAll()" title="'+LaRuche.i18n.t('settings.tlAllHint')+'">'+LaRuche.i18n.t('settings.tlTasks')+'</div>', lanes='';
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
        // A plain click anywhere on the lane opens the card. It used to do nothing unless
        // it landed exactly on a marker, so most of the row was dead surface.
        if(Math.abs(moved)<8){ tlDetail(parseInt(row.getAttribute('data-i'))); return; }
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
  // One card, so a single row and the whole list render exactly the same thing.
  function tlCarte(i){
    var job=_tlJobs[i]; if(!job)return '';
    return '<div class="tl-card"><div style="font-weight:600;color:var(--amber);margin-bottom:6px">'+LaRuche.Utils.esc(job.name||LaRuche.i18n.t('settings.tlNoName'))+'</div>'+
      '<div>'+LaRuche.i18n.t('settings.tlPlanLabel')+'<code>'+LaRuche.Utils.esc(job.cron_expr||job.fire_at||'-')+'</code></div>'+
      '<div style="color:var(--text-dim)">'+LaRuche.i18n.t('settings.tlLastRun')+(job.last_run||LaRuche.i18n.t('settings.tlNever'))+LaRuche.i18n.t('settings.tlRuns')+(job.run_count||0)+(job.channel?(LaRuche.i18n.t('settings.tlChannel')+LaRuche.Utils.esc(job.channel)):'')+'</div>'+
      '<div style="margin-top:8px;display:flex;gap:6px;flex-wrap:wrap">'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlRun('+i+')">'+LaRuche.i18n.t('settings.tlRunNow')+'</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlEdit('+i+')">'+LaRuche.i18n.t('settings.tlEdit')+'</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlToggle('+i+')">'+(job.enabled===false?LaRuche.i18n.t('settings.tlResume'):LaRuche.i18n.t('settings.tlPause'))+'</button>'+
      '<button class="tl-btn" onclick="if(confirm(LaRuche.i18n.t(\'settings.tlDeleteConfirm\')))fetch(\'/api/cron/'+job.id+'\',{method:\'DELETE\'}).then(function(){LaRuche.Settings.tlReload&&LaRuche.Settings.tlReload();})">'+LaRuche.i18n.t('settings.tlDelete')+'</button>'+
      '</div></div>';
  }
  function tlDetail(i){
    var d=document.getElementById('tlDetail'); if(!d)return;
    d.removeAttribute('data-all');
    d.innerHTML=tlCarte(i);
  }
  // Every card at once, from the gutter head. Clicking again folds them back.
  function tlAll(){
    var d=document.getElementById('tlDetail'); if(!d)return;
    if(d.getAttribute('data-all')==='1'){ d.innerHTML=''; d.removeAttribute('data-all'); return; }
    d.innerHTML=_tlJobs.map(function(_,i){ return tlCarte(i); }).join('');
    d.setAttribute('data-all','1');
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
        // Rotating a key is the common case, deleting it is the rare one. Both are
        // offered here so nobody has to retype the name in the form below and risk a
        // typo creating a second, orphaned entry.
        var arg = "'"+String(n).replace(/\\/g,'\\\\').replace(/'/g,"\\'")+"'";
        return '<div class="settings-row"><span class="settings-value" style="font-family:var(--mono,monospace)">'+LaRuche.Utils.esc(n)+' <span style="color:var(--text-dim);font-size:10px">= ••••••••</span></span>'+
          '<span style="display:flex;gap:5px">'+
          '<button onclick="LaRuche.Settings.secretUpdate('+LaRuche.Utils.esc(arg)+')" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:1px 8px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.secretUpdateBtn')+'</button>'+
          '<button onclick="LaRuche.Settings.secretDelete('+LaRuche.Utils.esc(arg)+')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:1px 8px;cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.secretDeleteBtn')+'</button>'+
          '</span></div>';
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
  // A key stored as `${NAME}` is a vault reference, not a literal secret.
  function _estRefSecret(v){ return /^\$\{[^}]+\}$/.test(String(v||'')); }
  function _nomRefSecret(v){ var m=/^\$\{([^}]+)\}$/.exec(String(v||'')); return m?m[1]:''; }

  /* Switch the API-key field between typing a value and picking a vault entry. The
   * select is filled from /api/secrets, which only ever returns NAMES, so no key value
   * is sent to the browser to populate this. */
  function secretPick(){
    var mode = document.getElementById('pfKeyMode'), inp = document.getElementById('pfApiKey'),
        sel = document.getElementById('pfSecretRef'), hint = document.getElementById('pfKeyHint');
    if(!mode || !inp || !sel) return;
    var coffre = mode.value === 'vault';
    inp.style.display = coffre ? 'none' : '';
    sel.style.display = coffre ? '' : 'none';
    if(hint) hint.textContent = coffre ? LaRuche.i18n.t('settings.pfKeyVaultHint') : '';
    if(!coffre){ if(_estRefSecret(inp.value)) inp.value = ''; return; }
    var choisi = _nomRefSecret(inp.value);
    fetch(LaRuche.API.base+'/api/secrets').then(function(r){return r.json();}).catch(function(){return {names:[]};})
      .then(function(d){
        var noms = (d.names||[]).filter(function(n){ return n.indexOf('WEBHOOK') !== 0; });
        sel.innerHTML = '<option value="">'+LaRuche.i18n.t('settings.pfKeyPickPrompt')+'</option>'+
          noms.map(function(n){ return '<option value="'+LaRuche.Utils.esc(n)+'"'+(n===choisi?' selected':'')+'>'+LaRuche.Utils.esc(n)+'</option>'; }).join('')+
          '<option value="__new__">'+LaRuche.i18n.t('settings.pfKeyCreateNew')+'</option>';
        secretPickCreate();
      });
  }
  // Mirror the chosen name back into pfApiKey as `${NAME}`; saveProfile keeps reading a
  // single field and needs no knowledge of the two modes.
  function secretPickCreate(){
    var sel = document.getElementById('pfSecretRef'), inp = document.getElementById('pfApiKey');
    if(!sel || !inp) return;
    if(sel.value === '__new__'){
      var nom = (window.prompt(LaRuche.i18n.t('settings.pfKeyNewName'))||'').trim();
      var val = nom ? (window.prompt(LaRuche.i18n.t('settings.pfKeyNewValue', {name:nom}))||'') : '';
      if(!nom || !val){ sel.value = _nomRefSecret(inp.value); return; }
      fetch(LaRuche.API.base+'/api/secrets',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify({name:nom,value:val})})
        .then(function(r){
          if(!r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.secretSaveFailed'),'err'); return; }
          LaRuche.Toast.show(LaRuche.i18n.t('settings.secretSaved'),'ok');
          if(LaRuche.Secrets) LaRuche.Secrets.refresh();
          inp.value = '${'+nom+'}';
          secretPick();                      // reload the list with the new entry selected
        });
      return;
    }
    inp.value = sel.value ? ('${'+sel.value+'}') : '';
  }

  /* Rotate a key: the name is carried over and locked, only the value is asked for. The
   * old value is never sent to the browser in the first place (the list only ever gets
   * names), so there is nothing to hide here - and POST /api/secrets is already an
   * upsert, so replacing needs no new endpoint. */
  function secretUpdate(name){
    var n = document.getElementById('secName'), v = document.getElementById('secVal');
    if(!n || !v) return;
    n.value = name; n.readOnly = true; n.style.opacity = '.6';
    v.value = ''; v.placeholder = LaRuche.i18n.t('settings.secretNewValuePlaceholder');
    var carte = n.closest('.settings-card');
    if(carte){
      var titre = carte.querySelector('.settings-card-title');
      if(titre) titre.textContent = LaRuche.i18n.t('settings.secretRotating', {name:name});
      carte.scrollIntoView({block:'center', behavior:'smooth'});
    }
    v.focus();
  }
  function secretDelete(name){
    fetch(LaRuche.API.base+'/api/secrets/'+encodeURIComponent(name),{method:'DELETE',credentials:'include'})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.secretDeleted'),'ok'); if(LaRuche.Secrets)LaRuche.Secrets.refresh(); refreshTab(); } });
  }

  // Dedicated MCP tab. It used to carry its own add form, a strictly poorer twin of the
  // one in Capabilities: name, command and arguments only, no remote server, no editing,
  // no enable switch. Two forms writing the same file taught the user that one of them
  // was lying. This tab now shows the state and sends every action to the single source.
  function loadMcp(el){
    var html = '<div class="settings-card" style="margin-bottom:16px">';
    html += '  <div class="settings-card-title">'+LaRuche.i18n.t('settings.mcpServersTitle')+'</div>';
    html += '  <div style="color:var(--text-dim);font-size:12px;margin-bottom:12px">'+LaRuche.i18n.t('settings.mcpDesc')+'</div>';
    html += '  <div id="mcp-list" style="margin-bottom:12px"></div>';
    html += '  <div style="border:1px solid var(--border);border-radius:6px;padding:10px;background:var(--bg-panel)">';
    html += '     <div style="color:var(--text-dim);font-size:12px;margin-bottom:8px">'+LaRuche.i18n.t('settings.mcpManageHint')+'</div>';
    html += '     <button class="form-btn" onclick="LaRuche.Settings.gotoMcpCapabilities()">'+LaRuche.i18n.t('settings.mcpManageBtn')+'</button>';
    html += '  </div>';
    html += '</div>';
    // The other direction: LaRuche AS a server. That surface executes the whole registry,
    // shell included, so its door gets its own card rather than a line in a list.
    html += '<div class="settings-card" id="mcp-porte"><div class="settings-card-title">'+LaRuche.i18n.t('settings.mcpDoorTitle')+'</div>'+
      '<div style="color:var(--text-dim);font-size:12px;margin-bottom:12px">'+LaRuche.i18n.t('settings.mcpDoorHint')+'</div>'+
      '<div id="mcp-porte-corps">'+LaRuche.i18n.t('settings.loading')+'</div></div>';
    el.innerHTML = html;
    loadMcpServers();
    loadMcpPorte();
  }

  // The door: server switch, IP allowlist, and who is currently banned.
  async function loadMcpPorte(){
    var corps = document.getElementById('mcp-porte-corps'); if(!corps) return;
    var cfg = await fetch(LaRuche.API.base+'/api/config/curateur',{credentials:'include'})
      .then(function(r){return r.json();}).catch(function(){return {};});
    var bans = await fetch(LaRuche.API.base+'/api/mcp/bans',{credentials:'include'})
      .then(function(r){return r.json();}).catch(function(){return {bans:[]};});
    var liste = (cfg.mcp_allowlist||[]).join('\n');
    var h = '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.mcpServerToggle')+
      '<span style="display:block;color:var(--text-dim);font-size:10px">'+LaRuche.i18n.t('settings.mcpServerHint')+'</span></span>'+
      '<label class="lr-switch"><input type="checkbox" id="mcpPorteOn" '+(cfg.mcp_server?'checked':'')+
      ' onchange="LaRuche.Settings.saveMcpPorte()"><span class="lr-slider"></span></label></div>';
    h += '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.mcpFirewall')+
      '<span style="display:block;color:var(--text-dim);font-size:10px">'+LaRuche.i18n.t('settings.mcpFirewallHint')+'</span></span>'+
      '<label class="lr-switch"><input type="checkbox" id="mcpPareFeuOn" '+(cfg.mcp_firewall?'checked':'')+
      ' onchange="LaRuche.Settings.saveMcpPorte()"><span class="lr-slider"></span></label></div>';
    h += '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.mcpAllowlist')+
      '<span style="display:block;font-weight:normal;color:var(--text-dim);font-size:10px">'+LaRuche.i18n.t('settings.mcpAllowlistHint')+'</span></label>'+
      '<textarea class="form-input" id="mcpAllowlist" rows="4" spellcheck="false" placeholder="127.0.0.1&#10;192.168.1.0/24">'+
      LaRuche.Utils.esc(liste)+'</textarea>'+
      '<button class="form-btn" style="margin-top:8px" onclick="LaRuche.Settings.saveMcpPorte()">'+LaRuche.i18n.t('settings.save')+'</button></div>';
    // Refused calls get an address banned; showing who, and letting it be lifted, is what
    // keeps the protection from turning into a mystery when it catches your own machine.
    h += '<div style="margin-top:14px"><div class="settings-label" style="margin-bottom:6px">'+LaRuche.i18n.t('settings.mcpBans')+'</div>';
    if(!(bans.bans||[]).length){
      h += '<div style="color:var(--text-dim);font-size:12px">'+LaRuche.i18n.t('settings.mcpBansNone')+'</div>';
    } else {
      h += (bans.bans||[]).map(function(b){
        return '<div class="settings-row" style="padding:4px 0"><span class="settings-label" style="flex:1">'+
          LaRuche.Utils.esc(b.ip)+' <span style="color:var(--text-dim);font-size:10px">'+
          LaRuche.i18n.t('settings.mcpBanLeft')+' '+Math.ceil(b.reste_s/60)+' min</span></span>'+
          '<button class="tl-btn" onclick="LaRuche.Settings.mcpUnban(\''+LaRuche.Utils.esc(b.ip)+'\')">'+
          LaRuche.i18n.t('settings.mcpUnban')+'</button></div>';
      }).join('');
    }
    h += '</div>';
    // "An external client will be able to call EVERY tool, shell included" is a warning
    // nobody can act on without knowing WHICH tools. Switched on, the exact surface is
    // spelled out: every name, what it does, and how dangerous it is.
    if(cfg.mcp_server) h += '<div id="mcpExposes" style="margin-top:14px">'+_slot('mcpExposesSlot')+'</div>';
    corps.innerHTML = h;
    if(cfg.mcp_server){
      _fillSlot(corps, 'mcpExposesSlot',
        fetch(LaRuche.API.base+'/api/tools',{credentials:'include'}).then(function(r){return r.json();}),
        _mcpExposesHtml);
    }
  }

  var _DANGER_RANG = { danger:0, dangereux:0, high:0, moderate:1, modere:1, medium:1, safe:2, sur:2 };
  function _mcpExposesHtml(d){
    var outils = (d && (d.tools||d)) || [];
    if(!outils.length) return '<div style="color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('settings.mcpExposedNone')+'</div>';
    // Most dangerous first: the shell is what matters on this list, not an alphabet.
    outils = outils.slice().sort(function(a,b){
      var ra = _DANGER_RANG[String(a.danger||'safe').toLowerCase()];
      var rb = _DANGER_RANG[String(b.danger||'safe').toLowerCase()];
      if(ra === undefined) ra = 1; if(rb === undefined) rb = 1;
      return ra - rb || String(a.name||'').localeCompare(String(b.name||''));
    });
    // MCP now honours disabled_tools, so what is off in Settings > Tools is off here too:
    // the card must list what is REACHABLE, not the whole registry.
    var coupes = outils.filter(function(t){ return t.enabled === false; }).length;
    outils = outils.filter(function(t){ return t.enabled !== false; });
    if(!outils.length) return '<div style="color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('settings.mcpExposedNone')+'</div>';
    var lignes = outils.map(function(t){
      var dg = String(t.danger||'safe').toLowerCase();
      var rang = _DANGER_RANG[dg]; if(rang === undefined) rang = 1;
      var coul = rang===0 ? 'var(--red)' : (rang===1 ? 'var(--amber)' : 'var(--text-dim)');
      return '<div class="mcp-outil">'+
        '<span class="mcp-outil-nom">'+LaRuche.Utils.esc(t.name||'')+'</span>'+
        '<span class="mcp-outil-danger" style="color:'+coul+';border-color:'+coul+'">'+LaRuche.Utils.esc(dg)+'</span>'+
        '<span class="mcp-outil-desc">'+LaRuche.Utils.esc(t.description||'')+'</span>'+
        '</div>';
    }).join('');
    return '<div class="settings-label" style="margin-bottom:4px">'+LaRuche.i18n.t('settings.mcpExposedTitle', {n:outils.length})+'</div>'+
      '<div class="settings-card-desc">'+LaRuche.i18n.t('settings.mcpExposedDesc')+'</div>'+
      (coupes ? '<div class="settings-card-desc">'+LaRuche.i18n.t('settings.mcpExposedOffInfo', {n:coupes})+'</div>' : '')+
      '<div class="mcp-outils">'+lignes+'</div>';
  }

  // Download the captured reviews reshaped for training. The browser does the saving:
  // the endpoint sets Content-Disposition, so no blob juggling here.
  function reineDataset(format){
    var a = document.createElement('a');
    a.href = LaRuche.API.base+'/api/reine/dataset?format='+encodeURIComponent(format);
    a.download = ''; a.style.display = 'none';
    document.body.appendChild(a); a.click();
    setTimeout(function(){ a.remove(); }, 0);
    LaRuche.Toast.show(LaRuche.i18n.t('reine.datasetExporting', {format:format}), 'ok');
  }

  function saveMcpPorte(){
    var on = document.getElementById('mcpPorteOn');
    var pf = document.getElementById('mcpPareFeuOn');
    var ta = document.getElementById('mcpAllowlist');
    fetch(LaRuche.API.base+'/api/config/curateur',{
      method:'POST', credentials:'include', headers:{'Content-Type':'application/json'},
      body: JSON.stringify({
        mcp_server: !!(on && on.checked),
        mcp_firewall: !!(pf && pf.checked),
        mcp_allowlist: ta ? ta.value.split(/[\n,;]+/).map(function(s){return s.trim();}).filter(Boolean) : []
      })
    }).then(function(r){return r.json();})
      .then(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.mcpDoorSaved'),'ok'); loadMcpPorte(); })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon'),'err'); });
  }

  function mcpUnban(ip){
    fetch(LaRuche.API.base+'/api/mcp/bans',{
      method:'POST', credentials:'include', headers:{'Content-Type':'application/json'},
      body: JSON.stringify({ip:ip})
    }).then(function(r){return r.json();})
      .then(function(d){
        LaRuche.Toast.show(d.ok?LaRuche.i18n.t('settings.mcpUnbanned'):LaRuche.i18n.t('settings.errorColon'), d.ok?'ok':'err');
        loadMcpPorte();
      });
  }

  // Jump to Capabilities, already filtered on MCP.
  function gotoMcpCapabilities(){
    LaRuche.Router.go('capabilities');
    setTimeout(function(){ if(LaRuche.Capabilities) LaRuche.Capabilities.showFamily('mcp'); }, 60);
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
        // Read-only: a remote server is named by its endpoint, a local one by its command.
        var detail = s.url ? s.url : ((s.command||'')+' '+(s.args?s.args.join(' '):'')).trim();
        var actif = (s.enabled !== false);
        html += '<div class="settings-row" style="margin-bottom:6px;padding-bottom:6px;border-bottom:1px solid rgba(42,42,46,0.3)">'+
          '<span class="settings-label" style="flex:1">'+LaRuche.Utils.esc(k)+
          ' <span style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t(s.url?'settings.mcpRemote':'settings.mcpLocal')+' - '+LaRuche.Utils.esc(detail)+'</span></span>'+
          '<span style="font-size:10px;color:'+(actif?'var(--green)':'var(--text-dim)')+'">'+LaRuche.i18n.t(actif?'settings.mcpOn':'settings.mcpOff')+'</span></div>';
      }
      if(!html) html = '<div style="color:var(--text-dim);font-size:12px;padding:8px">'+LaRuche.i18n.t('settings.mcpNone')+'</div>';
      el.innerHTML = html;
    } catch(e) {}
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
    // Kept for the inline editor, which prefills from the task it is opened on, and to
    // reload the list after a save.
    window._cronTasks = tasks;
    window._cronHost = el;
    
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.defaultModel')+'</option>';
    Object.keys(profiles).forEach(function(k){
        profOpts += '<option value="'+k+'">'+LaRuche.Utils.esc(profiles[k].name)+'</option>';
    });

    el.innerHTML='<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="document.getElementById(\'newCronForm\').style.display=\'block\'">'+LaRuche.i18n.t('settings.newTaskBtn')+'</button></div>'+
      '<div id="newCronForm" style="display:none" class="settings-card">'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.nameLabel')+'</label><input id="ncName" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.promptLabel')+'</label><input id="ncPrompt" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.bpScheduleLabel')+'</label><div id="ncCronBuilder"></div></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherChannelLabel')+'</label><select id="ncChannel" class="form-input"></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.providerLabel')+'</label><select id="ncProfileId" class="form-input" onchange="LaRuche.Settings.updateCronModelSelect()"></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.modelLabel')+'</label><select id="ncModel" class="form-input"><option value="">'+LaRuche.i18n.t('settings.providerDefault')+'</option></select></div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.createCron()">'+LaRuche.i18n.t('settings.createBtn')+'</button></div>'+
      (tasks.length
        ? '<div style="margin:22px 0 10px;padding-top:16px;border-top:1px solid var(--border);">'+
            '<span style="font-size:10px;text-transform:uppercase;letter-spacing:.5px;color:var(--text-dim)">'+
            LaRuche.i18n.t('settings.cronListTitle')+' ('+tasks.length+')</span></div>'
        : '')+
      tasks.map(function(t){ return cronCarte(t, profiles); }).join('');
    // Human-friendly cron builder for the creation form.
    if(LaRuche.CronBuilder){ _ncCronBuilderId = LaRuche.CronBuilder.mount('ncCronBuilder', { value:'' }); }
    // Real channels, like the four other forms: the hardcoded telegram|discord pair
    // ignored Slack even once configured, and could not offer memory.
    window.__fillChannels(document.getElementById('ncChannel'), '', LaRuche.i18n.t('settings.cronChannelNone'));
    // Providers: the local profiles AND the swarm nodes, which held models nothing
    // outside the chat could reach.
    window.__fillProviders(document.getElementById('ncProfileId'), '', profiles);
  }
  // Schedule in words, with the raw expression kept beside it in small type. A card that
  // only showed `0 9 * * *` asked the reader to parse cron in their head.
  function cronLisible(t){
    var brut = t.cron_expr || t.fire_at || '';
    if(!brut) return '<span class="settings-value">-</span>';
    var mots = '';
    if(t.cron_expr && LaRuche.Timeline && LaRuche.Timeline.humanCron){
      try { mots = LaRuche.Timeline.humanCron(t.cron_expr) || ''; } catch(e){ mots = ''; }
    }
    if(!mots) return '<span class="settings-value">'+LaRuche.Utils.esc(brut)+'</span>';
    return '<span class="settings-value">'+LaRuche.Utils.esc(mots)+
      ' <code style="color:var(--text-muted);font-size:10px">'+LaRuche.Utils.esc(brut)+'</code></span>';
  }

  // Prompt preview: one line, truncated, the whole text on hover. It is the field that
  // says what the task actually DOES, and the card did not show it at all.
  function cronApercuPrompt(t){
    var p = (t.prompt || '').trim();
    if(!p) return '';
    var coupe = p.length > 110 ? p.slice(0, 109) + '…' : p;
    return '<div class="settings-row"><span class="settings-label">'+LaRuche.i18n.t('settings.promptLabel')+'</span>'+
      '<span class="settings-value" title="'+LaRuche.Utils.esc(p)+'" '+
      'style="max-width:60%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;cursor:help">'+
      LaRuche.Utils.esc(coupe)+'</span></div>';
  }

  function cronProviderLisible(t, profiles){
    var eff = LaRuche.i18n.t('settings.watcherDefaut');
    if(t.profile_id && profiles[t.profile_id]) eff = profiles[t.profile_id].name;
    else if(t.profile_id) eff = t.profile_id;
    else if(t.provider) eff = t.provider + (t.model ? ' / ' + t.model : '');
    else if(t.model) eff = t.model;
    if(t.profile_id && t.model) eff += ' (' + t.model + ')';
    return eff;
  }

  function cronCarte(t, profiles){
    var esc = LaRuche.Utils.esc;
    var ligne = function(label, valeur){
      return '<div class="settings-row"><span class="settings-label">'+label+'</span>'+valeur+'</div>';
    };
    return '<div class="settings-card" data-cron="'+esc(t.id)+'">'+
      '<div style="display:flex;justify-content:space-between;align-items:flex-start;gap:8px">'+
        '<div class="settings-card-title" style="flex:1">'+esc(t.name)+'</div>'+
        '<div style="display:flex;gap:6px;flex:0 0 auto">'+
          '<button class="tl-btn tl-btn--active" onclick="LaRuche.Settings.lancerCronTask(\''+esc(t.id)+'\')">'+LaRuche.i18n.t('settings.tlRunNow')+'</button>'+
          '<button class="tl-btn" onclick="LaRuche.Settings.editCronTask(\''+esc(t.id)+'\')">'+LaRuche.i18n.t('settings.editBtn')+'</button>'+
          '<button class="tl-btn tl-btn--danger" onclick="LaRuche.Settings.deleteCronTask(\''+esc(t.id)+'\',this)">'+LaRuche.i18n.t('settings.deleteBtn')+'</button>'+
        '</div>'+
      '</div>'+
      ligne(LaRuche.i18n.t('settings.scheduleLabel'), cronLisible(t))+
      cronApercuPrompt(t)+
      ligne(LaRuche.i18n.t('settings.runsLabel'), '<span class="settings-value">'+(t.run_count||0)+'</span>')+
      ligne(LaRuche.i18n.t('settings.channelLabelShort'), '<span class="settings-value">'+esc(t.channel||LaRuche.i18n.t('settings.channelNone'))+'</span>')+
      ligne(LaRuche.i18n.t('settings.providerModelLabel'), '<span class="settings-value">'+esc(cronProviderLisible(t, profiles))+'</span>')+
      '<div class="cron-edit" style="display:none"></div>'+
    '</div>';
  }

  // Inline editor, opened in the card itself: the same six fields as creation, prefilled.
  // PUT /api/cron/:id already accepted them all; nothing in the interface reached it.
  /* Lancer une tache planifiee sans attendre son heure. Une erreur de prompt se
     corrigeait sinon en attendant le lendemain matin. */
  function lancerCronTask(id){
    LaRuche.Toast.show(LaRuche.i18n.t('settings.tlRunStarting'), 'info');
    fetch(LaRuche.API.base+'/api/cron/'+encodeURIComponent(id)+'/run', {method:'POST'})
      .then(function(r){ return r.json().catch(function(){ return {}; }); })
      .then(function(d){
        // Le motif du refus, pas une phrase generique: un lancement refuse faute
        // de droits ne doit pas ressembler a un lancement parti.
        if(d && d.status==='started'){ LaRuche.Toast.show(LaRuche.i18n.t('settings.tlRunning'), 'ok'); }
        else { LaRuche.Toast.show((d && d.error) ? String(d.error) : LaRuche.i18n.t('settings.tlFailed'), 'err'); }
      })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.tlFailed'), 'err'); });
  }

  function editCronTask(id){
    var carte = document.querySelector('[data-cron="'+id+'"]');
    var zone = carte && carte.querySelector('.cron-edit');
    if(!zone) return;
    if(zone.style.display !== 'none'){ zone.style.display='none'; zone.innerHTML=''; return; }
    var t = (window._cronTasks||[]).filter(function(x){ return String(x.id)===String(id); })[0];
    if(!t) return;
    var profiles = window._lastProfiles || {};
    var esc = LaRuche.Utils.esc;
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.defaultModel')+'</option>';
    Object.keys(profiles).forEach(function(k){
      profOpts += '<option value="'+esc(k)+'"'+(t.profile_id===k?' selected':'')+'>'+esc(profiles[k].name)+'</option>';
    });

    zone.style.display='';
    zone.innerHTML = '<div style="margin-top:10px;padding-top:10px;border-top:1px solid var(--border)">'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.nameLabel')+'</label><input id="ecName_'+esc(id)+'" class="form-input" value="'+esc(t.name||'')+'"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.promptLabel')+'</label><textarea id="ecPrompt_'+esc(id)+'" class="form-input" rows="3" style="resize:vertical">'+esc(t.prompt||'')+'</textarea></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.bpScheduleLabel')+'</label><div id="ecCron_'+esc(id)+'"></div></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherChannelLabel')+'</label><select id="ecChannel_'+esc(id)+'" class="form-input"></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.providerLabel')+'</label><select id="ecProfile_'+esc(id)+'" class="form-input">'+profOpts+'</select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.modelLabel')+'</label><input id="ecModel_'+esc(id)+'" class="form-input" placeholder="'+LaRuche.i18n.t('settings.providerDefault')+'" value="'+esc(t.model||'')+'"></div>'+
      '<div style="display:flex;gap:8px">'+
        '<button class="tl-btn tl-btn--active" onclick="LaRuche.Settings.saveCronTask(\''+esc(id)+'\')">'+LaRuche.i18n.t('common.save')+'</button>'+
        '<button class="tl-btn" onclick="LaRuche.Settings.editCronTask(\''+esc(id)+'\')">'+LaRuche.i18n.t('common.cancel')+'</button>'+
      '</div></div>';
    // Same source as everywhere else, and it preselects the channel the task already has.
    window.__fillChannels(document.getElementById('ecChannel_'+id), t.channel||'', LaRuche.i18n.t('settings.cronChannelNone'));
    // A task can already run on a swarm node: its provider is preselected, then its models.
    window.__fillProviders(document.getElementById('ecProfile_'+id), t.profile_id || t.provider || '', profiles)
      .then(function(){ window.__fillModels(t.profile_id || t.provider || '', document.getElementById('ecModelSel_'+id), t.model||'', profiles); });
    if(LaRuche.CronBuilder){
      zone._builderId = LaRuche.CronBuilder.mount('ecCron_'+id, { value: t.cron_expr || '' });
    }
  }

  function saveCronTask(id){
    var carte = document.querySelector('[data-cron="'+id+'"]');
    var zone = carte && carte.querySelector('.cron-edit');
    if(!zone) return;
    var val = function(prefixe){ var e = document.getElementById(prefixe+'_'+id); return e ? e.value : ''; };
    var expr = '';
    if(LaRuche.CronBuilder && zone._builderId) expr = LaRuche.CronBuilder.getValue(zone._builderId) || '';
    var corps = {
      name: val('ecName').trim(),
      prompt: val('ecPrompt').trim(),
      cron_expr: expr,
      channel: val('ecChannel'),
      // A swarm node is a provider, not a profile: sending it as profile_id would look
      // up a profile that does not exist and silently fall back to the default.
      profile_id: val('ecProfile').indexOf('peer:') === 0 ? '' : val('ecProfile'),
      provider: val('ecProfile').indexOf('peer:') === 0 ? val('ecProfile') : '',
      model: val('ecModelSel')
    };
    if(!corps.name || !corps.prompt){ LaRuche.Toast.show(LaRuche.i18n.t('settings.cronNamePromptRequired'),'err'); return; }
    fetch('/api/cron/'+encodeURIComponent(id), {
      method:'PUT', headers:{'Content-Type':'application/json'}, body:JSON.stringify(corps)
    }).then(function(r){ return r.json(); }).then(function(d){
      if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
      LaRuche.Toast.show(LaRuche.i18n.t('settings.cronUpdated'),'ok');
      if(window._cronHost) loadCron(window._cronHost);
    }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); });
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
  
  // One helper for both kinds of provider: a local profile lists its models, a swarm node
  // lists the ones it actually holds.
  function updateCronModelSelect() {
      var profSel = document.getElementById('ncProfileId');
      var modSel = document.getElementById('ncModel');
      if(!profSel || !modSel) return;
      window.__fillModels(profSel.value, modSel, '', window._lastProfiles);
  }
  // Same, for the editor opened inside a task card.
  function majModelesEdition(id) {
    var profSel = document.getElementById('ecProfile_'+id);
    var modSel = document.getElementById('ecModelSel_'+id);
    if(!profSel || !modSel) return;
    window.__fillModels(profSel.value, modSel, '', window._lastProfiles);
  }
  function createCron() {
    var name=document.getElementById('ncName').value;
    var prompt=document.getElementById('ncPrompt').value;
    var cron=(_ncCronBuilderId && LaRuche.CronBuilder) ? LaRuche.CronBuilder.getValue(_ncCronBuilderId) : '';
    var channel=document.getElementById('ncChannel').value;
    var profile_id=document.getElementById('ncProfileId').value;
    var model=document.getElementById('ncModel').value;
    
    var payload = {name:name,prompt:prompt,cron_expr:cron,channel:channel||null};
    // A swarm node is a provider, not a profile: sent as profile_id it would resolve to
    // nothing and the task would silently run on the default model instead.
    if(profile_id.indexOf('peer:') === 0) payload.provider = profile_id;
    else if(profile_id) payload.profile_id = profile_id;
    if(model) payload.model = model;
    
    fetch('/api/cron',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)}).then(function(){loadTab('cron');LaRuche.Toast.show(LaRuche.i18n.t('settings.cronTaskCreated'),'ok');});
  }

  async function loadWatchers(el) {
    // The watchers view is embedded by TWO pages (Settings and Automations):
    // remember the real container so in-place refreshes work from both.
    _watchersEl = el;
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
    el.innerHTML='<div id="watchersGraph"></div>'+
      '<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="document.getElementById(\'newWatcherForm\').style.display=\'block\'">'+LaRuche.i18n.t('settings.newWatcherBtn')+'</button></div>'+
      '<div id="newWatcherForm" style="display:none" class="settings-card">'+
      '<div style="font-weight:600;margin-bottom:8px">'+LaRuche.i18n.t('settings.newWatcherTitle')+'</div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.nameLabel')+'</label><input id="nwName" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherTypeLabel')+'</label><select id="nwType" class="form-input"><option value="file">'+LaRuche.i18n.t('settings.watcherTypeFile')+'</option><option value="url">'+LaRuche.i18n.t('settings.watcherTypeUrl')+'</option><option value="log">'+LaRuche.i18n.t('settings.watcherTypeLog')+'</option>'+'<option value="command">'+LaRuche.i18n.t('settings.watcherTypeCommand')+'</option>'+'</select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherTargetField')+'</label><input id="nwTarget" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherCondField')+'</label><input id="nwCondition" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.promptLabel')+'</label><input id="nwPrompt" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.providerLabel')+'</label><select id="watcher-profile" class="form-input" onchange="LaRuche.Settings.updateWatcherModelSelect()">'+profOpts+'</select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.modelLabel')+'</label><select id="watcher-model" class="form-input"><option value="">'+LaRuche.i18n.t('settings.parDefault')+'</option></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherChannelLabel')+'</label><select id="nwChannel" class="form-input"><option value="">'+LaRuche.i18n.t('settings.watcherHomeChannel')+'</option></select></div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.createWatcher()">'+LaRuche.i18n.t('settings.createBtn')+'</button></div>'+
      watchers.map(function(w){ return renderWatcherCard(w, profiles); }).join('');
    window.__fillChannels(document.getElementById('nwChannel'), '', LaRuche.i18n.t('settings.watcherHomeChannel'));
    // Channel selectors of the expanded cards need the live channel list too.
    watchers.forEach(function(w){
      if(_watcherOpen[w.id]){
        var c=document.getElementById('wf-chan-'+w.id);
        if(c) window.__fillChannels(c, w.channel||'', LaRuche.i18n.t('settings.watcherHomeChannel'));
      }
    });
    // Correlation graph. It draws nothing unless a watcher reads another's verdict, so
    // an ordinary setup sees exactly what it saw before.
    if(window.LaRuche && LaRuche.WatchersGraph){
      LaRuche.WatchersGraph.render(document.getElementById('watchersGraph'), watchers);
    }
  }

  // Compact human summary of a compiled rules tree (mirror of Regle::resume()).
  function resumeRegle(r){
    if(!r||!r.op) return '';
    switch(r.op){
      case 'et': return 'ET('+(r.regles||[]).map(resumeRegle).join(', ')+')';
      case 'ou': return 'OU('+(r.regles||[]).map(resumeRegle).join(', ')+')';
      case 'non': return 'NON('+resumeRegle(r.regle||{})+')';
      case 'jour_semaine': return 'jour∈['+(r.jours||[]).join(',')+']';
      case 'heure_entre': return (r.de||'?')+'-'+(r.a||'?');
      case 'plage_date': return (r.du||'?')+'..'+(r.au||'?');
      case 'apparu': return 'apparu';
      case 'supprime': return 'supprimé';
      case 'modifie': return 'modifié';
      case 'contenu_change': return 'contenu≠';
      case 'est_down': return 'down';
      case 'down_depuis_min': return 'down≥'+(r.minutes||0)+'min';
      case 'retour_en_ligne': return 'retour en ligne';
      case 'contient': return 'contient « '+(r.motif||'')+' »';
      case 'taille_depasse_mo': return 'taille≥'+(r.mo||0)+'Mo';
      case 'status_http': return 'http∈['+(r.codes||[]).join(',')+']';
      case 'llm_check': return '🧠« '+(r.question||'')+' »';
      default: return r.op;
    }
  }

  // Health dot + one-line synthesis derived from the watcher's persisted state.
  function watcherEtat(w){
    var t=LaRuche.i18n.t, ls=w.last_state||'';
    if(ls.indexOf('down:')===0) return {cls:'down', txt:t('settings.wfDown')+' · '+ls.slice(5,21).replace('T',' ')};
    if(ls==='absent') return {cls:'idle', txt:t('settings.wfAbsent')};
    if(ls.indexOf('present:')===0) return {cls:'up', txt:t('settings.wfPresent')};
    if(w.watcher_type==='log' && ls) return {cls:'log', txt:'offset '+ls};
    if(ls) return {cls:'up', txt:t('settings.wfUp')};
    return {cls:'idle', txt:t('settings.wfNoBaseline')};
  }

  // A watcher card: collapsed = synthesis line; expanded = the execution loop as
  // a bubble pipeline where each bubble IS the edit field (observe -> every ->
  // condition gate -> cooldown -> action -> deliver -> back to observe).
  function renderWatcherCard(w, profiles){
    var open=!!_watcherOpen[w.id], etat=watcherEtat(w), esc=LaRuche.Utils.esc, t=LaRuche.i18n.t;
    var ivDef=(w.watcher_type==='url')?60:10, cdDef=(w.watcher_type==='url')?900:0;
    var head='<div class="wcard-head" onclick="LaRuche.Settings.toggleWatcherCard(\''+w.id+'\')">'+
      '<span class="wdot '+etat.cls+'"></span>'+
      '<span class="wcard-name">'+esc(w.name||'?')+'</span>'+
      '<span class="wcard-type">'+esc(w.watcher_type||'')+'</span>'+
      (w.sustained?'<span class="wcard-sust" title="'+t('settings.wfSustained')+'">⟳</span>':'')+
      (w.active===false?'<span class="wcard-sust" style="border-color:var(--red);color:var(--red)">OFF</span>':'')+
      '<span class="wcard-synth">'+esc(w.target||'')+' · '+(w.interval_secs||ivDef)+'s · '+(w.run_count||0)+t('automations.runsSuffix')+'</span>'+
      '<span class="wcard-chev">▶</span></div>';
    if(!open) return '<div class="wcard">'+head+'</div>';
    var id=w.id;
    function opt(v,label,cur){ return '<option value="'+v+'"'+(cur===v?' selected':'')+'>'+label+'</option>'; }
    var typeSel=opt('file',t('settings.watcherTypeFile'),w.watcher_type)+opt('url',t('settings.watcherTypeUrl'),w.watcher_type)+opt('log',t('settings.watcherTypeLog'),w.watcher_type)+opt('command',t('settings.watcherTypeCommand'),w.watcher_type);
    var profOpts='<option value="">'+t('settings.parDefault')+'</option>';
    Object.keys(profiles).forEach(function(k){ profOpts+='<option value="'+esc(k)+'"'+((w.profile_id===k)?' selected':'')+'>'+esc(profiles[k].name||k)+'</option>'; });
    var modOpts='<option value="">'+t('settings.parDefault')+'</option>';
    if(w.profile_id && profiles[w.profile_id] && profiles[w.profile_id].models){
      profiles[w.profile_id].models.forEach(function(m){ modOpts+='<option value="'+esc(m)+'"'+((w.model===m)?' selected':'')+'>'+esc(m)+'</option>'; });
    }
    var fleche='<span class="warrow">→</span>';
    var condTitre=(w.watcher_type==='log')?('🔎 '+t('settings.wfPattern')):('🧠 '+t('settings.wfCondition'));
    // Compiled rules take over the condition bubble: summary line (auditable at
    // a glance) + the JSON tree, editable in place. Clearing the JSON falls back
    // to the legacy text condition.
    var condCorps;
    if(w.regles){
      condCorps='<div class="wnode-sub" style="color:var(--green);margin:0 0 4px" title="'+t('settings.wfReglesHint')+'">⚙ '+esc(resumeRegle(w.regles))+'</div>'+
        '<textarea id="wf-regles-'+id+'" rows="3" style="font-family:var(--mono);font-size:10px">'+esc(JSON.stringify(w.regles))+'</textarea>'+
        '<div class="wnode-sub">'+t('settings.wfReglesHint')+'</div>';
      condTitre='⚙ '+t('settings.wfRegles');
    } else {
      condCorps='<textarea id="wf-cond-'+id+'" rows="2">'+esc(w.condition||'')+'</textarea>'+
        '<label class="wnode-sub" style="display:flex;align-items:center;gap:4px;cursor:pointer"><input type="checkbox" id="wf-sust-'+id+'" style="width:auto"'+(w.sustained?' checked':'')+'> ⟳ '+t('settings.wfSustained')+'</label>'+
        '<div class="wnode-sub">'+t('settings.wfCondHint')+'</div>';
    }
    var flow='<div class="wflow">'+
      '<div class="wnode" style="max-width:150px"><div class="wnode-title">🏷 '+t('settings.wfName')+'</div><input id="wf-name-'+id+'" value="'+esc(w.name||'')+'"></div>'+fleche+
      '<div class="wnode" style="flex:2"><div class="wnode-title">👁 '+t('settings.wfObserve')+'</div><select id="wf-type-'+id+'" style="margin-bottom:4px">'+typeSel+'</select><input id="wf-target-'+id+'" value="'+esc(w.target||'')+'"></div>'+fleche+
      '<div class="wnode" style="max-width:110px"><div class="wnode-title">⏱ '+t('settings.wfEvery')+'</div><input id="wf-iv-'+id+'" type="number" min="5" placeholder="'+ivDef+'" value="'+(w.interval_secs||'')+'"><div class="wnode-sub">'+t('settings.wfEveryHint')+'</div></div>'+fleche+
      '<div class="wnode" style="flex:2"><div class="wnode-title">'+condTitre+'</div>'+condCorps+'</div>'+fleche+
      '<div class="wnode" style="max-width:110px"><div class="wnode-title">⏳ '+t('settings.wfCooldown')+'</div><input id="wf-cd-'+id+'" type="number" min="0" placeholder="'+cdDef+'" value="'+(w.cooldown_secs!=null?w.cooldown_secs:'')+'"><div class="wnode-sub">'+t('settings.wfCooldownHint')+'</div></div>'+fleche+
      '<div class="wnode" style="flex:2"><div class="wnode-title">🚀 '+t('settings.wfAction')+'</div><textarea id="wf-prompt-'+id+'" rows="2">'+esc(w.prompt||'')+'</textarea></div>'+fleche+
      '<div class="wnode" style="max-width:170px"><div class="wnode-title">📨 '+t('settings.wfDeliver')+'</div>'+
        '<select id="wf-chan-'+id+'" style="margin-bottom:4px"><option value="">'+t('settings.watcherHomeChannel')+'</option></select>'+
        '<select id="wf-prof-'+id+'" style="margin-bottom:4px" onchange="LaRuche.Settings.updateWatcherCardModelSelect(\''+id+'\')">'+profOpts+'</select>'+
        '<select id="wf-model-'+id+'">'+modOpts+'</select></div>'+
      '<span class="warrow wloop" title="'+t('settings.wfLoop')+'">↺</span>'+
      '</div>';
    var foot='<div class="wcard-foot">'+
      '<span class="wcard-state" title="'+esc(w.last_state||'')+'">'+esc(etat.txt)+'</span>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveWatcherEdit(\''+id+'\')">'+t('settings.watcherSave')+'</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.toggleWatcherActive(\''+id+'\','+(w.active===false?'true':'false')+')">'+(w.active===false?t('settings.wfResume'):t('settings.wfPause'))+'</button>'+
      '<button class="tl-btn tl-btn--danger" onclick="fetch(\'/api/watchers/'+id+'\',{method:\'DELETE\'}).then(function(){LaRuche.Settings.rechargerWatchers()})">'+t('settings.deleteWatcherBtn')+'</button>'+
      '</div>';
    return '<div class="wcard open">'+head+flow+foot+'</div>';
  }

  // In-place refresh that works from Settings AND Automations (loadTab only
  // targets the Settings container and silently no-ops elsewhere).
  function rechargerWatchers(){
    var el=(_watchersEl&&document.body.contains(_watchersEl))?_watchersEl:document.getElementById('settingsContent');
    if(el) loadWatchers(el);
  }

  function toggleWatcherCard(id){ _watcherOpen[id]=!_watcherOpen[id]; rechargerWatchers(); }

  function toggleWatcherActive(id, active){
    fetch(LaRuche.API.base+'/api/watchers/'+id,{method:'PATCH',headers:{'Content-Type':'application/json'},body:JSON.stringify({active:active})})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('toast.saved'),'ok'); rechargerWatchers(); } else LaRuche.Toast.show(LaRuche.i18n.t('toast.failed'),'err'); });
  }

  function updateWatcherCardModelSelect(id){
    var pId=document.getElementById('wf-prof-'+id).value, sel=document.getElementById('wf-model-'+id);
    if(!sel) return;
    sel.innerHTML='<option value="">'+LaRuche.i18n.t('settings.parDefault')+'</option>';
    if(pId && _profiles[pId] && _profiles[pId].models){ _profiles[pId].models.forEach(function(m){ sel.innerHTML+='<option value="'+LaRuche.Utils.esc(m)+'">'+LaRuche.Utils.esc(m)+'</option>'; }); }
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
    fetch('/api/watchers',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(function(){rechargerWatchers();LaRuche.Toast.show(LaRuche.i18n.t('settings.watcherCreated'),'ok');});
  }

  // Inline watcher editing (parity with cron/kanban).
  // The pipeline card replaced the old modal editor: editing happens in place.
  function editWatcher(id) {
    _watcherOpen[id]=true;
    rechargerWatchers();
  }

  function updateWatcherEditModelSelect() { /* kept for export compat; card variant below */ }

  function saveWatcherEdit(id) {
    function v(k){ var e=document.getElementById('wf-'+k+'-'+id); return e?e.value:''; }
    var sust=document.getElementById('wf-sust-'+id);
    var body={
      name: v('name'),
      watcher_type: v('type'),
      target: v('target'),
      prompt: v('prompt'),
      sustained: !!(sust&&sust.checked),
      interval_secs: parseInt(v('iv'),10)||0,   // 0/empty = back to the type default
      cooldown_secs: parseInt(v('cd'),10)||0,
      profile_id: v('prof'),
      model: v('model'),
      channel: v('chan')
    };
    // Compiled-rules bubble: parse the JSON tree (empty = clear, back to legacy).
    var reglesEl=document.getElementById('wf-regles-'+id);
    if(reglesEl){
      var txt=reglesEl.value.trim();
      if(!txt){ body.regles=null; }
      else {
        try{ body.regles=JSON.parse(txt); }
        catch(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.wfReglesInvalid'),'err'); return; }
      }
    } else {
      body.condition=v('cond');
    }
    fetch(LaRuche.API.base+'/api/watchers/'+id,{method:'PATCH',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.watcherSaved'),'ok'); rechargerWatchers(); } else { LaRuche.Toast.show(LaRuche.i18n.t('settings.watcherSaveFailed'),'err'); } });
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

  /* A channel token is a secret exactly like a provider API key, and it was sitting in
   * plain sight in the form (and in plaintext in channels-config.json). Same two modes:
   * type it, or point at a vault entry. Picking one stores `${NAME}`, which the node
   * resolves at send time via secrets::substituer. */
  function _champJeton(id, libelle, valeur, exemple){
    var ref = _estRefSecret(valeur);
    return '<div class="form-group"><label class="form-label">'+LaRuche.Utils.esc(libelle)+'</label>'+
      '<select class="form-input" id="'+id+'-mode" style="width:auto;padding:4px 8px;margin-bottom:6px" data-jeton="'+id+'">'+
        '<option value="manual"'+(ref?'':' selected')+'>'+LaRuche.i18n.t('settings.pfKeyModeManual')+'</option>'+
        '<option value="vault"'+(ref?' selected':'')+'>'+LaRuche.i18n.t('settings.pfKeyModeVault')+'</option>'+
      '</select>'+
      '<input class="form-input" type="password" id="'+id+'" value="'+LaRuche.Utils.esc(valeur)+'" placeholder="'+LaRuche.Utils.esc(exemple)+'" autocomplete="off"'+(ref?' style="display:none"':'')+'>'+
      '<select class="form-input" id="'+id+'-ref"'+(ref?'':' style="display:none"')+'></select></div>';
  }
  // Wired after render for every token field on the page.
  function _brancherJetons(el){
    el.querySelectorAll('[data-jeton]').forEach(function(sel){
      var id = sel.dataset.jeton;
      var maj = function(){ _basculeJeton(id); };
      sel.onchange = maj;
      var r = el.querySelector('#'+id+'-ref');
      if(r) r.onchange = function(){
        var inp = el.querySelector('#'+id);
        if(inp) inp.value = r.value ? ('${'+r.value+'}') : '';
      };
      if(sel.value === 'vault') maj();
    });
  }
  function _basculeJeton(id){
    var sel = document.getElementById(id+'-mode'), inp = document.getElementById(id),
        ref = document.getElementById(id+'-ref');
    if(!sel || !inp || !ref) return;
    var coffre = sel.value === 'vault';
    inp.style.display = coffre ? 'none' : '';
    ref.style.display = coffre ? '' : 'none';
    if(!coffre){ if(_estRefSecret(inp.value)) inp.value = ''; return; }
    var choisi = _nomRefSecret(inp.value);
    fetch(LaRuche.API.base+'/api/secrets').then(function(r){return r.json();}).catch(function(){return {names:[]};})
      .then(function(d){
        ref.innerHTML = '<option value="">'+LaRuche.i18n.t('settings.pfKeyPickPrompt')+'</option>'+
          (d.names||[]).map(function(n){ return '<option value="'+LaRuche.Utils.esc(n)+'"'+(n===choisi?' selected':'')+'>'+LaRuche.Utils.esc(n)+'</option>'; }).join('');
        if(!choisi) inp.value = '';
      });
  }

  async function loadChannels(el) {
    // Three independent endpoints: one round trip, not three in a row.
    var _ch = await Promise.all([
      _gj('/api/config/channels'), _gj('/api/config/notify')
    ]);
    var config = _ch[0], notify = _ch[1];
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
    // Internal usages: same override map, same selector, but they are places where LaRuche
    // thinks rather than places where it speaks, so they get their own card.
    function usageRows(){
      var libelles = {
        'consolidation': LaRuche.i18n.t('settings.usageConsolidation'),
        'memory-enrich': LaRuche.i18n.t('settings.usageMemoryEnrich'),
        'feed':          LaRuche.i18n.t('settings.usageFeed')
      };
      var aides = {
        'consolidation': LaRuche.i18n.t('settings.usageConsolidationHint'),
        'memory-enrich': LaRuche.i18n.t('settings.usageMemoryEnrichHint'),
        'feed':          LaRuche.i18n.t('settings.usageFeedHint')
      };
      return (chmodels.usages||[]).map(function(u){
        return '<div class="form-group"><label class="form-label">'+LaRuche.Utils.esc(libelles[u]||u)+
          '<span style="display:block;font-weight:normal;color:var(--text-dim);font-size:10px">'+LaRuche.Utils.esc(aides[u]||'')+'</span></label>'+
          chModelSel(u)+'</div>';
      }).join('');
    }
    el.innerHTML = '<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:16px">' +
      '<div class="settings-card"><div class="card-title" style="color:var(--purple)">'+LaRuche.i18n.t('settings.usageModelsTitle')+'</div>' +
        '<div style="font-size:11px;color:var(--text-dim);margin-bottom:8px">'+LaRuche.i18n.t('settings.usageModelsHint')+'</div>' +
        usageRows() + '</div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--amber)">'+LaRuche.i18n.t('settings.notificationsTitle')+'</div>' +
        '<div style="font-size:11px;color:var(--text-dim);margin-bottom:8px">'+LaRuche.i18n.t('settings.notifyHint')+'</div>' +
        '<label style="display:flex;align-items:center;gap:8px;cursor:pointer"><input type="checkbox" id="ch-notify-en" '+(notify.enabled?'checked':'')+'> <span>'+LaRuche.i18n.t('settings.notifyLabel')+'</span></label></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--blue)">Telegram</div>' +
        _champJeton('ch-tg-token', LaRuche.i18n.t('settings.botTokenLabel'), tg.bot_token||'', '7123456789:AAH...') +
        '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.tgAllowedChats')+'</label><input class="form-input" id="ch-tg-chats" value="'+LaRuche.Utils.esc(tg.allowed_chats||'')+'" placeholder="'+LaRuche.i18n.t('settings.chAllowedChats')+'"></div>' +
        '<div style="font-size:10px;color:var(--text-muted);margin-top:4px">'+LaRuche.i18n.t('settings.chTgLaunch')+'</div></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--purple)">Discord</div>' +
        _champJeton('ch-dc-token', LaRuche.i18n.t('settings.botTokenLabel'), dc.bot_token||'', 'MTIxxx...') +
        '<div class="form-group"><label class="form-label">'+LaRuche.i18n.t('settings.dcAllowedChannels')+'</label><input class="form-input" id="ch-dc-channels" value="'+LaRuche.Utils.esc(dc.allowed_channels||'')+'" placeholder="'+LaRuche.i18n.t('settings.chAllowedChats')+'"></div>' +
        '<div style="font-size:10px;color:var(--text-muted);margin-top:4px">'+LaRuche.i18n.t('settings.chDcLaunch')+'</div></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--green)">Slack</div>' +
        _champJeton('ch-sl-bot', LaRuche.i18n.t('settings.slBotToken'), sl.bot_token||'', 'xoxb-...') +
        _champJeton('ch-sl-app', LaRuche.i18n.t('settings.slAppToken'), sl.app_token||'', 'xapp-...') +
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
    // Every token field gets its mode switch and its vault picker.
    _brancherJetons(el);
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
        '<button class="tl-btn tl-btn--danger" onclick="if(confirm(LaRuche.i18n.t(\'settings.confirmDeleteSkill\',{name:LaRuche.Utils.esc(s.name)})))LaRuche.Settings.deleteSkill(\''+LaRuche.Utils.esc(s.name)+'\')">'+LaRuche.i18n.t('settings.skillDelBtn')+'</button>'+
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
    ov.innerHTML='<div style="width:680px;max-width:94vw;height:80vh;background:var(--bg-panel);border:1px solid var(--amber);border-radius:10px;display:flex;flex-direction:column">'+
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
    // Unified model: {name, group, desc}. group is a key, translated at render time.
    var items = [];
    var seen = {};
    (tools||[]).forEach(function(t){
      var n=t.name||t; if(seen[n])return; seen[n]=1;
      items.push({name:n, group:(pluginNames.indexOf(n)>=0?'plugins':'tools'), desc:(t.description||'')});
    });
    pluginNames.forEach(function(n){ if(!seen[n]){ seen[n]=1; items.push({name:n, group:'plugins', desc:''}); } });
    var m = content.match(/^\s*(?:allowed-)?tools:\s*\[([^\]]*)\]/m);
    var current = m ? m[1].split(',').map(function(s){return s.trim().replace(/['"]/g,'');}).filter(Boolean) : [];
    current.forEach(function(n){ if(!seen[n]){ seen[n]=1; items.push({name:n, group:'other', desc:LaRuche.i18n.t('settings.skillToolsRef')}); } });
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
    var groups=['tools','plugins','other']; var html='';
    var etiquette={tools:'settings.skillGroupTools',plugins:'settings.skillGroupPlugins',other:'settings.skillGroupOther'};
    groups.forEach(function(g){
      var list=items.filter(function(it){return it.group===g && (!f || it.name.toLowerCase().indexOf(f)>=0);});
      if(!list.length) return;
      // selected first, then alpha
      list.sort(function(a,b){ var ca=checked[a.name]?0:1, cb=checked[b.name]?0:1; return ca-cb || a.name.localeCompare(b.name); });
      html+='<div style="font-size:9px;text-transform:uppercase;letter-spacing:.5px;color:var(--text-dim);padding:6px 7px 2px">'+LaRuche.Utils.esc(LaRuche.i18n.t(etiquette[g]))+' ('+list.filter(function(i){return checked[i.name];}).length+'/'+list.length+')</div>';
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
    ov.innerHTML='<div style="width:680px;max-width:94vw;height:80vh;background:var(--bg-panel);border:1px solid var(--amber);border-radius:10px;display:flex;flex-direction:column">'+
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
  // Les taches telles qu'elles viennent du serveur, gardees pour l'editeur.
  //
  // Il les relisait dans `_kanbanLast`, qui n'est pas la liste mais une
  // SIGNATURE de comparaison: `vue|[...]`. Le JSON.parse echouait donc a chaque
  // ouverture, l'exception etait avalee par un catch vide, et le formulaire
  // s'ouvrait entierement vierge. Enregistrer effacait alors le titre et la
  // description de la tache qu'on venait d'ouvrir pour la corriger.
  var _kanbanTaches=[];
  var _kanbanView=(function(){ try{ return localStorage.getItem('lr_kanban_view')||'cols'; }catch(e){ return 'cols'; } })();
  var _profiles={}; // P1: profiles cache for the Provider selectors (kanban/watcher)
  var _watchersLast='[]'; // watchers cache for inline editing
  var _watcherOpen={}; // expanded watcher cards (the pipeline diagram IS the editor)
  var _watchersEl=null; // container the watchers tab last rendered into (Settings OR Automations)

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
    var h='<div class="kb-carte" data-id="'+t.id+'" style="background:#2a2a2e;border:1px solid var(--border);border-radius:4px;padding:8px;cursor:grab;touch-action:none">';
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
    h+='<span>';
    if(t.status==='Triage' || t.status==='Todo' || t.status==='Blocked'){
      h+='<button onclick="LaRuche.Settings.lancerKanbanTask(\''+t.id+'\')" title="'+LaRuche.Utils.esc(LaRuche.i18n.t('settings.kanbanLancerHint'))+'" style="background:none;border:none;color:var(--green);cursor:pointer;font-size:10px;font-weight:600">'+LaRuche.i18n.t('settings.kanbanLancerBtn')+'</button> ';
    }
    h+='<button onclick="LaRuche.Settings.editKanbanTask(\''+t.id+'\')" style="background:none;border:none;color:var(--amber);cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.kanbanEditBtn')+'</button> <button onclick="LaRuche.Settings.deleteKanbanTask(\''+t.id+'\')" style="background:none;border:none;color:var(--red);cursor:pointer;font-size:10px">'+LaRuche.i18n.t('settings.kanbanDelBtn')+'</button></span>';
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
      '<div style="flex:0 1 130px;min-width:110px"><label class="form-label">'+LaRuche.i18n.t('settings.kanbanColonne')+'</label><select class="form-input" id="kanban-statut">'+
        ['Triage','Todo','Ready','Blocked','Done','Archived'].map(function(c){
          return '<option value="'+c+'"'+(c==='Todo'?' selected':'')+'>'+LaRuche.i18n.t('kanban.col.'+c.toLowerCase())+'</option>';
        }).join('')+'</select></div>' +
      '<button class="form-btn" onclick="LaRuche.Settings.createKanbanTask()">'+LaRuche.i18n.t('settings.kanbanCreate')+'</button></div>' +
      '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px;flex-wrap:wrap;gap:8px">' +
        '<div style="display:flex;align-items:center;gap:6px"><label class="form-label" style="margin:0">'+LaRuche.i18n.t('settings.kanbanDefaultChannelLabel')+'</label>' +
        '<select class="form-input" id="kanban-default-channel" style="width:auto" onchange="LaRuche.Settings.setKanbanDefaultChannel(this.value)"><option value="">'+LaRuche.i18n.t('settings.kanbanBoardChannelNone')+'</option></select>' +
        '<label class="form-label" style="margin:0 0 0 14px">'+LaRuche.i18n.t('settings.kanbanInterval')+'</label>' +
        '<input class="form-input" id="kanban-interval" type="number" min="1" max="3600" step="1" style="width:74px" onchange="LaRuche.Settings.setKanbanInterval(this.value)">' +
        '<span class="settings-card-desc" style="margin:0">'+LaRuche.i18n.t('settings.kanbanIntervalDesc')+'</span></div>' +
        '<div id="kanbanViewToggle" style="display:inline-flex;border:1px solid var(--border);border-radius:6px;overflow:hidden">'+kanbanToggleInner()+'</div></div>' +
      '<p class="settings-card-desc" style="margin:0 0 10px">'+LaRuche.i18n.t('settings.kanbanFluxAide')+'</p>' +
      '<div id="kanbanTodoBloc" style="border:1px solid var(--border);border-radius:8px;padding:10px 12px;margin:0 0 12px"></div>' +
      '<div id="kanbanCols"></div>';
    _kanbanLast='';
    window.__fillChannels(document.getElementById('kanban-channel'), '', LaRuche.i18n.t('settings.kanbanBoardChannel'));
    try{ var dc=await fetch('/api/kanban/default_channel').then(function(r){return r.json();}); window.__fillChannels(document.getElementById('kanban-default-channel'), (dc&&dc.channel)||'', LaRuche.i18n.t('settings.kanbanBoardChannelNone')); }catch(e){}
    try{
      var iv=await fetch('/api/kanban/interval').then(function(r){return r.json();});
      var champ=document.getElementById('kanban-interval');
      if(champ && iv && iv.seconds) champ.value=iv.seconds;
    }catch(e){}
    await refreshKanbanCols();
    loadKanbanTodo();
    if(_kanbanTimer) LaRuche.Poll.stop(_kanbanTimer);
    // Auto-refresh (the agent/daemon can modify the board): re-render
    // only if the content changed -> doesn't break in-progress input.
    _kanbanTimer=LaRuche.Poll.every(function(){
      if(!document.getElementById('kanbanCols')){ LaRuche.Poll.stop(_kanbanTimer); _kanbanTimer=null; return; }
      refreshKanbanCols();
    }, 4000);
  }

  /* La releve de la colonne A faire.
   *
   * La cadence se saisit en nombre plus unite, et voyage en minutes: un champ
   * unique en minutes se lit mal des qu'on veut « tous les deux jours », et
   * trois champs separes se contredisent. */
  var _TODO_UNITES = { h: 60, j: 1440, s: 10080 };

  function todoDecouper(min){
    if(min % 10080 === 0) return { n: min/10080, u: 's' };
    if(min % 1440 === 0)  return { n: min/1440,  u: 'j' };
    return { n: Math.max(1, Math.round(min/60)), u: 'h' };
  }

  async function loadKanbanTodo(){
    var hote = document.getElementById('kanbanTodoBloc');
    if(!hote) return;
    var d = {};
    try{ d = await fetch('/api/kanban/todo_sweep').then(function(r){ return r.json(); }); }catch(e){}
    var actif = !!d.actif;
    var c = todoDecouper(d.periode_min || 1440);
    var quand = d.dernier ? new Date(d.dernier).toLocaleString() : null;
    var T = LaRuche.i18n.t.bind(LaRuche.i18n);
    hote.innerHTML =
      '<label class="bascule" style="display:flex;align-items:center;gap:8px;margin:0 0 6px">'+
        '<input type="checkbox" id="kanbanTodoActif"'+(actif?' checked':'')+' onchange="LaRuche.Settings.saveKanbanTodo()">'+
        '<span style="font-weight:600">'+T('settings.kanbanTodoTitre')+'</span>'+
      '</label>'+
      '<div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:6px">'+
        '<label class="form-label" style="margin:0">'+T('settings.kanbanTodoPeriode')+'</label>'+
        '<input class="form-input" id="kanbanTodoN" type="number" min="1" max="90" step="1" style="width:70px" value="'+c.n+'" onchange="LaRuche.Settings.saveKanbanTodo()">'+
        '<select class="form-input" id="kanbanTodoU" style="width:auto" onchange="LaRuche.Settings.saveKanbanTodo()">'+
          '<option value="h"'+(c.u==='h'?' selected':'')+'>'+T('settings.uniteHeures')+'</option>'+
          '<option value="j"'+(c.u==='j'?' selected':'')+'>'+T('settings.uniteJours')+'</option>'+
          '<option value="s"'+(c.u==='s'?' selected':'')+'>'+T('settings.uniteSemaines')+'</option>'+
        '</select>'+
        '<button class="tl-btn" onclick="LaRuche.Settings.kanbanTodoMaintenant()">'+T('settings.kanbanTodoMaintenant')+'</button>'+
        '<span class="settings-card-desc" style="margin:0">'+
          (quand ? T('settings.kanbanTodoDernier', {q: quand}) : T('settings.kanbanTodoJamais'))+'</span>'+
      '</div>'+
      '<p class="settings-card-desc" style="margin:0">'+T('settings.kanbanTodoAide')+'</p>';
  }

  function saveKanbanTodo(){
    var a = document.getElementById('kanbanTodoActif');
    var n = parseInt((document.getElementById('kanbanTodoN')||{}).value, 10);
    var u = (document.getElementById('kanbanTodoU')||{}).value || 'j';
    if(!Number.isInteger(n) || n < 1) n = 1;
    var min = n * (_TODO_UNITES[u] || 1440);
    fetch('/api/kanban/todo_sweep',{method:'POST',headers:{'Content-Type':'application/json'},
      body:JSON.stringify({ actif: !!(a && a.checked), periode_min: min })})
      .then(function(r){ return r.json(); })
      .then(function(){
        LaRuche.Toast.show(LaRuche.i18n.t('settings.kanbanTodoRegle'),'ok');
        loadKanbanTodo();
      });
  }

  function kanbanTodoMaintenant(){
    fetch('/api/kanban/todo_sweep/now',{method:'POST'})
      .then(function(r){ return r.json(); })
      .then(function(d){
        var n = (d && d.promues) || 0;
        LaRuche.Toast.show(n ? LaRuche.i18n.t('settings.kanbanTodoFait',{n:n})
                             : LaRuche.i18n.t('settings.kanbanTodoRien'), n ? 'ok' : 'info');
        _kanbanLast=''; refreshKanbanCols(); loadKanbanTodo();
      });
  }

  async function refreshKanbanCols(){
    var host=document.getElementById('kanbanCols'); if(!host)return;
    if(_kbGlisse && _kbGlisse.parti) return;   // une carte est en l'air
    var tasks=await fetch(LaRuche.API.base+'/api/kanban').then(function(r){return r.json();}).catch(function(){return [];});
    _kanbanTaches=tasks;
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
        html+='<div class="kb-cible" data-col="'+c+'" style="background:rgba(30,30,32,0.8);border:1px solid var(--amber-dim);border-radius:6px;overflow:hidden">';
        html+='<div style="padding:6px 10px;font-weight:600;color:var(--amber);border-bottom:1px solid var(--border);display:flex;justify-content:space-between;align-items:center"><span>'+kanbanColLabel(c)+'</span><span style="font-size:10px;color:var(--text-dim)">'+colTasks.length+'</span></div>';
        html+='<div style="padding:8px;display:flex;flex-wrap:wrap;gap:8px;min-height:36px">';
        if(!colTasks.length){ html+='<span style="font-size:10px;color:var(--text-muted);align-self:center">-</span>'; }
        colTasks.forEach(function(t){ html+='<div style="flex:0 0 230px;max-width:230px">'+kanbanCardHtml(t)+'</div>'; });
        html+='</div></div>';
      });
      html+='</div>';
    } else {
      // Mode colonnes. Les sept colonnes avaient une largeur fixe de 250 px,
      // pleines ou vides, et une hauteur plancher de 400 px: sur un tableau
      // neuf, sept colonnes vides prenaient 1750 px de large, imposaient une
      // barre de defilement horizontale, et couvraient la moitie de l'ecran
      // pour ne rien montrer. Une colonne vide se contente maintenant de son
      // titre, et les colonnes se replient sur plusieurs rangees plutot que de
      // deborder.
      html='<div class="kb-board">';
      cols.forEach(function(c){
        var colTasks=tasks.filter(function(t){return t.status===c;});
        var vide = !colTasks.length;
        html+='<div class="kb-col kb-cible'+(vide?' kb-col-vide':'')+'" data-col="'+c+'">';
        html+='<div class="kb-col-hdr"><span>'+kanbanColLabel(c)+'</span>'+(colTasks.length?('<span class="kb-col-n">'+colTasks.length+'</span>'):'')+'</div>';
        html+='<div class="kb-col-corps">';
        colTasks.forEach(function(t){ html+=kanbanCardHtml(t); });
        html+='</div></div>';
      });
      html+='</div>';
    }
    host.innerHTML=html;
    if(!host.dataset.glisse){
      host.dataset.glisse = '1';
      host.addEventListener('pointerdown', kanbanDebutGlisse);
    }
  }

  function setKanbanInterval(v){
    fetch('/api/kanban/interval',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({seconds: parseInt(v,10)||5})})
      .then(function(r){ return r.json(); })
      .then(function(d){
        // On reaffiche la valeur RETENUE: elle est bornee cote serveur, et un
        // champ qui garde 0 alors que le serveur applique 1 ment a la personne
        // qui vient de le regler.
        var champ=document.getElementById('kanban-interval');
        if(champ && d && d.seconds) champ.value=d.seconds;
        LaRuche.Toast.show(LaRuche.i18n.t('settings.kanbanIntervalUpdated',{n:(d&&d.seconds)||v}),'ok');
      });
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
var st = document.getElementById('kanban-statut')?document.getElementById('kanban-statut').value:'';
    if(!title) return;
    fetch(LaRuche.API.base+'/api/kanban',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({title: title, description: desc, profile_id: pId||null, model: m||null, channel: ch||null, status: st||null})})
      .then(function(r){if(r.ok) { LaRuche.Toast.show(LaRuche.i18n.t('settings.kanbanTaskCreated'),'ok'); document.getElementById('kanban-title').value=''; document.getElementById('kanban-desc').value=''; _kanbanLast=''; refreshKanbanCols(); }});
  }

  function deleteKanbanTask(id) {
    fetch(LaRuche.API.base+'/api/kanban/'+id,{method:'DELETE'})
      .then(function(r){if(r.ok) { _kanbanLast=''; refreshKanbanCols(); }});
  }

  function editKanbanTask(id) {
    var t=_kanbanTaches.filter(function(x){ return x.id===id; })[0] || null;
    if(!t){
      // Plutot que d'ouvrir un formulaire vide qui effacerait la tache: on le
      // dit, et on relit le tableau.
      LaRuche.Toast.show(LaRuche.i18n.t('settings.kanbanTaskIntrouvable'),'err');
      _kanbanLast=''; refreshKanbanCols();
      return;
    }
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
    ov.innerHTML='<div style="width:480px;max-width:92vw;background:var(--bg-panel);border:1px solid var(--amber);border-radius:10px;padding:16px">'+
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

  function lancerKanbanTask(id){
    fetch(LaRuche.API.base+'/api/kanban/'+id+'/status',{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({status:'Ready'})})
      .then(function(r){
        if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.kanbanLancee'),'ok'); _kanbanLast=''; refreshKanbanCols(); }
      });
  }

  /* Le glissement des cartes.
   *
   * A la souris et au doigt, pas en glisser-deposer HTML5. Celui-ci ne se
   * declenchait pas dans la fenetre de l'application, et n'existe pas du tout
   * sur ecran tactile. Ici: on suit le pointeur, on porte une copie de la carte
   * sous lui, et on lit la colonne qui se trouve dessous au moment du lacher.
   *
   * Le rafraichissement automatique du tableau est suspendu pendant le
   * glissement: il reecrit tout le HTML des que le contenu change, et emportait
   * la carte en cours de deplacement avec le reste.
   */
  var _kbGlisse = null;

  function kanbanCibleSous(x, y){
    var el = document.elementFromPoint(x, y);
    while(el && el !== document.body){
      if(el.classList && el.classList.contains('kb-cible')) return el;
      el = el.parentElement;
    }
    return null;
  }

  function kanbanFinGlisse(e){
    var g = _kbGlisse;
    if(!g) return;
    _kbGlisse = null;
    document.removeEventListener('pointermove', kanbanEnGlisse);
    document.removeEventListener('pointerup', kanbanFinGlisse);
    document.removeEventListener('pointercancel', kanbanFinGlisse);
    if(g.fantome && g.fantome.parentNode) g.fantome.parentNode.removeChild(g.fantome);
    var host = document.getElementById('kanbanCols');
    if(host) host.classList.remove('kb-glisse');
    if(g.carte) g.carte.style.opacity = '';
    if(g.derniere) g.derniere.classList.remove('kb-survol');

    if(!g.parti) return;                       // un simple clic, rien a deplacer
    var cible = kanbanCibleSous(e.clientX, e.clientY);
    var col = cible && cible.getAttribute('data-col');
    if(!col || col === g.depart){ _kanbanLast=''; refreshKanbanCols(); return; }
    fetch(LaRuche.API.base+'/api/kanban/'+g.id+'/status',{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({status:col})})
      .then(function(r){ if(r.ok){ _kanbanLast=''; refreshKanbanCols(); } });
  }

  function kanbanEnGlisse(e){
    var g = _kbGlisse;
    if(!g) return;
    var dx = e.clientX - g.x0, dy = e.clientY - g.y0;
    // Quelques pixels avant de considerer que c'est un glissement: sans ce
    // seuil, le moindre tremblement sur un bouton devient un deplacement.
    if(!g.parti){
      if(Math.abs(dx) + Math.abs(dy) < 6) return;
      g.parti = true;
      g.carte.style.opacity = '.35';
      var host = document.getElementById('kanbanCols');
      if(host) host.classList.add('kb-glisse');
      g.fantome = g.carte.cloneNode(true);
      g.fantome.style.cssText += ';position:fixed;z-index:9999;pointer-events:none;width:'+
        g.larg+'px;opacity:.92;transform:rotate(1.5deg);box-shadow:0 8px 24px rgba(0,0,0,.5)';
      document.body.appendChild(g.fantome);
    }
    g.fantome.style.left = (e.clientX - g.dx) + 'px';
    g.fantome.style.top  = (e.clientY - g.dy) + 'px';
    var cible = kanbanCibleSous(e.clientX, e.clientY);
    if(cible !== g.derniere){
      if(g.derniere) g.derniere.classList.remove('kb-survol');
      if(cible) cible.classList.add('kb-survol');
      g.derniere = cible;
    }
  }

  function kanbanDebutGlisse(e){
    if(e.button !== undefined && e.button !== 0) return;      // clic droit: non
    var carte = e.target.closest ? e.target.closest('.kb-carte') : null;
    if(!carte) return;
    // Les boutons de la carte gardent leur clic.
    if(e.target.closest('button') || e.target.closest('.kb-result')) return;
    var r = carte.getBoundingClientRect();
    _kbGlisse = {
      id: carte.getAttribute('data-id'),
      carte: carte,
      depart: (carte.closest('.kb-cible') || {}).getAttribute
              ? carte.closest('.kb-cible').getAttribute('data-col') : null,
      x0: e.clientX, y0: e.clientY,
      dx: e.clientX - r.left, dy: e.clientY - r.top,
      larg: r.width, parti: false, fantome: null, derniere: null
    };
    document.addEventListener('pointermove', kanbanEnGlisse);
    document.addEventListener('pointerup', kanbanFinGlisse);
    document.addEventListener('pointercancel', kanbanFinGlisse);
    e.preventDefault();
  }

  function _avatarInner(u){
    if(u && u.avatar) return '<img src="'+u.avatar+'" alt="" style="width:100%;height:100%;object-fit:cover">';
    return LaRuche.Utils.esc(((u&&u.display_name)||'?').charAt(0).toUpperCase());
  }
  function loadProfile(el){
    var T=LaRuche.i18n.t.bind(LaRuche.i18n), esc=LaRuche.Utils.esc;
    var u=(LaRuche.Auth&&LaRuche.Auth.getUser&&LaRuche.Auth.getUser())||{};
    el.innerHTML =
      '<div class="settings-card"><div class="settings-card-title">'+T('settings.profileAccount')+'</div>'+
        '<div class="settings-row"><span class="settings-label">'+T('settings.profileAvatar')+'</span><span style="display:flex;align-items:center;gap:8px">'+
          '<span id="profAvatar" title="'+T('settings.profileChangePhoto')+'" onclick="document.getElementById(\'profAvatarFile\').click()" style="width:44px;height:44px;border-radius:50%;background:var(--bg-card);border:1px solid var(--border);display:flex;align-items:center;justify-content:center;font-weight:700;overflow:hidden;cursor:pointer">'+_avatarInner(u)+'</span>'+
          '<input type="file" id="profAvatarFile" accept="image/*" style="display:none">'+
          '<button class="form-btn" style="font-size:10px;padding:2px 8px" onclick="document.getElementById(\'profAvatarFile\').click()">'+T('settings.profileChangePhoto')+'</button>'+
          (u.avatar?'<button class="form-btn" style="font-size:10px;padding:2px 8px;color:var(--red);border-color:var(--red)" onclick="LaRuche.Settings.profileRemoveAvatar()">'+T('settings.tlDelete')+'</button>':'')+
        '</span></div>'+
        '<div class="settings-card-desc" style="margin:-4px 0 8px">'+T('settings.profilePhotoAide')+'</div>'+
        '<div class="settings-row"><span class="settings-label">'+T('settings.profileName')+'</span><span style="display:flex;gap:6px"><input type="text" id="profName" class="form-input" style="width:160px;padding:2px 6px" value="'+esc(u.display_name||'')+'"><button class="form-btn" style="font-size:10px;padding:2px 8px" onclick="LaRuche.Settings.profileSaveName()">'+T('settings.save')+'</button></span></div>'+
      '</div>'+
      '<div class="settings-card"><div class="settings-card-title">'+T('settings.profilePassword')+'</div><div class="settings-card-desc">'+(u.has_password?T('settings.profilePwSet'):T('settings.profilePwNone'))+'</div>'+
        '<div class="settings-row"><span class="settings-label">'+T('settings.profileNewPw')+'</span><span style="display:flex;gap:6px"><input type="password" id="profPw" class="form-input" style="width:160px;padding:2px 6px" autocomplete="new-password"><button class="form-btn" style="font-size:10px;padding:2px 8px" onclick="LaRuche.Settings.profileSavePassword()">'+T('settings.save')+'</button></span></div>'+
      '</div>'+
      '<div class="settings-card"><div class="settings-card-title">'+T('settings.profile2fa')+'</div><div class="settings-card-desc">'+(u.totp_enabled?T('settings.profile2faOn'):T('settings.profile2faOff'))+'</div>'+
        '<div id="totpArea" style="margin-top:8px">'+
          (u.totp_enabled
            ? '<div style="display:flex;gap:6px;align-items:center"><input type="text" id="totpDisableCode" inputmode="numeric" maxlength="6" class="form-input" style="width:90px;padding:2px 6px" placeholder="000000"><button class="form-btn" style="font-size:10px;padding:2px 8px;color:var(--red);border-color:var(--red)" onclick="LaRuche.Settings.totpDisable()">'+T('settings.totpDisable')+'</button></div>'
            : '<button class="form-btn" onclick="LaRuche.Settings.totpStart()">'+T('settings.totpEnable')+'</button>')+
        '</div></div>'+
      '<div class="settings-card"><div class="settings-card-title">'+T('settings.profileFiche')+'</div><div class="settings-card-desc">'+T('settings.profileFicheDesc')+'</div>'+
        '<textarea id="profFiche" class="form-input" style="width:100%;min-height:120px;margin-top:6px;box-sizing:border-box"></textarea>'+
        '<button class="form-btn" style="margin-top:8px" onclick="LaRuche.Settings.profileSaveFiche()">'+T('settings.save')+'</button>'+
      '</div>';
    var f=document.getElementById('profAvatarFile'); if(f) f.addEventListener('change', _profAvatarPick);
    fetch('/api/profile').then(function(r){return r.json();}).then(function(d){ var ta=document.getElementById('profFiche'); if(ta) ta.value=(d&&(d.fiche||d.content||d.text))||''; }).catch(function(){});
  }
  function _profAvatarPick(e){
    _avatarDepuisFichier(e.target.files && e.target.files[0], function(data){
      _profSave({avatar:data}, function(){
        var a=document.getElementById('profAvatar');
        if(a) a.innerHTML='<img src="'+data+'" alt="" style="width:100%;height:100%;object-fit:cover">';
        if(LaRuche.Auth.getUser()) LaRuche.Auth.getUser().avatar=data;
        if(LaRuche.Auth.refreshBadge) LaRuche.Auth.refreshBadge();
        loadProfile(document.getElementById('settingsContent'));
      });
    });
  }
  function _profSave(body, ok){
    fetch('/api/auth/account',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(function(r){
      if(r.status===409){ LaRuche.Toast.show(LaRuche.i18n.t('core.nameTaken'),'err'); return; }
      if(!r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err'); return; }
      LaRuche.Toast.show(LaRuche.i18n.t('settings.profileSaved'),'ok'); if(ok) ok();
    });
  }
  function profileSaveName(){ var v=(document.getElementById('profName')||{}).value||''; if(!v.trim()) return; _profSave({display_name:v.trim()}, function(){ var uu=LaRuche.Auth.getUser&&LaRuche.Auth.getUser(); if(uu) uu.display_name=v.trim(); if(LaRuche.Auth.refreshBadge) LaRuche.Auth.refreshBadge(); }); }
  function profileRemoveAvatar(){ _profSave({avatar:null}, function(){ var uu=LaRuche.Auth.getUser&&LaRuche.Auth.getUser(); if(uu) uu.avatar=null; loadProfile(document.getElementById('settingsContent')); if(LaRuche.Auth.refreshBadge) LaRuche.Auth.refreshBadge(); }); }
  function profileSavePassword(){
    var el=document.getElementById('profPw'); var pw=el?el.value:'';
    if(pw.length<6){ LaRuche.Toast.show(LaRuche.i18n.t('core.passwordMin'),'err'); return; }
    fetch('/api/auth/password',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({password:pw})}).then(function(r){return r.ok?r.json():null;}).then(function(d){
      if(d&&!d.error){ LaRuche.Toast.show(LaRuche.i18n.t('settings.profilePwChanged'),'ok'); if(el) el.value=''; }
      else LaRuche.Toast.show((d&&d.error)||LaRuche.i18n.t('settings.saveFailed'),'err');
    });
  }
  function profileSaveFiche(){
    var ta=document.getElementById('profFiche'); var v=ta?ta.value:'';
    fetch('/api/profile',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({fiche:v})}).then(function(r){
      LaRuche.Toast.show(r.ok?LaRuche.i18n.t('settings.profileSaved'):LaRuche.i18n.t('settings.saveFailed'), r.ok?'ok':'err');
    });
  }
  function totpStart(){
    fetch('/api/auth/totp/setup',{method:'POST'}).then(function(r){return r.ok?r.json():null;}).then(function(d){
      if(!d){ LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err'); return; }
      var area=document.getElementById('totpArea'); if(!area) return;
      area.innerHTML='<div style="display:flex;flex-direction:column;gap:8px">'+
        '<div style="font-size:11px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.totpScan')+'</div>'+
        '<div style="width:160px;background:#fff;padding:6px;border-radius:6px">'+d.qr_svg+'</div>'+
        '<div style="font-size:10px;color:var(--text-dim);font-family:var(--mono);word-break:break-all">'+LaRuche.Utils.esc(d.secret)+'</div>'+
        '<div style="display:flex;gap:6px;align-items:center"><input type="text" id="totpCode" inputmode="numeric" maxlength="6" class="form-input" style="width:90px;padding:2px 6px" placeholder="000000"><button class="form-btn" onclick="LaRuche.Settings.totpEnable(\''+d.secret+'\')">'+LaRuche.i18n.t('settings.totpVerify')+'</button></div>'+
      '</div>';
    });
  }
  function totpEnable(secret){
    var code=((document.getElementById('totpCode')||{}).value||'').trim();
    fetch('/api/auth/totp/enable',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({secret:secret,code:code})}).then(function(r){
      if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.totpEnabled'),'ok'); var uu=LaRuche.Auth.getUser&&LaRuche.Auth.getUser(); if(uu) uu.totp_enabled=true; loadProfile(document.getElementById('settingsContent')); }
      else LaRuche.Toast.show(LaRuche.i18n.t('settings.totpBadCode'),'err');
    });
  }
  function totpDisable(){
    var code=((document.getElementById('totpDisableCode')||{}).value||'').trim();
    fetch('/api/auth/totp/disable',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({code:code})}).then(function(r){
      if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.totpDisabled'),'ok'); var uu=LaRuche.Auth.getUser&&LaRuche.Auth.getUser(); if(uu) uu.totp_enabled=false; loadProfile(document.getElementById('settingsContent')); }
      else LaRuche.Toast.show(LaRuche.i18n.t('settings.totpBadCode'),'err');
    });
  }

  function loadAdmin(el){
    el.innerHTML = '<div class="settings-card"><div class="settings-card-title">'+LaRuche.i18n.t('settings.navAdmin')+'</div><div class="settings-card-desc">'+LaRuche.i18n.t('settings.adminDesc')+'</div><div id="adminUserList" style="display:flex;flex-direction:column;gap:6px;margin-top:8px"><div style="color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('common.loading')+'</div></div></div>';
    fetch('/api/admin/users').then(function(r){ if(!r.ok) throw new Error(); return r.json(); }).then(function(d){
      var host = document.getElementById('adminUserList'); if(!host) return;
      var users = d.users||[];
      if(!users.length){ host.innerHTML = '<div style="color:var(--text-dim);font-size:11px">'+LaRuche.i18n.t('settings.adminNoUsers')+'</div>'; return; }
      // Whoever is looking is the super-admin only if their own row says so.
      var jeSuisSuper = users.some(function(u){ return u.is_self && u.is_super; });
      host.innerHTML = users.map(function(u){
        var isAdmin = u.role==='admin';
        var safeName = (u.display_name||'').replace(/[\\']/g,'');
        // Photo when there is one, coloured initial otherwise: the list is scanned by
        // face far faster than by name.
        var initiale = LaRuche.Utils.esc((u.display_name||'?').charAt(0).toUpperCase());
        var vignette = u.avatar
          ? '<img src="'+LaRuche.Utils.esc(u.avatar)+'" alt="" class="admin-av">'
          : '<span class="admin-av admin-av-txt'+(u.is_super?' admin-av-super':'')+'">'+initiale+'</span>';
        var badges = '<span class="admin-badge'+(isAdmin?' admin-badge-on':'')+'">'+(isAdmin?'admin':'user')+'</span>'+
          (u.is_super ? '<span class="admin-badge admin-badge-super" title="'+LaRuche.i18n.t('settings.adminSuperHint')+'">'+LaRuche.i18n.t('settings.adminSuper')+'</span>' : '')+
          (u.has_password ? '' : '<span class="admin-badge admin-badge-warn">'+LaRuche.i18n.t('settings.adminNoPw')+'</span>');
        // The super-admin is neither demotable nor deletable: those buttons are simply
        // absent rather than present-and-rejected.
        var roleBtn = (u.is_self || u.is_super) ? '' : '<button class="form-btn" style="font-size:10px;padding:2px 6px" onclick="LaRuche.Settings.adminSetRole(\''+u.id+'\',\''+(isAdmin?'user':'admin')+'\')">'+(isAdmin?LaRuche.i18n.t('settings.adminDemote'):LaRuche.i18n.t('settings.adminPromote'))+'</button>';
        var pwBtn = jeSuisSuper ? '<button class="form-btn" style="font-size:10px;padding:2px 6px" onclick="LaRuche.Settings.adminSetPassword(\''+u.id+'\',\''+safeName+'\')">'+LaRuche.i18n.t('settings.adminSetPw')+'</button>' : '';
        // La photo des autres comptes: meme porte que le mot de passe, le
        // super-admin seulement. La liste les MONTRAIT sans permettre d'en
        // changer une seule, pas meme celle d'un compte cree par un canal.
        var phBtn = jeSuisSuper ? '<button class="form-btn" style="font-size:10px;padding:2px 6px" onclick="LaRuche.Settings.adminPickAvatar(\''+u.id+'\')">'+LaRuche.i18n.t('settings.adminPhoto')+'</button>' : '';
        var delBtn = u.is_super
          ? ''
          : (u.is_self ? '<span style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.adminYou')+'</span>'
                       : '<button class="form-btn" style="font-size:10px;padding:2px 6px;color:var(--red);border-color:var(--red)" onclick="LaRuche.Settings.adminDeleteUser(\''+u.id+'\',\''+safeName+'\')">'+LaRuche.i18n.t('settings.tlDelete')+'</button>');
        return '<div class="admin-user'+(u.is_self?' admin-user-self':'')+'">'+vignette+
          '<span class="admin-user-id"><span class="admin-user-name">'+LaRuche.Utils.esc(u.display_name)+'</span>'+
          '<span class="admin-user-badges">'+badges+'</span></span>'+
          '<span style="display:flex;gap:6px;align-items:center;flex-shrink:0">'+roleBtn+phBtn+pwBtn+delBtn+'</span></div>';
      }).join('');
    }).catch(function(){ var host=document.getElementById('adminUserList'); if(host) host.innerHTML='<div style="color:var(--red);font-size:11px">'+LaRuche.i18n.t('settings.adminLoadError')+'</div>'; });
  }
  /* Le recadrage carre en 128, partage par le profil et la liste des comptes:
     deux implementations du meme geste finissent toujours par diverger. */
  function _avatarDepuisFichier(file, apres){
    if(!file) return;
    var reader = new FileReader();
    reader.onload = function(ev){
      var img = new Image();
      img.onload = function(){
        var c = document.createElement('canvas'); c.width = c.height = 128;
        var ctx = c.getContext('2d');
        var cote = Math.min(img.width, img.height);
        var sx = (img.width - cote)/2, sy = (img.height - cote)/2;
        ctx.drawImage(img, sx, sy, cote, cote, 0, 0, 128, 128);
        apres(c.toDataURL('image/jpeg', 0.82));
      };
      img.src = ev.target.result;
    };
    reader.readAsDataURL(file);
  }

  function adminPickAvatar(id){
    var inp = document.createElement('input');
    inp.type = 'file'; inp.accept = 'image/*';
    inp.onchange = function(){
      _avatarDepuisFichier(inp.files && inp.files[0], function(data){
        fetch('/api/admin/users/'+encodeURIComponent(id)+'/avatar',
              {method:'POST',headers:{'Content-Type':'application/json'},
               body:JSON.stringify({avatar:data})})
          .then(function(r){
            if(r.ok){
              LaRuche.Toast.show(LaRuche.i18n.t('settings.adminPhotoFaite'),'ok');
              loadAdmin(document.getElementById('settingsContent'));
            } else LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err');
          });
      });
    };
    inp.click();
  }

  function adminDeleteUser(id, name){
    if(!confirm(LaRuche.i18n.t('settings.adminConfirmDelete',{name:name}))) return;
    fetch('/api/admin/users/'+encodeURIComponent(id),{method:'DELETE'}).then(function(r){
      if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.adminDeleted'),'ok'); loadAdmin(document.getElementById('settingsContent')); }
      else LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err');
    });
  }
  /* Super-admin only. The current password is never asked for, never shown and never
   * sent: the endpoint takes the new value alone and writes a fresh hash. */
  function adminSetPassword(id, name){
    var pw = window.prompt(LaRuche.i18n.t('settings.adminSetPwPrompt', {name:name}));
    if(pw === null) return;                                  // cancelled
    pw = String(pw);
    if(pw.length < 8){ LaRuche.Toast.show(LaRuche.i18n.t('settings.adminPwTooShort'),'warn'); return; }
    if(window.prompt(LaRuche.i18n.t('settings.adminSetPwConfirm')) !== pw){
      LaRuche.Toast.show(LaRuche.i18n.t('settings.adminPwMismatch'),'warn'); return;
    }
    fetch('/api/admin/users/'+encodeURIComponent(id)+'/password',{
      method:'POST', credentials:'include', headers:{'Content-Type':'application/json'},
      body:JSON.stringify({password:pw})
    }).then(function(r){
      if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('settings.adminPwChanged',{name:name}),'ok'); loadAdmin(document.getElementById('settingsContent')); }
      else if(r.status===403) LaRuche.Toast.show(LaRuche.i18n.t('settings.adminPwForbidden'),'err');
      else LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err');
    }).catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err'); });
  }
  function adminSetRole(id, role){
    fetch('/api/admin/users/'+encodeURIComponent(id)+'/role',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({role:role})}).then(function(r){
      if(r.ok){ loadAdmin(document.getElementById('settingsContent')); }
      else LaRuche.Toast.show(LaRuche.i18n.t('settings.saveFailed'),'err');
    });
  }

  async function saveVoiceCfg() {
    var stt_external = !!document.getElementById('cfgSttExternal').checked;
    var speedEl = document.getElementById('cfgTtsSpeed');
    var voiceEl = document.getElementById('cfgTtsVoice');
    var backendEl = document.getElementById('cfgTtsBackend');
    var body = { stt_external: stt_external };
    if(speedEl) body.tts_speed = parseFloat(speedEl.value);
    if(voiceEl) body.tts_voice = voiceEl.value.trim();
    if(backendEl) body.tts_backend = backendEl.value;
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

  /* -- Aide: le dehors du logiciel -------------------------------------
     Une section qui ne regle rien. Elle rassemble ce qu'on cherche justement
     quand on ne sait plus quoi regler: la documentation, le code, un endroit
     ou signaler un probleme, et la reponse a "est-ce que ma version est
     vieille". Ces liens vivaient dans le README, c'est-a-dire nulle part pour
     quelqu'un qui utilise l'application sans avoir clone le depot. */
  var HELP_DEPOT = 'https://github.com/infinition/LaRuche';
  var HELP_WIKI  = 'https://infinition.github.io/LaRuche/wiki.html';
  var HELP_CAFE  = 'https://www.buymeacoffee.com/infinition';

  function _helpLien(href, ic, titre, desc){
    // rel="noopener": sans lui la page ouverte recoit `window.opener` et peut
    // rediriger celle-ci. noreferrer en prime, on n'a aucune raison d'annoncer
    // d'ou vient le clic.
    return '<a class="help-lien" href="' + href + '" target="_blank" rel="noopener noreferrer">' +
      '<span class="help-lien-ic">' + ic + '</span>' +
      '<span class="help-lien-txt">' +
        '<span class="help-lien-titre">' + titre + '</span>' +
        '<span class="help-lien-desc">' + desc + '</span>' +
      '</span>' +
      '<svg class="help-lien-fleche" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>' +
    '</a>';
  }

  // Compare deux versions "x.y.z". Rend -1, 0 ou 1.
  //
  // Comparaison NUMERIQUE segment par segment, pas alphabetique: "0.10.0" est
  // plus recent que "0.9.0", alors qu'un simple `<` sur les chaines dit
  // l'inverse. Le piege se declenche pile au moment ou le projet commence a
  // vivre, donc autant ne pas le poser.
  function _cmpVersion(a, b){
    var pa = String(a).replace(/^v/, '').split(/[.\-+]/);
    var pb = String(b).replace(/^v/, '').split(/[.\-+]/);
    for(var i = 0; i < Math.max(pa.length, pb.length); i++){
      var na = parseInt(pa[i], 10), nb = parseInt(pb[i], 10);
      if(isNaN(na)) na = -1;
      if(isNaN(nb)) nb = -1;
      if(na !== nb) return na < nb ? -1 : 1;
    }
    return 0;
  }

  async function _verifierMaj(){
    var esc = LaRuche.Utils.esc;
    var zone = document.getElementById('helpMajResultat');
    var btn  = document.getElementById('helpMajBtn');
    if(!zone || !btn) return;
    btn.disabled = true;
    btn.classList.add('help-maj-cherche');
    zone.className = 'help-maj-etat';
    zone.textContent = LaRuche.i18n.t('help.majEnCours');
    try{
      // Cote serveur: un appel direct a api.github.com depuis la webview de
      // l'application se heurte a la politique de securite du contenu et aux
      // regles d'origine croisee, et echouait sans un mot. Le noeud n'a ni
      // l'une ni les autres, et il porte l'en-tete User-Agent que GitHub exige.
      // Le detail de l'echec est LU, pas devine. La version precedente
      // remplacait toute panne par une phrase generique sur GitHub, y compris
      // quand le noeud repondait 404 parce qu'il datait d'avant cette route:
      // on cherchait alors du cote du reseau un probleme qui etait dans le
      // binaire.
      var rep = await fetch('/api/maj');
      if(!rep.ok){
        zone.className = 'help-maj-etat';
        zone.textContent = LaRuche.i18n.t('help.majEchecNoeud', {code: rep.status});
        return;
      }
      var d = await rep.json();
      var locale = d.installee || '0.0.0';
      var cible = document.getElementById('helpVersionLocale');
      if(cible) cible.textContent = 'v' + locale;
      if(d.error || !d.derniere){
        zone.className = 'help-maj-etat';
        zone.textContent = LaRuche.i18n.t('help.majEchec') +
          (d.error ? ' (' + String(d.error).slice(0, 140) + ')' : '');
        return;
      }
      var c = _cmpVersion(locale, d.derniere);
      if(c < 0){
        zone.className = 'help-maj-etat help-maj-neuve';
        zone.innerHTML = '<span class="help-maj-pastille"></span>' +
          LaRuche.i18n.t('help.majDispo') + ' <strong>v' + esc(d.derniere) + '</strong> ' +
          '<a href="' + esc(d.url || (HELP_DEPOT + '/releases')) + '" target="_blank" rel="noopener noreferrer">' +
          LaRuche.i18n.t('help.majTelecharger') + '</a>';
      } else if(c > 0){
        // Un binaire compile depuis les sources est normalement en avance sur la
        // derniere release. Le dire, plutot qu'annoncer "a jour" et laisser
        // croire que la verification a servi a quelque chose.
        zone.className = 'help-maj-etat';
        zone.textContent = LaRuche.i18n.t('help.majAvance');
      } else {
        zone.className = 'help-maj-etat help-maj-ok';
        zone.textContent = LaRuche.i18n.t('help.majAJour');
      }
    }catch(e){
      // Le message de l'exception, et pas une phrase choisie d'avance: ce filet
      // a deja cache une faute de code en la faisant passer pour une coupure
      // reseau, et personne n'avait de raison d'aller regarder ailleurs.
      zone.className = 'help-maj-etat';
      zone.textContent = LaRuche.i18n.t('help.majEchec') +
        ' (' + String((e && e.message) || e).slice(0, 140) + ')';
    }finally{
      btn.disabled = false;
      btn.classList.remove('help-maj-cherche');
    }
  }

  async function loadHelp(el){
    var icGithub = '<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .5a11.5 11.5 0 0 0-3.64 22.42c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.37-3.88-1.37-.53-1.34-1.29-1.7-1.29-1.7-1.05-.72.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.77 2.7 1.26 3.36.96.1-.75.4-1.26.73-1.55-2.55-.29-5.24-1.28-5.24-5.7 0-1.26.45-2.29 1.19-3.1-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.18 1.18a11 11 0 0 1 5.8 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.24 2.76.12 3.05.74.81 1.18 1.84 1.18 3.1 0 4.43-2.69 5.4-5.25 5.69.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5z"/></svg>';
    var icLivre  = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>';

    el.innerHTML =
      // 1. Ce que c'est, et quelle version tourne. La pastille "Miel v0.2.0"
      //    vivait dans la barre du haut, ou elle occupait une place permanente
      //    pour une information qu'on regarde deux fois par an.
      '<div class="settings-card help-entete">' +
        '<div class="help-entete-txt">' +
          '<div class="settings-card-title">' + LaRuche.i18n.t('help.aproposTitre') + '</div>' +
          '<div class="settings-card-desc">' + LaRuche.i18n.t('help.aproposDesc') + '</div>' +
        '</div>' +
        '<div class="help-versions">' +
          '<div class="help-version"><span class="help-version-eti">' + LaRuche.i18n.t('help.majVersionLocale') + '</span>' +
            '<span class="help-version-val" id="helpVersionLocale">...</span></div>' +
          '<div class="help-version"><span class="help-version-eti">' + LaRuche.i18n.t('settings.protocol') + '</span>' +
            '<span class="help-version-val">Miel v0.2.0</span></div>' +
        '</div>' +
        '<div class="help-maj">' +
          '<button id="helpMajBtn" class="help-maj-btn" type="button">' +
            '<svg class="help-maj-ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12a9 9 0 1 1-2.64-6.36"/><polyline points="21 3 21 9 15 9"/></svg>' +
            '<span>' + LaRuche.i18n.t('help.majBouton') + '</span>' +
          '</button>' +
          '<div id="helpMajResultat" class="help-maj-etat">' + LaRuche.i18n.t('help.majDesc') + '</div>' +
        '</div>' +
      '</div>' +

      // 2. Ou aller quand quelque chose resiste.
      '<div class="settings-card">' +
        '<div class="settings-card-title">' + LaRuche.i18n.t('help.ressourcesTitre') + '</div>' +
        '<div class="help-liens">' +
          _helpLien(HELP_WIKI,  icLivre,  LaRuche.i18n.t('help.wiki'),  LaRuche.i18n.t('help.wikiDesc')) +
          _helpLien(HELP_DEPOT, icGithub, LaRuche.i18n.t('help.depot'), LaRuche.i18n.t('help.depotDesc')) +
        '</div>' +
      '</div>' +

      // 3. Le soutien. C'est elle qui demande, pas nous: la meme abeille que
      //    dans le chat, avec une bulle. Une demande d'argent est toujours un
      //    peu genante a lire; venant d'elle elle passe pour ce qu'elle est,
      //    une proposition qu'on peut ignorer sans y penser.
      '<div class="settings-card help-cafe">' +
        '<div class="help-cafe-scene">' +
          '<div class="help-abeille-case"><div class="bee"><div class="bee--wings"></div><div class="bee--body"><span></span><span></span></div><div class="bee--head"><div class="bee--head-eyes"></div><div class="bee--head-antennas"></div></div></div></div>' +
          '<div class="help-bulle">' +
            '<div class="help-bulle-titre">' + LaRuche.i18n.t('help.soutien') + '</div>' +
            '<div class="help-bulle-txt">' + LaRuche.i18n.t('help.soutienDesc') + '</div>' +
          '</div>' +
        '</div>' +
        '<a class="help-cafe-btn" href="' + HELP_CAFE + '" target="_blank" rel="noopener noreferrer">' +
          '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 8h1a4 4 0 0 1 0 8h-1"/><path d="M2 8h16v9a4 4 0 0 1-4 4H6a4 4 0 0 1-4-4z"/><line x1="6" y1="1" x2="6" y2="4"/><line x1="10" y1="1" x2="10" y2="4"/><line x1="14" y1="1" x2="14" y2="4"/></svg>' +
          '<span>' + LaRuche.i18n.t('help.cafeBouton') + '</span>' +
        '</a>' +
      '</div>';

    var b = document.getElementById('helpMajBtn');
    if(b) b.addEventListener('click', _verifierMaj);
    // La version affichee vient du binaire, pas d'une constante ecrite dans la
    // page: une version en dur ment des la publication suivante.
    try{
      var v = (await fetch('/api/version').then(function(r){ return r.json(); })).version;
      var cible = document.getElementById('helpVersionLocale');
      if(cible && v) cible.textContent = 'v' + v;
    }catch(e){
      var c2 = document.getElementById('helpVersionLocale');
      if(c2) c2.textContent = '?';
    }
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
      dynamic_context_threshold: parseInt(document.getElementById('cfgCtxThreshold').value,10),
    };
    // `reactions_agent` deliberately ABSENT: its control now lives in the Chat section.
    // Sending `false` because the checkbox is not on screen would turn the feature off
    // every time this form is saved.
    var auxiliary = {
      fallback_models: document.getElementById('cfgProvFallback').value,
      review_model: document.getElementById('cfgProvReview').value
    };
    // What was just written must never be read back from the cache.
    _invalidateGeneral();
    Promise.all([
      fetch(LaRuche.API.base+'/api/config/runtime',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}),
      fetch(LaRuche.API.base+'/api/config/provider',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify(auxiliary)})
    ])
      .then(function(responses){ if(responses.every(function(r){return r.ok;})) LaRuche.Toast.show(LaRuche.i18n.t('settings.generationApplied'),'ok'); else LaRuche.Toast.show(LaRuche.i18n.t('settings.errorGeneric'),'err'); })
      .catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('settings.errorColon')+e,'err'); });
  }

  function toggleCurateur(on) {
    fetch(LaRuche.API.base+'/api/config/curateur',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify({enabled:!!on})})
      .then(function(r){return r.json();})
      .then(function(d){ if(d && d.status==='ok') LaRuche.Toast.show('Curateur '+(on?LaRuche.i18n.t('settings.curateEnabled'):LaRuche.i18n.t('settings.curateDisabled')),'ok'); else LaRuche.Toast.show(LaRuche.i18n.t('settings.curateFailed'),'err'); })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.curateFailed'),'err'); });
  }
  // Duree de vie des episodes. Le reglage part a zero, "tout garder": effacer la
  // memoire de quelqu'un sans le lui demander ne se fait pas.
  function saveEpisodesCfg(jours) {
    fetch(LaRuche.API.base+'/api/config/curateur',{method:'POST',credentials:'include',
      headers:{'Content-Type':'application/json'},
      body:JSON.stringify({episodes_retention_jours: Number(jours)||0})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d && d.status==='ok') LaRuche.Toast.show(LaRuche.i18n.t('settings.episodesSaved'),'ok');
        else LaRuche.Toast.show(LaRuche.i18n.t('settings.episodesFailed'),'err');
      })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.episodesFailed'),'err'); });
  }

  // Effacement total. Irreversible, donc une confirmation qui DIT ce qui part et
  // combien: "vous etes sur ?" ne renseigne personne.
  function clearEpisodes() {
    fetch(LaRuche.API.base+'/api/memory/episodes',{credentials:'include'})
      .then(function(r){return r.json();})
      .catch(function(){return {days:0};})
      .then(function(ep){
        var n = (ep && ep.days) || 0;
        if(!n){ LaRuche.Toast.show(LaRuche.i18n.t('settings.episodesNone'),'ok'); return; }
        if(!confirm(LaRuche.i18n.t('settings.episodesConfirm', { n: n }))) return;
        return fetch(LaRuche.API.base+'/api/memory/episodes/purge',{method:'POST',credentials:'include',
          headers:{'Content-Type':'application/json'}, body:JSON.stringify({older_than_days:0})})
          .then(function(r){return r.json();})
          .then(function(d){
            if(d && d.status==='ok'){
              LaRuche.Toast.show(LaRuche.i18n.t('settings.episodesCleared', { n: d.deleted_days||0 }),'ok');
              refreshTab();
            } else LaRuche.Toast.show(LaRuche.i18n.t('settings.episodesFailed'),'err');
          });
      })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.episodesFailed'),'err'); });
  }

  // Le decor du pilotage. Applique immediatement cote noeud, donc le geste
  // suivant de l'agent obeit deja au nouveau reglage.
  function toggleHalo(on) {
    fetch(LaRuche.API.base+'/api/config/curateur',{method:'POST',credentials:'include',
      headers:{'Content-Type':'application/json'},body:JSON.stringify({halo_actif:!!on})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d && d.status==='ok') LaRuche.Toast.show(LaRuche.i18n.t('settings.haloSaved'),'ok');
        else LaRuche.Toast.show(LaRuche.i18n.t('settings.haloFailed'),'err');
      })
      .catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('settings.haloFailed'),'err'); });
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
      contexte_messages: parseInt(document.getElementById('cfgReineCtx').value,10),
      dataset: !!(document.getElementById('cfgReineDataset')||{}).checked
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
            loadKnowledge(document.getElementById('settingsContent'));
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
    // The provider list of the instantiation form comes from here. Cached by the Cron tab
    // only, opening Blueprints first would have offered nothing but the default.
    if(!window._lastProfiles){
      try{
        var pr = await fetch('/api/profiles').then(function(r){return r.json();});
        window._lastProfiles = (pr && pr.profiles) || {};
      }catch(e){ window._lastProfiles = {}; }
    }
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
              '<div class="settings-card-title">'+LaRuche.Utils.esc(b.title||b.id)+' '+bpBadgeCible(b.cible)+'</div>' +
              '<div style="font-size:12px;color:var(--text-dim);margin-top:4px;">'+LaRuche.Utils.esc(b.description||'')+'</div>' +
            '</div>' +
            '<button onclick="event.stopPropagation();LaRuche.Settings.deleteBlueprint('+idx+')" title="'+LaRuche.i18n.t('settings.bpDeleteBtn')+'" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;flex:0 0 auto">'+LaRuche.i18n.t('settings.bpDeleteBtn')+'</button>' +
          '</div>' +
          '<div id="bpForm_'+idx+'" style="display:none;margin-top:12px;padding-top:12px;border-top:1px solid var(--border);" onclick="event.stopPropagation()">' +
            (b.slots||[]).map(function(slot){
              return '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.Utils.esc(slot.label||slot.name)+'</label><input id="bpInput_'+idx+'_'+slot.name+'" class="form-input" placeholder="'+LaRuche.Utils.esc(slot.placeholder||slot.default||'')+'" value="'+LaRuche.Utils.esc(slot.default||'')+'"></div>';
            }).join('') +
            bpRoutageHtml(idx) +
            '<button class="settings-save-btn" style="margin-top:8px" onclick="LaRuche.Settings.instanciateBlueprint('+idx+')">'+bpLibelleInstancier(b.cible)+'</button>' +
          '</div>' +
        '</div>';
      }).join('');
    el.innerHTML = head + creationSlot + cards;
  }

  // What the blueprint builds, shown on its card: the six of them looked alike while
  // three now create very different things.
  function bpBadgeCible(cible){
    var c = cible || 'cron';
    var couleurs = { cron:'var(--amber)', watcher:'var(--cyan)', recherche:'var(--purple)' };
    var libelles = { cron:'settings.bpKindCron', watcher:'settings.bpKindWatcher', recherche:'settings.bpKindRecherche' };
    var col = couleurs[c] || 'var(--text-dim)';
    return '<span style="font-size:9px;text-transform:uppercase;letter-spacing:.4px;border:1px solid '+col+
      ';color:'+col+';border-radius:999px;padding:1px 7px;margin-left:6px;vertical-align:middle">'+
      LaRuche.Utils.esc(LaRuche.i18n.t(libelles[c] || libelles.cron))+'</span>';
  }
  function bpLibelleInstancier(cible){
    return LaRuche.i18n.t(cible === 'watcher' ? 'settings.bpCreateWatcher'
      : cible === 'recherche' ? 'settings.bpCreateRecherche'
      : 'settings.bpInstanciateBtn');
  }

  // Routing fields of an instantiation. A blueprint templates WHAT runs and WHEN, never
  // where the answer goes nor which model answers, so those three were absent from the
  // form and the created task always landed on the activity log with the default model.
  function bpRoutageHtml(idx){
    var profiles = window._lastProfiles || {};
    var esc = LaRuche.Utils.esc;
    var profOpts = '<option value="">'+LaRuche.i18n.t('settings.defaultModel')+'</option>';
    Object.keys(profiles).forEach(function(k){
      profOpts += '<option value="'+esc(k)+'">'+esc(profiles[k].name)+'</option>';
    });
    return '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.watcherChannelLabel')+'</label>'+
        '<select id="bpChannel_'+idx+'" class="form-input"></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.providerLabel')+'</label>'+
        '<select id="bpProfile_'+idx+'" class="form-input">'+profOpts+'</select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('settings.modelLabel')+'</label>'+
        '<input id="bpModel_'+idx+'" class="form-input" placeholder="'+LaRuche.i18n.t('settings.providerDefault')+'"></div>';
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
      body: JSON.stringify({
        slots: slotsData,
        channel: (document.getElementById('bpChannel_'+idx)||{}).value || '',
        profile_id: (document.getElementById('bpProfile_'+idx)||{}).value || '',
        model: (document.getElementById('bpModel_'+idx)||{}).value || ''
      })
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

  return { init:init, loadAdmin:loadAdmin, adminDeleteUser:adminDeleteUser, adminSetRole:adminSetRole, adminSetPassword:adminSetPassword, saveChatCfg:saveChatCfg, ouvrirSection:ouvrirSection, deepLink:deepLink, loadProfile:loadProfile, profileSaveName:profileSaveName, profileRemoveAvatar:profileRemoveAvatar, profileSavePassword:profileSavePassword, profileSaveFiche:profileSaveFiche, totpStart:totpStart, totpEnable:totpEnable, totpDisable:totpDisable, openBlueprintForm:openBlueprintForm, instanciateBlueprint:instanciateBlueprint, openNewBlueprintForm:openNewBlueprintForm, saveNewBlueprint:saveNewBlueprint, addBlueprintSlotRow:addBlueprintSlotRow, deleteBlueprint:deleteBlueprint, enter:enter, leave:leave, createCron:createCron, deleteCronTask:deleteCronTask, createWatcher:createWatcher, editWatcher:editWatcher, saveWatcherEdit:saveWatcherEdit, updateWatcherEditModelSelect:updateWatcherEditModelSelect, toggleWatcherCard:toggleWatcherCard, toggleWatcherActive:toggleWatcherActive, updateWatcherCardModelSelect:updateWatcherCardModelSelect, rechargerWatchers:rechargerWatchers, refreshTab:refreshTab, dock:dock, fermerDock:fermerDock,
    loadGeneral:loadGeneral, loadCron:loadCron, loadWatchers:loadWatchers, loadKanban:loadKanban, loadBlueprints:loadBlueprints, loadCronTimeline:loadCronTimeline, saveChannels:saveChannels, setChannelModel:setChannelModel, saveContextCfg:saveContextCfg, saveRuntimeCfg:saveRuntimeCfg, saveReineCfg:saveReineCfg, reineToggleUnlim:reineToggleUnlim, renderReineProposals:renderReineProposals, reineApprove:reineApprove, reineReject:reineReject, reineApplySafe:reineApplySafe, toggleCurateur:toggleCurateur, toggleDynamicTools:toggleDynamicTools, toggleHalo:toggleHalo, saveEpisodesCfg:saveEpisodesCfg, clearEpisodes:clearEpisodes, saveVoiceCfg:saveVoiceCfg, addKnowledge:addKnowledge, exportOkf:exportOkf, importOkf:importOkf, deleteKnowledge:deleteKnowledge, editKnowledge:editKnowledge, saveKnowledgeEdit:saveKnowledgeEdit, startChannel:startChannel, stopChannel:stopChannel, showProfileForm:showProfileForm, editProfile:editProfile, deleteProfile:deleteProfile, testProfile:testProfile, saveProfile:saveProfile, onProfileProviderChange:onProfileProviderChange, startCodexLogin:startCodexLogin, logoutCodex:logoutCodex, toggleTool:toggleTool, toggleAllTools:toggleAllTools, loadSkills:loadSkills, toggleSkill:toggleSkill, deleteSkill:deleteSkill, newSkill:newSkill, viewSkill:viewSkill, saveSkill:saveSkill, applySkillTools:applySkillTools, toggleSkillTool:toggleSkillTool, filterSkillTools:filterSkillTools, clearSkillTools:clearSkillTools, newPlugin:newPlugin, viewPlugin:viewPlugin, savePlugin:savePlugin, deletePlugin:deletePlugin, createKanbanTask:createKanbanTask, setKanbanDefaultChannel:setKanbanDefaultChannel, setKanbanInterval:setKanbanInterval, loadSecrets: loadSecrets, secretSet: secretSet, secretDelete: secretDelete, reineDataset: reineDataset, secretUpdate: secretUpdate, secretPick: secretPick, secretPickCreate: secretPickCreate, loadMcp: loadMcp, loadMcpServers: loadMcpServers, loadMcpPorte: loadMcpPorte, saveMcpPorte: saveMcpPorte, mcpUnban: mcpUnban, gotoMcpCapabilities: gotoMcpCapabilities, deleteMcpServer: deleteMcpServer, updateKanbanModelSelect: updateKanbanModelSelect, updateKanbanEditModelSelect: updateKanbanEditModelSelect, updateWatcherModelSelect: updateWatcherModelSelect, editCronTask:editCronTask, lancerCronTask:lancerCronTask, visionReessayer:visionReessayer, saveCronTask:saveCronTask, majModelesEdition:majModelesEdition, deleteKanbanTask:deleteKanbanTask, editKanbanTask:editKanbanTask, saveKanbanEdit:saveKanbanEdit, toggleKanbanResult:toggleKanbanResult, setKanbanView:setKanbanView, lancerKanbanTask:lancerKanbanTask, adminPickAvatar:adminPickAvatar, loadKanbanTodo:loadKanbanTodo, saveKanbanTodo:saveKanbanTodo, kanbanTodoMaintenant:kanbanTodoMaintenant, addCredential:addCredential, deleteCredential:deleteCredential, updateCronModelSelect:updateCronModelSelect, updateCronEditModelSelect:updateCronEditModelSelect, toggleVisibility:toggleVisibility, openAccess:openAccess, tlZoom:tlZoom, tlRecenter:tlRecenter, tlDetail:tlDetail, tlAll:tlAll, tlReload:tlReload, tlRun:tlRun, tlEdit:tlEdit, tlSaveEdit:tlSaveEdit, tlToggle:tlToggle };
})();

/* ── CronBuilder: reusable "human-friendly" component (missions + cron) ── */
