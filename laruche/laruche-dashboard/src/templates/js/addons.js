/* ── Addons ───────────────────────────────────────────────────────────────
 * Native catalogue and host for package-provided SPAs. Addon documents are
 * always loaded in a sandbox without `allow-same-origin`: even though their
 * bytes come from LaRuche, their JavaScript receives an opaque origin and
 * cannot inspect the shell, cookies or localStorage.
 */
LaRuche.i18n.add({
  'nav.addons':             {fr:'Addons', en:'Addons'},
  'addons.title':           {fr:'Addons', en:'Addons'},
  'addons.subtitle':        {fr:'Applications locales branchées à LaRuche', en:'Local applications connected to LaRuche'},
  'addons.catalogue':       {fr:'Catalogue', en:'Catalogue'},
  'addons.views':           {fr:'Applications', en:'Applications'},
  'addons.install':         {fr:'Installer', en:'Install'},
  'addons.installing':      {fr:'Installation…', en:'Installing…'},
  'addons.refresh':         {fr:'Actualiser', en:'Refresh'},
  'addons.detach':          {fr:'Détacher', en:'Detach'},
  'addons.installed':       {fr:'Addons installés', en:'Installed addons'},
  'addons.catalogueHint':   {fr:'Active les applications auxquelles tu fais confiance. Leurs vues restent isolées de l’interface et des données de LaRuche.', en:'Enable applications you trust. Their views remain isolated from LaRuche UI and data.'},
  'addons.none':            {fr:'Aucun addon détecté. Installe un package .laruche-addon ou .zip pour commencer.', en:'No addon detected. Install a .laruche-addon or .zip package to get started.'},
  'addons.loading':         {fr:'Chargement des addons…', en:'Loading addons…'},
  'addons.loadFailed':      {fr:'Impossible de charger les addons.', en:'Could not load addons.'},
  'addons.enabled':         {fr:'Activé', en:'Enabled'},
  'addons.disabled':        {fr:'Désactivé', en:'Disabled'},
  'addons.broken':          {fr:'Invalide', en:'Invalid'},
  'addons.enable':          {fr:'Activer', en:'Enable'},
  'addons.disable':         {fr:'Désactiver', en:'Disable'},
  'addons.rescanDone':      {fr:'Catalogue actualisé', en:'Catalogue refreshed'},
  'addons.installDone':     {fr:'Addon installé et laissé désactivé :', en:'Addon installed and left disabled:'},
  'addons.installFailed':   {fr:'L’installation a échoué.', en:'Installation failed.'},
  'addons.installInvalid':  {fr:'Ce fichier n’est pas un package addon valide.', en:'This file is not a valid addon package.'},
  'addons.installTooLarge': {fr:'Le package dépasse la limite de 32 Mio.', en:'The package exceeds the 32 MiB limit.'},
  'addons.installExists':   {fr:'Cette version est déjà installée.', en:'This version is already installed.'},
  'addons.installType':     {fr:'Choisis un fichier .laruche-addon ou .zip.', en:'Choose a .laruche-addon or .zip file.'},
  'addons.changeFailed':    {fr:'La modification a échoué.', en:'The change failed.'},
  'addons.noViews':         {fr:'Aucune application activée.', en:'No enabled application.'},
  'addons.isolated':        {fr:'Vue isolée — aucun accès à LaRuche sans permission', en:'Isolated view — no LaRuche access without permission'},
  'addons.popupBlocked':    {fr:'La fenêtre a été bloquée par le navigateur.', en:'The browser blocked the window.'},
  'addons.detachFailed':    {fr:'La vue détachée n’a pas pu se connecter.', en:'The detached view could not connect.'},
  'addons.detachLimit':     {fr:'Ferme une vue détachée avant d’en ouvrir une autre.', en:'Close a detached view before opening another one.'},
  'addons.diagnostics':     {fr:'Diagnostics de découverte', en:'Discovery diagnostics'},
  'addons.permissions':     {fr:'Permissions', en:'Permissions'},
  'addons.adminOnly':       {fr:'Seul un administrateur peut changer cet état.', en:'Only an administrator can change this state.'}
});

LaRuche.Addons = (function(){
  var catalogue = null;
  var active = null;
  var pendingRoute = null;
  var requestSerial = 0;
  var detachedUrls = [];
  var installing = false;
  var maxPackageBytes = 32 * 1024 * 1024;
  var activeBridge = null;
  var supportedCapabilities = ['storage.private','ui.theme.read','ui.locale.read'];
  var bridgeMaxBytes = 64 * 1024;
  var detachedBridges = new Set();
  var detachedSweep = null;
  var maxDetachedViews = 8;
  var lifecycleBound = false;
  var pendingDetached = 0;

  function t(key, vars){ return LaRuche.i18n.t(key, vars); }
  function esc(value){ return LaRuche.Utils.esc(value == null ? '' : value); }
  function initial(value){ return String(value || '?').trim().charAt(0) || '?'; }
  function isAdmin(){ return !!(LaRuche.Auth && LaRuche.Auth.isAdmin && LaRuche.Auth.isAdmin()); }

  function bridgeFailure(code,message,retryable){
    var error=new Error(message); error.code=code; error.retryable=!!retryable; return error;
  }

  function randomToken(){
    var bytes=new Uint8Array(24); crypto.getRandomValues(bytes);
    return Array.prototype.map.call(bytes,function(value){ return value.toString(16).padStart(2,'0'); }).join('');
  }

  function messageIsBounded(value,depth){
    if(depth>20) return false;
    if(value===null || typeof value==='string' || typeof value==='boolean' || (typeof value==='number' && isFinite(value))) return true;
    if(Array.isArray(value)) return value.length<=256 && value.every(function(item){ return messageIsBounded(item,depth+1); });
    if(Object.prototype.toString.call(value)!=='[object Object]') return false;
    var keys=Object.keys(value);
    return keys.length<=128 && keys.every(function(key){ return key.length<=128 && messageIsBounded(value[key],depth+1); });
  }

  function bridgeMessageIsSafe(message){
    if(!messageIsBounded(message,0)) return false;
    try{ return JSON.stringify(message).length<=bridgeMaxBytes; }catch(error){ return false; }
  }

  function manifestPermissions(addon,kind){
    var permissions=addon&&addon.manifest&&addon.manifest.permissions;
    return permissions&&Array.isArray(permissions[kind])?permissions[kind].slice():[];
  }

  function bridgeCapabilities(addon){
    // Until the durable grants ledger lands, enabling an addon is the explicit
    // grant for the small, low-risk v1 host set. Optional permissions remain
    // denied and every future backend capability must be checked again in Rust.
    return manifestPermissions(addon,'required').filter(function(permission){ return supportedCapabilities.indexOf(permission)!==-1; });
  }

  function bridgeIsLive(bridge){
    return activeBridge===bridge || detachedBridges.has(bridge);
  }

  function stopDetachedSweep(){
    if(detachedSweep && !detachedBridges.size){ clearInterval(detachedSweep); detachedSweep=null; }
  }

  function revokeBridge(target,closePopup){
    var bridge=target||activeBridge;
    if(!bridge) return;
    if(activeBridge===bridge) activeBridge=null;
    detachedBridges.delete(bridge);
    clearTimeout(bridge.timer);
    try{ bridge.port.onmessage=null; bridge.port.close(); }catch(error){}
    if(closePopup!==false && bridge.popup && !bridge.popup.closed){ try{ bridge.popup.close(); }catch(error){} }
    stopDetachedSweep();
  }

  function revokeDetachedBridges(){
    Array.from(detachedBridges).forEach(function(bridge){ revokeBridge(bridge,true); });
  }

  function ensureDetachedSweep(){
    if(detachedSweep) return;
    detachedSweep=setInterval(function(){
      Array.from(detachedBridges).forEach(function(bridge){
        if(!bridge.popup || bridge.popup.closed) revokeBridge(bridge,false);
      });
    },1000);
  }

  function reconcileDetachedBridges(data){
    var records=data&&Array.isArray(data.addons)?data.addons:[];
    Array.from(detachedBridges).forEach(function(bridge){
      var current=records.find(function(addon){ return addon.id===bridge.addon.id; });
      if(!current || !current.enabled || current.activeVersion!==bridge.addon.activeVersion) revokeBridge(bridge,true);
    });
  }

  function bridgeReply(bridge,id,ok,result,error){
    if(!bridgeIsLive(bridge)) return;
    var response={v:1,kind:'response',id:id,ok:ok};
    if(ok) response.result=result==null?{}:result;
    else response.error={code:error.code||'internal_error',message:error.message||'Host request failed',retryable:!!error.retryable};
    bridge.port.postMessage(response);
  }

  function requireCapability(bridge,capability){
    if(bridge.capabilities.indexOf(capability)===-1) throw bridgeFailure('permission_denied','Capability not granted');
  }

  function paintBridgeTitle(bridge){
    var display=bridge.title+(bridge.dirty?' •':'');
    if(bridge.titleNode) bridge.titleNode.textContent=display;
    if(bridge.popup && !bridge.popup.closed){
      try{ bridge.popup.document.title=display; }catch(error){}
    }
  }

  function backendStorage(bridge,method,params){
    requireCapability(bridge,'storage.private');
    var operation=method.slice('storage.'.length);
    if(['get','set','delete','list'].indexOf(operation)===-1) throw bridgeFailure('not_found','Unknown storage method');
    var body={op:operation};
    if(operation==='get' || operation==='set' || operation==='delete') body.key=params&&params.key;
    if(operation==='set') body.value=params&&params.value;
    if(operation==='list') body.prefix=params&&Object.prototype.hasOwnProperty.call(params,'prefix')?params.prefix:'';
    return fetch(LaRuche.API.base+'/api/addons/'+encodeURIComponent(bridge.addon.id)+'/storage',{
      method:'POST', credentials:'include', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
    }).then(function(response){
      return response.json().catch(function(){ return null; }).then(function(payload){
        if(!response.ok){
          var error=payload&&payload.error;
          var fallback=response.status===401?'authentication_required':response.status===403?'permission_denied':response.status===404?'addon_not_found':response.status===409?'addon_disabled':response.status===413?'quota_exceeded':(response.status===400||response.status===422)?'validation_failed':'storage_unavailable';
          throw bridgeFailure(error&&error.code||fallback,error&&error.message||'Private storage request failed',response.status>=500);
        }
        return payload||{};
      });
    }).catch(function(error){
      if(error&&error.code) throw error;
      throw bridgeFailure('storage_unavailable','Private storage unavailable',true);
    });
  }

  function handleBridgeRequest(bridge,method,params){
    if(method.indexOf('storage.')===0){
      return backendStorage(bridge,method,params);
    }
    if(method==='ui.setTitle'){
      var title=params&&params.title;
      if(typeof title!=='string' || !title.trim() || title.length>80 || /[\u0000-\u001f\u007f]/.test(title)) throw bridgeFailure('validation_failed','Title is invalid');
      bridge.title=title.trim(); paintBridgeTitle(bridge); return {};
    }
    if(method==='ui.setDirty'){
      bridge.dirty=!!(params&&params.dirty); paintBridgeTitle(bridge); return {};
    }
    if(method==='ui.requestDetach'){
      if(bridge.view.detachable===false) throw bridgeFailure('permission_denied','This view cannot be detached');
      if(bridge.popup) throw bridgeFailure('conflict','View is already detached');
      detachActive(); return {};
    }
    if(method==='ui.close'){
      if(bridge.popup){ revokeBridge(bridge,true); return {}; }
      LaRuche.Router.go('addons/overview'); return {};
    }
    throw bridgeFailure('not_found','Unknown addon bridge method');
  }

  function onBridgeMessage(bridge,message){
    if(!bridgeIsLive(bridge) || !bridgeMessageIsSafe(message) || message.v!==1) return;
    if(!bridge.hello){
      if(message.kind!=='addon.hello' || message.nonce!==bridge.nonce || message.addonId!==bridge.addon.id || message.viewId!==bridge.view.id || message.apiVersion!==1){ revokeBridge(bridge,true); return; }
      bridge.hello=true;
      var context={
        sessionId:bridge.sessionId,
        addonId:bridge.addon.id,
        addonVersion:bridge.addon.activeVersion,
        viewId:bridge.view.id,
        grantedCapabilities:bridge.capabilities.slice(),
        locale:bridge.capabilities.indexOf('ui.locale.read')!==-1?(window.__LANG__||'fr'):null,
        theme:bridge.capabilities.indexOf('ui.theme.read')!==-1?(document.documentElement.getAttribute('data-theme')||'default'):null,
        limits:{messageBytes:bridgeMaxBytes,concurrentRequests:8,requestsPerMinute:120,storageBytes:1024*1024}
      };
      bridge.port.postMessage({v:1,kind:'host.welcome',context:context});
      return;
    }
    if(!bridge.ready){
      if(message.kind==='addon.ready' && message.sessionId===bridge.sessionId){ bridge.ready=true; clearTimeout(bridge.timer); }
      return;
    }
    if(message.kind!=='request' || typeof message.id!=='string' || message.id.length>80 || !/^[A-Za-z0-9._:-]+$/.test(message.id) || typeof message.method!=='string' || message.method.length>100) return;
    if(bridge.pending.has(message.id)){ bridgeReply(bridge,message.id,false,null,bridgeFailure('conflict','Request id is already active')); return; }
    var now=Date.now(); bridge.requests=bridge.requests.filter(function(time){ return now-time<60000; });
    if(bridge.requests.length>=120){ bridgeReply(bridge,message.id,false,null,bridgeFailure('rate_limited','Bridge rate limit exceeded',true)); return; }
    if(bridge.pending.size>=8){ bridgeReply(bridge,message.id,false,null,bridgeFailure('rate_limited','Too many concurrent requests',true)); return; }
    bridge.requests.push(now); bridge.pending.add(message.id);
    Promise.resolve().then(function(){ return handleBridgeRequest(bridge,message.method,message.params||{}); })
      .then(function(result){ bridgeReply(bridge,message.id,true,result); })
      .catch(function(error){ bridgeReply(bridge,message.id,false,null,error); })
      .finally(function(){ bridge.pending.delete(message.id); });
  }

  function startBridge(addon,view,title,titleNode,popup,deliver){
    if(!window.MessageChannel || !window.crypto || !crypto.getRandomValues) return null;
    var channel=new MessageChannel();
    var bridge={addon:addon,view:view,titleNode:titleNode||null,title:title,dirty:false,popup:popup||null,port:channel.port1,nonce:randomToken(),sessionId:randomToken(),capabilities:bridgeCapabilities(addon),hello:false,ready:false,pending:new Set(),requests:[],timer:null};
    if(popup) detachedBridges.add(bridge);
    else { revokeBridge(); activeBridge=bridge; }
    bridge.port.onmessage=function(event){ onBridgeMessage(bridge,event.data); };
    bridge.port.onmessageerror=function(){ revokeBridge(bridge,true); };
    bridge.port.start();
    bridge.timer=setTimeout(function(){ if(bridgeIsLive(bridge) && !bridge.ready) revokeBridge(bridge,true); },5000);
    try{
      deliver({v:1,kind:'laruche.host.init',nonce:bridge.nonce,addonId:addon.id,viewId:view.id},channel.port2);
    }catch(error){
      revokeBridge(bridge,true);
      return null;
    }
    return bridge;
  }

  function connectBridge(frame,addon,view,titleNode){
    frame.addEventListener('load',function(){
      if(!frame.contentWindow) return;
      startBridge(addon,view,titleNode.textContent,titleNode,null,function(init,port){
        frame.contentWindow.postMessage(init,'*',[port]);
      });
    });
  }

  function navIcon(){
    return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><path d="M17.5 14v7M14 17.5h7"/></svg>';
  }

  function ensureNavigation(){
    document.querySelectorAll('.header-nav, .mobile-tabs').forEach(function(nav){
      if(nav.querySelector('a[data-page="addons"]')) return;
      var link = document.createElement('a');
      link.href = '#addons/overview';
      link.dataset.page = 'addons';
      link.innerHTML = '<span class="tab-icon">'+navIcon()+'</span><span class="tab-text">'+esc(t('nav.addons'))+'</span>';
      var settings = nav.querySelector('a[data-page="settings"]');
      nav.insertBefore(link, settings || null);
    });
    if(LaRuche.Header && LaRuche.Header.fitNav) LaRuche.Header.fitNav();
  }

  function init(){
    ensureNavigation();
    if(!lifecycleBound){
      lifecycleBound=true;
      window.addEventListener('beforeunload',revokeDetachedBridges);
    }
    var install = document.getElementById('addonsInstall');
    var packageInput = document.getElementById('addonsPackageInput');
    var refresh = document.getElementById('addonsRefresh');
    var detach = document.getElementById('addonsDetach');
    if(refresh){
      refresh.textContent=t('addons.refresh');
      refresh.onclick=function(){ if(isAdmin()) rescan(); else load(false); };
    }
    if(install){
      install.textContent=t('addons.install');
      install.onclick=function(){
        if(!isAdmin()) { LaRuche.Toast.show(t('addons.adminOnly'),'err'); return; }
        if(!installing && packageInput) packageInput.click();
      };
    }
    if(packageInput){ packageInput.onchange=function(){ installPackage(packageInput.files && packageInput.files[0]); }; }
    if(detach){ detach.textContent=t('addons.detach'); detach.onclick=function(){ detachActive(); }; }
    syncInstallButton();
    renderLoading();
    load(false);
  }

  function enter(){
    ensureNavigation();
    syncInstallButton();
    if(!catalogue) load(false);
    else applyRoute(pendingRoute || 'overview');
  }

  function deepLink(route){
    pendingRoute = route || 'overview';
    if(catalogue) applyRoute(pendingRoute);
  }

  function load(forceRescan){
    var serial = ++requestSerial;
    var button = document.getElementById('addonsRefresh');
    if(button) button.disabled = true;
    var url = LaRuche.API.base + '/api/addons' + (forceRescan ? '/rescan' : '');
    fetch(url, {method:forceRescan?'POST':'GET', credentials:'include'})
      .then(function(response){ if(!response.ok) throw new Error('HTTP '+response.status); return response.json(); })
      .then(function(data){
        if(serial !== requestSerial) return;
        reconcileDetachedBridges(data);
        catalogue = data || {addons:[], diagnostics:[]};
        renderRail();
        applyRoute(pendingRoute || 'overview');
        if(forceRescan) LaRuche.Toast.show(t('addons.rescanDone'),'ok');
      })
      .catch(function(){
        if(serial !== requestSerial) return;
        catalogue = null;
        renderFailure();
      })
      .finally(function(){ if(serial===requestSerial && button) button.disabled=false; });
  }

  function rescan(){ load(true); }

  function syncInstallButton(){
    var button=document.getElementById('addonsInstall');
    if(!button) return;
    button.disabled=installing || !isAdmin();
    button.textContent=installing?t('addons.installing'):t('addons.install');
    button.title=!isAdmin()?t('addons.adminOnly'):'';
  }

  function installErrorMessage(payload){
    var error=payload && payload.error;
    var code=error && error.code;
    if(code==='addon_package_too_large') return t('addons.installTooLarge');
    if(code==='addon_version_exists') return t('addons.installExists');
    if(code==='addon_package_invalid') return t('addons.installInvalid');
    if(code==='admin_required') return t('addons.adminOnly');
    return (error && error.message) || t('addons.installFailed');
  }

  function installPackage(file){
    var input=document.getElementById('addonsPackageInput');
    if(input) input.value='';
    if(!file || !isAdmin() || installing) return;
    var name=String(file.name||'').toLowerCase();
    if(!name.endsWith('.zip') && !name.endsWith('.laruche-addon')){
      LaRuche.Toast.show(t('addons.installType'),'err'); return;
    }
    if(file.size>maxPackageBytes){ LaRuche.Toast.show(t('addons.installTooLarge'),'err'); return; }
    installing=true; syncInstallButton();
    fetch(LaRuche.API.base+'/api/addons/install',{
      method:'POST', credentials:'include', headers:{'Content-Type':'application/zip'}, body:file
    })
      .then(function(response){
        return response.json().catch(function(){ return null; }).then(function(payload){
          if(!response.ok) throw new Error(installErrorMessage(payload));
          return payload;
        });
      })
      .then(function(installed){
        installed=installed||{};
        var manifest=(installed&&installed.manifest)||{};
        LaRuche.Toast.show(t('addons.installDone')+' '+(manifest.name||installed.id)+' '+(installed.activeVersion||''),'ok');
        pendingRoute='overview';
        load(false);
      })
      .catch(function(error){ LaRuche.Toast.show(error.message||t('addons.installFailed'),'err'); })
      .finally(function(){ installing=false; syncInstallButton(); });
  }

  function addons(){ return (catalogue && Array.isArray(catalogue.addons)) ? catalogue.addons : []; }
  function enabledViews(){
    var result=[];
    addons().forEach(function(addon){
      var views = addon.enabled && addon.manifest && addon.manifest.ui && Array.isArray(addon.manifest.ui.views)
        ? addon.manifest.ui.views : [];
      views.forEach(function(view){
        if(view.navigation !== false) result.push({addon:addon, view:view});
      });
    });
    return result;
  }

  function renderLoading(){
    var stage=document.getElementById('addonsStage');
    if(stage) stage.innerHTML='<div class="addons-loading">'+esc(t('addons.loading'))+'</div>';
  }
  function renderFailure(){
    var rail=document.getElementById('addonsRail');
    var stage=document.getElementById('addonsStage');
    if(rail) rail.innerHTML='';
    if(stage) stage.innerHTML='<div class="addons-empty">'+esc(t('addons.loadFailed'))+'</div>';
    setDetach(false);
  }

  function renderRail(){
    var rail=document.getElementById('addonsRail'); if(!rail) return;
    rail.innerHTML='';
    rail.appendChild(navButton(t('addons.catalogue'), t('addons.installed'), '⌂', 'overview', function(){ LaRuche.Router.go('addons/overview'); }));
    var views=enabledViews();
    if(views.length){
      var title=document.createElement('div'); title.className='addons-rail-title'; title.textContent=t('addons.views'); rail.appendChild(title);
      views.forEach(function(item){
        var route=encodeURIComponent(item.addon.id)+'/'+encodeURIComponent(item.view.id);
        rail.appendChild(navButton(item.view.title, item.addon.manifest.name, initial(item.addon.manifest.name), route, function(){ LaRuche.Router.go('addons/'+route); }));
      });
    }
    paintRail();
  }

  function navButton(name, addonName, icon, route, onclick){
    var button=document.createElement('button');
    button.type='button'; button.className='addons-nav'; button.dataset.route=route;
    button.innerHTML='<span class="addons-nav-icon">'+esc(icon)+'</span><span class="addons-nav-copy"><span class="addons-nav-name">'+esc(name)+'</span><span class="addons-nav-addon">'+esc(addonName)+'</span></span>';
    button.onclick=onclick;
    return button;
  }

  function paintRail(){
    var route=active ? encodeURIComponent(active.addon.id)+'/'+encodeURIComponent(active.view.id) : 'overview';
    document.querySelectorAll('#addonsRail .addons-nav').forEach(function(button){ button.classList.toggle('active',button.dataset.route===route); });
  }

  function applyRoute(route){
    if(!catalogue) return;
    if(!route || route==='overview'){ showOverview(); return; }
    var parts=String(route).split('/');
    if(parts.length!==2){ showOverview(); return; }
    var addonId, viewId;
    try{ addonId=decodeURIComponent(parts[0]); viewId=decodeURIComponent(parts[1]); }catch(e){ showOverview(); return; }
    var found=null;
    enabledViews().some(function(item){ if(item.addon.id===addonId && item.view.id===viewId){ found=item; return true; } return false; });
    if(!found){ showOverview(); return; }
    showView(found.addon, found.view);
  }

  function showOverview(){
    revokeBridge(); active=null; setDetach(false); paintRail();
    var stage=document.getElementById('addonsStage'); if(!stage) return;
    var list=addons();
    var html='<div class="addons-catalogue"><div class="addons-catalogue-intro"><h2>'+esc(t('addons.installed'))+'</h2><p>'+esc(t('addons.catalogueHint'))+'</p></div>';
    if(!list.length) html+='<div class="addons-empty">'+esc(t('addons.none'))+'</div>';
    else {
      html+='<div class="addons-grid">';
      list.forEach(function(addon, index){
        var manifest=addon.manifest||{};
        var name=manifest.name||addon.id;
        var state=addon.error?'broken':(addon.enabled?'enabled':'disabled');
        var label=addon.error?t('addons.broken'):(addon.enabled?t('addons.enabled'):t('addons.disabled'));
        var action=addon.enabled?t('addons.disable'):t('addons.enable');
        var disabled=!!addon.error || !isAdmin();
        var permissions=manifestPermissions(addon,'required').concat(manifestPermissions(addon,'optional'));
        html+='<article class="addons-card"><div class="addons-card-top"><div class="addons-card-icon">'+esc(initial(name))+'</div><div class="addons-card-copy"><div class="addons-card-name">'+esc(name)+'</div><div class="addons-card-meta">'+esc(addon.id)+' · '+esc(addon.activeVersion||'')+'</div></div></div>'+
          '<div class="addons-card-desc">'+esc(manifest.description||'')+'</div>'+
          (permissions.length?'<div class="addons-card-permissions"><strong>'+esc(t('addons.permissions'))+'</strong> '+permissions.map(esc).join(' · ')+'</div>':'')+
          '<div class="addons-card-foot"><span class="addons-state '+state+'"><span class="addons-state-dot"></span>'+esc(label)+'</span>'+
          '<button type="button" class="addons-btn" data-addon-toggle="'+index+'" '+(disabled?'disabled title="'+esc(addon.error||t('addons.adminOnly'))+'"':'')+'>'+esc(action)+'</button></div>'+
          (addon.error?'<div class="addons-error">'+esc(addon.error)+'</div>':'')+'</article>';
      });
      html+='</div>';
    }
    var diagnostics=(catalogue&&catalogue.diagnostics)||[];
    if(diagnostics.length){
      html+='<div class="addons-diagnostics"><strong>'+esc(t('addons.diagnostics'))+'</strong>';
      diagnostics.forEach(function(item){ html+='<div>'+esc(item.path)+': '+esc(item.error)+'</div>'; });
      html+='</div>';
    }
    stage.innerHTML=html+'</div>';
    stage.querySelectorAll('[data-addon-toggle]').forEach(function(button){ button.onclick=function(){ toggleAddon(parseInt(button.dataset.addonToggle,10)); }; });
  }

  function toggleAddon(index){
    var addon=addons()[index]; if(!addon || addon.error || !isAdmin()) return;
    var action=addon.enabled?'disable':'enable';
    fetch(LaRuche.API.base+'/api/addons/'+encodeURIComponent(addon.id)+'/'+action,{method:'POST',credentials:'include'})
      .then(function(response){ if(!response.ok) throw new Error(); return response.json(); })
      .then(function(updated){
        catalogue.addons[index]=updated;
        if(!updated.enabled) reconcileDetachedBridges(catalogue);
        renderRail(); showOverview();
        LaRuche.Toast.show(updated.enabled?t('addons.enabled'):t('addons.disabled'),'ok');
      })
      .catch(function(){ LaRuche.Toast.show(t('addons.changeFailed'),'err'); });
  }

  function assetUrl(addon, view){
    var entry=String(view.entry||'');
    if(entry.indexOf('ui/')!==0) return null;
    var relative=entry.slice(3).split('/').map(encodeURIComponent).join('/');
    if(!relative) return null;
    return '/addons-assets/'+encodeURIComponent(addon.id)+'/'+encodeURIComponent(addon.activeVersion)+'/'+relative;
  }

  function showView(addon, view){
    var url=assetUrl(addon,view); if(!url){ showOverview(); return; }
    revokeBridge();
    active={addon:addon,view:view,url:url}; setDetach(view.detachable!==false); paintRail();
    var stage=document.getElementById('addonsStage'); if(!stage) return;
    stage.innerHTML='';
    var wrap=document.createElement('div'); wrap.className='addons-frame-wrap';
    var bar=document.createElement('div'); bar.className='addons-frame-bar';
    var title=document.createElement('div'); title.className='addons-frame-title'; title.textContent=(addon.manifest.name||addon.id)+' — '+view.title;
    var note=document.createElement('div'); note.className='addons-frame-note'; note.textContent=t('addons.isolated');
    var frame=document.createElement('iframe');
    frame.className='addons-frame'; frame.title=view.title; frame.src=url;
    frame.setAttribute('sandbox','allow-scripts allow-forms allow-downloads');
    frame.setAttribute('referrerpolicy','no-referrer');
    frame.setAttribute('allow',"clipboard-read 'none'; clipboard-write 'none'; camera 'none'; microphone 'none'; geolocation 'none'");
    connectBridge(frame,addon,view,title);
    bar.appendChild(title); bar.appendChild(note); wrap.appendChild(bar); wrap.appendChild(frame); stage.appendChild(wrap);
  }

  function setDetach(visible){
    var button=document.getElementById('addonsDetach');
    if(button) button.hidden=!visible;
  }

  function forgetDetachedUrl(url){
    try{ URL.revokeObjectURL(url); }catch(error){}
    detachedUrls=detachedUrls.filter(function(item){ return item!==url; });
  }

  function detachedHost(title,src,token){
    var script="(function(){'use strict';var token='"+token+"',upstream=window.opener,frame=null,init=null,port=null;function forward(){if(!frame||!init||!port)return;frame.contentWindow.postMessage(init,'*',[port]);init=null;port=null;window.removeEventListener('message',receive);try{window.opener=null;}catch(error){}upstream=null;}function receive(event){var message=event.data;if(event.source!==upstream||!message||message.v!==1||message.kind!=='laruche.detached.connect'||message.token!==token||!message.init||event.ports.length!==1)return;init=message.init;port=event.ports[0];forward();}window.__larucheAddonFrameReady=function(target){frame=target;forward();};window.addEventListener('message',receive);if(upstream)upstream.postMessage({v:1,kind:'laruche.detached.ready',token:token},'*');else window.close();})();";
    return '<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src \'none\'; frame-src \'self\'; script-src \'unsafe-inline\'; style-src \'unsafe-inline\'"><title>'+title+'</title><style>html,body,iframe{width:100%;height:100%;margin:0;border:0;background:#101014}body{overflow:hidden}</style><script>'+script+'</script></head><body><iframe title="'+title+'" src="'+src+'" sandbox="allow-scripts allow-forms allow-downloads" referrerpolicy="no-referrer" allow="clipboard-read \'none\'; clipboard-write \'none\'; camera \'none\'; microphone \'none\'; geolocation \'none\'" onload="window.__larucheAddonFrameReady(this)"></iframe></body></html>';
  }

  function detachActive(){
    if(!active || active.view.detachable===false) return;
    if(detachedBridges.size+pendingDetached>=maxDetachedViews){ LaRuche.Toast.show(t('addons.detachLimit'),'err'); return; }
    var item=active;
    var title=esc((item.addon.manifest.name||item.addon.id)+' — '+item.view.title);
    var src=esc(item.url);
    var token=randomToken();
    var host=detachedHost(title,src,token);
    var url=URL.createObjectURL(new Blob([host],{type:'text/html'}));
    detachedUrls.push(url);
    pendingDetached+=1;
    var popup=null;
    var finished=false;
    var timer=null;
    function cleanup(revokeUrl){
      window.removeEventListener('message',onReady);
      clearTimeout(timer);
      pendingDetached=Math.max(0,pendingDetached-1);
      if(revokeUrl) forgetDetachedUrl(url);
    }
    function fail(){
      if(finished) return; finished=true; cleanup(true);
      if(popup && !popup.closed) try{ popup.close(); }catch(error){}
      LaRuche.Toast.show(t('addons.detachFailed'),'err');
    }
    function onReady(event){
      var message=event.data;
      if(finished || !popup || event.source!==popup || !message || message.v!==1 || message.kind!=='laruche.detached.ready' || message.token!==token) return;
      finished=true; cleanup(false);
      setTimeout(function(){ forgetDetachedUrl(url); },60000);
      var detached=startBridge(item.addon,item.view,(item.addon.manifest.name||item.addon.id)+' — '+item.view.title,null,popup,function(init,port){
        popup.postMessage({v:1,kind:'laruche.detached.connect',token:token,init:init},'*',[port]);
      });
      if(!detached){ try{ popup.close(); }catch(error){} LaRuche.Toast.show(t('addons.detachFailed'),'err'); return; }
      ensureDetachedSweep();
    }
    window.addEventListener('message',onReady);
    popup=window.open(url,'_blank','popup,width=1100,height=760');
    if(!popup){ finished=true; cleanup(true); LaRuche.Toast.show(t('addons.popupBlocked'),'err'); return; }
    timer=setTimeout(fail,5000);
  }

  function leave(){ revokeBridge(); }
  return {init:init,enter:enter,leave:leave,deepLink:deepLink,rescan:rescan,detach:detachActive};
})();

LaRuche.Router.register('addons', LaRuche.Addons);
