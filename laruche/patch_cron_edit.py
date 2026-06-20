import re

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    html = f.read()

# For tlEdit
old_tlEdit_start = """    d.innerHTML='<div class="tl-detail"><div style="font-weight:600;color:var(--amber);margin-bottom:8px">diter : '+LaRuche.Utils.esc(job.name||'')+'</div>'+
      '<label class="form-label">Nom</label><input class="form-input" id="tlfName" value="'+LaRuche.Utils.esc(job.name||'')+'">'+
      '<label class="form-label">Prompt</label><textarea class="form-input" id="tlfPrompt" rows="3">'+LaRuche.Utils.esc(job.prompt||'')+'</textarea>'+
      '<label class="form-label">Cron (5 champs) ou vide</label><input class="form-input" id="tlfCron" value="'+LaRuche.Utils.esc(job.cron_expr||'')+'" placeholder="*/30 * * * *">'+
      '<label class="form-label">Canal</label><input class="form-input" id="tlfChannel" value="'+LaRuche.Utils.esc(job.channel||'')+'" placeholder="telegram / vide">'+
      '<label class="form-label">Modle</label><input class="form-input" id="tlfModel" value="'+LaRuche.Utils.esc(job.model||'')+'" placeholder="(dfaut)">'+"""

# We extract dynamically around it
idx = html.find("d.innerHTML='<div class=\"tl-detail\"")
end_idx = html.find("skillHtml+", idx)
old_tlEdit_part = html[idx:end_idx]

new_tlEdit_part = """    var profiles = window._lastProfiles || {};
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
      """
html = html.replace(old_tlEdit_part, new_tlEdit_part)

# For tlSaveEdit
old_tlSaveEdit = """  function tlSaveEdit(i){
    var job=_tlJobs[i]; if(!job)return;
    var skillBox=document.querySelector('#tlDetail [data-skills-unavailable]');
    var skills=skillBox ? (Array.isArray(job.skills)?job.skills:[]) : Array.prototype.map.call(document.querySelectorAll('#tlDetail .tlf-skill:checked'),function(input){return input.value;});
    var body={ name:(document.getElementById('tlfName').value||''), prompt:(document.getElementById('tlfPrompt').value||''),
      cron_expr:(document.getElementById('tlfCron').value||''), channel:(document.getElementById('tlfChannel').value||''),
      model:(document.getElementById('tlfModel').value||''), skills:skills };
    fetch('/api/cron/'+job.id,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})
      .then(function(){ LaRuche.Toast.show('Cron mis  jour','ok'); tlReload(); });
  }"""
old_tlSaveEdit = html[html.find("  function tlSaveEdit(i){"):html.find("  async function loadCron(el) {")]

new_tlSaveEdit = """  function updateCronEditModelSelect() {
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

"""
html = html.replace(old_tlSaveEdit, new_tlSaveEdit)

# Expose updateCronEditModelSelect
html = re.sub(r'(updateCronModelSelect:updateCronModelSelect)',
              r'\1, updateCronEditModelSelect:updateCronEditModelSelect', html)


with open('laruche-dashboard/src/templates/spa.html', 'w', encoding='utf-8') as f:
    f.write(html)
