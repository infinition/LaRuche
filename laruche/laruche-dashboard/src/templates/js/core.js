/* ================================================================ */
/*  LaRuche SPA - Global Namespace                                  */
/* ================================================================ */
window.LaRuche = {};

/* i18n: FR/EN language choice.
 * Central dictionary { "key": { fr, en } }. t('key') returns the string in the current
 * language (localStorage). LaRuche brand terms (LaRuche, l'essaim, butinage, Miel, ruche,
 * Curateur) are never translated; that is the french touch. The toggle reloads the page so
 * everything re-renders through t() in the new language. Each module registers its own keys. */
LaRuche.i18n = (function(){
  var DICT = {
    // Common
    'common.save':    { fr:'Enregistrer', en:'Save' },
    'common.cancel':  { fr:'Annuler',     en:'Cancel' },
    'common.delete':  { fr:'Supprimer',   en:'Delete' },
    'common.close':   { fr:'Fermer',      en:'Close' },
    'common.edit':    { fr:'Éditer',      en:'Edit' },
    'common.apply':   { fr:'Appliquer',   en:'Apply' },
    'common.create':  { fr:'Créer',       en:'Create' },
    'common.loading': { fr:'Chargement…', en:'Loading…' },
    'common.search':  { fr:'Rechercher',  en:'Search' },
    'common.none':    { fr:'Aucun',       en:'None' },
    'common.range7d': { fr:'7j',          en:'7d' },
    // Frequent toasts
    'toast.saved':    { fr:'Enregistré',  en:'Saved' },
    'toast.deleted':  { fr:'Supprimé',    en:'Deleted' },
    'toast.failed':   { fr:'Échec',       en:'Failed' },
    'toast.added':    { fr:'Ajouté',      en:'Added' },
    // Navigation
    'nav.chat':         { fr:'Chat',        en:'Chat' },
    'nav.memory':       { fr:'Mémoire',     en:'Memory' },
    'nav.missions':     { fr:'Missions',    en:'Missions' },
    'nav.capabilities': { fr:'Capacités',   en:'Capabilities' },
    'nav.dashboard':    { fr:'Dashboard',   en:'Dashboard' },
    'nav.settings':     { fr:'Settings',    en:'Settings' },
  };
  // Active language: injected by the node server (window.__LANG__), else localStorage.
  var lang = (typeof window !== 'undefined' && window.__LANG__) ? window.__LANG__
             : ((localStorage.getItem('laruche_lang') === 'en') ? 'en' : 'fr');
  // OVERRIDE: flat { key: string } map injected from laruche/lang/<code>.json (single source of
  // truth). Falls back to the inline DICT below if the file/injection is absent.
  function t(key, vars){
    var ov = (typeof window !== 'undefined' && window.__I18N__) ? window.__I18N__ : null;
    var s;
    if(ov && ov[key] != null){ s = ov[key]; }
    else { var e = DICT[key]; s = e ? (e[lang] || e.fr || key) : key; }
    if(vars){ for(var k in vars){ s = String(s).split('{'+k+'}').join(vars[k]); } }
    return s;
  }
  function setLang(l){
    if(l !== lang){
      localStorage.setItem('laruche_lang', l);
      document.cookie = 'laruche_lang=' + l + ';path=/;max-age=31536000;samesite=lax';
      location.reload();
    }
  }
  // Each module registers its own keys at load time, which avoids editing a central dict and conflicts.
  function add(obj){ if(obj){ for(var k in obj){ DICT[k] = obj[k]; } } }
  // Translate the static HTML shell (spa.html). Elements carry data-i18n (textContent),
  // data-i18n-html (innerHTML), data-i18n-ph (placeholder) or data-i18n-title (title).
  // Called once at boot so the shell follows the active language like the JS-rendered views.
  function applyStatic(root){
    root = root || document;
    root.querySelectorAll('[data-i18n]').forEach(function(el){ el.textContent = t(el.getAttribute('data-i18n')); });
    root.querySelectorAll('[data-i18n-html]').forEach(function(el){ el.innerHTML = t(el.getAttribute('data-i18n-html')); });
    root.querySelectorAll('[data-i18n-ph]').forEach(function(el){ el.setAttribute('placeholder', t(el.getAttribute('data-i18n-ph'))); });
    root.querySelectorAll('[data-i18n-title]').forEach(function(el){ el.setAttribute('title', t(el.getAttribute('data-i18n-title'))); });
  }
  return { t:t, add:add, setLang:setLang, applyStatic:applyStatic, get:function(){ return lang; }, DICT:DICT };
})();

LaRuche.i18n.add({
  // PluginFiles
  'core.pluginFilesTitle':   { fr:'📁 Fichiers du dossier <code>plugins/</code>', en:'📁 Files in <code>plugins/</code> folder' },
  'core.newFile':            { fr:'+ Nouveau fichier',    en:'+ New file' },
  'core.folderEmpty':        { fr:'Dossier vide.',        en:'Empty folder.' },
  'core.fileHint':           { fr:"Sélectionne un fichier à gauche, ou glisse-dépose un script ici pour l'ajouter.", en:'Select a file on the left, or drag and drop a script here to add it.' },
  'core.binaryFile':         { fr:'Fichier binaire ({size} o) - non éditable ici.', en:'Binary file ({size} bytes) - not editable here.' },
  'core.savedPlugins':       { fr:'Enregistré (plugins rechargés)', en:'Saved (plugins reloaded)' },
  'core.deleteFileConfirm':  { fr:'Supprimer plugins/{path} ?', en:'Delete plugins/{path}?' },
  'core.fileDeleted':        { fr:'Fichier supprimé.',    en:'File deleted.' },
  'core.newFilePrompt':      { fr:'Nom du fichier (ex: scripts/mon_script.py) :', en:'File name (e.g. scripts/my_script.py):' },
  'core.invalidName':        { fr:'Nom invalide',         en:'Invalid name' },
  'core.fileAdded':          { fr:'Ajouté : {dest}',      en:'Added: {dest}' },
  'core.fileRejected':       { fr:'Refusé : {name}',      en:'Rejected: {name}' },
  // Utils
  'core.fileLabel':          { fr:'Fichier',              en:'File' },
  // Console events / toasts
  'core.sessionFinishedPreview': { fr:'Réponse terminée', en:'Response finished' },
  'core.agentFinishedToast':    { fr:'Agent terminé: {preview}', en:'Agent finished: {preview}' },
  'core.agentStartedLabel':     { fr:'Agent démarré',     en:'Agent started' },
  'core.agentStartedToast':     { fr:'Agent démarré: {label}', en:'Agent started: {label}' },
  'core.watcherFiredToast':     { fr:'Watcher déclenché: {name}', en:'Watcher fired: {name}' },
  'core.kanbanTaskTitle':       { fr:'Tâche {id}',        en:'Task {id}' },
  'core.kanbanToast':           { fr:'Kanban: {title}',   en:'Kanban: {title}' },
  // Auth
  'core.enrollError':           { fr:'Erreur enrollment: {msg}', en:'Enrollment error: {msg}' },
  'core.welcomeUser':           { fr:'Bienvenue {name} !', en:'Welcome {name}!' },
  'core.namePasswordRequired':  { fr:'Nom et mot de passe requis', en:'Name and password required' },
  'core.badCredentials':        { fr:'Identifiants incorrects', en:'Incorrect credentials' },
  'core.passwordMin':           { fr:'Mot de passe : 6 caractères minimum', en:'Password: 6 characters minimum' },
  'core.nameTaken':             { fr:'Ce nom est déjà pris. Connecte-toi.', en:'That name is already taken. Please log in.' },
  'core.enrollFailed':          { fr:'Création du compte impossible', en:'Could not create the account' },
  'core.totpRequired':          { fr:'Entre ton code à 6 chiffres (2FA)', en:'Enter your 6-digit code (2FA)' },
  'core.helloUser':             { fr:'Bonjour {name} !',  en:'Hello {name}!' },
  'core.challengeExpires':      { fr:'Expire dans {s}s',  en:'Expires in {s}s' },
  'core.challengeError':        { fr:'Erreur challenge: {msg}', en:'Challenge error: {msg}' },
  // Header / permissions
  'core.permChangeFailed':      { fr:'Échec changement permissions', en:'Failed to change permissions' },
  // Blueprints
  'core.noBlueprintsAvailable': { fr:'Aucun blueprint disponible', en:'No blueprints available' },
  'core.blueprintHint':         { fr:"Sélectionnez un blueprint pour l'instancier en tant que tâche cron.", en:'Select a blueprint to instantiate it as a cron task.' },
  'core.instantiate':           { fr:'Instancier',         en:'Instantiate' },
  'core.blueprintOk':           { fr:'Blueprint instancié avec succès', en:'Blueprint instantiated successfully' },
  'core.blueprintErr':          { fr:"Erreur d'instanciation", en:'Instantiation error' },
  'core.errorPrefix':           { fr:'Erreur: {msg}',      en:'Error: {msg}' },
  // Secrets autocomplete
  'core.secretsHeader':         { fr:'Secrets',            en:'Secrets' },
  // Media / attachments
  'core.imageLabel':            { fr:'Image',              en:'Image' },
  'core.fileFallback':          { fr:'FICHIER',            en:'FILE' },
  'core.cannotPreviewFile':     { fr:'Impossible de prévisualiser ce fichier.', en:'Cannot preview this file.' },
  'core.cannotPreviewBinary':   { fr:'Impossible de prévisualiser un fichier binaire.', en:'Cannot preview binary file.' },
  'core.fileContentUnavailable':{ fr:'Contenu du fichier indisponible.', en:'File content not available.' },
  // Console
  'core.consoleEntries':        { fr:'{n} entrées',        en:'{n} entries' },
  // Watcher fallback
  'core.watcherLabel':          { fr:'Watcher',            en:'Watcher' },
  // Header / model
  'core.permissionsToast':      { fr:'Permissions : {mode}', en:'Permissions: {mode}' },
  'core.modelAuto':             { fr:'Auto',               en:'Auto' },
  'core.modelReady':            { fr:'Modèle {model} prêt ({ms}ms)', en:'Model {model} ready ({ms}ms)' },
  'core.modelSelected':         { fr:'Modèle : {model}',   en:'Model: {model}' },
});

/* ── Plugins file browser (plugins/ + scripts/ folder) ─────────────────
 * View/edit/delete/drop your own scripts (.py/.ps1/.sh/.json…) in addition to the JSON.
 * Server-side confined to plugins/ (anti-traversal). Drag and drop = upload. */
LaRuche.PluginFiles = (function(){
  var ov=null, current=null;
  function esc(s){ return LaRuche.Utils.esc(s); }
  function close(){ if(ov){ ov.remove(); ov=null; current=null; } }
  function open(){
    if(ov) close();
    ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) close(); };
    ov.innerHTML='<div style="width:880px;max-width:95vw;height:84vh;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;display:flex;flex-direction:column">'+
      '<div style="padding:10px 14px;border-bottom:1px solid var(--border);font-weight:600;color:var(--amber);display:flex;align-items:center;gap:10px">'+
        '<span style="flex:1">'+LaRuche.i18n.t('core.pluginFilesTitle')+'</span>'+
        '<button class="tl-btn" onclick="LaRuche.PluginFiles.newFile()">'+LaRuche.i18n.t('core.newFile')+'</button>'+
        '<button class="tl-btn" onclick="LaRuche.PluginFiles.close()">'+LaRuche.i18n.t('common.close')+'</button>'+
      '</div>'+
      '<div style="flex:1;display:flex;min-height:0">'+
        '<div id="pfTree" style="width:260px;border-right:1px solid var(--border);overflow:auto;padding:6px;font-size:12px"></div>'+
        '<div id="pfMain" style="flex:1;display:flex;flex-direction:column;min-width:0;padding:8px">'+
          '<div id="pfHint" style="color:var(--text-dim);font-size:12px;padding:20px;text-align:center">'+LaRuche.i18n.t('core.fileHint')+'</div>'+
        '</div>'+
      '</div></div>';
    document.body.appendChild(ov);
    // Drag and drop = upload into plugins/ (or plugins/scripts/ for .py/.sh/.ps1).
    var card=ov.firstChild;
    card.addEventListener('dragover', function(e){ e.preventDefault(); card.style.outline='2px dashed var(--amber)'; });
    card.addEventListener('dragleave', function(){ card.style.outline=''; });
    card.addEventListener('drop', function(e){ e.preventDefault(); card.style.outline=''; handleDrop(e); });
    refresh();
  }
  function refresh(){
    fetch('/api/plugin-files').then(function(r){return r.json();}).then(function(d){
      var files=(d&&d.files)||[];
      var t=document.getElementById('pfTree'); if(!t) return;
      if(!files.length){ t.innerHTML='<div style="color:var(--text-dim);padding:8px">'+LaRuche.i18n.t('core.folderEmpty')+'</div>'; return; }
      t.innerHTML=files.map(function(f){
        if(f.dir) return '<div style="padding:4px 6px;color:var(--text-dim);font-weight:600;margin-top:4px">📂 '+esc(f.path)+'</div>';
        var indent = f.path.indexOf('/')>=0 ? 16 : 4;
        var kb = f.size!=null ? ' <span style="color:var(--text-dim);font-size:10px">'+Math.max(1,Math.round(f.size/1024))+'k</span>' : '';
        return '<div class="pf-file" data-path="'+esc(f.path)+'" style="padding:4px 6px;padding-left:'+indent+'px;border-radius:4px;cursor:pointer;display:flex;align-items:center;gap:5px"><span>📄</span><span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">'+esc(f.path.split('/').pop())+'</span>'+kb+'</div>';
      }).join('');
      Array.prototype.forEach.call(t.querySelectorAll('.pf-file'), function(el){
        el.onclick=function(){ openFile(el.dataset.path); Array.prototype.forEach.call(t.querySelectorAll('.pf-file'),function(x){x.style.background='';}); el.style.background='rgba(245,158,11,.15)'; };
      });
    });
  }
  function openFile(path){
    fetch('/api/plugin-file/'+path.split('/').map(encodeURIComponent).join('/')).then(function(r){return r.json();}).then(function(d){
      var m=document.getElementById('pfMain'); if(!m) return;
      current=path;
      if(d.binary){ m.innerHTML='<div style="color:var(--text-dim);padding:20px">'+LaRuche.i18n.t('core.binaryFile',{size:d.size||'?'})+'</div>'; return; }
      m.innerHTML='<div style="display:flex;align-items:center;gap:8px;margin-bottom:6px"><code style="flex:1;color:var(--amber)">plugins/'+esc(path)+'</code>'+
        '<button class="tl-btn" onclick="LaRuche.PluginFiles.save()">'+LaRuche.i18n.t('common.save')+'</button>'+
        '<button class="tl-btn" style="border-color:var(--red);color:var(--red)" onclick="LaRuche.PluginFiles.del()">'+LaRuche.i18n.t('common.delete')+'</button></div>'+
        '<textarea id="pfEditor" spellcheck="false" style="flex:1;width:100%;font-family:var(--mono);font-size:12px;background:#16161a;border:1px solid var(--border);border-radius:6px;color:var(--text);padding:8px;resize:none">'+esc(d.content||'')+'</textarea>';
    });
  }
  function save(){
    if(!current) return;
    var ta=document.getElementById('pfEditor'); if(!ta) return;
    fetch('/api/plugin-file/'+current.split('/').map(encodeURIComponent).join('/'),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:ta.value})})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('core.savedPlugins'),'ok'); refresh(); } else LaRuche.Toast.show(LaRuche.i18n.t('toast.failed'),'err'); });
  }
  function del(){
    if(!current || !confirm(LaRuche.i18n.t('core.deleteFileConfirm',{path:current}))) return;
    fetch('/api/plugin-file/'+current.split('/').map(encodeURIComponent).join('/'),{method:'DELETE'})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('toast.deleted'),'ok'); current=null; document.getElementById('pfMain').innerHTML='<div id="pfHint" style="color:var(--text-dim);padding:20px;text-align:center">'+LaRuche.i18n.t('core.fileDeleted')+'</div>'; refresh(); } });
  }
  function newFile(){
    var name=prompt(LaRuche.i18n.t('core.newFilePrompt'),''); if(!name) return;
    fetch('/api/plugin-file/'+name.split('/').map(encodeURIComponent).join('/'),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:''})})
      .then(function(r){ if(r.ok){ refresh(); openFile(name); } else LaRuche.Toast.show(LaRuche.i18n.t('core.invalidName'),'err'); });
  }
  function handleDrop(e){
    var files=e.dataTransfer.files; if(!files||!files.length) return;
    Array.prototype.forEach.call(files, function(file){
      var reader=new FileReader();
      reader.onload=function(){
        var ext=(file.name.split('.').pop()||'').toLowerCase();
        var dest=(['py','sh','ps1'].indexOf(ext)>=0 ? 'scripts/' : '')+file.name;
        fetch('/api/plugin-file/'+dest.split('/').map(encodeURIComponent).join('/'),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:reader.result})})
          .then(function(r){ if(r.ok){ LaRuche.Toast.show(LaRuche.i18n.t('core.fileAdded',{dest:dest}),'ok'); refresh(); } else LaRuche.Toast.show(LaRuche.i18n.t('core.fileRejected',{name:file.name}),'err'); });
      };
      reader.readAsText(file);
    });
  }
  return { open:open, close:close, save:save, del:del, newFile:newFile, refresh:refresh };
})();

/* ── @@secret: CENTRAL secret autocompletion ──────────────────────────────
 * Type `@@` in ANY text field (chat, cron/mission/skill/watcher prompts,
 * memory editor, forms…) to get a clickable list of saved secrets. The VALUE is
 * never displayed: we insert the `@@NAME` reference, which the backend substitutes at
 * execution time (shell, web_fetch, provider, webhook via curl). Works without per-field
 * wiring: a single capture-phase listener on the whole document. */
LaRuche.Secrets = (function(){
  var names=[], box=null, items=[], sel=-1, targetEl=null, tokenStart=-1;
  function refresh(){ fetch('/api/secrets').then(function(r){return r.json();}).then(function(d){ names=(d&&d.names)||[]; }).catch(function(){}); }
  function isText(el){ return el && (el.tagName==='TEXTAREA' || (el.tagName==='INPUT' && (el.type==='text'||el.type==='search'||el.type===''))); }
  function ensureBox(){ if(box)return box; box=document.createElement('div'); box.id='secretAC'; box.style.cssText='position:fixed;z-index:100001;display:none;background:#16161a;border:1px solid var(--amber);border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,.55);max-height:220px;overflow:auto;min-width:170px;font-size:12px'; document.body.appendChild(box); return box; }
  function hide(){ if(box) box.style.display='none'; targetEl=null; sel=-1; }
  function currentToken(el){ var pos=el.selectionStart; if(pos==null)return null; var m=el.value.slice(0,pos).match(/@@([A-Za-z0-9_\-]*)$/); return m?{start:pos-m[0].length, partial:m[1]}:null; }
  function paint(){ if(!box)return; Array.prototype.forEach.call(box.querySelectorAll('.sac-item'), function(it,i){ it.style.background=(i===sel)?'rgba(245,158,11,.2)':''; }); }
  function render(el, tok){
    var f=tok.partial.toLowerCase();
    items = names.filter(function(n){ return n.toLowerCase().indexOf(f)>=0; });
    if(!items.length){ hide(); return; }
    sel=0; targetEl=el; tokenStart=tok.start;
    var b=ensureBox();
    b.innerHTML = '<div style="padding:3px 10px;color:var(--text-dim);font-size:9px;text-transform:uppercase;letter-spacing:.5px;border-bottom:1px solid var(--border)">'+LaRuche.i18n.t('core.secretsHeader')+'</div>'+
      items.map(function(n,i){ return '<div class="sac-item" data-i="'+i+'" style="padding:6px 10px;cursor:pointer;color:var(--amber);'+(i===0?'background:rgba(245,158,11,.2)':'')+'">@@'+LaRuche.Utils.esc(n)+'</div>'; }).join('');
    var r=el.getBoundingClientRect();
    b.style.left=Math.min(r.left, window.innerWidth-200)+'px'; b.style.top=(r.bottom+2)+'px'; b.style.display='block';
    Array.prototype.forEach.call(b.querySelectorAll('.sac-item'), function(it){ it.onmousedown=function(e){ e.preventDefault(); choose(parseInt(it.dataset.i)); }; });
  }
  function choose(i){
    if(!targetEl||!items[i])return;
    var el=targetEl, name=items[i], pos=el.selectionStart;
    el.value = el.value.slice(0,tokenStart)+'@@'+name+el.value.slice(pos);
    var caret=tokenStart+2+name.length; el.selectionStart=el.selectionEnd=caret;
    el.dispatchEvent(new Event('input',{bubbles:true})); hide(); el.focus();
  }
  function onInput(e){ var el=e.target; if(!isText(el)||!names.length){ return; } var t=currentToken(el); if(t) render(el,t); else hide(); }
  function onKey(e){
    if(!box||box.style.display==='none')return;
    if(e.key==='ArrowDown'){ e.preventDefault(); e.stopPropagation(); sel=Math.min(sel+1,items.length-1); paint(); }
    else if(e.key==='ArrowUp'){ e.preventDefault(); e.stopPropagation(); sel=Math.max(sel-1,0); paint(); }
    else if(e.key==='Enter'||e.key==='Tab'){ e.preventDefault(); e.stopPropagation(); choose(sel); }
    else if(e.key==='Escape'){ e.stopPropagation(); hide(); }
  }
  function init(){
    refresh(); setInterval(refresh, 45000);
    document.addEventListener('input', onInput, true);
    document.addEventListener('keydown', onKey, true);
    document.addEventListener('click', function(e){ if(box && box.style.display!=='none' && !box.contains(e.target)) hide(); }, true);
  }
  return { init:init, refresh:refresh };
})();

/* ── Global dynamic refresh (P7) ──────────────────────
 * After any mutating action (model choice, visibility, capability,
 * cron...), re-fetch + re-render the affected components WITHOUT F5.
 * Module references are resolved at call time (lazy) because this
 * function is defined before the modules. parts = optional subset
 * to refresh: 'models','mesh','capabilities','profiles','voice','cron'.
 */
LaRuche.refreshAll = function(parts) {
  var all = !parts || !parts.length;
  function want(p){ return all || parts.indexOf(p) !== -1; }
  try {
    // Top model dropdown + global capability badge
    if(want('models') && LaRuche.Header && LaRuche.Header.loadModels) LaRuche.Header.loadModels();
    // "Mesh services" panel + capabilities recap + voice selectors (Dashboard.fetchModels
    // re-reads /swarm/models AND /api/capabilities/selection, and updates the voices).
    if((want('mesh') || want('capabilities') || want('voice')) && LaRuche.Dashboard && LaRuche.Dashboard.fetchModels) LaRuche.Dashboard.fetchModels();
    // Voice status (selected STT/TTS)
    if(want('voice') && LaRuche.Voice && LaRuche.Voice.refreshStatus) LaRuche.Voice.refreshStatus();
    // Current Settings tab (profiles, cron, watchers, kanban...) if visible
    if(want('profiles') || want('cron')) {
      if(LaRuche.Settings && LaRuche.Settings.refreshTab && LaRuche.Router && LaRuche.Router.current && LaRuche.Router.current() === 'settings') {
        LaRuche.Settings.refreshTab();
      }
    }
  } catch(e){ /* best-effort, never break the action */ }
};
// Historical alias (already called in the visibility code)
LaRuche.forceReactivityUpdate = function(){ LaRuche.refreshAll(); };

/* ── Utils ─────────────────────────────────────────────────────── */
LaRuche.Utils = {
  esc: function(t) { var d=document.createElement('div'); d.textContent=t; return d.innerHTML; },
  clamp: function(v,lo,hi) { return Math.min(hi,Math.max(lo,v)); },
  fmtMB: function(mb) { return mb>=1024?(mb/1024).toFixed(1)+' GB':mb+' MB'; },
  fmtTime: function(d) {
    return String(d.getHours()).padStart(2,'0')+':'+String(d.getMinutes()).padStart(2,'0')+':'+String(d.getSeconds()).padStart(2,'0');
  },
  fmtDuration: function(ms) {
    var s=Math.floor(ms/1000);
    if(s<60)return s+'s';
    if(s<3600)return Math.floor(s/60)+'m';
    var h=Math.floor(s/3600),m=Math.floor((s%3600)/60);
    return h+'h'+(m>0?m+'m':'');
  },
  formatElapsed: function(ms) { return ms<1000?ms+'ms':(ms/1000).toFixed(1)+'s'; },
  normalizeCap: function(cap) { return String(cap||'').toLowerCase().replace(/^capability:/,'').trim(); },
  b64ToUtf8: function(b64) {
    try {
      var bin = atob(b64);
      var bytes = new Uint8Array(bin.length);
      for(var i=0; i<bin.length; i++) bytes[i] = bin.charCodeAt(i);
      return new TextDecoder().decode(bytes);
    } catch(e) { return decodeURIComponent(escape(window.atob(b64))); }
  },
  openMediaModal: function(type, srcOrText) {
    var m = document.getElementById('mediaModal');
    var img = document.getElementById('mediaModalImg');
    var txt = document.getElementById('mediaModalText');
    if(!m) {
      m = document.createElement('div'); m.id='mediaModal';
      m.style.cssText = 'position:fixed; top:0; left:0; width:100%; height:100%; background:rgba(0,0,0,0.85); z-index:10000; display:none; align-items:center; justify-content:center; backdrop-filter:blur(5px);';
      m.onclick=function(){m.style.display='none';};
      m.innerHTML='<div style="position:relative; max-width:90%; max-height:90%; display:flex; flex-direction:column; align-items:center;" onclick="event.stopPropagation()"><span onclick="document.getElementById(\'mediaModal\').style.display=\'none\'" style="position:absolute; top:-40px; right:0; color:#fff; font-size:30px; cursor:pointer; font-weight:bold;">&times;</span><img id="mediaModalImg" style="display:none; max-width:100%; max-height:85vh; object-fit:contain; border-radius:8px; box-shadow:0 10px 30px rgba(0,0,0,0.5);"><div id="mediaModalText" style="display:none; width:80vw; height:80vh; overflow:auto; background:#1e1e1e; color:#ccc; padding:20px; font-family:monospace; font-size:13px; white-space:pre-wrap; border-radius:8px; text-align:left; box-shadow:0 10px 30px rgba(0,0,0,0.5);"></div></div>';
      document.body.appendChild(m);
      img = document.getElementById('mediaModalImg');
      txt = document.getElementById('mediaModalText');
    }
    img.style.display='none'; txt.style.display='none';
    if(type === 'image') {
      img.src = srcOrText;
      img.style.display='block';
    } else {
      txt.textContent = srcOrText;
      txt.style.display='block';
    }
    m.style.display='flex';
  },
  createAttachmentBox: function(att, isPending, index) {
    var box = document.createElement('div');
    box.className = 'chat-attachment-box';
    var titleStr = att.filename || (att.kind === 'image' ? LaRuche.i18n.t('core.imageLabel') : LaRuche.i18n.t('core.fileLabel'));
    box.title = titleStr;
    box.style.cssText = 'position:relative; display:inline-flex; flex-direction:column; align-items:center; justify-content:center; gap:6px; background:rgba(0,0,0,0.3); padding:4px; border-radius:8px; border:1px solid var(--border); overflow:hidden; cursor:pointer; width: 80px; height: 80px; transition:border-color 0.2s;';
    box.onmouseover = function() { box.style.borderColor='var(--primary)'; };
    box.onmouseout = function() { box.style.borderColor='var(--border)'; };

    var nameOverlay = document.createElement('div');
    nameOverlay.textContent = titleStr;
    nameOverlay.style.cssText = 'position:absolute; bottom:0; left:0; width:100%; background:rgba(0,0,0,0.85); color:#fff; font-size:10px; padding:4px; text-align:center; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; opacity:0; transition:opacity 0.2s; pointer-events:none; box-sizing:border-box;';
    
    box.onmouseenter = function() { box.style.borderColor='var(--primary)'; nameOverlay.style.opacity='1'; };
    box.onmouseleave = function() { box.style.borderColor='var(--border)'; nameOverlay.style.opacity='0'; };

    if (att.kind === 'image') {
        var img = document.createElement('img');
        var src = att.fileUrl || ('data:' + att.mime_type + ';base64,' + att.data);
        img.src = src;
        img.style.cssText = 'width:100%; height:100%; object-fit:cover; border-radius:4px;';
        box.onclick = function() { LaRuche.Utils.openMediaModal('image', src); };
        box.appendChild(img);
    } else {
        var icon = document.createElement('div');
        icon.textContent = '\uD83D\uDCC4';
        icon.style.cssText = 'font-size:28px; color:var(--text-dim); margin-top: 4px;';
        box.appendChild(icon);
        
        var ext = document.createElement('div');
        var extMatch = titleStr.match(/\.([^.]+)$/);
        ext.textContent = extMatch ? extMatch[1].toUpperCase() : LaRuche.i18n.t('core.fileFallback');
        ext.style.cssText = 'font-size:10px; font-weight:bold; color:var(--text-dim); margin-top: -2px;';
        box.appendChild(ext);
        
        box.onclick = function() { 
            if(att.fileUrl) { 
                fetch(att.fileUrl).then(function(r){return r.text();}).then(function(txt){
                    LaRuche.Utils.openMediaModal('text', txt);
                }).catch(function(){
                    LaRuche.Utils.openMediaModal('text', LaRuche.i18n.t('core.cannotPreviewFile'));
                });
            } else if(att.data) {
                try {
                    var text = LaRuche.Utils.b64ToUtf8(att.data);
                    LaRuche.Utils.openMediaModal('text', text); 
                } catch(e) {
                    LaRuche.Utils.openMediaModal('text', LaRuche.i18n.t('core.cannotPreviewBinary'));
                }
            } else {
                LaRuche.Utils.openMediaModal('text', LaRuche.i18n.t('core.fileContentUnavailable'));
            }
        };
    }
    
    if(isPending) {
        var rm = document.createElement('div');
        rm.innerHTML = '&times;';
        rm.style.cssText = 'position:absolute; top:2px; right:2px; width:16px; height:16px; background:rgba(0,0,0,0.6); color:#fff; border-radius:50%; display:flex; align-items:center; justify-content:center; font-size:12px; cursor:pointer; transition:background 0.2s; z-index:10;';
        rm.onmouseover = function(e){ rm.style.background='var(--red)'; e.stopPropagation(); };
        rm.onmouseout = function(e){ rm.style.background='rgba(0,0,0,0.6)'; e.stopPropagation(); };
        rm.onclick = function(e) { e.stopPropagation(); LaRuche.Chat.removePendingFile(index); };
        box.appendChild(rm);
    }

    box.appendChild(nameOverlay);
    return box;
  }
};

/* ── API ───────────────────────────────────────────────────────── */
LaRuche.API = (function(){
  var base = window.location.protocol+'//'+window.location.hostname+':'+(window.location.port||'8419');
  if(window.location.port==='8420') base=window.location.protocol+'//'+window.location.hostname+':8419';
  return {
    base: base,
    get: function(path){ return fetch(base+path).then(function(r){if(!r.ok)throw new Error('HTTP '+r.status);return r.json();}); },
    post: function(path,body){ return fetch(base+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(function(r){if(!r.ok)throw new Error('HTTP '+r.status);return r.json();}); },
    del: function(path){ return fetch(base+path,{method:'DELETE'}); },
    getText: function(path){ return fetch(base+path).then(function(r){return r.text();}); },
    getLocal: function(path){ return fetch(path).then(function(r){if(!r.ok)throw new Error('HTTP '+r.status);return r.json();}); },
    postLocal: function(path,body){ return fetch(path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(function(r){return r.json();}); },
    delLocal: function(path){ return fetch(path,{method:'DELETE'}); }
  };
})();

/* ── Toast ─────────────────────────────────────────────────────── */
LaRuche.Toast = {
  show: function(msg, type, duration, onClick) {
    type = type || 'info';
    duration = duration || 5100;
    var c = document.getElementById('toasts');
    var t = document.createElement('div');
    t.className = 'toast toast-'+type;
    t.textContent = msg;
    if(onClick) {
      t.style.cursor = 'pointer';
      t.addEventListener('click', function(){ onClick(); t.remove(); });
    }
    c.appendChild(t);
    setTimeout(function(){ if(t.parentElement) t.remove(); }, duration);
  }
};

/* ── Console ───────────────────────────────────────────────────── */
LaRuche.Console = (function(){
  var entries = [];
  var activeFilter = 'all';
  var origError = console.error;
  var origWarn = console.warn;
  var lastEventId = 0;
  var pollTimer = null;

  // P8: open the session tied to an event (Chat tab + re-attach live stream).
  function openSession(sid) {
    if(!sid) return;
    LaRuche.Router.go('chat');
    setTimeout(function(){ if(LaRuche.Chat && LaRuche.Chat.switchSession) LaRuche.Chat.switchSession(sid); }, 120);
  }

  // P8: clickable notification/toast for events carrying a session_id/task_id.
  function notifyEvent(ev) {
    if(!ev || !ev.payload) return;
    var pl = ev.payload;
    var sid = pl.session_id || pl.task_session_id || null;
    var tid = pl.task_id || null;
    switch(ev.kind) {
      case 'SessionFinished': {
        var preview = pl.preview || LaRuche.i18n.t('core.sessionFinishedPreview');
        LaRuche.Toast.show(LaRuche.i18n.t('core.agentFinishedToast',{preview:String(preview).substring(0,60)+'…'}), 'ok', 8000,
          sid ? function(){ openSession(sid); } : null);
        break;
      }
      case 'AgentFinished': { if (typeof LaRuche !== 'undefined' && LaRuche.Memory && typeof LaRuche.Memory.current === 'function') { var cnode = LaRuche.Memory.current(); if (cnode) { LaRuche.Memory.loadNode(cnode); } } break; } case 'AgentStarted': {
        // payload: {session_id, prompt} (chat) or {task_id, prompt} (cron)
        var label = pl.prompt || pl.preview || LaRuche.i18n.t('core.agentStartedLabel');
        LaRuche.Toast.show(LaRuche.i18n.t('core.agentStartedToast',{label:String(label).substring(0,60)}), 'info', 6000,
          sid ? function(){ openSession(sid); } : null);
        break;
      }
      case 'WatcherFired': {
        // payload: {watcher_id, prompt, context}
        var wname = pl.prompt || pl.watcher_id || LaRuche.i18n.t('core.watcherLabel');
        LaRuche.Toast.show(LaRuche.i18n.t('core.watcherFiredToast',{name:String(wname).substring(0,50)}), 'info', 7000,
          sid ? function(){ openSession(sid); } : null);
        break;
      }
      case 'MemorySaved': { if (typeof LaRuche !== 'undefined' && LaRuche.Memory && typeof LaRuche.Memory.current === 'function') { var cnode = LaRuche.Memory.current(); if (cnode && pl.node_id && String(cnode) === String(pl.node_id)) { LaRuche.Memory.loadNode(cnode); } } break; } case 'KanbanTask': {
        // payload: {task_id, title}
        var ktitle = pl.title || LaRuche.i18n.t('core.kanbanTaskTitle',{id:tid||''});
        LaRuche.Toast.show(LaRuche.i18n.t('core.kanbanToast',{title:String(ktitle).substring(0,55)}), 'info', 7000,
          sid ? function(){ openSession(sid); } : null);
        break;
      }
    }
  }

  function pollEvents() {
    fetch(LaRuche.API.base+'/api/events?since='+lastEventId).then(function(r){return r.json();}).then(function(evts){
      if(!evts || !evts.length) return;
      evts.forEach(function(ev){
        if(ev.id > lastEventId) lastEventId = ev.id;
        var p = typeof ev.payload === 'object' ? JSON.stringify(ev.payload) : ev.payload;
        log('event', ev.actor || 'system', '['+ev.kind+'] ' + p);
        notifyEvent(ev);
      });
    }).catch(function(){});
  }

  function log(level, source, msg) {
    var entry = { time: new Date(), level: level, source: source, msg: String(msg) };
    entries.push(entry);
    if(entries.length > 500) entries.shift();
    renderIfActive();
  }

  function renderIfActive() {
    var el = document.getElementById('consoleEntries');
    if(!el || !document.getElementById('page-dashboard').classList.contains('active')) return;
    render();
  }

  function render() {
    var el = document.getElementById('consoleEntries');
    if(!el) return;
    var filtered = activeFilter === 'all' ? entries : entries.filter(function(e){ return e.level === activeFilter; });
    el.innerHTML = '';
    filtered.forEach(function(e){
      var row = document.createElement('div');
      row.className = 'console-entry';
      row.innerHTML =
        '<span class="console-entry-time">'+LaRuche.Utils.fmtTime(e.time)+'</span>'+
        '<span class="console-entry-level '+e.level+'">'+e.level+'</span>'+
        '<span class="console-entry-source">'+LaRuche.Utils.esc(e.source)+'</span>'+
        '<span class="console-entry-msg">'+LaRuche.Utils.esc(e.msg)+'</span>';
      el.appendChild(row);
    });
    el.scrollTop = el.scrollHeight;
    var countEl = document.getElementById('consoleCount');
    if(countEl) countEl.textContent = LaRuche.i18n.t('core.consoleEntries',{n:filtered.length});
  }

  function clear() { entries=[]; render(); }

  function setFilter(level) {
    activeFilter = level;
    document.querySelectorAll('.console-toolbar .filter-btn').forEach(function(b){
      b.classList.toggle('active', b.dataset.level === level);
    });
    render();
  }

  // Capture window errors
  window.onerror = function(msg, src, line, col, err) {
    log('error', 'window', msg + (src ? ' ('+src+':'+line+':'+col+')' : ''));
  };
  window.addEventListener('unhandledrejection', function(e) {
    log('error', 'promise', e.reason ? String(e.reason) : 'Unhandled rejection');
  });
  // Wrap console.error and console.warn
  console.error = function() {
    var args = Array.prototype.slice.call(arguments);
    log('error', 'console', args.join(' '));
    origError.apply(console, arguments);
  };
  console.warn = function() {
    var args = Array.prototype.slice.call(arguments);
    log('warn', 'console', args.join(' '));
    origWarn.apply(console, arguments);
  };

  return { log:log, render:render, clear:clear, setFilter:setFilter, init:function(){
    document.querySelector('.console-toolbar').addEventListener('click', function(e){
      var btn = e.target.closest('.filter-btn');
      if(btn) setFilter(btn.dataset.level);
    });
    pollTimer = setInterval(pollEvents, 3000);
    pollEvents();
  }, enter:render, leave:function(){} };
})();

/* ── Auth ──────────────────────────────────────────────────────── */
LaRuche.Auth = (function(){
  var currentUser = null;
  var pollTimer = null;
  var challengeTimer = null;

  function init(cb) {
    fetch('/api/auth/me',{credentials:'include'}).then(function(r){
      if(r.ok) return r.json();
      throw new Error('not auth');
    }).then(function(u){
      currentUser = u;
      showUserBadge();
      if(cb) cb(true);
    }).catch(function(){
      currentUser = null;
      if(cb) cb(false);
    });
  }

  function isAuthenticated(){ return !!currentUser; }
  function getUser(){ return currentUser; }

  function showUserBadge(){
    if(!currentUser) return;
    var badge=document.getElementById('userBadge');
    var avatar=document.getElementById('userAvatar');
    var name=document.getElementById('userName');
    if(badge){badge.style.display='flex';}
    var initial=(currentUser.display_name||'?').charAt(0).toUpperCase();
    if(avatar){
      if(currentUser.avatar){ avatar.innerHTML='<img src="'+currentUser.avatar+'" alt="" style="width:100%;height:100%;object-fit:cover;border-radius:50%">'; }
      else { avatar.innerHTML=''; avatar.textContent=initial; }
      if(currentUser.role==='admin') avatar.style.borderColor='var(--amber)';
    }
    if(name){name.textContent=currentUser.display_name+(currentUser.role==='admin'?' (admin)':'');}
  }
  function isAdmin(){ return currentUser && currentUser.role==='admin'; }

  function hideUserBadge(){
    var badge=document.getElementById('userBadge');
    if(badge) badge.style.display='none';
  }

  function enroll(){
    var nameEl=document.getElementById('enrollName');
    var displayName=(nameEl?nameEl.value:'').trim();
    if(!displayName){if(nameEl)nameEl.focus();return;}

    var pwEl=document.getElementById('enrollPassword');
    var password=(pwEl?pwEl.value:'').trim();
    if(password.length<6){ LaRuche.Toast.show(LaRuche.i18n.t('core.passwordMin'),'error'); if(pwEl)pwEl.focus(); return; }

    fetch('/api/auth/enroll',{
      method:'POST',credentials:'include',
      headers:{'Content-Type':'application/json'},
      body:JSON.stringify({display_name:displayName, password:password})
    }).then(function(r){
      if(r.status===409) throw new Error(LaRuche.i18n.t('core.nameTaken'));
      if(!r.ok) throw new Error(LaRuche.i18n.t('core.enrollFailed'));
      return r.json();
    }).then(function(data){
      currentUser={user_id:data.user_id,display_name:data.display_name,role:data.role};
      showUserBadge();

      // Show success with QR to save
      document.getElementById('login-enroll').style.display='none';
      document.getElementById('login-success').style.display='block';
      document.getElementById('login-welcome-name').textContent=LaRuche.i18n.t('core.welcomeUser',{name:data.display_name});
      document.getElementById('login-enroll-qr').innerHTML=data.qr_svg;

      LaRuche.Console.log('info','AUTH','User enrolled: '+data.display_name);
    }).catch(function(e){
      LaRuche.Toast.show(LaRuche.i18n.t('core.enrollError',{msg:e.message}),'error');
    });
  }

  function hideAllLoginSections(){
    ['login-enroll','login-scan','login-password','login-success','login-authenticated'].forEach(function(id){
      var el=document.getElementById(id); if(el) el.style.display='none';
    });
  }

  function showLoginMode(){
    stopPolling();
    hideAllLoginSections();
    document.getElementById('login-password').style.display='block';
  }

  function showQrMode(){
    hideAllLoginSections();
    document.getElementById('login-scan').style.display='block';
    startChallenge();
  }

  function showEnrollMode(){
    stopPolling();
    hideAllLoginSections();
    document.getElementById('login-enroll').style.display='block';
  }

  function loginPassword(){
    var name=(document.getElementById('loginName').value||'').trim();
    var pw=document.getElementById('loginPassword').value||'';
    var errEl=document.getElementById('loginError');
    var totpEl=document.getElementById('loginTotp');
    var totp=totpEl?(totpEl.value||'').trim():'';
    if(!name||!pw){if(errEl){errEl.textContent=LaRuche.i18n.t('core.namePasswordRequired');errEl.style.display='block';}return;}
    if(errEl) errEl.style.display='none';

    fetch('/api/auth/login',{
      method:'POST',credentials:'include',
      headers:{'Content-Type':'application/json'},
      body:JSON.stringify({display_name:name,password:pw,totp_code:totp||undefined})
    }).then(function(r){
      if(!r.ok) throw new Error(LaRuche.i18n.t('core.badCredentials'));
      return r.json();
    }).then(function(data){
      if(data&&data.totp_required){
        // Password ok, 2FA needed: reveal a code field and ask again.
        if(!totpEl){
          var pwEl=document.getElementById('loginPassword');
          totpEl=document.createElement('input');
          totpEl.id='loginTotp'; totpEl.type='text'; totpEl.inputMode='numeric'; totpEl.maxLength=6;
          totpEl.placeholder='000000'; totpEl.autocomplete='one-time-code';
          totpEl.className=(pwEl&&pwEl.className)||'';
          totpEl.style.cssText=(pwEl&&pwEl.style.cssText)||'';
          if(pwEl&&pwEl.parentNode) pwEl.parentNode.insertBefore(totpEl, pwEl.nextSibling);
        }
        totpEl.style.display='';
        if(errEl){errEl.textContent=LaRuche.i18n.t('core.totpRequired');errEl.style.display='block';}
        totpEl.focus();
        return;
      }
      currentUser={user_id:data.user_id,display_name:data.display_name,role:data.role};
      showUserBadge();
      hideAllLoginSections();
      document.getElementById('login-authenticated').style.display='block';
      document.getElementById('login-auth-name').textContent=LaRuche.i18n.t('core.helloUser',{name:data.display_name});
      setTimeout(function(){ LaRuche.Router.go('chat'); },1000);
    }).catch(function(e){
      if(errEl){errEl.textContent=e.message;errEl.style.display='block';}
    });
  }

  function startChallenge(){
    fetch('/api/auth/challenge',{credentials:'include'}).then(function(r){return r.json();}).then(function(data){
      document.getElementById('login-qr').innerHTML=data.qr_svg;
      var remaining=data.expires_in||60;
      var timerEl=document.getElementById('login-timer');

      // Countdown
      if(challengeTimer) clearInterval(challengeTimer);
      challengeTimer=setInterval(function(){
        remaining--;
        if(timerEl) timerEl.textContent=LaRuche.i18n.t('core.challengeExpires',{s:remaining});
        if(remaining<=0){
          clearInterval(challengeTimer);
          startChallenge(); // auto-refresh
        }
      },1000);

      // Poll status
      stopPolling();
      pollTimer=setInterval(function(){
        fetch('/api/auth/status/'+data.challenge_id,{credentials:'include'})
          .then(function(r){return r.json();})
          .then(function(s){
            if(s.status==='authenticated'){
              stopPolling();
              if(challengeTimer) clearInterval(challengeTimer);
              // Set cookie from token
              document.cookie='laruche_auth='+s.token+'; path=/; max-age=2592000; samesite=lax';
              currentUser={user_id:s.user_id,display_name:s.display_name};
              showUserBadge();
              // Show success animation
              document.getElementById('login-scan').style.display='none';
              document.getElementById('login-authenticated').style.display='block';
              document.getElementById('login-auth-name').textContent=LaRuche.i18n.t('core.helloUser',{name:s.display_name});
              setTimeout(function(){ LaRuche.Router.go('chat'); },1500);
              LaRuche.Console.log('info','AUTH','Login via QR: '+s.display_name);
            } else if(s.status==='expired'){
              startChallenge();
            }
          }).catch(function(){});
      },1500);
    }).catch(function(e){
      LaRuche.Toast.show(LaRuche.i18n.t('core.challengeError',{msg:e.message}),'error');
    });
  }

  function stopPolling(){
    if(pollTimer){clearInterval(pollTimer);pollTimer=null;}
  }

  function continueToChat(){
    LaRuche.Router.go('chat');
  }

  function logout(){
    LaRuche.WS.close(); // Close WebSocket before logout
    fetch('/api/auth/logout',{method:'POST',credentials:'include'}).then(function(){
      currentUser=null;
      hideUserBadge();
      document.cookie='laruche_auth=; path=/; max-age=0';
      // Reset login page
      document.getElementById('login-enroll').style.display='block';
      document.getElementById('login-scan').style.display='none';
      document.getElementById('login-success').style.display='none';
      document.getElementById('login-authenticated').style.display='none';
      LaRuche.Router.go('login');
    }).catch(function(){});
  }

  return {
    init:init, isAuthenticated:isAuthenticated, getUser:getUser, isAdmin:isAdmin, refreshBadge:showUserBadge,
    enroll:enroll, loginPassword:loginPassword,
    showLoginMode:showLoginMode, showEnrollMode:showEnrollMode, showQrMode:showQrMode,
    startChallenge:startChallenge, continueToChat:continueToChat, logout:logout
  };
})();

/* ── Router ────────────────────────────────────────────────────── */
LaRuche.Router = (function(){
  var currentPage = null;
  var pages = ['chat','dashboard','memory','missions','automations','capabilities','settings','console','login'];
  var modules = {};

  function go(page) {
    if(pages.indexOf(page) < 0) page = 'chat';
    // Auth guard: redirect to login if not authenticated (except login page itself)
    if(page !== 'login' && !LaRuche.Auth.isAuthenticated()) { page = 'login'; }
    if(currentPage === page) return;
    // leave old page
    if(currentPage && modules[currentPage] && modules[currentPage].leave) modules[currentPage].leave();
    // hide all
    pages.forEach(function(p) {
      var el = document.getElementById('page-'+p);
      if(el) el.classList.remove('active');
    });
    // show new
    var newEl = document.getElementById('page-'+page);
    if(newEl) newEl.classList.add('active');
    currentPage = page;
    // update nav
    document.querySelectorAll('.header-nav a').forEach(function(a){
      a.classList.toggle('active', a.dataset.page === page);
    });
    document.querySelectorAll('.mobile-tabs a').forEach(function(a){
      a.classList.toggle('active', a.dataset.page === page);
    });
    // Hide nav on login page
    var isLogin = (page === 'login');
    var headerNav = document.getElementById('headerNav');
    var mobileTabs = document.getElementById('mobileTabs');
    var headerRight = document.querySelector('.header-right');
    if(headerNav) headerNav.style.display = isLogin ? 'none' : '';
    if(mobileTabs) mobileTabs.style.display = isLogin ? 'none' : '';
    if(headerRight) headerRight.style.display = isLogin ? 'none' : '';
    // enter new page
    if(modules[page] && modules[page].enter) modules[page].enter();
    // Reposition mesh windows: the bottom bars change per tab (chat input absent
    // outside chat). go() uses replaceState (no hashchange), so we must call it explicitly.
    if(window.LaRuche && LaRuche.Mesh && LaRuche.Mesh.repositionWindows){
      LaRuche.Mesh.repositionWindows();
      requestAnimationFrame(function(){ LaRuche.Mesh.repositionWindows(); }); // after the new page reflows
    }
    // update hash without triggering hashchange
    if(location.hash !== '#'+page) {
      history.replaceState(null, '', '#'+page);
    }
  }

  function register(name, mod) { modules[name] = mod; }

  function init() {
    // Init all modules
    pages.forEach(function(p) { if(modules[p] && modules[p].init) modules[p].init(); });
    // Listen for hash changes
    window.addEventListener('hashchange', function(){ go(location.hash.replace('#','')||'chat'); });
    // Navigate to initial page
    go(location.hash.replace('#','')||'chat');
  }

  return { go:go, register:register, init:init, current:function(){return currentPage;} };
})();

/* ── Header ────────────────────────────────────────────────────── */
LaRuche.Header = (function(){
  var currentModelName = '';
  var currentProfileId = '';
  var lastModelChangeAt = 0;

  function init() {
    loadModels();
    loadPermissionMode();
    // Refresh the model list in the background (without F5): a closed local provider
    // (llama.cpp/ollama) disappears, and reappears as soon as it comes back. Avoid re-rendering
    // while the user has the menu open.
    setInterval(function(){
      var drop=document.getElementById('sbModelDrop');
      if(drop && drop.classList.contains('open')) return;
      if(Date.now()-lastModelChangeAt < 8000) return; // not right after a manual choice
      loadModels();
    }, 20000);
    setInterval(fetchContextStats, 1500);
    fetchContextStats();
  }

  async function fetchContextStats(){
    try{
      var chatPage = document.getElementById('page-chat');
      var gauge = document.getElementById('chatCtxGauge');
      if(!chatPage || !gauge) return;
      var isChat = chatPage.classList.contains('active');
      gauge.style.display = isChat ? 'flex' : 'none';
      if(!isChat) return;

      var sid = (window.LaRuche && LaRuche.Chat && typeof LaRuche.Chat.getSessionId === 'function') ? LaRuche.Chat.getSessionId() : null;
      var url = '/api/context/stats';
      if(sid) url += '?session_id=' + encodeURIComponent(sid);
      var r=await fetch(url);
      if(!r.ok)return;
      var d=await r.json();
      var pct = Math.round((d.ratio||0)*100);
      var fill = document.getElementById('chatCtxFill');
      if(fill) { fill.style.width=pct+'%'; fill.className='chat-ctx-fill'+(pct>66?' hot':pct>33?' warm':''); }
      var pctEl = document.getElementById('chatCtxPct');
      if(pctEl) pctEl.textContent = pct+'%';
      var detail = document.getElementById('chatCtxDetail');
      if(detail) {
        var used = d.used_tokens||0; var max = d.max_tokens||0;
        var usedStr = used >= 1000 ? (used/1000).toFixed(1)+'k' : used;
        var maxStr = max >= 1000 ? (max/1000).toFixed(0)+'k' : max;
        detail.textContent = usedStr+' / '+maxStr+' tokens · '+d.messages+' msg';
      }
    }catch(e){}
  }

  function loadPermissionMode() {
    fetch('/api/config/permission').then(function(r){return r.json();}).then(function(data){
      var sel = document.getElementById('permModeSelect');
      if(!sel) return;
      sel.innerHTML = '';
      (data.modes||[]).forEach(function(m){
        var opt = document.createElement('option');
        opt.value = m.id;
        opt.textContent = m.label;
        if(m.id === data.mode) opt.selected = true;
        sel.appendChild(opt);
      });
    }).catch(function(){});
  }

  function changePermissionMode(mode) {
    if(!mode) return;
    fetch('/api/config/permission',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},
      body:JSON.stringify({mode:mode})
    }).then(function(r){return r.json();}).then(function(d){
      if(d && d.status==='ok') LaRuche.Toast.show(LaRuche.i18n.t('core.permissionsToast',{mode:mode}),'ok');
      else LaRuche.Toast.show(LaRuche.i18n.t('core.permChangeFailed'),'err');
    }).catch(function(){ LaRuche.Toast.show(LaRuche.i18n.t('core.permChangeFailed'),'err'); });
  }

  function loadModels() {
    fetch('/api/profiles/models').then(function(r){return r.json();}).then(function(data){
      var models = data.models || [];
      var active = data.active || {};
      var select = document.getElementById('modelSelect');
      select.innerHTML = '';

      // Group models by profile_name
      var groups = {};
      var groupOrder = [];
      models.forEach(function(m){
        if(!groups[m.profile_name]) {
          groups[m.profile_name] = [];
          groupOrder.push(m.profile_name);
        }
        groups[m.profile_name].push(m);
      });

      // SOURCE icon per group: 🐝 mesh/shared · 🖥️ local · ☁️ cloud.
      function srcIcon(name){
        var n=(name||'').toLowerCase();
        if(/mesh|partag|\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b/.test(n)) return '🐝 ';
        if(/llama|local|127\.0\.0\.1|ollama|vllm|lm studio/.test(n)) return '🖥️ ';
        return '☁️ ';
      }
      groupOrder.forEach(function(gname){
        var optGroup = document.createElement('optgroup');
        optGroup.label = srcIcon(gname)+gname;
        groups[gname].forEach(function(m){
          var opt = document.createElement('option');
          opt.value = m.id; // "profile_id/model_name"
          opt.textContent = m.name;
          if(m.profile === active.profile_id && m.name === active.model) opt.selected = true;
          optGroup.appendChild(opt);
        });
        select.appendChild(optGroup);
      });

      // If active model not in list, add it
      if(active.profile_id && active.model && !select.querySelector('option[selected]')) {
        var opt = document.createElement('option');
        opt.value = active.profile_id + '/' + active.model;
        opt.textContent = active.model + ' (active)';
        opt.selected = true;
        select.appendChild(opt);
      }

      if (Date.now() - lastModelChangeAt > 2000) {
        currentModelName = active.model || '';
        currentProfileId = active.profile_id || '';
        var recapEl = document.getElementById('sessionCapsRecap');
        if (recapEl) recapEl.textContent = currentModelName || LaRuche.i18n.t('core.modelAuto');
      }
    }).catch(function(){});
  }

  function changeModel(value) {
    // value format: "profile_id/model_name"
    var slashIdx = value.indexOf('/');
    if(slashIdx === -1) return;
    var profileId = value.substring(0, slashIdx);
    var modelName = value.substring(slashIdx + 1);
    currentModelName = modelName;
    currentProfileId = profileId;
    lastModelChangeAt = Date.now();

    var recapEl = document.getElementById('sessionCapsRecap');
    if (recapEl) recapEl.textContent = modelName;

    var select = document.getElementById('modelSelect');
    select.disabled = true;
    LaRuche.Console.log('info','Header','Switching to '+profileId+'/'+modelName);

    // Update the global active model (persisted + engine sync) so the
    // General tab and state reflect the real model, and also keep the
    // per-user preference.
    fetch('/api/profiles/active',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},
      body:JSON.stringify({profile_id:profileId, model:modelName})
    }).catch(function(){});
    fetch('/api/auth/model',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},
      body:JSON.stringify({model:modelName, provider:profileId})
    }).then(function(r){return r.json();}).then(function(d){
      select.disabled = false;
      // Preload for Ollama profiles
      if(profileId.indexOf('ollama') !== -1) {
        fetch('/api/preload',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({model:modelName})}).then(function(r){return r.json();}).then(function(data){
          if(data.status==='loaded') LaRuche.Toast.show(LaRuche.i18n.t('core.modelReady',{model:modelName,ms:data.elapsed_ms}),'ok');
        }).catch(function(){});
      }
      LaRuche.Toast.show(LaRuche.i18n.t('core.modelSelected',{model:modelName}),'ok');
      // P7: reflect the new active model in the mesh/capabilities recap without F5
      LaRuche.refreshAll(['mesh','capabilities']);
    }).catch(function(){ select.disabled = false; });
  }

  function getModel() { return currentModelName; }
  function getProfileId() { return currentProfileId; }
  function setModel(name) { currentModelName = name; var s = document.getElementById('modelSelect'); if(s) s.value = name; }

    async function loadBlueprints(el) {
    var bps=[];try{bps=await fetch('/api/blueprints').then(function(r){return r.json();});}catch(e){}
    if(!bps.length){el.innerHTML='<div style="text-align:center;color:var(--text-muted);padding:20px">'+LaRuche.i18n.t('core.noBlueprintsAvailable')+'</div>';return;}

    window._blueprints = bps;
    el.innerHTML = '<div style="margin-bottom:12px;color:var(--amber);font-size:12px;">'+LaRuche.i18n.t('core.blueprintHint')+'</div>' +
      bps.map(function(b, idx) {
        return '<div class="settings-card" style="margin-bottom:12px;cursor:pointer;" onclick="LaRuche.Settings.openBlueprintForm('+idx+')">' +
          '<div class="settings-card-title">'+LaRuche.Utils.esc(b.title||b.id)+'</div>' +
          '<div style="font-size:12px;color:var(--text-dim);margin-top:4px;">'+LaRuche.Utils.esc(b.description||'')+'</div>' +
          '<div id="bpForm_'+idx+'" style="display:none;margin-top:12px;padding-top:12px;border-top:1px solid var(--border);" onclick="event.stopPropagation()">' +
            (b.slots||[]).map(function(slot){
              return '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.Utils.esc(slot.label||slot.name)+'</label><input id="bpInput_'+idx+'_'+slot.name+'" class="form-input" placeholder="'+LaRuche.Utils.esc(slot.placeholder||'')+'"></div>';
            }).join('') +
            '<button class="settings-save-btn" style="margin-top:8px" onclick="LaRuche.Settings.instanciateBlueprint('+idx+')">'+LaRuche.i18n.t('core.instantiate')+'</button>' +
          '</div>' +
        '</div>';
      }).join('');
  }

  function openBlueprintForm(idx) {
    var form = document.getElementById('bpForm_'+idx);
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
      slotsData[slot.name] = document.getElementById('bpInput_'+idx+'_'+slot.name).value;
    });
    fetch('/api/blueprints/'+b.id+'/instancier', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(slotsData)
    }).then(function(res) {
      if(res.ok) {
        LaRuche.Toast.show(LaRuche.i18n.t('core.blueprintOk'), 'ok');
        document.getElementById('bpForm_'+idx).style.display = 'none';
      } else {
        LaRuche.Toast.show(LaRuche.i18n.t('core.blueprintErr'), 'err');
      }
    }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('core.errorPrefix',{msg:e}), 'err'); });
  }

  return { init:init, openBlueprintForm:openBlueprintForm, instanciateBlueprint:instanciateBlueprint, changeModel:changeModel, getModel:getModel, getProfileId:getProfileId, setModel:setModel, loadModels:loadModels, changePermissionMode:changePermissionMode, loadPermissionMode:loadPermissionMode };
})();

/* ── WS (Chat WebSocket) ──────────────────────────────────────── */
LaRuche.WS = (function(){
  var ws = null;
  var reconnectAttempts = 0;

  function connect() {
    var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(protocol+'//'+location.host+'/ws/chat');
    ws.onopen = function(){
      document.getElementById('statusHoneycomb').className = 'honeycomb-loader hc-sm';
      
      document.getElementById('sendBtn').disabled = false;
      document.getElementById('reconnectBanner').classList.remove('visible');
      reconnectAttempts = 0;
      LaRuche.Console.log('info','WS','Chat WebSocket connected');
    };
    ws.onclose = function(){
      document.getElementById('statusHoneycomb').className = 'honeycomb-loader hc-sm offline';
      
      document.getElementById('sendBtn').disabled = true;
      scheduleReconnect();
    };
    ws.onerror = function(){
      document.getElementById('statusHoneycomb').className = 'honeycomb-loader hc-sm offline';
      
    };
    ws.onmessage = function(e){ LaRuche.Chat.handleEvent(JSON.parse(e.data)); };
  }

  function scheduleReconnect() {
    reconnectAttempts++;
    if(reconnectAttempts > 20) {
      document.getElementById('statusHoneycomb').className = 'honeycomb-loader hc-sm offline';
      
      document.getElementById('reconnectBanner').classList.remove('visible');
      return;
    }
    var delay = Math.min(2000*Math.pow(1.5, reconnectAttempts-1), 30000);
    document.getElementById('statusHoneycomb').className = 'honeycomb-loader hc-sm offline';
    
    document.getElementById('reconnectBanner').classList.add('visible');
    setTimeout(connect, delay);
  }

  function send(obj) { if(ws && ws.readyState===WebSocket.OPEN) ws.send(JSON.stringify(obj)); }
  function isOpen() { return ws && ws.readyState===WebSocket.OPEN; }
  function close() { if(ws){ ws.onclose=null; ws.close(); ws=null; } reconnectAttempts=0; }

  /* P8: Re-attach to the live stream of an ongoing session.
   * We open a dedicated SECONDARY WS connection (the server, on receiving
   * {type:"subscribe",session_id}, blocks the socket in a relay loop for that
   * session's ChatEvent events: so we CANNOT reuse the chat's main
   * socket). If the session is no longer ongoing, the server returns
   * nothing active and we close after a short delay. On 'done'/'error' we close. */
  var attachWs = null;
  var attachTimer = null;
  function reattach(sessionId) {
    if(!sessionId) return;
    // Close any previous re-attach
    detach();
    try {
      var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
      attachWs = new WebSocket(protocol+'//'+location.host+'/ws/chat');
    } catch(e){ return; }
    attachWs.onopen = function(){
      try { attachWs.send(JSON.stringify({type:'subscribe', session_id:sessionId})); } catch(e){}
      // Safety net: if no live event arrives (session already finished),
      // close the useless socket after 8 s. Reset on each event.
      armIdleClose();
    };
    attachWs.onmessage = function(e){
      armIdleClose();
      var data; try { data = JSON.parse(e.data); } catch(err){ return; }
      // Ignore the 'session' ack (already handled by switchSession); relay the rest
      // to the chat rendering pipeline (tokens, tool_call, done, ...).
      if(data.type === 'session') return;
      if(LaRuche.Chat && LaRuche.Chat.handleEvent) LaRuche.Chat.handleEvent(data);
      if(data.type === 'done' || data.type === 'error') detach();
    };
    attachWs.onclose = function(){ if(attachTimer){clearTimeout(attachTimer);attachTimer=null;} };
    attachWs.onerror = function(){ detach(); };
  }
  function armIdleClose(){
    if(attachTimer) clearTimeout(attachTimer);
    attachTimer = setTimeout(function(){ detach(); }, 8000);
  }
  function detach() {
    if(attachTimer){ clearTimeout(attachTimer); attachTimer=null; }
    if(attachWs){ try{ attachWs.onclose=null; attachWs.close(); }catch(e){} attachWs=null; }
  }

  return { connect:connect, send:send, isOpen:isOpen, close:close, reattach:reattach, detach:detach };
})();

/* ── Chat Module ──────────────────────────────────────────────── */
