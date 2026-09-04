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
  'addons.diagnostics':     {fr:'Diagnostics de découverte', en:'Discovery diagnostics'},
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

  function t(key, vars){ return LaRuche.i18n.t(key, vars); }
  function esc(value){ return LaRuche.Utils.esc(value == null ? '' : value); }
  function initial(value){ return String(value || '?').trim().charAt(0) || '?'; }
  function isAdmin(){ return !!(LaRuche.Auth && LaRuche.Auth.isAdmin && LaRuche.Auth.isAdmin()); }

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
    active=null; setDetach(false); paintRail();
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
        html+='<article class="addons-card"><div class="addons-card-top"><div class="addons-card-icon">'+esc(initial(name))+'</div><div class="addons-card-copy"><div class="addons-card-name">'+esc(name)+'</div><div class="addons-card-meta">'+esc(addon.id)+' · '+esc(addon.activeVersion||'')+'</div></div></div>'+
          '<div class="addons-card-desc">'+esc(manifest.description||'')+'</div><div class="addons-card-foot"><span class="addons-state '+state+'"><span class="addons-state-dot"></span>'+esc(label)+'</span>'+
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
    frame.setAttribute('allow','clipboard-write');
    bar.appendChild(title); bar.appendChild(note); wrap.appendChild(bar); wrap.appendChild(frame); stage.appendChild(wrap);
  }

  function setDetach(visible){
    var button=document.getElementById('addonsDetach');
    if(button) button.hidden=!visible;
  }

  function detachActive(){
    if(!active || active.view.detachable===false) return;
    var title=esc((active.addon.manifest.name||active.addon.id)+' — '+active.view.title);
    var src=esc(active.url);
    var host='<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>'+title+'</title><style>html,body,iframe{width:100%;height:100%;margin:0;border:0;background:#101014}body{overflow:hidden}</style></head><body><iframe title="'+title+'" src="'+src+'" sandbox="allow-scripts allow-forms allow-downloads" referrerpolicy="no-referrer" allow="clipboard-write"></iframe></body></html>';
    var url=URL.createObjectURL(new Blob([host],{type:'text/html'}));
    detachedUrls.push(url);
    // The top-level blob contains only this trusted host document; untrusted code
    // remains in the opaque sandbox below it. Clear `opener` immediately, while
    // retaining a real return value so a blocked popup can be reported.
    var popup=window.open(url,'_blank','popup,width=1100,height=760');
    if(popup) popup.opener=null;
    else LaRuche.Toast.show(t('addons.popupBlocked'),'err');
    setTimeout(function(){ URL.revokeObjectURL(url); detachedUrls=detachedUrls.filter(function(item){return item!==url;}); },60000);
  }

  function leave(){ /* iframe lifecycle follows the page; split mode keeps it mounted. */ }
  return {init:init,enter:enter,leave:leave,deepLink:deepLink,rescan:rescan,detach:detachActive};
})();

LaRuche.Router.register('addons', LaRuche.Addons);
