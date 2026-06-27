import re

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    html = f.read()

old_loadCron = """  async function loadCron(el) {
    var tasks=[];try{tasks=await fetch('/api/cron').then(function(r){return r.json();});}catch(e){}
    var modelsResp={models:[]};try{modelsResp=await fetch('/api/profiles/models').then(function(r){return r.json();});}catch(e){}
    var modelOpts = '<option value="">Default model</option>' + modelsResp.models.map(function(m){
        return '<option value="'+LaRuche.Utils.esc(m.provider+'|'+m.name)+'">'+LaRuche.Utils.esc(m.provider+' - '+m.name)+'</option>';
    }).join('');

    el.innerHTML='<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="document.getElementById(\\'newCronForm\\').style.display=\\'block\\'">+ New Task</button></div>'+
      '<div id="newCronForm" style="display:none" class="settings-card">'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Name</label><input id="ncName" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Prompt</label><input id="ncPrompt" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Cron</label><input id="ncCron" class="form-input" placeholder="*/5 * * * *"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Canal de feedback</label><select id="ncChannel" class="form-input"><option value="">None (Activity Log)</option><option value="telegram">Telegram</option><option value="discord">Discord</option></select></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Modle</label><select id="ncModel" class="form-input">'+modelOpts+'</select></div>'+
      '<button class="settings-save-btn" onclick="LaRuche.Settings.createCron()">Create</button></div>'+
      tasks.map(function(t){return '<div class="settings-card"><div class="settings-card-title">'+LaRuche.Utils.esc(t.name)+'</div><div class="settings-row"><span class="settings-label">Schedule</span><span class="settings-value">'+(t.cron_expr||t.fire_at||'-')+'</span></div><div class="settings-row"><span class="settings-label">Runs</span><span class="settings-value">'+(t.run_count||0)+'</span></div><div class="settings-row"><span class="settings-label">Channel</span><span class="settings-value">'+LaRuche.Utils.esc(t.channel||'None')+'</span></div><div class="settings-row"><span class="settings-label">Model</span><span class="settings-value">'+LaRuche.Utils.esc(t.model||'Default')+'</span></div><button onclick="fetch(\\'/api/cron/'+t.id+'\\',{method:\\'DELETE\\'}).then(function(){LaRuche.Settings.refreshTab()})" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;margin-top:6px">Delete</button></div>';}).join('');
  }"""

# Handle encoding issues with 'Modle' -> 'Modèle'
old_loadCron = html[html.find("  async function loadCron(el) {"):html.find("  function createCron() {")]

new_loadCron = """  async function loadCron(el) {
    var tasks=[];try{tasks=await fetch('/api/cron').then(function(r){return r.json();});}catch(e){}
    var profilesResp={profiles:{}};try{profilesResp=await fetch('/api/profiles').then(function(r){return r.json();});}catch(e){}
    var profiles = profilesResp.profiles || {};
    window._lastProfiles = profiles;
    
    var profOpts = '<option value="">Default (modele actif)</option>';
    Object.keys(profiles).forEach(function(k){
        profOpts += '<option value="'+k+'">'+LaRuche.Utils.esc(profiles[k].name)+'</option>';
    });

    el.innerHTML='<div style="margin-bottom:12px"><button class="settings-save-btn" onclick="document.getElementById(\\'newCronForm\\').style.display=\\'block\\'">+ New Task</button></div>'+
      '<div id="newCronForm" style="display:none" class="settings-card">'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Name</label><input id="ncName" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Prompt</label><input id="ncPrompt" class="form-input"></div>'+
      '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">Cron</label><input id="ncCron" class="form-input" placeholder="*/5 * * * *"></div>'+
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
          return '<div class="settings-card"><div class="settings-card-title">'+LaRuche.Utils.esc(t.name)+'</div><div class="settings-row"><span class="settings-label">Schedule</span><span class="settings-value">'+(t.cron_expr||t.fire_at||'-')+'</span></div><div class="settings-row"><span class="settings-label">Runs</span><span class="settings-value">'+(t.run_count||0)+'</span></div><div class="settings-row"><span class="settings-label">Channel</span><span class="settings-value">'+LaRuche.Utils.esc(t.channel||'None')+'</span></div><div class="settings-row"><span class="settings-label">Provider/Model</span><span class="settings-value">'+LaRuche.Utils.esc(effProv)+'</span></div><button onclick="fetch(\\'/api/cron/'+t.id+'\\',{method:\\'DELETE\\'}).then(function(){LaRuche.Settings.refreshTab()})" style="background:none;border:1px solid var(--red);color:var(--red);border-radius:4px;padding:2px 8px;cursor:pointer;font-size:10px;margin-top:6px">Delete</button></div>';
      }).join('');
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
"""
html = html.replace(old_loadCron, new_loadCron)


old_createCron = html[html.find("  function createCron() {"):html.find("  async function loadWatchers(el) {")]

new_createCron = """  function createCron() {
    var name=document.getElementById('ncName').value;
    var prompt=document.getElementById('ncPrompt').value;
    var cron=document.getElementById('ncCron').value;
    var channel=document.getElementById('ncChannel').value;
    var profile_id=document.getElementById('ncProfileId').value;
    var model=document.getElementById('ncModel').value;
    
    var payload = {name:name,prompt:prompt,cron_expr:cron,channel:channel||null};
    if(profile_id) payload.profile_id = profile_id;
    if(model) payload.model = model;
    
    fetch('/api/cron',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)}).then(function(){loadTab('cron');LaRuche.Toast.show('Cron task created','ok');});
  }

"""
html = html.replace(old_createCron, new_createCron)

# Expose updateCronModelSelect
html = re.sub(r'(deleteCredential:deleteCredential)',
              r'\1, updateCronModelSelect:updateCronModelSelect', html)


with open('laruche-dashboard/src/templates/spa.html', 'w', encoding='utf-8') as f:
    f.write(html)
