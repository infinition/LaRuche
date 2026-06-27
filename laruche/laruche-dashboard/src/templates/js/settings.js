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
    // Anti-course : on donne à CHAQUE chargement un canvas neuf. Si un loader async lent se
    // termine APRÈS qu'on a changé d'onglet, il écrit dans SON ancien `el` (désormais détaché
    // du DOM) → invisible. Fini le « General s'affiche alors que j'ai cliqué Provider ».
    var el = document.createElement('div');
    el.className = 'settings-tab-canvas';
    host.innerHTML = '';
    host.appendChild(el);
    el.innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px">Chargement...</div>';
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
    // Les 6 appels sont INDÉPENDANTS → en PARALLÈLE (Promise.all) au lieu de 6 await en série
    // (c'était ça la lenteur : chaque fetch attendait le précédent). gj = fetch tolérant aux erreurs.
    function gj(u){ return fetch(u).then(function(r){return r.json();}).catch(function(){return {};}); }
    var _r = await Promise.all([
      gj('/api/doctor'), gj('/api/voice/status'), gj('/api/config/provider'),
      gj('/api/context/stats'), gj('/api/config/compaction'), gj('/api/config/curateur'),
      gj('/api/config/runtime')
    ]);
    var doc=_r[0], voice=_r[1], provCfg=_r[2], ctxStats=_r[3], ctxCfg=_r[4], curCfg=_r[5], rt=_r[6]||{};
    el.innerHTML = '<div class="settings-grid">'+
      '<div class="settings-card"><div class="settings-card-title">Génération (à chaud, sans redémarrage)</div>'+
      '<div class="settings-row" style="flex-direction:column;align-items:stretch;gap:4px;">'+
      '<div class="settings-row" style="padding:0;"><span class="settings-label" title="Passes ReAct max par tâche (anti-runaway)">Max passes</span><input type="number" id="cfgMaxIter" class="form-input" style="width:80px;padding:2px 6px;" value="'+(rt.max_iterations||40)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">Température</span><input type="number" id="cfgTemp" class="form-input" style="width:80px;padding:2px 6px;" step="0.05" min="0" max="2" value="'+(rt.temperature!=null?rt.temperature:0.7)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">Max tokens (sortie)</span><input type="number" id="cfgMaxTok" class="form-input" style="width:90px;padding:2px 6px;" value="'+(rt.max_tokens||4096)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label" title="Nb d\'outils injectés en sélection dynamique">Limite outils dyn.</span><input type="number" id="cfgToolLim" class="form-input" style="width:80px;padding:2px 6px;" value="'+(rt.tool_selection_limit||24)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label" title="Sous ce n_ctx, outils ET catalogue de skills passent en sélection dynamique (DB sémantique)">Seuil contexte étroit</span><input type="number" id="cfgCtxThreshold" class="form-input" style="width:90px;padding:2px 6px;" value="'+(rt.dynamic_context_threshold||40000)+'"></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveRuntimeCfg()" style="margin-top:8px;">Appliquer</button></div></div>'+
      '<div class="settings-card"><div class="settings-card-title">Contexte &amp; compaction</div>'+
      '<div class="settings-row" style="flex-direction:column;align-items:stretch;gap:4px;">'+
      '<div class="settings-row" style="padding:0;"><span class="settings-label">Max Messages</span><input type="number" id="cfgCtxMax" class="form-input" style="width:80px;padding:2px 6px;" value="'+(ctxCfg.context_max_messages||50)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">Seuil Compaction</span><input type="number" id="cfgCtxThresh" class="form-input" style="width:80px;padding:2px 6px;" step="0.05" value="'+(ctxCfg.compaction_threshold||0.75)+'"></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveContextCfg()" style="margin-top:8px;">Sauvegarder</button></div>'+
      '<div class="settings-card"><div class="settings-card-title">Inference Config</div>'+
      '<div class="settings-row" style="padding:0;"><span class="settings-label">Fallback Models</span><input type="text" id="cfgProvFallback" class="form-input" style="width:120px;padding:2px 6px;" value="'+(provCfg.fallback_models||'')+'" placeholder="claude-3-haiku, ..."></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">Max Tokens</span><input type="number" id="cfgProvMaxTokens" class="form-input" style="width:80px;padding:2px 6px;" value="'+(provCfg.max_tokens||4096)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">Temperature</span><input type="number" id="cfgProvTemp" class="form-input" style="width:80px;padding:2px 6px;" step="0.1" value="'+(provCfg.temperature||0.7)+'"></div>'+
      '<div class="settings-row" style="padding:0;margin-top:4px;"><span class="settings-label">Review Model</span><input type="text" id="cfgProvReview" class="form-input" style="width:120px;padding:2px 6px;" value="'+(provCfg.review_model||'')+'" placeholder="ex: gpt-4o"></div>'+
      '<button class="form-btn" onclick="LaRuche.Settings.saveProviderCfg()" style="margin-top:8px;">Sauvegarder</button>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:8px">Active: '+(provCfg.provider||'ollama')+' / '+(provCfg.model||'-')+'</div></div>'+
      '<div class="settings-card"><div class="settings-card-title">Voice</div>'+
      '<div class="settings-row"><span class="settings-label">STT</span><span style="color:'+(voice.stt&&voice.stt.available?'var(--green)':'var(--red)')+'">'+(voice.stt&&voice.stt.available?'OK':'Off')+'</span></div>'+
      '<div class="settings-row"><span class="settings-label">TTS</span><span style="color:'+(voice.tts&&voice.tts.available?'var(--green)':'var(--red)')+'">'+(voice.tts&&voice.tts.available?'OK':'Off')+'</span></div></div>'+
      '<div class="settings-card"><div class="settings-card-title">Security</div>'+
      '<div class="settings-row"><span class="settings-label">Secrets</span><span class="settings-value">17 patterns</span></div>'+
      '<div class="settings-row"><span class="settings-label">Protocol</span><span class="settings-value">Miel v'+(doc.version||'0.2.0')+'</span></div></div>'+
      '<div class="settings-card"><div class="settings-card-title">Curateur · Butinage</div>'+
      '<div class="settings-row"><span class="settings-label">Auto-création de skills/outils vérifiés</span><label class="lr-switch"><input type="checkbox" id="cfgCurateur" '+(curCfg.enabled?'checked':'')+' '+(curCfg.env_forced?'disabled':'')+' onchange="LaRuche.Settings.toggleCurateur(this.checked)"><span class="lr-slider"></span></label></div>'+
      '<div class="settings-row"><span class="settings-label">Sélection dynamique des outils <span style="color:var(--text-dim);font-size:10px">(prompt léger — recommandé pour petits modèles / llama.cpp)</span></span><label class="lr-switch"><input type="checkbox" id="cfgDynTools" '+(curCfg.dynamic_tools?'checked':'')+' onchange="LaRuche.Settings.toggleDynamicTools(this.checked)"><span class="lr-slider"></span></label></div>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-top:6px">'+(curCfg.env_forced?'Forcé par RUCHE_CURATEUR=1 (variable d\'env).':'En arrière-plan, conservateur (dédup auto). Off = ne crée rien.')+'</div></div>'+
      '<div class="settings-card"><div class="settings-card-title">System</div>'+
      '<div class="settings-row"><span class="settings-label">Afficher la transparence (outils/mémoire)</span><label class="lr-switch"><input type="checkbox" id="cfgTransparence" onchange="window.localStorage.setItem(\'laruche_hide_transparency\', this.checked ? \'false\' : \'true\')" \'+(window.localStorage.getItem(\'laruche_hide_transparency\') !== \'true\' ? \'checked\' : \'\')+\'><span class="lr-slider"></span></label></div>'+
      ((doc.checks||[]).map(function(c){return '<div class="settings-row"><span class="settings-label">'+c.name+'</span><span style="color:'+(c.status==='ok'?'var(--green)':'var(--red)')+'">'+c.status+'</span></div>';}).join('')||'<div class="settings-row"><span class="settings-label">Status</span><span class="settings-value">OK</span></div>')+
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

    // Carte dǸdiǸe : connexion ChatGPT Codex via abonnement (OAuth).
    var html = '<div class="settings-card" id="codexAuthCard" style="margin-bottom:16px;border:1px solid var(--amber)">'+
      '<div class="settings-card-title">ChatGPT Codex <span style="color:var(--text-dim);font-size:10px;font-weight:normal">— abonnement (OAuth, sans clé API)</span></div>'+
      '<div id="codexAuthBox" style="font-size:12px;color:var(--text-dim)">Chargement…</div>'+
      '</div>';

    html += '<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="LaRuche.Settings.showProfileForm()">+ Add Provider</button></div>';
    html += '<div id="profileFormContainer" style="display:none"></div>';

    // (Les serveurs MCP ont désormais leur propre onglet « MCP » — voir loadMcp.)

    html += '<div class="settings-grid">';
    var sharedHtml = '';

    ids.forEach(function(id) {
      var p = profiles[id];
      var isActive = (id === active.profile_id);
      var modelCount = (p.models || []).length;
      var provLabel = p.provider === 'ollama' ? 'Ollama' : p.provider === 'anthropic' ? 'Anthropic' : p.provider === 'codex' ? 'ChatGPT Codex' : 'OpenAI-compat';
      // PARTAGÉ PAR UN PAIR : base_url = IP LAN privée (≠ loopback) → carte séparée, lecture seule
      // (on ne re-partage pas / n'édite pas le provider d'un autre).
      var _bu = (p.base_url||'').toLowerCase();
      var _shared = /(^|\/\/)(10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.)/.test(_bu) && !/127\.0\.0\.1|localhost/.test(_bu);
      if(_shared){
        sharedHtml += '<div class="settings-card">'+
          '<div class="settings-card-title" style="display:flex;align-items:center;gap:6px;flex-wrap:wrap"><span>'+LaRuche.Utils.esc(p.name)+'</span>'+
          '<span style="color:var(--cyan);font-size:10px;font-weight:normal">🐝 partagé par un pair · lecture seule</span></div>'+
          '<div class="settings-row"><span class="settings-label">URL</span><span class="settings-value" style="font-size:10px;word-break:break-all">'+LaRuche.Utils.esc(p.base_url)+'</span></div>'+
          '<div class="settings-row"><span class="settings-label">Modèles</span><span class="settings-value">'+modelCount+'</span></div>'+
          '<div style="margin-top:10px"><button onclick="LaRuche.Settings.deleteProfile(\''+id+'\')" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">Retirer de ma liste</button></div>'+
          '</div>';
        return; // pas de carte normale : ni « Rendre Public », ni « Edit »
      }

      var pCreds = allCreds.filter(function(c){ return c.provider.toLowerCase() === p.provider.toLowerCase(); });
      var credsHtml = '';
      if(pCreds.length > 0) {
        credsHtml += '<div style="margin-top:10px;padding-top:10px;border-top:1px dashed var(--border)">';
        credsHtml += '<div style="font-size:10px;color:var(--text-dim);margin-bottom:6px;font-weight:bold">Pool de Credentials</div>';
        pCreds.forEach(function(c){
           var maskedKey = c.api_key ? (c.api_key.substring(0,6) + '...' + c.api_key.substring(c.api_key.length-4)) : '';
           var cdText = c.cooldown_until ? (' <span style="color:var(--red)">(cooldown)</span>') : '';
           var lbl = c.label ? ('<span style="color:var(--amber);margin-right:6px">['+LaRuche.Utils.esc(c.label)+']</span>') : '';
           credsHtml += '<div style="font-size:10px;display:flex;justify-content:space-between;align-items:center;margin-bottom:4px;background:var(--bg-lighter);padding:4px;border-radius:4px">'+
             '<div>'+lbl+'<span style="font-family:monospace">'+LaRuche.Utils.esc(maskedKey)+'</span> '+cdText+' <span style="color:var(--text-dim)">['+c.request_count+' reqs]</span></div>'+
             '<button onclick="LaRuche.Settings.deleteCredential(\''+p.provider+'\', \''+c.api_key+'\')" style="background:none;border:none;color:var(--red);cursor:pointer;font-size:12px;padding:0 4px" title="Supprimer">&times;</button>'+
             '</div>';
        });
        credsHtml += '</div>';
      }
      var addCredBtn = '<button onclick="LaRuche.Settings.addCredential(\''+p.provider+'\')" style="margin-top:8px;background:none;border:1px dashed var(--border);color:var(--text-dim);border-radius:4px;padding:4px 10px;cursor:pointer;font-size:10px;width:100%">+ Add Credential Key</button>';

      var _vis = p.visibility || 'prive';
      var _nAllowed = (p.allowed_peers||[]).length;
      var visBadge = _vis==='public_proxy'
        ? '<span style="color:var(--blue);font-size:10px;font-weight:bold;margin-left:8px;">🌐 Public 📡</span>'
        : _vis==='restricted'
        ? '<span style="color:var(--cyan);font-size:10px;font-weight:bold;margin-left:8px;">🐝 Restreint ('+_nAllowed+')</span>'
        : '<span style="color:var(--text-dim);font-size:10px;font-weight:bold;margin-left:8px;">🔒 Privé</span>';
      var visToggleBtn = '<button onclick="LaRuche.Settings.openAccess(\''+id+'\', \''+_vis+'\', \''+encodeURIComponent(JSON.stringify(p.allowed_peers||[]))+'\')" style="margin-left:auto;background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;">🔐 Accès</button>';
      html += '<div class="settings-card" style="'+(isActive?'border:1px solid var(--amber);':'')+'">'+
        '<div class="settings-card-title" style="display:flex;align-items:center;"><span>'+LaRuche.Utils.esc(p.name)+'</span>'+
        (isActive?' <span style="color:var(--amber);font-size:10px;font-weight:normal;margin-left:4px;">(active)</span>':'')+
        visBadge+visToggleBtn+
        '</div>'+
        '<div class="settings-row"><span class="settings-label">Type</span><span class="settings-value">'+provLabel+'</span></div>'+
        '<div class="settings-row"><span class="settings-label">URL</span><span class="settings-value" style="font-size:10px;word-break:break-all">'+LaRuche.Utils.esc(p.base_url)+'</span></div>'+
        '<div class="settings-row"><span class="settings-label">API Key</span><span class="settings-value">'+(p.api_key?'***set***':'(none)')+'</span></div>'+
        '<div class="settings-row"><span class="settings-label">Models</span><span class="settings-value">'+modelCount+'</span></div>'+
        credsHtml + addCredBtn +
        '<div style="margin-top:12px;display:flex;gap:6px">'+
        '<button onclick="LaRuche.Settings.editProfile(\''+id+'\')" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">Edit</button>'+
        '<button onclick="LaRuche.Settings.deleteProfile(\''+id+'\')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">Delete</button>'+
        '</div></div>';
    });

    html += '</div>';
    if(sharedHtml){
      html += '<div class="settings-card-title" style="margin:18px 0 8px;color:var(--cyan)">🐝 Partagés avec moi (mesh)</div>'+
        '<div style="color:var(--text-dim);font-size:11px;margin-bottom:10px">Modèles exposés par d\'autres ruches. Tu peux les utiliser, mais pas les éditer ni les re-partager.</div>'+
        '<div class="settings-grid">'+sharedHtml+'</div>';
    }
    el.innerHTML = html;
    refreshCodexStatus();
  }

  // ── ChatGPT Codex (OAuth abonnement) ──────────────────────────
  var _codexPoll = null;

  function renderCodexBox(s) {
    var box = document.getElementById('codexAuthBox');
    if(!box) return;
    s = s || {};
    if(s.phase === 'connected') {
      box.innerHTML = '<div style="color:var(--green)">✓ Connecté'+
        (s.account_id?(' <span style="color:var(--text-dim)">('+LaRuche.Utils.esc(s.account_id)+')</span>'):'')+'</div>'+
        (s.expiring?'<div style="color:var(--amber);font-size:10px;margin-top:4px">Token expiré — refresh auto au prochain appel.</div>':'')+
        '<div style="margin-top:8px"><button onclick="LaRuche.Settings.logoutCodex()" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 10px;cursor:pointer;font-size:10px">Déconnecter</button></div>';
    } else if(s.phase === 'pending' && s.user_code) {
      box.innerHTML = '<div>Pour vous connecter :</div>'+
        '<ol style="margin:6px 0 6px 16px;padding:0;line-height:1.7">'+
        '<li>Ouvrez <a href="'+LaRuche.Utils.esc(s.verification_url)+'" target="_blank" rel="noopener" style="color:var(--amber)">'+LaRuche.Utils.esc(s.verification_url)+'</a></li>'+
        '<li>Entrez ce code : <span style="font-size:16px;font-weight:bold;color:var(--amber);letter-spacing:2px">'+LaRuche.Utils.esc(s.user_code)+'</span></li>'+
        '</ol>'+
        '<div style="color:var(--text-dim);font-size:11px">⏳ En attente de validation…</div>';
    } else if(s.phase === 'error') {
      box.innerHTML = '<div style="color:var(--red)">Échec : '+LaRuche.Utils.esc(s.message||'erreur')+'</div>'+
        '<div style="margin-top:8px"><button onclick="LaRuche.Settings.startCodexLogin()" style="background:var(--amber);border:none;color:#000;border-radius:4px;padding:4px 12px;cursor:pointer;font-size:11px">Réessayer</button></div>';
    } else {
      box.innerHTML = '<div>Utilisez votre abonnement ChatGPT (Plus/Pro) au lieu d\'une clé API.</div>'+
        '<div style="margin-top:8px"><button onclick="LaRuche.Settings.startCodexLogin()" style="background:var(--amber);border:none;color:#000;border-radius:4px;padding:4px 12px;cursor:pointer;font-size:11px;font-weight:bold">Se connecter avec ChatGPT</button></div>';
    }
  }

  function refreshCodexStatus() {
    fetch('/api/auth/codex/status').then(function(r){return r.json();})
      .then(renderCodexBox).catch(function(){});
  }

  function startCodexLogin() {
    var box = document.getElementById('codexAuthBox');
    if(box) box.innerHTML = '<div style="color:var(--text-dim)">Initialisation…</div>';
    fetch('/api/auth/codex/start',{method:'POST'}).then(function(r){return r.json();})
      .then(function(s){
        renderCodexBox(s);
        if(s.phase === 'pending' && s.user_code) startCodexPoll();
      }).catch(function(){
        renderCodexBox({phase:'error',message:'réseau'});
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
    if(!confirm('Déconnecter ChatGPT Codex ?')) return;
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
      '<div class="settings-card-title">'+(editId?'Edit':'Add')+' Provider</div>'+
      '<div class="form-group"><label class="form-label">Profile ID'+(editId?' (read-only)':'')+'</label>'+
      '<input class="form-input" id="pfId" value="'+LaRuche.Utils.esc(editId)+'" '+(editId?'readonly':'')+' placeholder="e.g. groq-free"></div>'+
      '<div class="form-group"><label class="form-label">Display Name</label>'+
      '<input class="form-input" id="pfName" value="'+LaRuche.Utils.esc(p.name||'')+'" placeholder="e.g. Groq Free Tier"></div>'+
      '<div class="form-group"><label class="form-label">Provider Type</label>'+
      '<select class="form-select" id="pfProvider" onchange="LaRuche.Settings.onProfileProviderChange()">'+
      '<option value="ollama"'+(provType==='ollama'?' selected':'')+'>Ollama</option>'+
      '<option value="openai"'+(provType==='openai'?' selected':'')+'>OpenAI-compatible</option>'+
      '<option value="anthropic"'+(provType==='anthropic'?' selected':'')+'>Anthropic</option>'+
      '</select></div>'+
      '<div class="form-group"><label class="form-label">Base URL</label>'+
      '<input class="form-input" id="pfBaseUrl" value="'+LaRuche.Utils.esc(p.base_url||defaultUrls[provType]||'')+'" placeholder="'+defaultUrls[provType]+'"></div>'+
      '<div class="form-group"><label class="form-label">API Key</label>'+
      '<input class="form-input" id="pfApiKey" type="password" value="'+LaRuche.Utils.esc(p.api_key||'')+'" placeholder="sk-... (leave empty for Ollama)" autocomplete="off"></div>'+
      '<div class="form-group"><label class="form-label">Models (comma-separated, auto-detected for Ollama)</label>'+
      '<input class="form-input" id="pfModels" value="'+LaRuche.Utils.esc((p.models||[]).join(', '))+'" placeholder="gpt-4o, gpt-4o-mini"></div>'+
      '<div style="display:flex;gap:8px;margin-top:8px">'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.saveProfile()">Save</button>'+
      '<button style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:6px;padding:6px 16px;cursor:pointer" onclick="document.getElementById(\'profileFormContainer\').style.display=\'none\'">Cancel</button>'+
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
    if(!id) { LaRuche.Toast.show('Profile ID is required','err'); return; }
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
        LaRuche.Toast.show('Profile "'+d.name+'" saved','ok');
        document.getElementById('profileFormContainer').style.display = 'none';
        loadTab('providers');
        // Refresh dropdown after a short delay to ensure server has processed
        setTimeout(function(){ LaRuche.Header.loadModels(); }, 300);
      } else {
        LaRuche.Toast.show('Error: '+(d.error||'?'),'err');
      }
    }).catch(function(e){LaRuche.Toast.show('Error: '+e,'err');});
  }

  function editProfile(id) {
    showProfileForm(id);
  }

  function deleteProfile(id) {
    if(!confirm('Delete provider profile "'+id+'"?')) return;
    fetch('/api/profiles/'+id,{method:'DELETE'}).then(function(r){return r.json();}).then(function(d){
      if(d.status==='ok') {
        LaRuche.Toast.show('Profile deleted','ok');
        loadTab('providers');
        LaRuche.Header.loadModels();
      } else {
        LaRuche.Toast.show('Error: '+(d.error||'?'),'err');
      }
    });
  }

  async function loadTools(el) {
    var tools=[];try{tools=await fetch('/api/tools').then(function(r){return r.json();});}catch(e){}
    tools.sort(function(a,b){return String(a.name).localeCompare(String(b.name));});
    window._allTools = tools;
    
    var html = '<div style="display:flex;justify-content:flex-end;gap:10px;margin-bottom:20px;">';
    html += '<button onclick="LaRuche.Settings.toggleAllTools(true)" style="background:rgba(16,185,129,0.15);color:var(--green);border:1px solid var(--green);padding:6px 14px;border-radius:6px;cursor:pointer;font-size:12px;font-weight:600;transition:all 0.2s;" onmouseover="this.style.background=\'var(--green)\';this.style.color=\'#000\'" onmouseout="this.style.background=\'rgba(16,185,129,0.15)\';this.style.color=\'var(--green)\'">Tout Activer</button>';
    html += '<button onclick="LaRuche.Settings.toggleAllTools(false)" style="background:rgba(239,68,68,0.15);color:var(--red);border:1px solid var(--red);padding:6px 14px;border-radius:6px;cursor:pointer;font-size:12px;font-weight:600;transition:all 0.2s;" onmouseover="this.style.background=\'var(--red)\';this.style.color=\'#000\'" onmouseout="this.style.background=\'rgba(239,68,68,0.15)\';this.style.color=\'var(--red)\'">Tout Désactiver</button>';
    html += '</div>';

    html += '<div class="settings-grid">'+tools.map(function(t, idx){
      var enabled = t.enabled !== false;
      var originBadge = (t.origin === 'Custom') ? '<span style="margin-left:8px;font-size:9px;color:var(--purple);border:1px solid var(--purple-dim);background:var(--purple-dim);padding:2px 4px;border-radius:4px;">Custom</span>' : '<span style="margin-left:8px;font-size:9px;color:var(--text-dim);border:1px solid var(--border);padding:2px 4px;border-radius:4px;">Rust natif</span>';
      var customActions = (t.origin === 'Custom') ? '<div style="margin-top:10px;display:flex;gap:8px;border-top:1px solid rgba(255,255,255,0.05);padding-top:8px;"><button style="background:none;border:1px solid var(--border);color:var(--text-muted);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();LaRuche.Toast.show(\'Source non disponible\',\'err\')">Voir source</button><button style="background:none;border:1px solid var(--border);color:var(--text-muted);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();LaRuche.Toast.show(\'JSON non modifiable ici\',\'err\')">Éditer JSON</button><button style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;font-size:10px;cursor:pointer;" onclick="event.stopPropagation();fetch(\'/api/tools/\'+LaRuche.Utils.esc(t.name),{method:\'DELETE\'}).then(function(){LaRuche.Settings.refreshTab()})">Supprimer</button></div>' : '';
      return '<div class="settings-card" style="cursor:pointer; transition:transform 0.2s, box-shadow 0.2s; position:relative;" onmouseover="this.style.transform=\'translateY(-2px)\';this.style.boxShadow=\'0 4px 12px rgba(0,0,0,0.3)\';" onmouseout="this.style.transform=\'\';this.style.boxShadow=\'\';" onclick="LaRuche.Utils.openMediaModal(\'text\', JSON.stringify(window._allTools['+idx+'], null, 2))">'+
        '<div class="settings-card-title" style="display:flex;justify-content:space-between;gap:8px;align-items:center">'+
          '<span style="color:var(--cyan);font-weight:600;">'+LaRuche.Utils.esc(t.name)+originBadge+'</span>'+
          '<label onclick="event.stopPropagation()" style="display:flex;align-items:center;gap:6px;color:'+(enabled?'var(--green)':'var(--red)')+';font-size:10px;text-transform:none;letter-spacing:0;background:'+(enabled?'rgba(16,185,129,0.1)':'rgba(239,68,68,0.1)')+';padding:3px 8px;border-radius:12px;font-weight:bold;">'+
            '<input type="checkbox" '+(enabled?'checked':'')+' onchange="LaRuche.Settings.toggleTool(\''+LaRuche.Utils.esc(t.name)+'\',this.checked)"> '+(enabled?'ON':'OFF')+
          '</label>'+
        '</div>'+
        '<div class="settings-row" style="margin-top:8px;"><span class="settings-label">Danger</span><span class="settings-value" style="color:'+(t.danger==='high'?'var(--red)':(t.danger==='medium'?'var(--orange)':'var(--text-dim)'))+';font-weight:bold;">'+LaRuche.Utils.esc(t.danger||'safe')+'</span></div>'+
        '<div style="font-size:12px;color:var(--text-dim);line-height:1.5;margin-top:10px;border-top:1px solid rgba(255,255,255,0.05);padding-top:10px;">'+LaRuche.Utils.esc((t.description||'').substring(0,180))+'</div>'+
        customActions+
      '</div>';
    }).join('')+'</div>';
    
    el.innerHTML = html;
    if(!tools.length) el.innerHTML='<div style="text-align:center;color:var(--text-muted);padding:20px">Aucune abeille configurée</div>';
  }

  async function toggleAllTools(enable) {
    var disabled = enable ? [] : (window._allTools || []).map(function(t){return t.name;});
    fetch('/api/tools/config',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({disabled_tools:disabled})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d.status !== 'ok') LaRuche.Toast.show('Erreur configuration Abeilles','err');
        else { LaRuche.Toast.show(enable ? 'Toutes les abeilles activées' : 'Toutes les abeilles désactivées','ok'); loadTab('tools'); }
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
        if(d.status !== 'ok') LaRuche.Toast.show('Erreur configuration Abeilles','err');
        else { LaRuche.Toast.show(name+(enabled?' activee':' desactivee'),'ok'); loadTab('tools'); }
      })
      .catch(function(e){LaRuche.Toast.show('Erreur Abeilles: '+e,'err');});
  }

  async function loadNetwork(el) {
    var codeSet=false; try{ codeSet=(await fetch('/api/mesh/code').then(function(r){return r.json();})).set; }catch(e){}
    var codeCard='<div class="settings-card"><div class="settings-card-title">Code de mesh '+
      (codeSet?'<span style="color:var(--green);font-size:11px">(configuré)</span>':'<span style="color:var(--text-muted);font-size:11px">(non configuré — auth par IP LAN)</span>')+'</div>'+
      '<p style="color:var(--text-dim);font-size:12px;margin:4px 0 8px">Secret partagé entre tes ruches (comme un mot de passe WiFi). Mets le <b>même</b> code sur toutes tes ruches : il authentifie les échanges du mesh (fin des « rejected » / flapping) et servira de base au chiffrement.</p>'+
      '<div style="display:flex;gap:8px"><input id="meshCodeInput" type="password" placeholder="'+(codeSet?'•••• (vide = inchangé)':'choisis un code')+'" style="flex:1;background:var(--bg-input);color:var(--text);border:1px solid var(--border);border-radius:8px;padding:8px 10px;font-size:14px"><button class="send-btn" id="meshCodeSave"><span>Enregistrer</span></button></div></div>';
    var d={nodes:[]};try{d=await fetch('/swarm').then(function(r){return r.json();});}catch(e){}
    var nodesHtml=(d.nodes||[]).map(function(n){
      var caps=(n.capabilities||[]).map(function(c){return '<span style="background:rgba(6,182,212,.15);color:var(--cyan);padding:1px 6px;border-radius:8px;font-size:10px">'+c+'</span>';}).join(' ');
      return '<div class="settings-card"><div class="settings-card-title">'+(n.name||'?')+'</div><div class="settings-row"><span class="settings-label">Host</span><span class="settings-value">'+n.host+':'+(n.port||'?')+'</span></div><div style="margin-top:4px">'+caps+'</div></div>';
    }).join('')||'<div style="text-align:center;color:var(--text-muted);padding:20px">Aucun nœud</div>';
    el.innerHTML=codeCard+nodesHtml;
    var btn=document.getElementById('meshCodeSave');
    if(btn) btn.onclick=async function(){
      var v=(document.getElementById('meshCodeInput').value||'');
      if(!v.trim()){ LaRuche.Toast.show('Code inchangé.','info'); return; }
      try{ await fetch('/api/mesh/code',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({code:v})});
        LaRuche.Toast.show('Code enregistré. Mets le MÊME sur tes autres ruches, puis relance-les.','ok'); loadNetwork(el);
      }catch(e){ LaRuche.Toast.show('Échec.','err'); }
    };
  }

  // ── Timeline des crons (porté de la PR third-party #47944, en vanilla JS) ──────
  var _tlSpanH = 24;            // fenêtre : 24 / 48 / 168 h
  var _tlFromMs = 0;           // bord gauche
  var _tlJobs = [];
  var _tlTimer = null;
  var _tlHost = null;          // élément conteneur du rendu
  var _tlPxPerH = 64;          // px par heure (selon le zoom)
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
    var html='<div class="tl-ctrls"><div class="tl-seg">'+seg(24,'24h')+seg(48,'48h')+seg(168,'7j')+'</div>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlRecenter()">Recentrer</button>'+
      '<span style="color:var(--text-dim);font-size:10px">'+_tlJobs.length+' cron(s)</span></div>';
    if(!_tlJobs.length){ el.innerHTML=html+'<div style="color:var(--text-dim);padding:20px">Aucun cron planifié.</div>'; return; }
    // axe (ticks)
    var ticks=''; var stepH=_tlSpanH<=24?2:(_tlSpanH<=48?4:24);
    for(var h=0;h<=_tlSpanH;h+=stepH){ var d=new Date(_tlFromMs+h*3600000);
      var lbl=_tlSpanH<=48?(('0'+d.getHours()).slice(-2)+'h'):((d.getDate())+'/'+(d.getMonth()+1));
      ticks+='<div class="tl-tick" style="left:'+(h*pxPerH)+'px">'+lbl+'</div>'; }
    var gutter='<div class="tl-head"></div>', lanes='';
    _tlJobs.forEach(function(job,i){
      var paused=job.enabled===false;
      gutter+='<div class="tl-name'+(paused?' tl-row paused':'')+'" onclick="LaRuche.Settings.tlDetail('+i+')"><span class="n">'+LaRuche.Utils.esc(job.name||'(sans nom)')+'</span><span class="s">'+LaRuche.Utils.esc(job.cron_expr||job.fire_at||'')+'</span></div>';
      var occ=tlOccurrences(job,_tlFromMs,toMs); var now=Date.now();
      var nextT=occ.find(function(t){return t>=now;});
      var mk='';
      occ.forEach(function(t){
        var cls=t<now?'past':(t===nextT?'next':'future');
        var err=(job.last_status==='error');
        mk+='<span class="tl-mk '+cls+(err&&cls==='next'?' err':'')+'" style="left:'+((t-_tlFromMs)/spanMs*width)+'px" title="'+new Date(t).toLocaleString('fr-FR')+'" onclick="LaRuche.Settings.tlDetail('+i+')"></span>';
      });
      lanes+='<div class="tl-row'+(paused?' paused':'')+'" data-i="'+i+'" title="Glisser horizontalement pour décaler l\'heure (crons à heure fixe)">'+mk+'</div>';
    });
    html+='<div class="tl-wrap"><div class="tl-gutter">'+gutter+'</div><div class="tl-scroll"><div class="tl-strip" id="tlStrip" style="width:'+width+'px">'+
      '<div class="tl-head" style="width:'+width+'px">'+ticks+'</div>'+lanes+
      '<div class="tl-now" id="tlNow"></div></div></div></div><div id="tlDetail"></div>';
    el.innerHTML=html; positionNow();
    // auto-scroll pour placer "now" ~28% du bord gauche
    var sc=el.querySelector('.tl-scroll'); if(sc){ var nowX=(Date.now()-_tlFromMs)/spanMs*width; sc.scrollLeft=Math.max(0,nowX-sc.offsetWidth*0.28); }
    wireTlDrag(el);
  }
  // Drag horizontal d'une lane → décale l'heure d'un cron à heure fixe ("m h * * ...").
  function wireTlDrag(el){
    el.querySelectorAll('.tl-row[data-i]').forEach(function(row){
      var startX=0, dragging=false, moved=0;
      row.style.cursor='grab';
      row.addEventListener('pointerdown',function(e){ startX=e.clientX; dragging=true; moved=0; row.setPointerCapture(e.pointerId); row.style.cursor='grabbing'; });
      row.addEventListener('pointermove',function(e){ if(!dragging)return; moved=e.clientX-startX; row.style.transform='translateX('+(moved*0.15)+'px)'; });
      row.addEventListener('pointerup',function(e){
        if(!dragging)return; dragging=false; row.style.cursor='grab'; row.style.transform='';
        if(Math.abs(moved)<8) return; // simple clic → géré par le marqueur
        var job=_tlJobs[parseInt(row.getAttribute('data-i'))]; if(!job||!job.cron_expr){ LaRuche.Toast.show('Décalage non supporté pour ce planning','warn'); return; }
        var p=job.cron_expr.trim().split(/\s+/); if(p.length<5||isNaN(parseInt(p[1]))){ LaRuche.Toast.show('Décalage : crons à heure fixe uniquement','warn'); return; }
        var dh=Math.round(moved/_tlPxPerH); if(dh===0)return;
        var nh=((parseInt(p[1])+dh)%24+24)%24; p[1]=String(nh);
        var expr=p.join(' ');
        fetch('/api/cron/'+job.id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({cron_expr:expr})})
          .then(function(){ LaRuche.Toast.show('Heure décalée → '+expr,'ok'); tlReload(); });
      });
    });
  }
  function tlZoom(h){ _tlSpanH=h; var spanMs=h*3600000; _tlFromMs=Date.now()-0.28*spanMs; if(_tlHost)renderTimeline(_tlHost); }
  function tlRecenter(){ tlZoom(_tlSpanH); }
  function tlReload(){ if(_tlHost) loadCronTimeline(_tlHost); }
  function tlDetail(i){
    var job=_tlJobs[i]; if(!job)return; var d=document.getElementById('tlDetail'); if(!d)return;
    d.innerHTML='<div style="font-weight:600;color:var(--amber);margin-bottom:6px">'+LaRuche.Utils.esc(job.name||'(sans nom)')+'</div>'+
      '<div>Planning : <code>'+LaRuche.Utils.esc(job.cron_expr||job.fire_at||'-')+'</code></div>'+
      '<div style="color:var(--text-dim)">Dernier : '+(job.last_run||'jamais')+' · Exécutions : '+(job.run_count||0)+(job.channel?(' · Canal : '+LaRuche.Utils.esc(job.channel)):'')+'</div>'+
      '<div style="margin-top:8px;display:flex;gap:6px;flex-wrap:wrap">'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlRun('+i+')">Lancer maintenant</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlEdit('+i+')">Éditer</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlToggle('+i+')">'+(job.enabled===false?'Réactiver':'Mettre en pause')+'</button>'+
      '<button class="tl-btn" onclick="if(confirm(\'Supprimer ce cron ?\'))fetch(\'/api/cron/'+job.id+'\',{method:\'DELETE\'}).then(function(){LaRuche.Settings.tlReload&&LaRuche.Settings.tlReload();})">Supprimer</button>'+
      '</div>';
  }
  function tlRun(i){ var job=_tlJobs[i]; if(!job)return; fetch('/api/cron/'+job.id+'/run',{method:'POST'}).then(function(r){return r.json();}).then(function(d){ LaRuche.Toast.show(d.status==='started'?'Cron lancé':'Échec', d.status==='started'?'ok':'err'); }).catch(function(){LaRuche.Toast.show('Échec','err');}); }
  function tlToggle(i){ var job=_tlJobs[i]; if(!job)return; fetch('/api/cron/'+job.id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({enabled: job.enabled===false})}).then(function(){tlReload();}); }
  async function tlEdit(i){
    var job=_tlJobs[i]; if(!job)return; var d=document.getElementById('tlDetail'); if(!d)return;
    var skillsLoaded=true, skills=[];
    try{ skills=await fetch(LaRuche.API.base+'/api/skills').then(function(r){ if(!r.ok)throw new Error('skills'); return r.json(); }); }
    catch(e){ skillsLoaded=false; }
    var selected=Array.isArray(job.skills)?job.skills:[];
    var skillHtml;
    if(!skillsLoaded){
      skillHtml='<div data-skills-unavailable style="margin-top:10px;color:var(--red);font-size:11px">Skills indisponibles : les associations existantes seront conservées.</div>';
    }else if(!skills.length){
      skillHtml='<div style="margin-top:10px;color:var(--text-dim);font-size:11px">Aucun skill disponible. Créez-en dans Settings → Skills.</div>';
    }else{
      skillHtml='<fieldset style="margin:10px 0 0;padding:8px;border:1px solid var(--border);border-radius:6px"><legend style="padding:0 4px;color:var(--text-dim);font-size:11px">Skills injectés à ce cron</legend>'+
        skills.map(function(skill){
          var name=String(skill.name||''), enabled=skill.enabled!==false, checked=selected.indexOf(name)!==-1;
          return '<label style="display:flex;align-items:flex-start;gap:7px;margin:5px 0;cursor:'+(enabled?'pointer':'not-allowed')+';opacity:'+(enabled?'1':'0.55')+'">'+
            '<input class="tlf-skill" type="checkbox" value="'+LaRuche.Utils.esc(name)+'" '+(checked?'checked ':'')+(enabled?'':'disabled ')+'>'+ 
            '<span><strong>'+LaRuche.Utils.esc(name)+'</strong>'+(skill.description?' <span style="color:var(--text-dim)">— '+LaRuche.Utils.esc(skill.description)+'</span>':'')+(enabled?'':' <span style="color:var(--red)">(désactivé : non injecté)</span>')+'</span></label>';
        }).join('')+'</fieldset>';
    }
        var profiles = window._lastProfiles || {};
    var profOpts = '<option value="">Default (modele actif)</option>';
    Object.keys(profiles).forEach(function(k){
        profOpts += '<option value="'+k+'" '+(job.profile_id===k?'selected':'')+'>'+LaRuche.Utils.esc(profiles[k].name)+'</option>';
    });
    var modOpts = '<option value="">D&eacute;faut du provider</option>';
    if(job.profile_id && profiles[job.profile_id]) {
        var models = profiles[job.profile_id].models || [];
        models.forEach(function(m){
            modOpts += '<option value="'+LaRuche.Utils.esc(m)+'" '+(job.model===m?'selected':'')+'>'+LaRuche.Utils.esc(m)+'</option>';
        });
    } else if (!job.profile_id && job.model) {
        modOpts += '<option value="'+LaRuche.Utils.esc(job.model)+'" selected>'+LaRuche.Utils.esc(job.model)+'</option>';
    }

    d.innerHTML='<div class="tl-detail"><div style="font-weight:600;color:var(--amber);margin-bottom:8px">&Eacute;diter : '+LaRuche.Utils.esc(job.name||'')+'</div>'+
      '<label class="form-label">Nom</label><input class="form-input" id="tlfName" value="'+LaRuche.Utils.esc(job.name||'')+'">'+
      '<label class="form-label">Prompt</label><textarea class="form-input" id="tlfPrompt" rows="3">'+LaRuche.Utils.esc(job.prompt||'')+'</textarea>'+
      '<label class="form-label">Cron (5 champs) ou vide</label><input class="form-input" id="tlfCron" value="'+LaRuche.Utils.esc(job.cron_expr||'')+'" placeholder="*/30 * * * *">'+
      '<label class="form-label">Canal</label><input class="form-input" id="tlfChannel" value="'+LaRuche.Utils.esc(job.channel||'')+'" placeholder="telegram / vide">'+
      '<label class="form-label">Provider</label><select class="form-input" id="tlfProfileId" onchange="LaRuche.Settings.updateCronEditModelSelect()">'+profOpts+'</select>'+
      '<label class="form-label">Mod&egrave;le</label><select class="form-input" id="tlfModel">'+modOpts+'</select>'+
      skillHtml+
      '<div style="margin-top:10px;display:flex;gap:6px">'+
      '<button class="tl-btn" style="background:var(--amber);color:#000" onclick="LaRuche.Settings.tlSaveEdit('+i+')">Enregistrer</button>'+
      '<button class="tl-btn" onclick="LaRuche.Settings.tlDetail('+i+')">Annuler</button></div></div>';
  }
  function updateCronEditModelSelect() {
      var profSel = document.getElementById('tlfProfileId');
      var modSel = document.getElementById('tlfModel');
      if(!profSel || !modSel) return;
      var pid = profSel.value;
      modSel.innerHTML = '<option value="">D&eacute;faut du provider</option>';
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
      .then(function(){ LaRuche.Toast.show('Cron mis  jour','ok'); tlReload(); });
  }

  // MCP logic
  // Onglet Secrets : vault chiffré. L'UI ne reçoit JAMAIS les valeurs, seulement les noms.
  async function loadSecrets(el){
    var data={names:[]};
    try{ data=await fetch('/api/secrets').then(function(r){return r.json();}); }catch(e){}
    var names=(data.names||[]);
    var hooks=names.filter(function(n){return n.indexOf('WEBHOOK')===0;});
    var others=names.filter(function(n){return n.indexOf('WEBHOOK')!==0;});
    function card(list,title,hint){
      var rows = list.length ? list.map(function(n){
        return '<div class="settings-row"><span class="settings-value" style="font-family:var(--mono,monospace)">'+LaRuche.Utils.esc(n)+' <span style="color:var(--text-dim);font-size:10px">= ••••••••</span></span><button onclick="LaRuche.Settings.secretDelete(\''+LaRuche.Utils.esc(n)+'\')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:1px 8px;cursor:pointer;font-size:10px">Suppr</button></div>';
      }).join('') : '<div style="color:var(--text-dim);font-size:11px">Aucun.</div>';
      return '<div class="settings-card"><div class="settings-card-title">'+title+'</div><div style="color:var(--text-dim);font-size:11px;margin-bottom:8px">'+hint+'</div>'+rows+'</div>';
    }
    el.innerHTML =
      '<div style="color:var(--text-dim);font-size:12px;margin-bottom:12px">Les secrets sont <b>chiffrés au repos</b>. Le LLM ne voit JAMAIS leur valeur — seulement leur nom. Dans une commande, un script ou un champ clé d\'API, référence-les par <code>${NOM}</code> : la vraie valeur est substituée à l\'exécution.</div>'+
      card(others,'Secrets','Ex: API_OPENAI, TOKEN_TELEGRAM, USERID_TELEGRAM…')+
      card(hooks,'Webhooks','Nomme-les WEBHOOK_… (ex: WEBHOOK_DISCORD). Référence dans un script : ${WEBHOOK_DISCORD}')+
      '<div class="settings-card"><div class="settings-card-title">Ajouter / mettre à jour</div>'+
      '<label class="form-label">Nom (A-Z, 0-9, _)</label><input class="form-input" id="secName" placeholder="ex: WEBHOOK_DISCORD">'+
      '<label class="form-label">Valeur (jamais ré-affichée)</label><input class="form-input" id="secVal" type="password" placeholder="collez la valeur ici">'+
      '<button class="form-btn" style="margin-top:8px" onclick="LaRuche.Settings.secretSet()">Enregistrer</button></div>';
  }
  function secretSet(){
    var name=(document.getElementById('secName').value||'').trim();
    var value=document.getElementById('secVal').value||'';
    if(!name||!value){ LaRuche.Toast.show('Nom et valeur requis','warn'); return; }
    fetch(LaRuche.API.base+'/api/secrets',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify({name:name,value:value})})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show('Secret enregistré','ok'); if(LaRuche.Secrets)LaRuche.Secrets.refresh(); refreshTab(); } else { LaRuche.Toast.show('Échec (nom invalide ? A-Z/0-9/_ uniquement)','err'); } });
  }
  function secretDelete(name){
    fetch(LaRuche.API.base+'/api/secrets/'+encodeURIComponent(name),{method:'DELETE',credentials:'include'})
      .then(function(r){ if(r.ok){ LaRuche.Toast.show('Secret supprimé','ok'); if(LaRuche.Secrets)LaRuche.Secrets.refresh(); refreshTab(); } });
  }

  // Onglet MCP dédié (sorti de Providers).
  function loadMcp(el){
    var html = '<div class="settings-card" style="margin-bottom:16px">';
    html += '  <div class="settings-card-title">Serveurs MCP (Model Context Protocol)</div>';
    html += '  <div style="color:var(--text-dim);font-size:12px;margin-bottom:12px">Configurez des serveurs MCP locaux. LaRuche les utilisera pour étendre ses capacités via les agents.</div>';
    html += '  <div id="mcp-list" style="margin-bottom:12px"></div>';
    html += '  <div style="border:1px solid var(--border);border-radius:6px;padding:8px;background:var(--bg-panel)">';
    html += '     <div style="margin-bottom:8px"><label class="form-label">Nom du serveur</label><input id="mcp-new-name" class="form-input" placeholder="ex: local-sqlite"></div>';
    html += '     <div style="margin-bottom:8px"><label class="form-label">Commande</label><input id="mcp-new-cmd" class="form-input" placeholder="ex: node"></div>';
    html += '     <div style="margin-bottom:8px"><label class="form-label">Arguments (séparés par un espace)</label><input id="mcp-new-args" class="form-input" placeholder="ex: src/index.js --db sqlite.db"></div>';
    html += '     <button class="settings-save-btn" onclick="LaRuche.Settings.createMcpServer()">Ajouter le serveur</button>';
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
        html += '<div class="settings-row" style="margin-bottom:6px;padding-bottom:6px;border-bottom:1px solid rgba(42,42,46,0.3)"><span class="settings-label" style="flex:1">'+k+' <span style="font-size:10px;color:var(--text-dim)">('+s.command+' '+(s.args?s.args.join(' '):'')+')</span></span><button onclick="LaRuche.Settings.deleteMcpServer(\''+k+'\')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">Suppr</button></div>';
      }
      if(!html) html = '<div style="color:var(--text-dim);font-size:12px;padding:8px">Aucun serveur configuré.</div>';
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
         LaRuche.Toast.show('Serveur MCP ajouté','ok');
         document.getElementById('mcp-new-name').value = '';
         document.getElementById('mcp-new-cmd').value = '';
         document.getElementById('mcp-new-args').value = '';
         loadMcpServers();
      }
    });
  }

  function deleteMcpServer(n) {
    if(!confirm('Supprimer ce serveur MCP ?')) return;
    fetch('/api/mcp/servers/'+encodeURIComponent(n), {method:'DELETE'}).then(function(r){
      if(r.ok) { loadMcpServers(); LaRuche.Toast.show('Serveur MCP supprimé','ok'); }
    });
  }

  // Provider/Model selectors logic for Kanban/Watcher
  function updateKanbanModelSelect() {
    var pId = document.getElementById('kanban-profile').value;
    var modelSel = document.getElementById('kanban-model');
    if(!modelSel) return;
    modelSel.innerHTML = '<option value="">(Par défaut)</option>';
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
    modelSel.innerHTML = '<option value="">(Par défaut)</option>';
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
    
    var profOpts = '<option value="">Default (modele actif)</option>';
    Object.keys(profiles).forEach(function(k){
        profOpts += '<option value="'+k+'">'+LaRuche.Utils.esc(profiles[k].name)+'</option>';
    });

    el.innerHTML='<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="document.getElementById(\'newCronForm\').style.display=\'block\'">+ New Task</button></div>'+
      '<div id="newCronForm" style="display:none" class="settings-card">'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Name</label><input id="ncName" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Prompt</label><input id="ncPrompt" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Cadence (cron)</label><div id="ncCronBuilder"></div></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Canal de feedback</label><select id="ncChannel" class="form-input"><option value="">None (Activity Log)</option><option value="telegram">Telegram</option><option value="discord">Discord</option></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Provider</label><select id="ncProfileId" class="form-input" onchange="LaRuche.Settings.updateCronModelSelect()">'+profOpts+'</select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Mod&egrave;le</label><select id="ncModel" class="form-input"><option value="">D&eacute;faut du provider</option></select></div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.createCron()">Create</button></div>'+
      tasks.map(function(t){
          var effProv = "Default";
          if(t.profile_id && profiles[t.profile_id]) effProv = profiles[t.profile_id].name;
          else if(t.profile_id) effProv = t.profile_id;
          else if(t.provider) effProv = t.provider + (t.model ? " / " + t.model : "");
          else if(t.model) effProv = t.model;
          if(t.profile_id && t.model) effProv += " (" + t.model + ")";
          return '<div class="settings-card"><div class="settings-card-title">'+LaRuche.Utils.esc(t.name)+'</div><div class="settings-row"><span class="settings-label">Schedule</span><span class="settings-value">'+(t.cron_expr||t.fire_at||'-')+'</span></div><div class="settings-row"><span class="settings-label">Runs</span><span class="settings-value">'+(t.run_count||0)+'</span></div><div class="settings-row"><span class="settings-label">Channel</span><span class="settings-value">'+LaRuche.Utils.esc(t.channel||'None')+'</span></div><div class="settings-row"><span class="settings-label">Provider/Model</span><span class="settings-value">'+LaRuche.Utils.esc(effProv)+'</span></div><button onclick="LaRuche.Settings.deleteCronTask(\''+t.id+'\',this)" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;margin-top:6px">Delete</button></div>';
      }).join('');
    // Builder cron human-friendly pour le formulaire de creation.
    if(LaRuche.CronBuilder){ _ncCronBuilderId = LaRuche.CronBuilder.mount('ncCronBuilder', { value:'' }); }
  }
  // Suppression cron OPTIMISTE : retire la carte du DOM des que le DELETE reussit. Marche
  // dans n'importe quel conteneur (page Cron OU hub Missions) — fini le F5 (refreshTab
  // rechargeait le mauvais onglet selon le contexte).
  function deleteCronTask(id, btn){
    if(!confirm('Supprimer ce cron ?')) return;
    fetch('/api/cron/'+id,{method:'DELETE'}).then(function(r){
      if(!r.ok){ LaRuche.Toast.show('Suppression impossible','err'); return; }
      var card = btn && btn.closest('.settings-card'); if(card) card.remove();
      LaRuche.Toast.show('Cron supprimé','ok');
    }).catch(function(){ LaRuche.Toast.show('Suppression impossible','err'); });
  }
  
  function updateCronModelSelect() {
      var profSel = document.getElementById('ncProfileId');
      var modSel = document.getElementById('ncModel');
      if(!profSel || !modSel) return;
      var pid = profSel.value;
      modSel.innerHTML = '<option value="">D&eacute;faut du provider</option>';
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
    
    fetch('/api/cron',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)}).then(function(){loadTab('cron');LaRuche.Toast.show('Cron task created','ok');});
  }

  async function loadWatchers(el) {
    var watchers=[];try{watchers=await fetch('/api/watchers').then(function(r){return r.json();});}catch(e){}
    _watchersLast = JSON.stringify(watchers);
    // P1 : profils pour le selecteur Provider du watcher.
    var profilesResp={profiles:{}};try{profilesResp=await fetch('/api/profiles').then(function(r){return r.json();});}catch(e){}
    var profiles = profilesResp.profiles || {};
    _profiles = profiles;
    var profOpts = '<option value="">Défaut (modèle actif)</option>';
    Object.keys(profiles).forEach(function(k){
        profOpts += '<option value="'+k+'">'+LaRuche.Utils.esc(profiles[k].name||k)+'</option>';
    });
    el.innerHTML='<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="document.getElementById(\'newWatcherForm\').style.display=\'block\'">+ New Watcher</button></div>'+
      '<div id="newWatcherForm" style="display:none" class="settings-card">'+
      '<div style="font-weight:600;margin-bottom:8px">New Watcher</div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Name</label><input id="nwName" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Type</label><select id="nwType" class="form-input"><option value="file">File</option><option value="url">URL</option><option value="log">Log Pattern</option></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Target (Path/URL)</label><input id="nwTarget" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Condition (optional)</label><input id="nwCondition" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Prompt</label><input id="nwPrompt" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Provider</label><select id="watcher-profile" class="form-input" onchange="LaRuche.Settings.updateWatcherModelSelect()">'+profOpts+'</select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Mod&egrave;le</label><select id="watcher-model" class="form-input"><option value="">(Par défaut)</option></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Canal (déclenchement → notification)</label><select id="nwChannel" class="form-input"><option value="">Home channel (défaut)</option></select></div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.createWatcher()">Create</button></div>'+
      watchers.map(function(w){
        var effProv = "Défaut";
        if(w.profile_id && profiles[w.profile_id]) effProv = profiles[w.profile_id].name || w.profile_id;
        else if(w.profile_id) effProv = w.profile_id;
        else if(w.model) effProv = w.model;
        if(w.profile_id && w.model) effProv += " (" + w.model + ")";
        return '<div class="settings-card"><div class="settings-card-title">'+LaRuche.Utils.esc(w.name)+'</div><div class="settings-row"><span class="settings-label">Type</span><span class="settings-value">'+LaRuche.Utils.esc(w.watcher_type)+'</span></div><div class="settings-row"><span class="settings-label">Target</span><span class="settings-value">'+LaRuche.Utils.esc(w.target)+'</span></div><div class="settings-row"><span class="settings-label">Provider/Model</span><span class="settings-value">'+LaRuche.Utils.esc(effProv)+'</span></div><div class="settings-row"><span class="settings-label">Runs</span><span class="settings-value">'+(w.run_count||0)+'</span></div><div style="margin-top:6px;display:flex;gap:6px"><button onclick="LaRuche.Settings.editWatcher(\''+w.id+'\')" style="background:none;border:1px solid var(--amber);color:var(--amber);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">Éditer</button><button onclick="fetch(\'/api/watchers/'+w.id+'\',{method:\'DELETE\'}).then(function(){LaRuche.Settings.refreshTab()})" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">Delete</button></div></div>';}).join('');
    window.__fillChannels(document.getElementById('nwChannel'), '', 'Home channel (défaut)');
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
    fetch('/api/watchers',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}).then(function(){loadTab('watchers');LaRuche.Toast.show('Watcher created','ok');});
  }

  // Édition inline d'un watcher (parité avec cron/kanban).
  function editWatcher(id) {
    var w=null; try{ w=JSON.parse(_watchersLast).find(function(x){return x.id===id;}); }catch(e){}
    if(!w){ LaRuche.Toast.show('Watcher introuvable','err'); return; }
    function opt(v,label,cur){ return '<option value="'+v+'" '+(cur===v?'selected':'')+'>'+label+'</option>'; }
    var typeSel = opt('file','File',w.watcher_type)+opt('url','URL',w.watcher_type)+opt('log','Log Pattern',w.watcher_type);
    var profOpts = '<option value="">Défaut (modèle actif)</option>';
    Object.keys(_profiles).forEach(function(k){ profOpts += '<option value="'+k+'" '+((w.profile_id===k)?'selected':'')+'>'+LaRuche.Utils.esc(_profiles[k].name||k)+'</option>'; });
    var modOpts = '<option value="">(Par défaut)</option>';
    if(w.profile_id && _profiles[w.profile_id] && _profiles[w.profile_id].models){
      _profiles[w.profile_id].models.forEach(function(mm){ modOpts += '<option value="'+LaRuche.Utils.esc(mm)+'" '+((w.model===mm)?'selected':'')+'>'+LaRuche.Utils.esc(mm)+'</option>'; });
    }
    var ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div style="width:480px;max-width:92vw;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;padding:16px;max-height:90vh;overflow:auto">'+
      '<div style="font-weight:600;color:var(--amber);margin-bottom:10px">Éditer le watcher</div>'+
      '<label class="form-label">Nom</label><input class="form-input" id="weName" value="'+LaRuche.Utils.esc(w.name||'')+'">'+
      '<label class="form-label">Type</label><select class="form-input" id="weType">'+typeSel+'</select>'+
      '<label class="form-label">Cible (Path/URL)</label><input class="form-input" id="weTarget" value="'+LaRuche.Utils.esc(w.target||'')+'">'+
      '<label class="form-label">Condition</label><input class="form-input" id="weCondition" value="'+LaRuche.Utils.esc(w.condition||'')+'">'+
      '<label class="form-label">Prompt</label><textarea class="form-input" id="wePrompt" rows="3">'+LaRuche.Utils.esc(w.prompt||'')+'</textarea>'+
      '<label class="form-label">Provider</label><select class="form-input" id="weProfile" onchange="LaRuche.Settings.updateWatcherEditModelSelect()">'+profOpts+'</select>'+
      '<label class="form-label">Modèle</label><select class="form-input" id="weModel">'+modOpts+'</select>'+
      '<label class="form-label">Canal</label><select class="form-input" id="weChannel"><option value="">Home channel (défaut)</option></select>'+
      '<label class="form-label" style="display:flex;align-items:center;gap:8px;margin-top:8px"><input type="checkbox" id="weActive" '+(w.active?'checked':'')+'> Actif</label>'+
      '<div style="margin-top:12px;display:flex;gap:8px"><button class="form-btn" onclick="LaRuche.Settings.saveWatcherEdit(\''+id+'\',this)">Enregistrer</button>'+
      '<button class="form-btn" style="background:none;border:1px solid var(--border);color:var(--text-dim)" onclick="this.closest(\'div[style*=fixed]\')&&this.closest(\'div[style*=fixed]\').remove()">Annuler</button></div></div>';
    document.body.appendChild(ov);
    window.__fillChannels(document.getElementById('weChannel'), (w&&w.channel)||'', 'Home channel (défaut)');
  }

  function updateWatcherEditModelSelect() {
    var pId=document.getElementById('weProfile').value, sel=document.getElementById('weModel');
    if(!sel) return;
    sel.innerHTML='<option value="">(Par défaut)</option>';
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
      .then(function(r){ if(r.ok){ LaRuche.Toast.show('Watcher modifié','ok'); var ov=btn.closest('div[style*=fixed]'); if(ov)ov.remove(); refreshTab(); } else { LaRuche.Toast.show('Échec modification','err'); } });
  }

  function addCredential(provider) {
    var key = prompt('Nouvelle cle API pour ' + provider + ' :');
    if(!key) return;
    var label = prompt('Label optionnel (ex: Compte Dev) :') || '';
    fetch('/api/credentials', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({provider: provider, api_key: key, label: label})
    }).then(function(){
      loadProviders(document.getElementById('settingsContent'));
    }).catch(function(e){ LaRuche.Toast.show('Erreur: '+e, 'err'); });
  }


  function toggleVisibility(id, providerType, currentVis) {
    var newVis = currentVis === 'public_proxy' ? 'prive' : 'public_proxy';
    if(newVis === 'public_proxy' && (providerType === 'openai' || providerType === 'anthropic' || providerType === 'codex')) {
      if(!confirm("Public = le mesh utilise ce provider VIA ce node (ta clé reste locale, ce node relaie et exécute les appels). N'expose jamais une clé que tu ne veux pas voir consommée par le réseau. Continuer ?")) {
        return;
      }
    }
    fetch('/api/profiles/'+id+'/visibility', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({visibility: newVis})})
    .then(function(r){return r.json();})
    .then(function(d){
      if(d.status==='ok') {
        LaRuche.Toast.show('Visibilité modifiée avec succès','ok'); window.LaRuche.forceReactivityUpdate();
        loadTab('providers');
      } else {
        LaRuche.Toast.show('Erreur: '+(d.error||'?'),'err');
      }
    }).catch(function(e){LaRuche.Toast.show('Erreur: '+e,'err');});
  }

  // Menu permissions : Privé / Public / Restreint (cases par ruche) → couche grants.
  async function openAccess(id, currentVis, allowedEnc){
    var esc = LaRuche.Utils.esc;
    var allowed=[]; try{ allowed=JSON.parse(decodeURIComponent(allowedEnc||'%5B%5D')); }catch(e){}
    var peers=[]; try{ peers=(await fetch('/api/mesh/peers').then(function(r){return r.json();})).peers||[]; }catch(e){}
    function peersHtml(){
      if(!peers.length) return '<div style="color:var(--text-dim);font-size:12px">Aucune ruche découverte sur le réseau.</div>';
      return peers.map(function(pr){ var ck=allowed.indexOf(pr.id)!==-1?'checked':''; return '<label style="display:flex;gap:8px;align-items:center;padding:3px 0;font-size:13px"><input type="checkbox" class="acc-peer" value="'+esc(pr.id)+'" '+ck+'> 🐝 '+esc(pr.name||pr.id)+'</label>'; }).join('');
    }
    var ov=document.createElement('div'); ov.className='profile-modal-overlay open';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div class="profile-modal"><div class="profile-modal-head"><span class="profile-modal-name">🔐 Accès mesh du provider</span><button class="fd-btn" id="accClose">&#x2716;</button></div>'+
      '<p class="profile-modal-hint">Qui peut utiliser ce provider/LLM via le mesh ? (la clé API reste toujours locale)</p>'+
      '<div style="display:flex;flex-direction:column;gap:8px">'+
        '<label><input type="radio" name="accvis" value="prive" '+(currentVis==='prive'?'checked':'')+'> 🔒 <b>Privé</b> — moi seulement</label>'+
        '<label><input type="radio" name="accvis" value="public_proxy" '+(currentVis==='public_proxy'?'checked':'')+'> 🌐 <b>Public</b> — toutes les ruches du mesh</label>'+
        '<label><input type="radio" name="accvis" value="restricted" '+(currentVis==='restricted'?'checked':'')+'> 🐝 <b>Restreint</b> — seulement ces ruches :</label>'+
        '<div id="accPeers" style="margin-left:24px;'+(currentVis==='restricted'?'':'opacity:.45;pointer-events:none')+'">'+peersHtml()+'</div>'+
      '</div>'+
      '<div class="profile-modal-actions"><button class="send-btn" id="accSave"><span>Enregistrer</span></button></div></div>';
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
          if(d.status==='ok'){ LaRuche.Toast.show('Accès mis à jour','ok'); ov.remove(); if(window.LaRuche.forceReactivityUpdate)window.LaRuche.forceReactivityUpdate(); loadTab('providers'); }
          else LaRuche.Toast.show('Erreur: '+(d.error||'?'),'err');
        }).catch(function(e){ LaRuche.Toast.show('Erreur: '+e,'err'); });
    };
  }

  function deleteCredential(provider, apiKey) {
    if(!confirm('Supprimer cette cle du pool ?')) return;
    fetch('/api/credentials', {
      method: 'DELETE',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({provider: provider, api_key: apiKey})
    }).then(function(){
      loadProviders(document.getElementById('settingsContent'));
    }).catch(function(e){ LaRuche.Toast.show('Erreur: '+e, 'err'); });
  }

  async function loadChannels(el) {
    var config = await fetch(LaRuche.API.base+'/api/config/channels').then(function(r){return r.json();}).catch(function(){return {};});
    var notify = await fetch(LaRuche.API.base+'/api/config/notify').then(function(r){return r.json();}).catch(function(){return {};});
    var tg = config.telegram || {};
    var dc = config.discord || {};
    var sl = config.slack || {};
    el.innerHTML = '<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:16px">' +
      '<div class="settings-card"><div class="card-title" style="color:var(--amber)">Notifications</div>' +
        '<div style="font-size:11px;color:var(--text-dim);margin-bottom:8px">Envoi proactif des events (AgentCompleted, WatcherFired) via Telegram (le premier Chat ID configuré est utilisé).</div>' +
        '<label style="display:flex;align-items:center;gap:8px;cursor:pointer"><input type="checkbox" id="ch-notify-en" '+(notify.enabled?'checked':'')+'> <span>Activer Notifier proactif</span></label></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--blue)">Telegram</div>' +
        '<div class="form-group"><label class="form-label">Bot Token</label><input class="form-input" id="ch-tg-token" value="'+LaRuche.Utils.esc(tg.bot_token||'')+'" placeholder="7123456789:AAH..."></div>' +
        '<div class="form-group"><label class="form-label">Allowed Chat IDs</label><input class="form-input" id="ch-tg-chats" value="'+LaRuche.Utils.esc(tg.allowed_chats||'')+'" placeholder="vide = tous"></div>' +
        '<div style="font-size:10px;color:var(--text-muted);margin-top:4px">Lancer: python -m src.telegram</div></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--purple)">Discord</div>' +
        '<div class="form-group"><label class="form-label">Bot Token</label><input class="form-input" id="ch-dc-token" value="'+LaRuche.Utils.esc(dc.bot_token||'')+'" placeholder="MTIxxx..."></div>' +
        '<div class="form-group"><label class="form-label">Allowed Channel IDs</label><input class="form-input" id="ch-dc-channels" value="'+LaRuche.Utils.esc(dc.allowed_channels||'')+'" placeholder="vide = tous"></div>' +
        '<div style="font-size:10px;color:var(--text-muted);margin-top:4px">Lancer: python -m src.discord_bot</div></div>' +
      '<div class="settings-card"><div class="card-title" style="color:var(--green)">Slack</div>' +
        '<div class="form-group"><label class="form-label">Bot Token (xoxb-)</label><input class="form-input" id="ch-sl-bot" value="'+LaRuche.Utils.esc(sl.bot_token||'')+'" placeholder="xoxb-..."></div>' +
        '<div class="form-group"><label class="form-label">App Token (xapp-)</label><input class="form-input" id="ch-sl-app" value="'+LaRuche.Utils.esc(sl.app_token||'')+'" placeholder="xapp-..."></div>' +
        '<div style="font-size:10px;color:var(--text-muted);margin-top:4px">Lancer: python -m src.slack_bot</div></div>' +
      '<div class="settings-card" style="opacity:0.5;border-style:dashed"><div class="card-title" style="color:#25D366">WhatsApp</div>' +
        '<div style="color:var(--text-muted);font-size:12px;padding:12px 0">Coming soon</div></div>' +
      '<div class="settings-card" style="opacity:0.5;border-style:dashed"><div class="card-title" style="color:#3A76F0">Signal</div>' +
        '<div style="color:var(--text-muted);font-size:12px;padding:12px 0">Coming soon</div></div>' +
      '<div class="settings-card" style="opacity:0.5;border-style:dashed"><div class="card-title" style="color:#0DBD8B">Matrix</div>' +
        '<div style="color:var(--text-muted);font-size:12px;padding:12px 0">Coming soon</div></div>' +
    '</div>' +
    '<div style="margin-top:16px;display:flex;gap:8px">' +
      '<button class="form-btn" onclick="LaRuche.Settings.saveChannels()">Sauvegarder</button>' +
      '<button class="form-btn" style="background:var(--green)" onclick="LaRuche.Settings.startChannel(\'telegram\')" id="ch-tg-start">Demarrer Telegram</button>' +
      '<button class="form-btn" style="background:var(--red);color:#fff" onclick="LaRuche.Settings.stopChannel(\'telegram\')" id="ch-tg-stop" style="display:none">Arreter Telegram</button>' +
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
  // ── Page Skills (OKF en mémoire, capacities.skills.*) ──────────────────
  var SKILL_TEMPLATE='---\ntype: skill\nname: mon-skill\ndescription: "Ce que ce skill apprend a faire."\nallowed-tools: []\n---\n\n# Mon Skill\n\n## Quand l\'utiliser\n- ...\n\n## Procedure\n1. ...\n';
  async function loadSkills(el){
    var skills=await fetch(LaRuche.API.base+'/api/skills').then(function(r){return r.json();}).catch(function(){return [];});
    var html='<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">'+
      '<div style="color:var(--text-dim);font-size:12px">Connaissances procédurales (OKF). Activées = injectables dans le contexte / attachables aux crons.</div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.newSkill()">+ Nouveau skill</button></div>';
    if(!skills.length){ html+='<div style="color:var(--text-dim);padding:20px">Aucun skill. L\'agent peut en créer (memory_write capacities.skills.*) ou clique « + Nouveau skill ».</div>'; }
    html+='<div class="settings-grid">';
    skills.forEach(function(s){
      html+='<div class="settings-card">'+
        '<div style="display:flex;justify-content:space-between;align-items:center;gap:8px">'+
        '<div class="settings-card-title" style="margin:0">'+LaRuche.Utils.esc(s.name)+'</div>'+
        '<label class="lr-switch"><input type="checkbox" '+(s.enabled?'checked':'')+' onchange="LaRuche.Settings.toggleSkill(\''+LaRuche.Utils.esc(s.name)+'\')"><span class="lr-slider"></span></label>'+
        '</div>'+
        '<div style="font-size:11px;color:var(--text-dim);margin:6px 0;min-height:28px">'+LaRuche.Utils.esc(s.description||'')+'</div>'+
        '<div style="display:flex;gap:6px">'+
        '<button class="tl-btn" onclick="LaRuche.Settings.viewSkill(\''+LaRuche.Utils.esc(s.name)+'\')">Voir / Éditer</button>'+
        '<button class="tl-btn" style="border-color:var(--red);color:var(--red)" onclick="if(confirm(\'Supprimer '+LaRuche.Utils.esc(s.name)+' ?\'))LaRuche.Settings.deleteSkill(\''+LaRuche.Utils.esc(s.name)+'\')">Suppr</button>'+
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
  function toggleSkill(name){ fetch(LaRuche.API.base+'/api/skills/'+encodeURIComponent(name)+'/toggle',{method:'POST'}).then(function(r){return r.json();}).then(function(d){ LaRuche.Toast.show('Skill '+(d.enabled?'activé':'désactivé'),'ok'); }); }
  function deleteSkill(name){ fetch(LaRuche.API.base+'/api/skills/'+encodeURIComponent(name),{method:'DELETE'}).then(function(){ LaRuche.Settings.refreshTab&&LaRuche.Settings.refreshTab(); }); }
  var PLUGIN_TEMPLATE = '{\n  "name": "mon_plugin",\n  "description": "Description de mon plugin",\n  "danger": "safe",\n  "parameters": {\n    "type": "object",\n    "properties": {},\n    "required": []\n  },\n  "command": "echo {{arg}}"\n}';
  function newPlugin(){ pluginEditor('nouveau_plugin', PLUGIN_TEMPLATE); }
  function newSkill(){ skillEditor('', SKILL_TEMPLATE); }
  function viewSkill(name){ fetch(LaRuche.API.base+'/api/skills/'+encodeURIComponent(name)).then(function(r){return r.json();}).then(function(d){ skillEditor(name, d.content||''); }); }
  function skillEditor(name, content){
    var ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div style="width:680px;max-width:94vw;height:80vh;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;display:flex;flex-direction:column">'+
      '<div style="padding:10px 14px;border-bottom:1px solid var(--border);font-weight:600;color:var(--amber)">'+(name?('Éditer : '+LaRuche.Utils.esc(name)):'Nouveau skill')+' <span style="color:var(--text-dim);font-size:10px;font-weight:normal">— SKILL.md (frontmatter validé au save)</span></div>'+
      '<textarea id="skEditor" class="form-input" style="flex:1;margin:12px 12px 6px;font-family:var(--mono);font-size:12px;resize:none">'+LaRuche.Utils.esc(content)+'</textarea>'+
      '<div style="margin:0 12px 6px">'+
        '<div style="display:flex;align-items:center;gap:8px;margin-bottom:4px">'+
          '<span style="font-size:10px;color:var(--text-dim);flex:1">Abeilles / plugins de ce skill (→ <code>tools:</code>) <span id="skToolsCount" style="color:var(--amber)"></span></span>'+
          '<input id="skToolsSearch" placeholder="filtrer…" oninput="LaRuche.Settings.filterSkillTools()" style="font-size:11px;padding:2px 6px;width:120px;background:#16161a;border:1px solid var(--border);border-radius:4px;color:var(--text)">'+
          '<button class="tl-btn" style="font-size:10px;padding:2px 6px" onclick="LaRuche.Settings.clearSkillTools()">Vider</button>'+
        '</div>'+
        '<div id="skToolsBox" style="max-height:200px;overflow:auto;border:1px solid var(--border);border-radius:6px;padding:4px"><span style="color:var(--text-dim);font-size:11px">Chargement…</span></div></div>'+
      '<div style="padding:10px 14px;border-top:1px solid var(--border);display:flex;gap:8px;justify-content:flex-end">'+
      '<button class="tl-btn" onclick="this.closest(\'div[style*=fixed]\').remove()">Annuler</button>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.saveSkill(this)">Enregistrer</button></div></div>';
    document.body.appendChild(ov);
    mountSkillTools(content);
  }
  // Construit la checklist d'outils du skill (groupée Abeilles/Plugins, recherchable,
  // sélectionnés en tête) et synchronise la ligne `tools:` du frontmatter.
  async function mountSkillTools(content){
    var box=document.getElementById('skToolsBox'); if(!box) return;
    var tools = window._allTools;
    if(!tools){ try{ tools=await fetch('/api/tools').then(function(r){return r.json();}); window._allTools=tools; }catch(e){ tools=[]; } }
    var plugins = [];
    try{ plugins=await fetch('/api/plugins').then(function(r){return r.json();}); }catch(e){}
    var pluginNames = (plugins||[]).map(function(p){return p.name||p;});
    // Modèle unifié : {name, group, desc}. group = Plugins | Abeilles | Autres.
    var items = [];
    var seen = {};
    (tools||[]).forEach(function(t){
      var n=t.name||t; if(seen[n])return; seen[n]=1;
      items.push({name:n, group:(pluginNames.indexOf(n)>=0?'Plugins':'Abeilles'), desc:(t.description||'')});
    });
    pluginNames.forEach(function(n){ if(!seen[n]){ seen[n]=1; items.push({name:n, group:'Plugins', desc:''}); } });
    var m = content.match(/^\s*(?:allowed-)?tools:\s*\[([^\]]*)\]/m);
    var current = m ? m[1].split(',').map(function(s){return s.trim().replace(/['"]/g,'');}).filter(Boolean) : [];
    current.forEach(function(n){ if(!seen[n]){ seen[n]=1; items.push({name:n, group:'Autres', desc:'(référence)'}); } });
    window._skItems = items;
    window._skChecked = {}; current.forEach(function(n){ window._skChecked[n]=1; });
    renderSkillTools();
  }
  // (Ré)affiche la liste selon le filtre + l'état coché courant. Sélectionnés en tête de groupe.
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
      // sélectionnés d'abord, puis alpha
      list.sort(function(a,b){ var ca=checked[a.name]?0:1, cb=checked[b.name]?0:1; return ca-cb || a.name.localeCompare(b.name); });
      html+='<div style="font-size:9px;text-transform:uppercase;letter-spacing:.5px;color:var(--text-dim);padding:6px 7px 2px">'+g+' ('+list.filter(function(i){return checked[i.name];}).length+'/'+list.length+')</div>';
      html+='<div style="display:grid;grid-template-columns:1fr 1fr;gap:1px">'+list.map(row).join('')+'</div>';
    });
    box.innerHTML = html || '<span style="color:var(--text-dim);font-size:11px;padding:6px;display:block">Aucun résultat.</span>';
    var cnt=document.getElementById('skToolsCount');
    if(cnt){ var n=Object.keys(checked).filter(function(k){return checked[k];}).length; cnt.textContent=n?('— '+n+' coché'+(n>1?'s':'')):''; }
  }
  function toggleSkillTool(name, on){ window._skChecked=window._skChecked||{}; if(on) window._skChecked[name]=1; else delete window._skChecked[name]; applySkillTools(); renderSkillTools(); }
  function filterSkillTools(){ renderSkillTools(); }
  function clearSkillTools(){ window._skChecked={}; applySkillTools(); renderSkillTools(); }
  function applySkillTools(){
    // Lit le MODÈLE (_skChecked), pas le DOM : sinon un filtre actif masquerait des cochés
    // et on les perdrait à l'enregistrement.
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
        LaRuche.Toast.show('Skill « '+d.name+' » enregistré','ok');
        var ov=btn.closest('div[style*=fixed]'); if(ov)ov.remove();
        LaRuche.Settings.refreshTab&&LaRuche.Settings.refreshTab();
      }).catch(function(){ LaRuche.Toast.show('Échec','err'); });
  }

  function viewPlugin(name){ fetch(LaRuche.API.base+'/api/plugins/'+encodeURIComponent(name)).then(function(r){return r.json();}).then(function(d){ pluginEditor(name, d.content||''); }).catch(function(){ LaRuche.Toast.show('Fichier non trouvé','err'); }); }
  function pluginEditor(name, content){
    var ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div style="width:680px;max-width:94vw;height:80vh;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;display:flex;flex-direction:column">'+
      '<div style="padding:10px 14px;border-bottom:1px solid var(--border);font-weight:600;color:var(--amber)">Éditer Plugin : '+LaRuche.Utils.esc(name)+' <span style="color:var(--text-dim);font-size:10px;font-weight:normal">— JSON (rechargé au save)</span></div>'+
      '<textarea id="plEditor" data-name="'+LaRuche.Utils.esc(name)+'" class="form-input" style="flex:1;margin:12px;font-family:var(--mono);font-size:12px;resize:none" spellcheck="false">'+LaRuche.Utils.esc(content)+'</textarea>'+
      '<div style="padding:10px 14px;border-top:1px solid var(--border);display:flex;gap:8px;justify-content:flex-end">'+
      '<button class="tl-btn" onclick="this.closest(\'div[style*=fixed]\').remove()">Annuler</button>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.savePlugin(this)">Enregistrer</button></div></div>';
    document.body.appendChild(ov);
  }
  function savePlugin(btn){
    var ta=document.getElementById('plEditor');
    var content=ta.value; var name=ta.dataset.name;
    fetch(LaRuche.API.base+'/api/plugins/'+encodeURIComponent(name),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({content:content})})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show('Plugin « '+d.name+' » enregistré','ok');
        var ov=btn.closest('div[style*=fixed]'); if(ov)ov.remove();
        LaRuche.Settings.refreshTab&&LaRuche.Settings.refreshTab();
      }).catch(function(){ LaRuche.Toast.show('Échec','err'); });
  }
  function deletePlugin(name){
    fetch(LaRuche.API.base+'/api/plugins/'+encodeURIComponent(name),{method:'DELETE'})
      .then(function(r){return r.json();}).then(function(d){
        LaRuche.Toast.show('Plugin supprimé','ok');
        LaRuche.Settings.refreshTab&&LaRuche.Settings.refreshTab();
      }).catch(function(){ LaRuche.Toast.show('Échec','err'); });
  }

  var _kanbanTimer=null, _kanbanLast='';
  var _kanbanView=(function(){ try{ return localStorage.getItem('lr_kanban_view')||'cols'; }catch(e){ return 'cols'; } })();
  var _profiles={}; // P1 : cache des profils pour les selecteurs Provider (kanban/watcher)
  var _watchersLast='[]'; // cache des watchers pour l'edition inline

  function setKanbanView(mode){
    _kanbanView = mode;
    try{ localStorage.setItem('lr_kanban_view', mode); }catch(e){}
    var tg=document.getElementById('kanbanViewToggle'); if(tg) tg.innerHTML = kanbanToggleInner();
    _kanbanLast=''; refreshKanbanCols();
  }
  function kanbanToggleInner(){
    return '<button class="tl-btn" style="border-radius:0'+(_kanbanView==='cols'?';background:var(--amber);color:#000':'')+'" onclick="LaRuche.Settings.setKanbanView(\'cols\')">Colonnes</button>'+
      '<button class="tl-btn" style="border-radius:0'+(_kanbanView==='rows'?';background:var(--amber);color:#000':'')+'" onclick="LaRuche.Settings.setKanbanView(\'rows\')">Horizontal</button>';
  }
  // Carte kanban (HTML) — partagee entre mode colonnes et mode horizontal.
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
      // Accordeon : clic pour deplier/replier le commentaire LLM (lisible en entier, mobile-friendly).
      // stopPropagation evite d'interferer avec le drag&drop de la carte.
      h+='<div class="kb-result" onclick="event.stopPropagation();LaRuche.Settings.toggleKanbanResult(this)" '+
         'data-collapsed="1" style="font-size:10px;color:var(--green);margin-bottom:6px;cursor:pointer" '+
         'title="'+(_trunc?'Cliquer pour deplier/replier':'')+'">'+
         '<span class="kb-result-label">Résultat'+(_trunc?' ▸':'')+': </span>'+
         '<span class="kb-result-short" style="white-space:pre-wrap;word-break:break-word">'+LaRuche.Utils.esc(_short)+'</span>'+
         '<span class="kb-result-full" style="display:none;white-space:pre-wrap;word-break:break-word">'+LaRuche.Utils.esc(_full)+'</span>'+
         '</div>';
    }
    h+='<div style="display:flex;justify-content:space-between;align-items:center">';
    h+='<span style="font-size:9px;color:var(--text-muted);font-family:var(--mono)">'+t.id.split('-')[0]+'</span>';
    h+='<span><button onclick="LaRuche.Settings.editKanbanTask(\''+t.id+'\')" style="background:none;border:none;color:var(--amber);cursor:pointer;font-size:10px">Éditer</button> <button onclick="LaRuche.Settings.deleteKanbanTask(\''+t.id+'\')" style="background:none;border:none;color:var(--red);cursor:pointer;font-size:10px">Suppr</button></span>';
    h+='</div></div>';
    return h;
  }
  // Deplie/replie le commentaire LLM (champ result) d'une carte kanban.
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
    // P1 : profils pour le selecteur Provider de la tache kanban.
    var profilesResp={profiles:{}};try{profilesResp=await fetch('/api/profiles').then(function(r){return r.json();});}catch(e){}
    _profiles = profilesResp.profiles || {};
    var profOpts = '<option value="">Défaut (modèle actif)</option>';
    Object.keys(_profiles).forEach(function(k){
        profOpts += '<option value="'+k+'">'+LaRuche.Utils.esc(_profiles[k].name||k)+'</option>';
    });
    el.innerHTML = '<div style="margin-bottom:16px;display:flex;gap:8px;align-items:end;flex-wrap:wrap">' +
      '<div style="flex:1;min-width:140px"><label class="form-label">Titre de la tâche</label><input class="form-input" id="kanban-title" placeholder="Nouvelle tâche..."></div>' +
      '<div style="flex:2;min-width:160px"><label class="form-label">Description</label><input class="form-input" id="kanban-desc" placeholder="Détails..."></div>' +
      '<div style="flex:1;min-width:130px"><label class="form-label">Provider</label><select class="form-input" id="kanban-profile" onchange="LaRuche.Settings.updateKanbanModelSelect()">'+profOpts+'</select></div>' +
      '<div style="flex:1;min-width:130px"><label class="form-label">Mod&egrave;le</label><select class="form-input" id="kanban-model"><option value="">(Par défaut)</option></select></div>' +
      '<div style="flex:1;min-width:150px"><label class="form-label">Canal</label><select class="form-input" id="kanban-channel"><option value="">Défaut du board</option></select></div>' +
      '<button class="form-btn" onclick="LaRuche.Settings.createKanbanTask()">Créer</button></div>' +
      '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px;flex-wrap:wrap;gap:8px">' +
        '<div style="display:flex;align-items:center;gap:6px"><label class="form-label" style="margin:0">Canal par défaut du board</label>' +
        '<select class="form-input" id="kanban-default-channel" style="width:auto" onchange="LaRuche.Settings.setKanbanDefaultChannel(this.value)"><option value="">Aucun (→ home channel)</option></select></div>' +
        '<div id="kanbanViewToggle" style="display:inline-flex;border:1px solid var(--border);border-radius:6px;overflow:hidden">'+kanbanToggleInner()+'</div></div>' +
      '<div id="kanbanCols"></div>';
    _kanbanLast='';
    window.__fillChannels(document.getElementById('kanban-channel'), '', 'Défaut du board');
    try{ var dc=await fetch('/api/kanban/default_channel').then(function(r){return r.json();}); window.__fillChannels(document.getElementById('kanban-default-channel'), (dc&&dc.channel)||'', 'Aucun (→ home channel)'); }catch(e){}
    await refreshKanbanCols();
    if(_kanbanTimer) clearInterval(_kanbanTimer);
    // Auto-refresh (l'agent/daemon peuvent modifier le board) : re-render seulement
    // si le contenu a changé → ne casse pas la saisie en cours.
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
    var html;
    if(_kanbanView==='rows'){
      // Mode horizontal condense : chaque statut = une bande, cartes en flex-wrap, hauteur = contenu.
      html='<div style="display:flex;flex-direction:column;gap:10px">';
      cols.forEach(function(c){
        var colTasks=tasks.filter(function(t){return t.status===c;});
        html+='<div style="background:rgba(30,30,32,0.8);border:1px solid var(--amber-dim);border-radius:6px;overflow:hidden" ondragover="LaRuche.Settings.kanbanDragOver(event)" ondrop="LaRuche.Settings.kanbanDrop(event,\''+c+'\')">';
        html+='<div style="padding:6px 10px;font-weight:600;color:var(--amber);border-bottom:1px solid var(--border);display:flex;justify-content:space-between;align-items:center"><span>'+c+'</span><span style="font-size:10px;color:var(--text-dim)">'+colTasks.length+'</span></div>';
        html+='<div style="padding:8px;display:flex;flex-wrap:wrap;gap:8px;min-height:36px">';
        if(!colTasks.length){ html+='<span style="font-size:10px;color:var(--text-muted);align-self:center">—</span>'; }
        colTasks.forEach(function(t){ html+='<div style="flex:0 0 230px;max-width:230px">'+kanbanCardHtml(t)+'</div>'; });
        html+='</div></div>';
      });
      html+='</div>';
    } else {
      // Mode colonnes (existant).
      html='<div style="display:flex;gap:12px;overflow-x:auto;padding-bottom:10px;min-height:400px">';
      cols.forEach(function(c){
        html+='<div style="flex:0 0 250px;background:rgba(30,30,32,0.8);border:1px solid var(--amber-dim);border-radius:6px;display:flex;flex-direction:column" ondragover="LaRuche.Settings.kanbanDragOver(event)" ondrop="LaRuche.Settings.kanbanDrop(event,\''+c+'\')">';
        var colTasks=tasks.filter(function(t){return t.status===c;});
        html+='<div style="padding:10px;font-weight:600;color:var(--amber);border-bottom:1px solid var(--border);text-align:center">'+c+(colTasks.length?(' ('+colTasks.length+')'):'')+'</div>';
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
      .then(function(){ LaRuche.Toast.show('Canal par défaut du board mis à jour','ok'); });
  }
  function createKanbanTask() {
    var title = document.getElementById('kanban-title').value;
    var desc = document.getElementById('kanban-desc').value;
var pId = document.getElementById('kanban-profile')?document.getElementById('kanban-profile').value:'';
var m = document.getElementById('kanban-model')?document.getElementById('kanban-model').value:'';
var ch = document.getElementById('kanban-channel')?document.getElementById('kanban-channel').value:'';
    if(!title) return;
    fetch(LaRuche.API.base+'/api/kanban',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({title: title, description: desc, profile_id: pId||null, model: m||null, channel: ch||null})})
      .then(function(r){if(r.ok) { LaRuche.Toast.show('Tâche créée','ok'); document.getElementById('kanban-title').value=''; document.getElementById('kanban-desc').value=''; _kanbanLast=''; refreshKanbanCols(); }});
  }

  function deleteKanbanTask(id) {
    fetch(LaRuche.API.base+'/api/kanban/'+id,{method:'DELETE'})
      .then(function(r){if(r.ok) { _kanbanLast=''; refreshKanbanCols(); }});
  }

  function editKanbanTask(id) {
    var t=null; try{ t=JSON.parse(_kanbanLast).find(function(x){return x.id===id;}); }catch(e){}
    // P1 : selecteur Provider dans l'edition kanban.
    var profOpts = '<option value="">Défaut (modèle actif)</option>';
    Object.keys(_profiles).forEach(function(k){
        profOpts += '<option value="'+k+'" '+((t&&t.profile_id===k)?'selected':'')+'>'+LaRuche.Utils.esc(_profiles[k].name||k)+'</option>';
    });
    var modOpts = '<option value="">(Par défaut)</option>';
    if(t && t.profile_id && _profiles[t.profile_id] && _profiles[t.profile_id].models){
      _profiles[t.profile_id].models.forEach(function(mm){
        modOpts += '<option value="'+LaRuche.Utils.esc(mm)+'" '+((t.model===mm)?'selected':'')+'>'+LaRuche.Utils.esc(mm)+'</option>';
      });
    }
    var ov=document.createElement('div');
    ov.style.cssText='position:fixed;inset:0;background:rgba(0,0,0,.72);z-index:99999;display:flex;align-items:center;justify-content:center';
    ov.onclick=function(e){ if(e.target===ov) ov.remove(); };
    ov.innerHTML='<div style="width:480px;max-width:92vw;background:#0d0d10;border:1px solid var(--amber);border-radius:10px;padding:16px">'+
      '<div style="font-weight:600;color:var(--amber);margin-bottom:10px">Éditer la tâche</div>'+
      '<label class="form-label">Titre</label><input class="form-input" id="kbeTitle" value="'+LaRuche.Utils.esc(t?t.title:'')+'">'+
      '<label class="form-label">Description</label><textarea class="form-input" id="kbeDesc" rows="4">'+LaRuche.Utils.esc(t?(t.description||''):'')+'</textarea>'+
      '<label class="form-label">Provider</label><select class="form-input" id="kbeProfile" onchange="LaRuche.Settings.updateKanbanEditModelSelect()">'+profOpts+'</select>'+
      '<label class="form-label">Mod&egrave;le</label><select class="form-input" id="kbeModel">'+modOpts+'</select>'+
      '<label class="form-label">Canal</label><select class="form-input" id="kbeChannel"><option value="">Défaut du board</option></select>'+
      '<div style="margin-top:12px;display:flex;gap:8px"><button class="form-btn" onclick="LaRuche.Settings.saveKanbanEdit(\''+id+'\',this)">Enregistrer</button>'+
      '<button class="form-btn" style="background:none;border:1px solid var(--border);color:var(--text-dim)" onclick="this.closest(\'div[style*=fixed]\')&&this.closest(\'div[style*=fixed]\').remove()">Annuler</button></div></div>';
    document.body.appendChild(ov);
    window.__fillChannels(document.getElementById('kbeChannel'), (t&&t.channel)||'', 'Défaut du board');
  }

  // P1 : repeuple le selecteur modele de l'edition kanban quand on change de provider.
  function updateKanbanEditModelSelect() {
    var pId = document.getElementById('kbeProfile').value;
    var modelSel = document.getElementById('kbeModel');
    if(!modelSel) return;
    modelSel.innerHTML = '<option value="">(Par défaut)</option>';
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
      .then(function(r){ if(r.ok){ LaRuche.Toast.show('Tâche modifiée','ok'); _kanbanLast=''; refreshKanbanCols(); var ov=btn.closest('div[style*=fixed]'); if(ov)ov.remove(); } });
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
      if(res.ok) LaRuche.Toast.show('Inference config saved','ok');
      else LaRuche.Toast.show('Save failed','err');
    } catch(e) { LaRuche.Toast.show('Error: '+e,'err'); }
  }

  async function loadOnboarding(el) {
    var data = await fetch(LaRuche.API.base+'/api/onboarding').then(function(r){return r.json();}).catch(function(){return {steps:[],progress:'0/0',complete:false};});
    var html = '<div style="margin-bottom:16px"><span style="font-size:18px;font-weight:600">Setup Checklist</span>' +
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
      .then(function(r){if(r.ok) LaRuche.Toast.show('Configuration Contexte sauvegardée', 'ok'); else LaRuche.Toast.show('Erreur de sauvegarde', 'err');})
      .catch(function(e){LaRuche.Toast.show('Error: '+e,'err');});
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
      .then(function(r){ if(r.ok) LaRuche.Toast.show('Génération appliquée (à chaud)','ok'); else LaRuche.Toast.show('Erreur','err'); })
      .catch(function(e){ LaRuche.Toast.show('Error: '+e,'err'); });
  }

  function toggleCurateur(on) {
    fetch(LaRuche.API.base+'/api/config/curateur',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify({enabled:!!on})})
      .then(function(r){return r.json();})
      .then(function(d){ if(d && d.status==='ok') LaRuche.Toast.show('Curateur '+(on?'activé':'désactivé'),'ok'); else LaRuche.Toast.show('Échec curateur','err'); })
      .catch(function(){ LaRuche.Toast.show('Échec curateur','err'); });
  }
  function toggleDynamicTools(on) {
    fetch(LaRuche.API.base+'/api/config/curateur',{method:'POST',credentials:'include',headers:{'Content-Type':'application/json'},body:JSON.stringify({dynamic_tools:!!on})})
      .then(function(r){return r.json();})
      .then(function(d){ if(d && d.status==='ok') LaRuche.Toast.show('Sélection dynamique des outils '+(on?'activée':'désactivée'),'ok'); else LaRuche.Toast.show('Échec','err'); })
      .catch(function(){ LaRuche.Toast.show('Échec','err'); });
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
      .then(function(){LaRuche.Toast.show('Channels config saved','ok');})
      .catch(function(e){LaRuche.Toast.show('Error: '+e,'err');});
  }

  async function loadKnowledge(el) {
    var data = await fetch(LaRuche.API.base+'/api/knowledge').then(function(r){return r.json();}).catch(function(){return {entries:[],count:0};});
    var html = '<div style="margin-bottom:16px;display:flex;gap:8px;align-items:end">' +
      '<div style="flex:1"><label class="form-label">Ajouter une connaissance</label><input class="form-input" id="kb-text" placeholder="Information a memoriser..."></div>' +
      '<div><label class="form-label">Source</label><input class="form-input" id="kb-source" placeholder="optionnel" style="width:150px"></div>' +
      '<button class="form-btn" onclick="LaRuche.Settings.addKnowledge()">Ajouter</button></div>';
    html += '<div style="margin-bottom:16px;display:flex;gap:8px;">' +
      '<button class="form-btn" onclick="LaRuche.Settings.exportOkf()">Export OKF</button>' +
      '<button class="form-btn" onclick="LaRuche.Settings.importOkf()">Import OKF</button>' +
      '</div>';
    html += '<div style="font-size:12px;color:var(--text-dim);margin-bottom:12px">'+data.count+' entree(s) dans la base de connaissances</div>';
    if(data.entries && data.entries.length > 0) {
      html += '<table style="width:100%;border-collapse:collapse;font-size:12px">';
      html += '<tr><th style="text-align:left;padding:6px;color:var(--text-dim);border-bottom:1px solid var(--border)">ID</th>';
      html += '<th style="text-align:left;padding:6px;color:var(--text-dim);border-bottom:1px solid var(--border)">Texte</th>';
      html += '<th style="padding:6px;color:var(--text-dim);border-bottom:1px solid var(--border)">Source</th>';
      html += '<th style="padding:6px;color:var(--text-dim);border-bottom:1px solid var(--border)">Actions</th></tr>';
      data.entries.forEach(function(e) {
        html += '<tr><td style="padding:6px;border-bottom:1px solid rgba(42,42,46,.3);font-family:var(--mono);font-size:10px;color:var(--text-muted)">'+LaRuche.Utils.esc(e.id)+'</td>';
        html += '<td style="padding:6px;border-bottom:1px solid rgba(42,42,46,.3)">'+LaRuche.Utils.esc((e.text||'').substring(0,100))+'</td>';
        html += '<td style="padding:6px;border-bottom:1px solid rgba(42,42,46,.3);color:var(--text-dim)">'+LaRuche.Utils.esc(e.source||'-')+'</td>';
        html += '<td style="padding:6px;border-bottom:1px solid rgba(42,42,46,.3);text-align:center">' +
          '<button onclick="LaRuche.Settings.editKnowledge(\''+e.id+'\',this)" style="background:none;border:1px solid var(--amber);color:var(--amber);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;margin-right:4px">Editer</button>' +
          '<button onclick="LaRuche.Settings.deleteKnowledge(\''+e.id+'\')" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">Suppr</button></td></tr>';
      });
      html += '</table>';
    } else {
      html += '<div style="text-align:center;color:var(--text-muted);padding:30px">Base vide. L\'agent peut ajouter des connaissances via l\'outil knowledge_add, ou ajoutez-en manuellement ci-dessus.</div>';
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
        if(d.error) LaRuche.Toast.show('Erreur: '+d.error,'err');
        else { LaRuche.Toast.show('Connaissance ajoutee ('+d.id+')','ok'); loadTab('knowledge'); }
      })
      .catch(function(e){LaRuche.Toast.show('Erreur: '+e,'err');});
  }

  function exportOkf() {
    // Telechargement .zip navigateur (toute la memoire) au lieu d'un dossier serveur.
    var a = document.createElement('a');
    a.href = LaRuche.API.base+'/api/memory/export.zip';
    a.download = ''; a.style.display = 'none';
    document.body.appendChild(a); a.click();
    setTimeout(function(){ a.remove(); }, 0);
    LaRuche.Toast.show('Telechargement OKF lance (tout)', 'ok');
  }

  function importOkf() {
    fetch(LaRuche.API.base+'/api/memory/import_okf?dir=okf-export', {method:'POST'})
      .then(function(r){return r.json();})
      .then(function(res){
        if(res.ok) {
            LaRuche.Toast.show('OKF importe avec succes', 'ok');
            loadKnowledge(document.getElementById('settings-content'));
        }
        else LaRuche.Toast.show('Erreur import: ' + res.error, 'err');
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
    actionsCell.innerHTML = '<button onclick="LaRuche.Settings.saveKnowledgeEdit(\''+id+'\',this)" style="background:var(--green);color:#000;border:none;border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;margin-right:4px">OK</button>' +
      '<button onclick="LaRuche.Settings.refreshTab()" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px">Annuler</button>';
  }

  function saveKnowledgeEdit(id, btn) {
    var row = btn.closest('tr');
    var newText = row.cells[1].querySelector('textarea').value;
    var newSource = row.cells[2].querySelector('input').value;
    fetch(LaRuche.API.base+'/api/knowledge/'+id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({text:newText,source:newSource||'manual'})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d.error) LaRuche.Toast.show('Erreur: '+d.error,'err');
        else { LaRuche.Toast.show('Mis a jour','ok'); loadTab('knowledge'); }
      })
      .catch(function(e){LaRuche.Toast.show('Erreur: '+e,'err');});
  }

  function deleteKnowledge(id) {
    fetch(LaRuche.API.base+'/api/knowledge/'+id,{method:'DELETE'})
      .then(function(){LaRuche.Toast.show('Supprime','ok'); loadTab('knowledge');})
      .catch(function(e){LaRuche.Toast.show('Erreur: '+e,'err');});
  }

  function refreshTab() { loadTab(currentTab); }

  function startChannel(name) {
    fetch(LaRuche.API.base+'/api/channels/start',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({channel:name})})
      .then(function(r){return r.json();})
      .then(function(d){
        if(d.status==='started') LaRuche.Toast.show(name+' demarre !','ok');
        else if(d.status==='already_running') LaRuche.Toast.show(name+' deja en marche','info');
        else LaRuche.Toast.show(d.message||'Erreur','err');
        loadTab('channels');
      });
  }

  function stopChannel(name) {
    fetch(LaRuche.API.base+'/api/channels/stop',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({channel:name})})
      .then(function(r){return r.json();})
      .then(function(d){
        LaRuche.Toast.show(name+' arrete','ok');
        loadTab('channels');
      });
  }

    var _bpCronBuilderId = null; // instance CronBuilder du formulaire de creation

    async function loadBlueprints(el) {
    var bps=[];try{bps=await fetch('/api/blueprints').then(function(r){return r.json();});}catch(e){}
    window._blueprints = bps || [];
    var head = '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px;gap:8px;flex-wrap:wrap">' +
      '<span style="color:var(--amber);font-size:12px;">Sélectionnez un blueprint pour l\'instancier en tant que tâche cron.</span>' +
      '<button class="settings-save-btn" onclick="LaRuche.Settings.openNewBlueprintForm()">+ Nouveau blueprint</button>' +
      '</div>';
    var creationSlot = '<div id="bpNewFormWrap"></div>';
    var cards = (!window._blueprints.length)
      ? '<div style="text-align:center;color:var(--text-muted);padding:20px">Aucun blueprint disponible</div>'
      : window._blueprints.map(function(b, idx) {
        return '<div class="settings-card" style="margin-bottom:12px;cursor:pointer;" onclick="LaRuche.Settings.openBlueprintForm('+idx+')">' +
          '<div style="display:flex;justify-content:space-between;align-items:flex-start;gap:8px">' +
            '<div style="flex:1">' +
              '<div class="settings-card-title">'+LaRuche.Utils.esc(b.title||b.id)+'</div>' +
              '<div style="font-size:12px;color:var(--text-dim);margin-top:4px;">'+LaRuche.Utils.esc(b.description||'')+'</div>' +
            '</div>' +
            '<button onclick="event.stopPropagation();LaRuche.Settings.deleteBlueprint('+idx+')" title="Supprimer ce blueprint perso" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;flex:0 0 auto">Supprimer</button>' +
          '</div>' +
          '<div id="bpForm_'+idx+'" style="display:none;margin-top:12px;padding-top:12px;border-top:1px solid var(--border);" onclick="event.stopPropagation()">' +
            (b.slots||[]).map(function(slot){
              return '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.Utils.esc(slot.label||slot.name)+'</label><input id="bpInput_'+idx+'_'+slot.name+'" class="form-input" placeholder="'+LaRuche.Utils.esc(slot.placeholder||slot.default||'')+'" value="'+LaRuche.Utils.esc(slot.default||'')+'"></div>';
            }).join('') +
            '<button class="settings-save-btn" style="margin-top:8px" onclick="LaRuche.Settings.instanciateBlueprint('+idx+')">Instancier</button>' +
          '</div>' +
        '</div>';
      }).join('');
    el.innerHTML = head + creationSlot + cards;
  }

  // --- Formulaire de creation d'un blueprint perso ---
  function bpSlotRowHtml(){
    return '<div class="bp-slot-row" style="display:flex;gap:6px;margin-bottom:6px;align-items:center">' +
      '<input class="form-input bp-slot-name" placeholder="name" style="flex:1">' +
      '<input class="form-input bp-slot-label" placeholder="label" style="flex:1">' +
      '<input class="form-input bp-slot-default" placeholder="default" style="flex:1">' +
      '<button onclick="this.parentNode.remove()" title="Supprimer cette variable" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:4px 8px;cursor:pointer;font-size:11px;flex:0 0 auto">×</button>' +
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
        '<div class="settings-card-title">Nouveau blueprint</div>' +
        '<div style="margin-top:8px"><label style="font-size:10px;color:var(--text-dim)">Titre</label>' +
          '<input id="bpNewTitle" class="form-input" placeholder="Ex: Veille quotidienne"></div>' +
        '<div style="margin-top:8px"><label style="font-size:10px;color:var(--text-dim)">Prompt (template)</label>' +
          '<textarea id="bpNewPrompt" class="form-input" style="min-height:90px;resize:vertical" placeholder="Utilise {nom} pour referencer une variable..."></textarea></div>' +
        '<div style="margin-top:8px"><label style="font-size:10px;color:var(--text-dim)">Cadence (cron)</label><div id="bpNewCron"></div></div>' +
        '<div style="margin-top:10px"><label style="font-size:10px;color:var(--text-dim)">Variables (slots) — referencees via <code>{name}</code> dans les templates</label>' +
          '<div id="bpSlotsList" style="margin-top:6px"></div>' +
          '<button onclick="LaRuche.Settings.addBlueprintSlotRow()" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:4px 10px;cursor:pointer;font-size:11px;margin-top:2px">+ Variable</button>' +
        '</div>' +
        '<div style="margin-top:12px;display:flex;gap:8px">' +
          '<button class="settings-save-btn" onclick="LaRuche.Settings.saveNewBlueprint()">Créer le blueprint</button>' +
          '<button onclick="LaRuche.Settings.openNewBlueprintForm()" style="background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:6px 12px;cursor:pointer;font-size:12px">Annuler</button>' +
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
    if(!title){ LaRuche.Toast.show('Titre requis','warn'); return; }
    if(!prompt.trim()){ LaRuche.Toast.show('Prompt requis','warn'); return; }
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
          LaRuche.Toast.show('Blueprint créé','ok');
          var el = document.getElementById('autoContent'); if(el) loadBlueprints(el);
        } else {
          LaRuche.Toast.show('Erreur création: '+((res.d&&res.d.error)||'?'),'err');
        }
      }).catch(function(e){ LaRuche.Toast.show('Erreur: '+e,'err'); });
  }

  function deleteBlueprint(idx){
    var b = window._blueprints[idx]; if(!b) return;
    if(!window.confirm('Supprimer le blueprint "'+(b.title||b.id)+'" ? (les blueprints intégrés ne peuvent pas être supprimés)')) return;
    fetch('/api/blueprints/'+encodeURIComponent(b.id), { method:'DELETE' })
      .then(function(r){ return r.json().catch(function(){ return {}; }).then(function(d){ return {ok:r.ok, d:d}; }); })
      .then(function(res){
        if(res.ok && !(res.d && res.d.error)){
          LaRuche.Toast.show('Blueprint supprimé','ok');
          var el = document.getElementById('autoContent'); if(el) loadBlueprints(el);
        } else {
          LaRuche.Toast.show('Suppression refusée: '+((res.d&&res.d.error)||'blueprint intégré ?'),'err');
        }
      }).catch(function(e){ LaRuche.Toast.show('Erreur: '+e,'err'); });
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
        LaRuche.Toast.show('Blueprint instancié avec succès', 'ok');
        document.getElementById('bpForm_'+idx).style.display = 'none';
      } else {
        LaRuche.Toast.show('Erreur d\'instanciation', 'err');
      }
    }).catch(function(e){ LaRuche.Toast.show('Erreur: '+e, 'err'); });
  }

  return { init:init, openBlueprintForm:openBlueprintForm, instanciateBlueprint:instanciateBlueprint, openNewBlueprintForm:openNewBlueprintForm, saveNewBlueprint:saveNewBlueprint, addBlueprintSlotRow:addBlueprintSlotRow, deleteBlueprint:deleteBlueprint, enter:enter, leave:leave, createCron:createCron, deleteCronTask:deleteCronTask, createWatcher:createWatcher, editWatcher:editWatcher, saveWatcherEdit:saveWatcherEdit, updateWatcherEditModelSelect:updateWatcherEditModelSelect, refreshTab:refreshTab,
    loadCron:loadCron, loadWatchers:loadWatchers, loadKanban:loadKanban, loadBlueprints:loadBlueprints, loadCronTimeline:loadCronTimeline, saveChannels:saveChannels, saveContextCfg:saveContextCfg, saveRuntimeCfg:saveRuntimeCfg, toggleCurateur:toggleCurateur, toggleDynamicTools:toggleDynamicTools, saveProviderCfg:saveProviderCfg, addKnowledge:addKnowledge, exportOkf:exportOkf, importOkf:importOkf, deleteKnowledge:deleteKnowledge, editKnowledge:editKnowledge, saveKnowledgeEdit:saveKnowledgeEdit, startChannel:startChannel, stopChannel:stopChannel, showProfileForm:showProfileForm, editProfile:editProfile, deleteProfile:deleteProfile, saveProfile:saveProfile, onProfileProviderChange:onProfileProviderChange, startCodexLogin:startCodexLogin, logoutCodex:logoutCodex, toggleTool:toggleTool, toggleAllTools:toggleAllTools, loadSkills:loadSkills, toggleSkill:toggleSkill, deleteSkill:deleteSkill, newSkill:newSkill, viewSkill:viewSkill, saveSkill:saveSkill, applySkillTools:applySkillTools, toggleSkillTool:toggleSkillTool, filterSkillTools:filterSkillTools, clearSkillTools:clearSkillTools, newPlugin:newPlugin, viewPlugin:viewPlugin, savePlugin:savePlugin, deletePlugin:deletePlugin, createKanbanTask:createKanbanTask, setKanbanDefaultChannel:setKanbanDefaultChannel, loadSecrets: loadSecrets, secretSet: secretSet, secretDelete: secretDelete, loadMcp: loadMcp, loadMcpServers: loadMcpServers, createMcpServer: createMcpServer, deleteMcpServer: deleteMcpServer, updateKanbanModelSelect: updateKanbanModelSelect, updateKanbanEditModelSelect: updateKanbanEditModelSelect, updateWatcherModelSelect: updateWatcherModelSelect, deleteKanbanTask:deleteKanbanTask, editKanbanTask:editKanbanTask, saveKanbanEdit:saveKanbanEdit, toggleKanbanResult:toggleKanbanResult, setKanbanView:setKanbanView, kanbanDragStart:kanbanDragStart, kanbanDragOver:kanbanDragOver, kanbanDrop:kanbanDrop, addCredential:addCredential, deleteCredential:deleteCredential, updateCronModelSelect:updateCronModelSelect, updateCronEditModelSelect:updateCronEditModelSelect, toggleVisibility:toggleVisibility, openAccess:openAccess, tlZoom:tlZoom, tlRecenter:tlRecenter, tlDetail:tlDetail, tlReload:tlReload, tlRun:tlRun, tlEdit:tlEdit, tlSaveEdit:tlSaveEdit, tlToggle:tlToggle };
})();

/* ── CronBuilder : composant "human-friendly" reutilisable (missions + cron) ── */
