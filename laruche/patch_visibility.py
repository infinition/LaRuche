import sys
import re

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    html = f.read()

# 1. Modify loadProviders
# Original line:
# html += '<div class="settings-card" style="'+(isActive?'border:1px solid var(--amber);':'')+'">'+
# '<div class="settings-card-title">'+LaRuche.Utils.esc(p.name)+
# (isActive?' <span style="color:var(--amber);font-size:10px;font-weight:normal">(active)</span>':'')+
# '</div>'+
target_hdr = """      html += '<div class="settings-card" style="'+(isActive?'border:1px solid var(--amber);':'')+'">'+
        '<div class="settings-card-title">'+LaRuche.Utils.esc(p.name)+
        (isActive?' <span style="color:var(--amber);font-size:10px;font-weight:normal">(active)</span>':'')+
        '</div>'+"""

new_hdr = """      var isPublic = p.visibility === 'public_proxy';
      var visBadge = isPublic ? '<span style="color:var(--blue);font-size:10px;font-weight:bold;margin-left:8px;">🌐 Public 📡</span>' : '<span style="color:var(--text-dim);font-size:10px;font-weight:bold;margin-left:8px;">🔒 Privé</span>';
      var visToggleBtn = '<button onclick="LaRuche.Settings.toggleVisibility(\\''+id+'\\', \\''+p.provider+'\\', \\''+(p.visibility||'prive')+'\\')" style="margin-left:auto;background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:4px;padding:2px 6px;font-size:10px;cursor:pointer;">'+(isPublic?'Rendre Privé':'Rendre Public')+'</button>';
      html += '<div class="settings-card" style="'+(isActive?'border:1px solid var(--amber);':'')+'">'+
        '<div class="settings-card-title" style="display:flex;align-items:center;"><span>'+LaRuche.Utils.esc(p.name)+'</span>'+
        (isActive?' <span style="color:var(--amber);font-size:10px;font-weight:normal;margin-left:4px;">(active)</span>':'')+
        visBadge+visToggleBtn+
        '</div>'+"""

if target_hdr in html:
    html = html.replace(target_hdr, new_hdr)
else:
    print("Error: Could not find target_hdr")
    sys.exit(1)

# 2. Add toggleVisibility to Settings
# Look for function deleteCredential
toggle_func = """
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
        LaRuche.Toast.show('Visibilité modifiée','ok');
        loadTab('providers');
      } else {
        LaRuche.Toast.show('Erreur: '+(d.error||'?'),'err');
      }
    }).catch(function(e){LaRuche.Toast.show('Erreur: '+e,'err');});
  }

"""
html = html.replace("  function deleteCredential(provider, key) {", toggle_func + "  function deleteCredential(provider, key) {")

# Export it in LaRuche.Settings
html = re.sub(r'(return \{ init:init.*?)(deleteCredential:deleteCredential)(.*?\};)',
              r'\1\2, toggleVisibility:toggleVisibility\3', html)

# 3. Add useMeshModel to Dashboard and add the button to fetchModels
# Add useMeshModel function
use_mesh_model = """
  function useMeshModel(host, name, capability, node_id, base_url) {
    fetch('/api/models/use', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({
      host: host, name: name, capability: capability, node_id: node_id, base_url: base_url
    })}).then(function(r){return r.json();}).then(function(d){
      if(d.status==='ok') {
        LaRuche.Toast.show(name + ' actif pour ' + capability, 'ok');
        fetchModels();
      } else {
        LaRuche.Toast.show('Erreur: '+(d.error||'?'), 'err');
        fetchModels();
      }
    }).catch(function(e){LaRuche.Toast.show('Erreur: '+e, 'err'); fetchModels();});
  }
"""
html = html.replace("  async function fetchModels(){", use_mesh_model + "\n  async function fetchModels(){")

# Append button to card
target_btn = """          card.appendChild(nameDiv);
          card.appendChild(metaDiv);"""

new_btn = """          card.appendChild(nameDiv);
          card.appendChild(metaDiv);
          
          var useBtn = document.createElement('button');
          useBtn.style.cssText = 'background:rgba(245,158,11,0.1); border:1px solid var(--amber); color:var(--amber); border-radius:4px; padding:4px 8px; font-size:10px; cursor:pointer; margin-top:4px; transition:var(--transition-fast); text-align:center;';
          useBtn.textContent = 'Utiliser';
          useBtn.onmouseover = function() { this.style.background = 'rgba(245,158,11,0.2)'; };
          useBtn.onmouseout = function() { this.style.background = 'rgba(245,158,11,0.1)'; };
          useBtn.onclick = function(e){ 
            e.stopPropagation();
            useBtn.disabled = true;
            useBtn.textContent = '...';
            useMeshModel(m.host, m.name, cap, m.node_id, m.base_url); 
          };
          card.appendChild(useBtn);"""

if target_btn in html:
    html = html.replace(target_btn, new_btn)
else:
    print("Error: Could not find target_btn")
    sys.exit(1)


with open('laruche-dashboard/src/templates/spa.html', 'w', encoding='utf-8') as f:
    f.write(html)
