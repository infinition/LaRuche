LaRuche.i18n.add({
  'missions.unavailable':      { fr: 'Missions indisponibles',                                              en: 'Missions unavailable' },
  'missions.unavailableErr':   { fr: 'Missions indisponibles: ',                                            en: 'Missions unavailable: ' },
  'missions.never':            { fr: 'jamais',                                                              en: 'never' },
  'missions.empty':            { fr: 'Aucune mission. Creez-en une.',                                       en: 'No missions. Create one.' },
  'missions.manual':           { fr: 'manuel',                                                              en: 'manual' },
  'missions.lastRun':          { fr: 'derniere: ',                                                          en: 'last: ' },
  'missions.btnRun':           { fr: 'Lancer une iteration',                                               en: 'Run an iteration' },
  'missions.btnDossier':       { fr: 'Dossier',                                                            en: 'Dossier' },
  'missions.btnPause':         { fr: 'Pause',                                                              en: 'Pause' },
  'missions.btnResume':        { fr: 'Reprendre',                                                          en: 'Resume' },
  'missions.btnDone':          { fr: 'Terminer',                                                           en: 'Finish' },
  'missions.btnDelete':        { fr: 'Suppr',                                                              en: 'Del' },
  'missions.objectiveRequired':{ fr: 'Objectif requis',                                                    en: 'Objective required' },
  'missions.created':          { fr: 'Mission creee: ',                                                     en: 'Mission created: ' },
  'missions.createErr':        { fr: 'Creation: ',                                                         en: 'Creation: ' },
  'missions.iterLaunched':     { fr: 'lancee (en arriere-plan)',                                            en: 'launched (in background)' },
  'missions.statusLabel':      { fr: 'Statut: ',                                                           en: 'Status: ' },
  'missions.confirmDelete':    { fr: 'Supprimer la mission "{slug}" ? (le savoir reste en memoire)',        en: 'Delete mission "{slug}"? (knowledge stays in memory)' },
  'missions.deleted':          { fr: 'Mission supprimee',                                                   en: 'Mission deleted' },
  'missions.deleteErr':        { fr: 'Suppression: ',                                                      en: 'Deletion: ' },
  'missions.selectHint':       { fr: 'Selectionnez une mission pour voir son dossier, ou creez-en une.',   en: 'Select a mission to view its dossier, or create one.' },
  'missions.loading':          { fr: 'Chargement du dossier...',                                           en: 'Loading dossier...' },
  'missions.dossierPrefix':    { fr: 'Dossier · ',                                                    en: 'Dossier · ' },
  'missions.btnEdit':          { fr: 'Modifier',                                                           en: 'Edit' },
  'missions.btnExport':        { fr: 'Exporter',                                                           en: 'Export' },
  'missions.editTitle':        { fr: 'Modifier la mission',                                                en: 'Edit mission' },
  'missions.labelObjective':   { fr: 'Objectif',                                                           en: 'Objective' },
  'missions.labelCadence':     { fr: 'Cadence',                                                            en: 'Cadence' },
  'missions.labelStatus':      { fr: 'Statut',                                                             en: 'Status' },
  'missions.optActive':        { fr: 'Active',                                                             en: 'Active' },
  'missions.optPaused':        { fr: 'En pause',                                                           en: 'Paused' },
  'missions.optDone':          { fr: 'Terminee',                                                           en: 'Done' },
  'missions.btnSave':          { fr: 'Enregistrer',                                                        en: 'Save' },
  'missions.btnCancel':        { fr: 'Annuler',                                                            en: 'Cancel' },
  'missions.errorPrefix':      { fr: 'Erreur: ',                                                           en: 'Error: ' },
  'missions.updated':          { fr: 'Mission mise a jour',                                                en: 'Mission updated' },
  'missions.saveErr':          { fr: 'Enregistrement: ',                                                   en: 'Save: ' },
  'missions.exported':         { fr: 'Dossier exporte',                                                    en: 'Dossier exported' },
  'missions.exportErr':        { fr: 'Export: ',                                                           en: 'Export: ' },
  'missions.cadenceCron':      { fr: 'cadence cron',                                                       en: 'cadence cron' },
  'missions.iterSuffix':       { fr: ' iter.',                                                             en: ' iter.' },
  'missions.iterPrefix':       { fr: 'Iteration ',                                                         en: 'Iteration ' },
  'missions.runErr':           { fr: 'Lancement: ',                                                        en: 'Run: ' }
});

LaRuche.Missions = (function(){
  var list = [];
  var current = null; // slug of the displayed dossier
  var loaded = false;

  function esc(t){ return LaRuche.Utils.esc(t); }

  function init() {}
  function enter() { if(!loaded) refresh(); }
  function leave() {}

  function refresh() {
    fetch(LaRuche.API.base+'/api/missions').then(function(r){return r.json();}).then(function(data){
      loaded = true;
      list = Array.isArray(data) ? data : (data.missions || []);
      renderList();
    }).catch(function(e){
      var el = document.getElementById('misList');
      if(el) el.innerHTML = '<div class="mem2-empty">'+LaRuche.i18n.t('missions.unavailable')+'</div>';
      LaRuche.Toast.show(LaRuche.i18n.t('missions.unavailableErr')+e, 'err');
    });
  }

  function fmtDate(s) {
    if(!s) return LaRuche.i18n.t('missions.never');
    try { var d = new Date(s); if(isNaN(d)) return s; return d.toLocaleString(); } catch(e){ return s; }
  }

  function renderList() {
    var el = document.getElementById('misList'); if(!el) return;
    if(!list.length){ el.innerHTML = '<div class="mem2-empty">'+LaRuche.i18n.t('missions.empty')+'</div>'; return; }
    el.innerHTML = list.map(function(m){
      var status = m.status || 'active';
      var cad = m.cadence ? ('<span title="'+LaRuche.i18n.t('missions.cadenceCron')+'">&#x23F1; '+esc(m.cadence)+'</span>') : '<span>'+LaRuche.i18n.t('missions.manual')+'</span>';
      return '<div class="mis-card'+(current===m.slug?' active':'')+'" data-slug="'+esc(m.slug)+'">'+
        '<div class="mis-card-obj">'+esc(m.objective || m.slug)+'</div>'+
        '<div class="mis-card-meta">'+
          '<span class="mis-badge '+esc(status)+'">'+esc(status)+'</span>'+
          '<span>&#x21BB; '+(m.iterations!=null?m.iterations:0)+LaRuche.i18n.t('missions.iterSuffix')+'</span>'+
          cad+
          '<span>'+LaRuche.i18n.t('missions.lastRun')+esc(fmtDate(m.last_run))+'</span>'+
        '</div>'+
        '<div class="mis-card-acts">'+
          '<button class="mis-act run" data-act="run" data-slug="'+esc(m.slug)+'">'+LaRuche.i18n.t('missions.btnRun')+'</button>'+
          '<button class="mis-act" data-act="dossier" data-slug="'+esc(m.slug)+'">'+LaRuche.i18n.t('missions.btnDossier')+'</button>'+
          (status==='active'
            ? '<button class="mis-act" data-act="pause" data-slug="'+esc(m.slug)+'">'+LaRuche.i18n.t('missions.btnPause')+'</button>'
            : '<button class="mis-act" data-act="resume" data-slug="'+esc(m.slug)+'">'+LaRuche.i18n.t('missions.btnResume')+'</button>')+
          '<button class="mis-act" data-act="done" data-slug="'+esc(m.slug)+'">'+LaRuche.i18n.t('missions.btnDone')+'</button>'+
          '<button class="mis-act danger" data-act="delete" data-slug="'+esc(m.slug)+'">'+LaRuche.i18n.t('missions.btnDelete')+'</button>'+
        '</div>'+
      '</div>';
    }).join('');
    el.querySelectorAll('.mis-card').forEach(function(card){
      card.onclick = function(e){ if(e.target.closest('[data-act]')) return; openDossier(card.dataset.slug); };
    });
    el.querySelectorAll('[data-act]').forEach(function(b){
      b.onclick = function(e){
        e.stopPropagation();
        var act=b.dataset.act, slug=b.dataset.slug;
        if(act==='run') runMission(slug);
        else if(act==='dossier') openDossier(slug);
        else if(act==='pause') setStatus(slug,'paused');
        else if(act==='resume') setStatus(slug,'active');
        else if(act==='done') setStatus(slug,'done');
        else if(act==='delete') deleteMission(slug);
      };
    });
  }

  var _cadenceId = null;
  // Mount the cadence builder (reuses the Crons one) + populate the provider selector.
  function mountForm() {
    if(LaRuche.CronBuilder){ _cadenceId = LaRuche.CronBuilder.mount('misCadenceBuilder', { value:'' }); }
    var sel = document.getElementById('misProvider');
    if(sel){
      fetch('/api/profiles').then(function(r){return r.json();}).then(function(d){
        var profs = (d && d.profiles) || {};
        Object.keys(profs).forEach(function(k){
          var o=document.createElement('option'); o.value=k; o.textContent=profs[k].name||k; sel.appendChild(o);
        });
      }).catch(function(){});
    }
  }

  function create() {
    var objective = (document.getElementById('misObjective').value || '').trim();
    var cadence = (_cadenceId && LaRuche.CronBuilder) ? LaRuche.CronBuilder.getValue(_cadenceId) : '';
    var slug = (document.getElementById('misSlug').value || '').trim();
    var provEl = document.getElementById('misProvider');
    var chEl = document.getElementById('misChannel');
    var profile_id = provEl ? provEl.value : '';
    var channel = chEl ? (chEl.value||'').trim() : '';
    if(!objective){ LaRuche.Toast.show(LaRuche.i18n.t('missions.objectiveRequired'),'warn'); return; }
    var body = { objective:objective };
    if(cadence) body.cadence = cadence;
    if(slug) body.slug = slug;
    if(profile_id) body.profile_id = profile_id;
    if(channel) body.channel = channel;
    fetch(LaRuche.API.base+'/api/missions', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show(LaRuche.i18n.t('missions.created')+(d.slug||''),'ok');
        document.getElementById('misObjective').value='';
        if(chEl) chEl.value='';
        document.getElementById('misSlug').value='';
        refresh();
      }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('missions.createErr')+e,'err'); });
  }

  function runMission(slug) {
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug)+'/run', {method:'POST'})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show(LaRuche.i18n.t('missions.iterPrefix')+(d.iteration!=null?'#'+d.iteration+' ':'')+LaRuche.i18n.t('missions.iterLaunched'),'ok');
        setTimeout(refresh, 1200);
      }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('missions.runErr')+e,'err'); });
  }

  function setStatus(slug, status) {
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug), {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({status:status})})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show(LaRuche.i18n.t('missions.statusLabel')+status,'ok');
        refresh();
      }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('missions.statusLabel')+e,'err'); });
  }

  function deleteMission(slug) {
    if(!window.confirm(LaRuche.i18n.t('missions.confirmDelete').replace('{slug}', slug))) return;
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug), {method:'DELETE'})
      .then(function(r){return r.json();}).then(function(d){
        if(d && d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show(LaRuche.i18n.t('missions.deleted'),'ok');
        if(current===slug){ current=null; renderMainEmpty(); }
        refresh();
      }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('missions.deleteErr')+e,'err'); });
  }

  function renderMainEmpty() {
    var main = document.getElementById('misMain'); if(!main) return;
    main.innerHTML = '<div class="mem2-empty">'+LaRuche.i18n.t('missions.selectHint')+'</div>';
  }

  /* ---- B2: Dossier view (markdown) ---- */
  function openDossier(slug) {
    current = slug;
    renderList();
    var main = document.getElementById('misMain'); if(!main) return;
    main.innerHTML = '<div class="mem2-empty">'+LaRuche.i18n.t('missions.loading')+'</div>';
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug)+'/dossier').then(function(r){return r.json();}).then(function(d){
      if(d.error){ main.innerHTML = '<div class="mem2-empty">'+esc(d.error)+'</div>'; return; }
      var md = d.markdown || '';
      var mObj = list.filter(function(m){return m.slug===slug;})[0] || {};
      var curObjective = d.objective || mObj.objective || '';
      var curCadence = mObj.cadence || d.cadence || '';
      var curStatus = mObj.status || d.status || 'active';
      function statOpt(v,lbl){ return '<option value="'+v+'"'+(curStatus===v?' selected':'')+'>'+lbl+'</option>'; }
      main.innerHTML =
        '<div class="mis-dossier-bar">'+
          '<span class="mis-dossier-title">'+LaRuche.i18n.t('missions.dossierPrefix')+esc(curObjective || slug)+'</span>'+
          '<span><button class="mem2-tbtn" id="misEditBtn">'+LaRuche.i18n.t('missions.btnEdit')+'</button> '+
          '<button class="mem2-tbtn" id="misExportBtn">'+LaRuche.i18n.t('missions.btnExport')+'</button></span>'+
        '</div>'+
        '<div id="misEditBox" style="display:none;border:1px solid var(--amber);border-radius:8px;padding:12px;margin-bottom:14px;background:rgba(245,158,11,.06)">'+
          '<div style="font-weight:600;color:var(--amber);margin-bottom:8px">'+LaRuche.i18n.t('missions.editTitle')+'</div>'+
          '<label class="form-label" style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('missions.labelObjective')+'</label>'+
          '<textarea class="form-input" id="misEditObj" rows="3" style="width:100%;margin-bottom:10px">'+esc(curObjective)+'</textarea>'+
          '<label class="form-label" style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('missions.labelCadence')+'</label>'+
          '<div id="misEditCadence" style="margin-bottom:10px"></div>'+
          '<label class="form-label" style="font-size:10px;color:var(--text-dim)">'+LaRuche.i18n.t('missions.labelStatus')+'</label>'+
          '<select class="form-input" id="misEditStatus" style="margin-bottom:12px">'+statOpt('active',LaRuche.i18n.t('missions.optActive'))+statOpt('paused',LaRuche.i18n.t('missions.optPaused'))+statOpt('done',LaRuche.i18n.t('missions.optDone'))+'</select>'+
          '<div style="display:flex;gap:8px"><button class="mem2-btn-primary" id="misEditSave">'+LaRuche.i18n.t('missions.btnSave')+'</button>'+
          '<button class="mem2-tbtn" id="misEditCancel">'+LaRuche.i18n.t('missions.btnCancel')+'</button></div>'+
        '</div>'+
        '<div class="mis-dossier-body mem2-md" id="misDossierMd"></div>';
      var body = document.getElementById('misDossierMd');
      body.innerHTML = LaRuche.MD.render(md);
      LaRuche.MD.wireWikilinks(body, function(node){ LaRuche.Router.go('memory'); setTimeout(function(){ LaRuche.Memory.loadNode(node); }, 60); });
      document.getElementById('misExportBtn').onclick = function(){ exportDossier(slug, md); };
      var _cadBuilderId = null;
      document.getElementById('misEditBtn').onclick = function(){
        var box=document.getElementById('misEditBox');
        var open = box.style.display==='none';
        box.style.display = open?'block':'none';
        if(open && !_cadBuilderId && LaRuche.CronBuilder){
          _cadBuilderId = LaRuche.CronBuilder.mount('misEditCadence', { value: curCadence });
        }
      };
      document.getElementById('misEditCancel').onclick = function(){ document.getElementById('misEditBox').style.display='none'; };
      document.getElementById('misEditSave').onclick = function(){
        var objective = (document.getElementById('misEditObj').value||'').trim();
        var status = document.getElementById('misEditStatus').value;
        var cadence = _cadBuilderId ? LaRuche.CronBuilder.getValue(_cadBuilderId) : curCadence;
        saveMissionEdit(slug, { objective:objective, cadence:cadence||'', status:status });
      };
    }).catch(function(e){ main.innerHTML = '<div class="mem2-empty">'+LaRuche.i18n.t('missions.errorPrefix')+esc(e)+'</div>'; });
  }

  function saveMissionEdit(slug, payload) {
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug), {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(payload)})
      .then(function(r){return r.json().catch(function(){return {};});}).then(function(d){
        if(d && d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show(LaRuche.i18n.t('missions.updated'),'ok');
        // refresh list + dossier
        fetch(LaRuche.API.base+'/api/missions').then(function(r){return r.json();}).then(function(data){
          list = Array.isArray(data) ? data : (data.missions || []);
          renderList();
          openDossier(slug);
        }).catch(function(){ openDossier(slug); });
      }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('missions.saveErr')+e,'err'); });
  }

  function exportDossier(slug, md) {
    try {
      var blob = new Blob([md], {type:'text/markdown;charset=utf-8'});
      var url = URL.createObjectURL(blob);
      var a = document.createElement('a');
      a.href = url; a.download = 'dossier-'+slug+'.md';
      document.body.appendChild(a); a.click();
      setTimeout(function(){ document.body.removeChild(a); URL.revokeObjectURL(url); }, 100);
      LaRuche.Toast.show(LaRuche.i18n.t('missions.exported'),'ok');
    } catch(e){ LaRuche.Toast.show(LaRuche.i18n.t('missions.exportErr')+e,'err'); }
  }

  return { init:init, enter:enter, leave:leave, current:function(){return current;}, refresh:refresh, create:create, mountForm:mountForm };
})();


