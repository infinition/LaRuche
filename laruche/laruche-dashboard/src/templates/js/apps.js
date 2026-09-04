/* ── Apps ───────────────────────────────────────────────────────────────
 * Native catalogue and host for package-provided SPAs. App documents are
 * always loaded in a sandbox without `allow-same-origin`: even though their
 * bytes come from LaRuche, their JavaScript receives an opaque origin and
 * cannot inspect the shell, cookies or localStorage.
 */
LaRuche.i18n.add({
  'nav.apps':             {fr:'Apps', en:'Apps'},
  'apps.title':           {fr:'Apps', en:'Apps'},
  'apps.subtitle':        {fr:'Applications locales branchées à LaRuche', en:'Local applications connected to LaRuche'},
  'apps.catalogue':       {fr:'Catalogue', en:'Catalogue'},
  'apps.views':           {fr:'Applications', en:'Applications'},
  'apps.install':         {fr:'Installer', en:'Install'},
  'apps.installing':      {fr:'Installation…', en:'Installing…'},
  'apps.refresh':         {fr:'Actualiser', en:'Refresh'},
  'apps.detach':          {fr:'Détacher', en:'Detach'},
  'apps.installed':       {fr:'Apps installées', en:'Installed apps'},
  'apps.catalogueHint':   {fr:'Active les applications auxquelles tu fais confiance. Leurs vues restent isolées de l’interface et des données de LaRuche.', en:'Enable applications you trust. Their views remain isolated from LaRuche UI and data.'},
  'apps.none':            {fr:'Aucune app détectée. Installe un package .laruche-app ou .zip pour commencer.', en:'No app detected. Install a .laruche-app or .zip package to get started.'},
  'apps.loading':         {fr:'Chargement des apps…', en:'Loading apps…'},
  'apps.loadFailed':      {fr:'Impossible de charger les apps.', en:'Could not load apps.'},
  'apps.enabled':         {fr:'Activé', en:'Enabled'},
  'apps.disabled':        {fr:'Désactivé', en:'Disabled'},
  'apps.broken':          {fr:'Invalide', en:'Invalid'},
  'apps.enable':          {fr:'Activer', en:'Enable'},
  'apps.disable':         {fr:'Désactiver', en:'Disable'},
  'apps.rescanDone':      {fr:'Catalogue actualisé', en:'Catalogue refreshed'},
  'apps.installDone':     {fr:'App installée et laissée désactivée :', en:'App installed and left disabled:'},
  'apps.installFailed':   {fr:'L’installation a échoué.', en:'Installation failed.'},
  'apps.installInvalid':  {fr:'Ce fichier n’est pas un package app valide.', en:'This file is not a valid app package.'},
  'apps.installTooLarge': {fr:'Le package dépasse la limite de 32 Mio.', en:'The package exceeds the 32 MiB limit.'},
  'apps.installExists':   {fr:'Cette version est déjà installée.', en:'This version is already installed.'},
  'apps.installType':     {fr:'Choisis un fichier .laruche-app ou .zip.', en:'Choose a .laruche-app or .zip file.'},
  'apps.changeFailed':    {fr:'La modification a échoué.', en:'The change failed.'},
  'apps.noViews':         {fr:'Aucune application activée.', en:'No enabled application.'},
  'apps.isolated':        {fr:'Vue isolée, aucun accès à LaRuche sans permission', en:'Isolated view, no LaRuche access without permission'},
  'apps.popupBlocked':    {fr:'La fenêtre a été bloquée par le navigateur.', en:'The browser blocked the window.'},
  'apps.detachFailed':    {fr:'La vue détachée n’a pas pu se connecter.', en:'The detached view could not connect.'},
  'apps.detachLimit':     {fr:'Ferme une vue détachée avant d’en ouvrir une autre.', en:'Close a detached view before opening another one.'},
  'apps.diagnostics':     {fr:'Diagnostics de découverte', en:'Discovery diagnostics'},
  'apps.permissions':     {fr:'Permissions', en:'Permissions'},
  'apps.required':        {fr:'requise', en:'required'},
  'apps.optional':        {fr:'optionnelle', en:'optional'},
  'apps.unavailable':     {fr:'indisponible', en:'unavailable'},
  'apps.adminOnly':       {fr:'Seul un administrateur peut changer cet état.', en:'Only an administrator can change this state.'}
});

LaRuche.Apps = (function(){
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

  function packageAssetUrl(app,path){
    var entry=String(path||'');
    if(!app || !app.activeVersion || entry.indexOf('ui/')!==0) return null;
    var relative=entry.slice(3).split('/').map(encodeURIComponent).join('/');
    if(!relative) return null;
    return '/apps-assets/'+encodeURIComponent(app.id)+'/'+encodeURIComponent(app.activeVersion)+'/'+relative;
  }

  function appIconMarkup(app,view,fallback){
    var manifest=(app&&app.manifest)||{};
    var source=packageAssetUrl(app,(view&&view.icon)||manifest.icon);
    if(!app || !app.enabled || !source) return esc(fallback);
    return '<img class="apps-package-icon" src="'+esc(source)+'" alt="">';
  }

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

  function manifestPermissions(app,kind){
    var permissions=app&&app.manifest&&app.manifest.permissions;
    return permissions&&Array.isArray(permissions[kind])?permissions[kind].slice():[];
  }

  function permissionMarkup(app,index){
    var granted=app&&Array.isArray(app.grantedPermissions)?app.grantedPermissions:[];
    var rows=[];
    manifestPermissions(app,'required').forEach(function(permission){
      var available=supportedCapabilities.indexOf(permission)!==-1;
      rows.push('<div class="apps-permission '+(available?'':'unavailable')+'"><span>'+(granted.indexOf(permission)!==-1?'✓':'●')+'</span><code>'+esc(permission)+'</code><small>'+esc(available?t('apps.required'):t('apps.unavailable'))+'</small></div>');
    });
    manifestPermissions(app,'optional').forEach(function(permission){
      var available=supportedCapabilities.indexOf(permission)!==-1;
      var checked=granted.indexOf(permission)!==-1;
      rows.push('<label class="apps-permission '+(available?'':'unavailable')+'"><input type="checkbox" data-app-permission="'+index+'" value="'+esc(permission)+'" '+(checked?'checked ':'')+((app.enabled||!available)?'disabled ':'')+'><code>'+esc(permission)+'</code><small>'+esc(available?t('apps.optional'):t('apps.unavailable'))+'</small></label>');
    });
    return rows.join('');
  }

  function bridgeCapabilities(app){
    var granted=app&&Array.isArray(app.grantedPermissions)?app.grantedPermissions:[];
    return granted.filter(function(permission){ return supportedCapabilities.indexOf(permission)!==-1; });
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
    var records=data&&Array.isArray(data.apps)?data.apps:[];
    Array.from(detachedBridges).forEach(function(bridge){
      var current=records.find(function(app){ return app.id===bridge.app.id; });
      if(!current || !current.enabled || current.activeVersion!==bridge.app.activeVersion) revokeBridge(bridge,true);
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
    return fetch(LaRuche.API.base+'/api/apps/'+encodeURIComponent(bridge.app.id)+'/storage',{
      method:'POST', credentials:'include', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
    }).then(function(response){
      return response.json().catch(function(){ return null; }).then(function(payload){
        if(!response.ok){
          var error=payload&&payload.error;
          var fallback=response.status===401?'authentication_required':response.status===403?'permission_denied':response.status===404?'app_not_found':response.status===409?'app_disabled':response.status===413?'quota_exceeded':(response.status===400||response.status===422)?'validation_failed':'storage_unavailable';
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
      LaRuche.Router.go('apps/overview'); return {};
    }
    throw bridgeFailure('not_found','Unknown app bridge method');
  }

  function onBridgeMessage(bridge,message){
    if(!bridgeIsLive(bridge) || !bridgeMessageIsSafe(message) || message.v!==1) return;
    if(!bridge.hello){
      if(message.kind!=='app.hello' || message.nonce!==bridge.nonce || message.appId!==bridge.app.id || message.viewId!==bridge.view.id || message.apiVersion!==1){ revokeBridge(bridge,true); return; }
      bridge.hello=true;
      var context={
        sessionId:bridge.sessionId,
        appId:bridge.app.id,
        appVersion:bridge.app.activeVersion,
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
      if(message.kind==='app.ready' && message.sessionId===bridge.sessionId){ bridge.ready=true; clearTimeout(bridge.timer); }
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

  function startBridge(app,view,title,titleNode,popup,deliver){
    if(!window.MessageChannel || !window.crypto || !crypto.getRandomValues) return null;
    var channel=new MessageChannel();
    var bridge={app:app,view:view,titleNode:titleNode||null,title:title,dirty:false,popup:popup||null,port:channel.port1,nonce:randomToken(),sessionId:randomToken(),capabilities:bridgeCapabilities(app),hello:false,ready:false,pending:new Set(),requests:[],timer:null};
    if(popup) detachedBridges.add(bridge);
    else { revokeBridge(); activeBridge=bridge; }
    bridge.port.onmessage=function(event){ onBridgeMessage(bridge,event.data); };
    bridge.port.onmessageerror=function(){ revokeBridge(bridge,true); };
    bridge.port.start();
    bridge.timer=setTimeout(function(){ if(bridgeIsLive(bridge) && !bridge.ready) revokeBridge(bridge,true); },5000);
    try{
      deliver({v:1,kind:'laruche.host.init',nonce:bridge.nonce,appId:app.id,viewId:view.id},channel.port2);
    }catch(error){
      revokeBridge(bridge,true);
      return null;
    }
    return bridge;
  }

  function connectBridge(frame,app,view,titleNode){
    frame.addEventListener('load',function(){
      if(!frame.contentWindow) return;
      startBridge(app,view,titleNode.textContent,titleNode,null,function(init,port){
        frame.contentWindow.postMessage(init,'*',[port]);
      });
    });
  }

  function navIcon(){
    return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m12 2.5 8 4.4v10.2l-8 4.4-8-4.4V6.9l8-4.4Z"/><path d="m4.4 7.2 7.6 4.3 7.6-4.3M12 11.5v10"/><path d="m8.2 4.6 7.7 4.3"/></svg>';
  }

  function ensureNavigation(){
    document.querySelectorAll('.header-nav, .mobile-tabs').forEach(function(nav){
      if(nav.querySelector('a[data-page="apps"]')) return;
      var link = document.createElement('a');
      link.href = '#apps/overview';
      link.dataset.page = 'apps';
      link.innerHTML = '<span class="tab-icon">'+navIcon()+'</span><span class="tab-text">'+esc(t('nav.apps'))+'</span>';
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
    var install = document.getElementById('appsInstall');
    var packageInput = document.getElementById('appsPackageInput');
    var refresh = document.getElementById('appsRefresh');
    var detach = document.getElementById('appsDetach');
    if(refresh){
      refresh.textContent=t('apps.refresh');
      refresh.onclick=function(){ if(isAdmin()) rescan(); else load(false); };
    }
    if(install){
      install.textContent=t('apps.install');
      install.onclick=function(){
        if(!isAdmin()) { LaRuche.Toast.show(t('apps.adminOnly'),'err'); return; }
        if(!installing && packageInput) packageInput.click();
      };
    }
    if(packageInput){ packageInput.onchange=function(){ installPackage(packageInput.files && packageInput.files[0]); }; }
    if(detach){ detach.textContent=t('apps.detach'); detach.onclick=function(){ detachActive(); }; }
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
    var button = document.getElementById('appsRefresh');
    if(button) button.disabled = true;
    var url = LaRuche.API.base + '/api/apps' + (forceRescan ? '/rescan' : '');
    fetch(url, {method:forceRescan?'POST':'GET', credentials:'include'})
      .then(function(response){ if(!response.ok) throw new Error('HTTP '+response.status); return response.json(); })
      .then(function(data){
        if(serial !== requestSerial) return;
        reconcileDetachedBridges(data);
        catalogue = data || {apps:[], diagnostics:[]};
        renderRail();
        applyRoute(pendingRoute || 'overview');
        if(forceRescan) LaRuche.Toast.show(t('apps.rescanDone'),'ok');
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
    var button=document.getElementById('appsInstall');
    if(!button) return;
    button.disabled=installing || !isAdmin();
    button.textContent=installing?t('apps.installing'):t('apps.install');
    button.title=!isAdmin()?t('apps.adminOnly'):'';
  }

  function installErrorMessage(payload){
    var error=payload && payload.error;
    var code=error && error.code;
    if(code==='app_package_too_large') return t('apps.installTooLarge');
    if(code==='app_version_exists') return t('apps.installExists');
    if(code==='app_package_invalid') return t('apps.installInvalid');
    if(code==='admin_required') return t('apps.adminOnly');
    return (error && error.message) || t('apps.installFailed');
  }

  function installPackage(file){
    var input=document.getElementById('appsPackageInput');
    if(input) input.value='';
    if(!file || !isAdmin() || installing) return;
    var name=String(file.name||'').toLowerCase();
    if(!name.endsWith('.zip') && !name.endsWith('.laruche-app')){
      LaRuche.Toast.show(t('apps.installType'),'err'); return;
    }
    if(file.size>maxPackageBytes){ LaRuche.Toast.show(t('apps.installTooLarge'),'err'); return; }
    installing=true; syncInstallButton();
    fetch(LaRuche.API.base+'/api/apps/install',{
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
        LaRuche.Toast.show(t('apps.installDone')+' '+(manifest.name||installed.id)+' '+(installed.activeVersion||''),'ok');
        pendingRoute='overview';
        load(false);
      })
      .catch(function(error){ LaRuche.Toast.show(error.message||t('apps.installFailed'),'err'); })
      .finally(function(){ installing=false; syncInstallButton(); });
  }

  function apps(){ return (catalogue && Array.isArray(catalogue.apps)) ? catalogue.apps : []; }
  function enabledViews(){
    var result=[];
    apps().forEach(function(app){
      var views = app.enabled && app.manifest && app.manifest.ui && Array.isArray(app.manifest.ui.views)
        ? app.manifest.ui.views : [];
      views.forEach(function(view){
        if(view.navigation !== false) result.push({app:app, view:view});
      });
    });
    return result;
  }

  function renderLoading(){
    var stage=document.getElementById('appsStage');
    if(stage) stage.innerHTML='<div class="apps-loading">'+esc(t('apps.loading'))+'</div>';
  }
  function renderFailure(){
    var rail=document.getElementById('appsRail');
    var stage=document.getElementById('appsStage');
    if(rail) rail.innerHTML='';
    if(stage) stage.innerHTML='<div class="apps-empty">'+esc(t('apps.loadFailed'))+'</div>';
    setDetach(false);
  }

  function renderRail(){
    var rail=document.getElementById('appsRail'); if(!rail) return;
    rail.innerHTML='';
    rail.appendChild(navButton(t('apps.catalogue'), t('apps.installed'), '⌂', 'overview', function(){ LaRuche.Router.go('apps/overview'); }));
    var views=enabledViews();
    if(views.length){
      var title=document.createElement('div'); title.className='apps-rail-title'; title.textContent=t('apps.views'); rail.appendChild(title);
      views.forEach(function(item){
        var route=encodeURIComponent(item.app.id)+'/'+encodeURIComponent(item.view.id);
        rail.appendChild(navButton(item.view.title, item.app.manifest.name, appIconMarkup(item.app,item.view,initial(item.app.manifest.name)), route, function(){ LaRuche.Router.go('apps/'+route); }));
      });
    }
    paintRail();
  }

  function navButton(name, appName, iconMarkup, route, onclick){
    var button=document.createElement('button');
    button.type='button'; button.className='apps-nav'; button.dataset.route=route;
    button.innerHTML='<span class="apps-nav-icon">'+iconMarkup+'</span><span class="apps-nav-copy"><span class="apps-nav-name">'+esc(name)+'</span><span class="apps-nav-app">'+esc(appName)+'</span></span>';
    button.onclick=onclick;
    return button;
  }

  function paintRail(){
    var route=active ? encodeURIComponent(active.app.id)+'/'+encodeURIComponent(active.view.id) : 'overview';
    document.querySelectorAll('#appsRail .apps-nav').forEach(function(button){ button.classList.toggle('active',button.dataset.route===route); });
  }

  function applyRoute(route){
    if(!catalogue) return;
    if(!route || route==='overview'){ showOverview(); return; }
    var parts=String(route).split('/');
    if(parts.length!==2){ showOverview(); return; }
    var appId, viewId;
    try{ appId=decodeURIComponent(parts[0]); viewId=decodeURIComponent(parts[1]); }catch(e){ showOverview(); return; }
    var found=null;
    enabledViews().some(function(item){ if(item.app.id===appId && item.view.id===viewId){ found=item; return true; } return false; });
    if(!found){ showOverview(); return; }
    showView(found.app, found.view);
  }

  function showOverview(){
    revokeBridge(); active=null; setDetach(false); paintRail();
    var stage=document.getElementById('appsStage'); if(!stage) return;
    var list=apps();
    var html='<div class="apps-catalogue"><div class="apps-catalogue-intro"><h2>'+esc(t('apps.installed'))+'</h2><p>'+esc(t('apps.catalogueHint'))+'</p></div>';
    if(!list.length) html+='<div class="apps-empty">'+esc(t('apps.none'))+'</div>';
    else {
      html+='<div class="apps-grid">';
      list.forEach(function(app, index){
        var manifest=app.manifest||{};
        var name=manifest.name||app.id;
        var state=app.error?'broken':(app.enabled?'enabled':'disabled');
        var label=app.error?t('apps.broken'):(app.enabled?t('apps.enabled'):t('apps.disabled'));
        var action=app.enabled?t('apps.disable'):t('apps.enable');
        var disabled=!!app.error || !isAdmin();
        var permissions=manifestPermissions(app,'required').concat(manifestPermissions(app,'optional'));
        html+='<article class="apps-card"><div class="apps-card-top"><div class="apps-card-icon">'+appIconMarkup(app,null,initial(name))+'</div><div class="apps-card-copy"><div class="apps-card-name">'+esc(name)+'</div><div class="apps-card-meta">'+esc(app.id)+' · '+esc(app.activeVersion||'')+'</div></div></div>'+
          '<div class="apps-card-desc">'+esc(manifest.description||'')+'</div>'+
          (permissions.length?'<div class="apps-card-permissions"><strong>'+esc(t('apps.permissions'))+'</strong>'+permissionMarkup(app,index)+'</div>':'')+
          '<div class="apps-card-foot"><span class="apps-state '+state+'"><span class="apps-state-dot"></span>'+esc(label)+'</span>'+
          '<button type="button" class="apps-btn" data-app-toggle="'+index+'" '+(disabled?'disabled title="'+esc(app.error||t('apps.adminOnly'))+'"':'')+'>'+esc(action)+'</button></div>'+
          (app.error?'<div class="apps-error">'+esc(app.error)+'</div>':'')+'</article>';
      });
      html+='</div>';
    }
    var diagnostics=(catalogue&&catalogue.diagnostics)||[];
    if(diagnostics.length){
      html+='<div class="apps-diagnostics"><strong>'+esc(t('apps.diagnostics'))+'</strong>';
      diagnostics.forEach(function(item){ html+='<div>'+esc(item.path)+': '+esc(item.error)+'</div>'; });
      html+='</div>';
    }
    stage.innerHTML=html+'</div>';
    stage.querySelectorAll('[data-app-toggle]').forEach(function(button){ button.onclick=function(){ toggleApp(parseInt(button.dataset.appToggle,10)); }; });
  }

  function toggleApp(index){
    var app=apps()[index]; if(!app || app.error || !isAdmin()) return;
    var action=app.enabled?'disable':'enable';
    var options={method:'POST',credentials:'include'};
    if(action==='enable'){
      var granted=manifestPermissions(app,'required');
      document.querySelectorAll('[data-app-permission="'+index+'"]:checked').forEach(function(input){ granted.push(input.value); });
      options.headers={'Content-Type':'application/json'};
      options.body=JSON.stringify({grantedPermissions:granted});
    }
    fetch(LaRuche.API.base+'/api/apps/'+encodeURIComponent(app.id)+'/'+action,options)
      .then(function(response){
        return response.json().catch(function(){ return null; }).then(function(payload){
          if(!response.ok) throw new Error(payload&&payload.error&&payload.error.message||t('apps.changeFailed'));
          return payload;
        });
      })
      .then(function(updated){
        catalogue.apps[index]=updated;
        if(!updated.enabled) reconcileDetachedBridges(catalogue);
        renderRail(); showOverview();
        LaRuche.Toast.show(updated.enabled?t('apps.enabled'):t('apps.disabled'),'ok');
      })
      .catch(function(error){ LaRuche.Toast.show(error.message||t('apps.changeFailed'),'err'); });
  }

  function assetUrl(app, view){
    return packageAssetUrl(app,view&&view.entry);
  }

  function showView(app, view){
    var url=assetUrl(app,view); if(!url){ showOverview(); return; }
    revokeBridge();
    active={app:app,view:view,url:url}; setDetach(view.detachable!==false); paintRail();
    var stage=document.getElementById('appsStage'); if(!stage) return;
    stage.innerHTML='';
    var wrap=document.createElement('div'); wrap.className='apps-frame-wrap';
    var bar=document.createElement('div'); bar.className='apps-frame-bar';
    var title=document.createElement('div'); title.className='apps-frame-title'; title.textContent=(app.manifest.name||app.id)+' · '+view.title;
    var note=document.createElement('div'); note.className='apps-frame-note'; note.textContent=t('apps.isolated');
    var frame=document.createElement('iframe');
    frame.className='apps-frame'; frame.title=view.title; frame.src=url;
    frame.setAttribute('sandbox','allow-scripts allow-forms allow-downloads');
    frame.setAttribute('referrerpolicy','no-referrer');
    frame.setAttribute('allow',"clipboard-read 'none'; clipboard-write 'none'; camera 'none'; microphone 'none'; geolocation 'none'");
    connectBridge(frame,app,view,title);
    bar.appendChild(title); bar.appendChild(note); wrap.appendChild(bar); wrap.appendChild(frame); stage.appendChild(wrap);
  }

  function setDetach(visible){
    var button=document.getElementById('appsDetach');
    if(button) button.hidden=!visible;
  }

  function forgetDetachedUrl(url){
    try{ URL.revokeObjectURL(url); }catch(error){}
    detachedUrls=detachedUrls.filter(function(item){ return item!==url; });
  }

  function detachedHost(title,src,token){
    var script="(function(){'use strict';var token='"+token+"',upstream=window.opener,frame=null,init=null,port=null;function forward(){if(!frame||!init||!port)return;frame.contentWindow.postMessage(init,'*',[port]);init=null;port=null;window.removeEventListener('message',receive);try{window.opener=null;}catch(error){}upstream=null;}function receive(event){var message=event.data;if(event.source!==upstream||!message||message.v!==1||message.kind!=='laruche.detached.connect'||message.token!==token||!message.init||event.ports.length!==1)return;init=message.init;port=event.ports[0];forward();}window.__larucheAppFrameReady=function(target){frame=target;forward();};window.addEventListener('message',receive);if(upstream)upstream.postMessage({v:1,kind:'laruche.detached.ready',token:token},'*');else window.close();})();";
    return '<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src \'none\'; frame-src \'self\'; script-src \'unsafe-inline\'; style-src \'unsafe-inline\'"><title>'+title+'</title><style>html,body,iframe{width:100%;height:100%;margin:0;border:0;background:#101014}body{overflow:hidden}</style><script>'+script+'</script></head><body><iframe title="'+title+'" src="'+src+'" sandbox="allow-scripts allow-forms allow-downloads" referrerpolicy="no-referrer" allow="clipboard-read \'none\'; clipboard-write \'none\'; camera \'none\'; microphone \'none\'; geolocation \'none\'" onload="window.__larucheAppFrameReady(this)"></iframe></body></html>';
  }

  function detachActive(){
    if(!active || active.view.detachable===false) return;
    if(detachedBridges.size+pendingDetached>=maxDetachedViews){ LaRuche.Toast.show(t('apps.detachLimit'),'err'); return; }
    var item=active;
    var title=esc((item.app.manifest.name||item.app.id)+' · '+item.view.title);
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
      LaRuche.Toast.show(t('apps.detachFailed'),'err');
    }
    function onReady(event){
      var message=event.data;
      if(finished || !popup || event.source!==popup || !message || message.v!==1 || message.kind!=='laruche.detached.ready' || message.token!==token) return;
      finished=true; cleanup(false);
      setTimeout(function(){ forgetDetachedUrl(url); },60000);
      var detached=startBridge(item.app,item.view,(item.app.manifest.name||item.app.id)+' · '+item.view.title,null,popup,function(init,port){
        popup.postMessage({v:1,kind:'laruche.detached.connect',token:token,init:init},'*',[port]);
      });
      if(!detached){ try{ popup.close(); }catch(error){} LaRuche.Toast.show(t('apps.detachFailed'),'err'); return; }
      ensureDetachedSweep();
    }
    window.addEventListener('message',onReady);
    popup=window.open(url,'_blank','popup,width=1100,height=760');
    if(!popup){ finished=true; cleanup(true); LaRuche.Toast.show(t('apps.popupBlocked'),'err'); return; }
    timer=setTimeout(fail,5000);
  }

  function leave(){ revokeBridge(); }
  return {init:init,enter:enter,leave:leave,deepLink:deepLink,rescan:rescan,detach:detachActive};
})();

LaRuche.Router.register('apps', LaRuche.Apps);
