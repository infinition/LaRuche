import sys
import re

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    html = f.read()

# 1. Update the panel-models header
new_panel_hdr = """          <div class="panel-hdr" style="justify-content:space-between; display:flex;">
            <div style="display:flex; align-items:center; gap:8px;">
              <span class="panel-title">&diamondsuit; Services du Mesh</span>
              <span class="panel-badge" id="model-badge" style="background:rgba(245,158,11,.12);color:var(--amber);border:1px solid rgba(245,158,11,.3)">0</span>
            </div>
            <button onclick="LaRuche.Dashboard.fetchModels()" style="background:transparent; border:1px solid var(--border); color:var(--text-dim); border-radius:4px; padding:2px 6px; font-size:10px; cursor:pointer;">&#x21BA; Rescanner</button>
          </div>"""

html = re.sub(
    r'<div class="panel-hdr">\s*<span class="panel-title">&diamondsuit; Models</span>\s*<span class="panel-badge" id="model-badge".*?</span>\s*</div>',
    new_panel_hdr,
    html,
    flags=re.DOTALL
)

# 2. Add CSS
css = """
/* ============================== */
/* MESH CARDS                     */
/* ============================== */
.mesh-cap-section { margin-bottom: 16px; }
.mesh-cap-hdr { font-size: 11px; font-weight: bold; color: var(--amber); text-transform: uppercase; letter-spacing: 1px; margin-bottom: 8px; border-bottom: 1px solid var(--border); padding-bottom: 4px; display:flex; justify-content:space-between;}
.mesh-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 8px; }
.mesh-card { background: rgba(245,158,11,0.03); border: 1px solid rgba(245,158,11,0.2); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 6px; position: relative; transition: var(--transition-fast); }
.mesh-card:hover { border-color: var(--amber); background: rgba(245,158,11,0.08); }
.mesh-card-name { font-size: 12px; font-weight: 600; color: var(--text); word-break: break-all; }
.mesh-card-meta { font-size: 10px; color: var(--text-dim); display: flex; justify-content: space-between; align-items: center; }
.mesh-badge { font-size: 9px; padding: 2px 6px; border-radius: 4px; font-weight: bold; }
.mesh-local { background: rgba(34,197,94,0.15); color: var(--green); border: 1px solid rgba(34,197,94,0.3); }
.mesh-remote { background: rgba(59,130,246,0.15); color: var(--blue); border: 1px solid rgba(59,130,246,0.3); }
"""
html = html.replace('</style>', css + '\n</style>')

# 3. Replace fetchModels
new_fetch_models = """  async function fetchModels(){
    try{
      var r=await fetch(LaRuche.API.base+'/swarm/models');if(!r.ok)return;var d=await r.json();
      var list=document.getElementById('model-list');list.innerHTML='';
      
      var sourcesSet = new Set();
      if(d.models) {
        d.models.forEach(function(m) {
           sourcesSet.add(m.is_local ? 'local' : m.node_id);
        });
      }
      var sourcesCount = sourcesSet.size;
      
      if(!d.models||d.models.length===0){
        list.innerHTML='<div style="font-size:10px;color:var(--text-dim);text-align:center;padding:8px">Aucun service</div>';
        document.getElementById('model-badge').textContent='0 sources';
        return;
      }
      
      document.getElementById('model-badge').textContent=d.models.length + ' modeles sur ' + sourcesCount + ' sources';
      
      var defaultModels=d.default_models||{};
      var groups={},groupOrder=[];
      
      d.models.forEach(function(m){
        var cap=(m.capability||'llm').toLowerCase();
        if(!groups[cap]){groups[cap]=[];groupOrder.push(cap);}
        groups[cap].push(m);
      });
      
      groupOrder.forEach(function(cap){
        var section = document.createElement('div');
        section.className = 'mesh-cap-section';
        
        var hdr = document.createElement('div');
        hdr.className = 'mesh-cap-hdr';
        hdr.innerHTML = '<span>' + cap + '</span> <span>' + groups[cap].length + '</span>';
        section.appendChild(hdr);
        
        var grid = document.createElement('div');
        grid.className = 'mesh-grid';
        
        groups[cap].forEach(function(m){
          var isPreferred = preferredModels[cap] ? (m.name === preferredModels[cap]) : m.is_default;
          var card = document.createElement('div');
          card.className = 'mesh-card';
          if(isPreferred) card.style.borderColor = 'var(--amber)';
          card.title = 'Click to set default for ' + cap;
          
          var nameDiv = document.createElement('div');
          nameDiv.className = 'mesh-card-name';
          nameDiv.textContent = m.name;
          if(isPreferred) nameDiv.innerHTML += ' <span style="color:var(--amber); font-size:10px;">★</span>';
          
          var metaDiv = document.createElement('div');
          metaDiv.className = 'mesh-card-meta';
          
          var sizeSpan = document.createElement('span');
          sizeSpan.textContent = m.size_gb > 0 ? m.size_gb.toFixed(1) + ' GB' : '';
          
          var badgeSpan = document.createElement('span');
          badgeSpan.className = 'mesh-badge ' + (m.is_local ? 'mesh-local' : 'mesh-remote');
          badgeSpan.textContent = m.is_local ? 'LOCAL (' + m.host + ')' : 'MESH (' + (m.node_name || 'unknown') + ')';
          
          metaDiv.appendChild(sizeSpan);
          metaDiv.appendChild(badgeSpan);
          
          card.appendChild(nameDiv);
          card.appendChild(metaDiv);
          
          card.addEventListener('click', function(){ setPreferredModel(m.name, cap); });
          
          grid.appendChild(card);
        });
        
        section.appendChild(grid);
        list.appendChild(section);
      });
      
      lastModelCount=d.models.length;
    } catch(e){}
  }"""

html = re.sub(
    r'async function fetchModels\(\)\{.*?(?=function setPreferredModel)',
    new_fetch_models + '\n\n  ',
    html,
    flags=re.DOTALL
)

# 4. Modify setInterval to refresh models
# Current logic might already refresh fetchModels. Let's see if we can add a specific interval.
# But wait, we can just let `update_mesh.py` do this. Let's write the modified HTML back.

with open('laruche-dashboard/src/templates/spa.html', 'w', encoding='utf-8') as f:
    f.write(html)
