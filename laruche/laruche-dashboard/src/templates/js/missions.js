LaRuche.Missions = (function(){
  var list = [];
  var current = null; // slug du dossier affiche
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
      if(el) el.innerHTML = '<div class="mem2-empty">Missions indisponibles</div>';
      LaRuche.Toast.show('Missions indisponibles: '+e, 'err');
    });
  }

  function fmtDate(s) {
    if(!s) return 'jamais';
    try { var d = new Date(s); if(isNaN(d)) return s; return d.toLocaleString(); } catch(e){ return s; }
  }

  function renderList() {
    var el = document.getElementById('misList'); if(!el) return;
    if(!list.length){ el.innerHTML = '<div class="mem2-empty">Aucune mission. Creez-en une.</div>'; return; }
    el.innerHTML = list.map(function(m){
      var status = m.status || 'active';
      var cad = m.cadence ? ('<span title="cadence cron">&#x23F1; '+esc(m.cadence)+'</span>') : '<span>manuel</span>';
      return '<div class="mis-card'+(current===m.slug?' active':'')+'" data-slug="'+esc(m.slug)+'">'+
        '<div class="mis-card-obj">'+esc(m.objective || m.slug)+'</div>'+
        '<div class="mis-card-meta">'+
          '<span class="mis-badge '+esc(status)+'">'+esc(status)+'</span>'+
          '<span>&#x21BB; '+(m.iterations!=null?m.iterations:0)+' iter.</span>'+
          cad+
          '<span>derniere: '+esc(fmtDate(m.last_run))+'</span>'+
        '</div>'+
        '<div class="mis-card-acts">'+
          '<button class="mis-act run" data-act="run" data-slug="'+esc(m.slug)+'">Lancer une iteration</button>'+
          '<button class="mis-act" data-act="dossier" data-slug="'+esc(m.slug)+'">Dossier</button>'+
          (status==='active'
            ? '<button class="mis-act" data-act="pause" data-slug="'+esc(m.slug)+'">Pause</button>'
            : '<button class="mis-act" data-act="resume" data-slug="'+esc(m.slug)+'">Reprendre</button>')+
          '<button class="mis-act" data-act="done" data-slug="'+esc(m.slug)+'">Terminer</button>'+
          '<button class="mis-act danger" data-act="delete" data-slug="'+esc(m.slug)+'">Suppr</button>'+
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
  // Monte le builder de cadence (réutilise celui des Crons) + peuple le sélecteur provider.
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
    if(!objective){ LaRuche.Toast.show('Objectif requis','warn'); return; }
    var body = { objective:objective };
    if(cadence) body.cadence = cadence;
    if(slug) body.slug = slug;
    if(profile_id) body.profile_id = profile_id;
    if(channel) body.channel = channel;
    fetch(LaRuche.API.base+'/api/missions', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show('Mission creee: '+(d.slug||''),'ok');
        document.getElementById('misObjective').value='';
        if(chEl) chEl.value='';
        document.getElementById('misSlug').value='';
        refresh();
      }).catch(function(e){ LaRuche.Toast.show('Creation: '+e,'err'); });
  }

  function runMission(slug) {
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug)+'/run', {method:'POST'})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show('Iteration '+(d.iteration!=null?'#'+d.iteration+' ':'')+'lancee (en arriere-plan)','ok');
        setTimeout(refresh, 1200);
      }).catch(function(e){ LaRuche.Toast.show('Run: '+e,'err'); });
  }

  function setStatus(slug, status) {
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug), {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({status:status})})
      .then(function(r){return r.json();}).then(function(d){
        if(d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show('Statut: '+status,'ok');
        refresh();
      }).catch(function(e){ LaRuche.Toast.show('Statut: '+e,'err'); });
  }

  function deleteMission(slug) {
    if(!window.confirm('Supprimer la mission "'+slug+'" ? (le savoir reste en memoire)')) return;
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug), {method:'DELETE'})
      .then(function(r){return r.json();}).then(function(d){
        if(d && d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show('Mission supprimee','ok');
        if(current===slug){ current=null; renderMainEmpty(); }
        refresh();
      }).catch(function(e){ LaRuche.Toast.show('Suppression: '+e,'err'); });
  }

  function renderMainEmpty() {
    var main = document.getElementById('misMain'); if(!main) return;
    main.innerHTML = '<div class="mem2-empty">Selectionnez une mission pour voir son dossier, ou creez-en une.</div>';
  }

  /* ---- B2 : vue Dossier (markdown) ---- */
  function openDossier(slug) {
    current = slug;
    renderList();
    var main = document.getElementById('misMain'); if(!main) return;
    main.innerHTML = '<div class="mem2-empty">Chargement du dossier...</div>';
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
          '<span class="mis-dossier-title">Dossier &middot; '+esc(curObjective || slug)+'</span>'+
          '<span><button class="mem2-tbtn" id="misEditBtn">Modifier</button> '+
          '<button class="mem2-tbtn" id="misExportBtn">Exporter</button></span>'+
        '</div>'+
        '<div id="misEditBox" style="display:none;border:1px solid var(--amber);border-radius:8px;padding:12px;margin-bottom:14px;background:rgba(245,158,11,.06)">'+
          '<div style="font-weight:600;color:var(--amber);margin-bottom:8px">Modifier la mission</div>'+
          '<label class="form-label" style="font-size:10px;color:var(--text-dim)">Objectif</label>'+
          '<textarea class="form-input" id="misEditObj" rows="3" style="width:100%;margin-bottom:10px">'+esc(curObjective)+'</textarea>'+
          '<label class="form-label" style="font-size:10px;color:var(--text-dim)">Cadence</label>'+
          '<div id="misEditCadence" style="margin-bottom:10px"></div>'+
          '<label class="form-label" style="font-size:10px;color:var(--text-dim)">Statut</label>'+
          '<select class="form-input" id="misEditStatus" style="margin-bottom:12px">'+statOpt('active','Active')+statOpt('paused','En pause')+statOpt('done','Terminee')+'</select>'+
          '<div style="display:flex;gap:8px"><button class="mem2-btn-primary" id="misEditSave">Enregistrer</button>'+
          '<button class="mem2-tbtn" id="misEditCancel">Annuler</button></div>'+
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
    }).catch(function(e){ main.innerHTML = '<div class="mem2-empty">Erreur: '+esc(e)+'</div>'; });
  }

  function saveMissionEdit(slug, payload) {
    fetch(LaRuche.API.base+'/api/missions/'+encodeURIComponent(slug), {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(payload)})
      .then(function(r){return r.json().catch(function(){return {};});}).then(function(d){
        if(d && d.error){ LaRuche.Toast.show(d.error,'err'); return; }
        LaRuche.Toast.show('Mission mise a jour','ok');
        // refresh liste + dossier
        fetch(LaRuche.API.base+'/api/missions').then(function(r){return r.json();}).then(function(data){
          list = Array.isArray(data) ? data : (data.missions || []);
          renderList();
          openDossier(slug);
        }).catch(function(){ openDossier(slug); });
      }).catch(function(e){ LaRuche.Toast.show('Enregistrement: '+e,'err'); });
  }

  function exportDossier(slug, md) {
    try {
      var blob = new Blob([md], {type:'text/markdown;charset=utf-8'});
      var url = URL.createObjectURL(blob);
      var a = document.createElement('a');
      a.href = url; a.download = 'dossier-'+slug+'.md';
      document.body.appendChild(a); a.click();
      setTimeout(function(){ document.body.removeChild(a); URL.revokeObjectURL(url); }, 100);
      LaRuche.Toast.show('Dossier exporte','ok');
    } catch(e){ LaRuche.Toast.show('Export: '+e,'err'); }
  }

  return { init:init, enter:enter, leave:leave, current:function(){return current;}, refresh:refresh, create:create, mountForm:mountForm };
})();


