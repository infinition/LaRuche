import re

f = open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8')
c = f.read()
f.close()

# 1. Update Navigation links
c = c.replace(
    '<a href="#memory" data-page="memory">Memoire</a>',
    '<a href="#memory" data-page="memory">Memoire</a>\n    <a href="#skills" data-page="skills">Skills</a>'
)

c = c.replace(
    '<a href="#memory" data-page="memory"><span class="tab-icon">&#x2B22;</span>Memoire</a>',
    '<a href="#memory" data-page="memory"><span class="tab-icon">&#x2B22;</span>Memoire</a>\n  <a href="#skills" data-page="skills"><span class="tab-icon">&#x1F9E0;</span>Skills</a>'
)

# 2. Update pages array
c = c.replace(
    "var pages = ['chat','dashboard','sessions','memory','settings','console','login'];",
    "var pages = ['chat','dashboard','sessions','memory','skills','settings','console','login'];"
)

# 3. Add #page-skills container
# The chat page ends at `<div class="page" id="page-dashboard">` probably, or we can insert before `<div class="toast-container"`
# Let's insert before `<!-- TOAST CONTAINER -->`
skills_page_html = """
<!-- ============================== -->
<!-- SKILLS PAGE                    -->
<!-- ============================== -->
<div class="page" id="page-skills">
  <div class="dash-panels" style="display:grid; grid-template-columns:1fr 1fr; gap:16px; padding:16px; height:100%; overflow:auto;">
    <div class="panel" style="display:flex; flex-direction:column; gap:12px;">
      <div class="panel-header" style="display:flex; justify-content:space-between; align-items:center;">
        <h3>Liste des Skills</h3>
        <button class="btn btn-primary" onclick="LaRuche.Skills.newSkill()">+ Nouveau</button>
      </div>
      <div id="skills-list" style="display:flex; flex-direction:column; gap:8px; overflow-y:auto; flex:1;">
        <!-- Filled by JS -->
      </div>
    </div>
    <div class="panel" style="display:flex; flex-direction:column; gap:12px;">
      <div class="panel-header">
        <h3>Skills proposes (Revue)</h3>
      </div>
      <div id="proposed-skills-list" style="display:flex; flex-direction:column; gap:8px; overflow-y:auto; flex:1;">
        <!-- Filled by JS -->
      </div>
    </div>
  </div>
</div>
"""
c = c.replace('<!-- TOAST CONTAINER -->', skills_page_html + '\n<!-- TOAST CONTAINER -->')

# 4. Add LaRuche.Skills module and handleEvent modifications
skills_js = """
/* ============================== */
/* SKILLS MODULE                  */
/* ============================== */
LaRuche.Skills = (function() {
  function init() {
    //
  }
  function enter() {
    loadSkills();
    loadProposedSkills();
  }
  function leave() {}

  function loadSkills() {
    LaRuche.API.getLocal('/api/skills').then(function(res) {
      var container = document.getElementById('skills-list');
      container.innerHTML = '';
      res.forEach(function(skill) {
        var card = document.createElement('div');
        card.style.cssText = 'padding:12px; background:var(--bg-input); border-radius:8px; border:1px solid var(--border); display:flex; flex-direction:column; gap:8px;';
        
        var header = document.createElement('div');
        header.style.cssText = 'display:flex; justify-content:space-between; align-items:center;';
        var titleContainer = document.createElement('div');
        titleContainer.style.cssText = 'display:flex; align-items:center; gap:8px;';
        var title = document.createElement('strong');
        title.style.color = 'var(--amber)';
        title.textContent = skill.name;
        titleContainer.appendChild(title);
        
        if (skill.source === 'auto-skill' || skill.name.includes('auto')) {
          var badge = document.createElement('span');
          badge.style.cssText = 'background:var(--amber-dim); color:#fff; padding:2px 6px; border-radius:12px; font-size:10px; font-weight:bold;';
          badge.textContent = 'Auto';
          titleContainer.appendChild(badge);
        }
        
        header.appendChild(titleContainer);
        
        var controls = document.createElement('div');
        controls.style.cssText = 'display:flex; gap:8px;';
        
        var toggleBtn = document.createElement('button');
        toggleBtn.className = 'btn ' + (skill.enabled ? 'btn-primary' : '');
        toggleBtn.textContent = skill.enabled ? 'On' : 'Off';
        toggleBtn.onclick = function() { toggleSkill(skill.name); };
        
        var delBtn = document.createElement('button');
        delBtn.className = 'btn';
        delBtn.style.color = 'var(--red)';
        delBtn.textContent = 'X';
        delBtn.onclick = function() { deleteSkill(skill.name); };
        
        controls.appendChild(toggleBtn);
        controls.appendChild(delBtn);
        header.appendChild(controls);
        
        var desc = document.createElement('div');
        desc.style.fontSize = '12px';
        desc.style.color = 'var(--text-dim)';
        desc.textContent = skill.description;
        
        var viewBtn = document.createElement('button');
        viewBtn.className = 'btn';
        viewBtn.textContent = 'Voir OKF';
        viewBtn.style.alignSelf = 'flex-start';
        viewBtn.onclick = function() { viewSkill(skill.name); };
        
        card.appendChild(header);
        card.appendChild(desc);
        card.appendChild(viewBtn);
        container.appendChild(card);
      });
    });
  }

  function loadProposedSkills() {
    LaRuche.API.getLocal('/api/memory/proposed').then(function(res) {
      if(res.status !== 'ok') return;
      var container = document.getElementById('proposed-skills-list');
      container.innerHTML = '';
      var proposed = res.result || [];
      var skillsProposed = proposed.filter(function(p) {
        return p.content && p.content.indexOf('type: skill') !== -1;
      });
      
      if(skillsProposed.length === 0) {
        container.innerHTML = '<div style="color:var(--text-dim); text-align:center; padding:20px;">Aucun skill propose.</div>';
        return;
      }
      
      skillsProposed.forEach(function(item) {
        var card = document.createElement('div');
        card.style.cssText = 'padding:12px; background:var(--bg-input); border-radius:8px; border:1px solid var(--border); display:flex; flex-direction:column; gap:8px;';
        
        var title = document.createElement('strong');
        title.style.color = 'var(--purple)';
        title.textContent = item.label || 'Skill sans nom';
        card.appendChild(title);
        
        var pre = document.createElement('pre');
        pre.style.cssText = 'font-size:10px; max-height:100px; overflow:auto; background:var(--bg); padding:8px; border-radius:4px;';
        pre.textContent = item.content;
        card.appendChild(pre);
        
        var controls = document.createElement('div');
        controls.style.cssText = 'display:flex; gap:8px; margin-top:4px;';
        
        var acceptBtn = document.createElement('button');
        acceptBtn.className = 'btn btn-primary';
        acceptBtn.textContent = 'Accepter';
        acceptBtn.onclick = function() { reviewSkill(item.id, 'accept'); };
        
        var rejectBtn = document.createElement('button');
        rejectBtn.className = 'btn';
        rejectBtn.textContent = 'Rejeter';
        rejectBtn.onclick = function() { reviewSkill(item.id, 'reject'); };
        
        controls.appendChild(acceptBtn);
        controls.appendChild(rejectBtn);
        card.appendChild(controls);
        container.appendChild(card);
      });
    });
  }

  function toggleSkill(name) {
    LaRuche.API.postLocal('/api/skills/'+encodeURIComponent(name)+'/toggle', {}).then(function(){ loadSkills(); });
  }
  
  function deleteSkill(name) {
    if(confirm('Supprimer le skill ' + name + ' ?')) {
      LaRuche.API.delLocal('/api/skills/'+encodeURIComponent(name)).then(function(){ loadSkills(); });
    }
  }
  
  function reviewSkill(itemId, action) {
    LaRuche.API.postLocal('/api/memory/review', { item_id: itemId, action: action, reason: 'Revue UI' }).then(function(res){
      if(res.status === 'ok') {
        LaRuche.Toast.show('Skill ' + (action==='accept'?'accepte':'rejete'), action==='accept'?'ok':'info');
        loadProposedSkills();
        loadSkills();
      }
    });
  }

  function viewSkill(name) {
    LaRuche.API.getLocal('/api/skills/'+encodeURIComponent(name)).then(function(res){
      alert("--- " + name + " ---\\n" + res.content);
    });
  }

  function newSkill() {
    var content = prompt("Entrez le contenu OKF du skill (inclure 'type: skill' et 'name: ...') :");
    if(content) {
      LaRuche.API.postLocal('/api/skills', { content: content }).then(function(){
        LaRuche.Toast.show('Skill cree', 'ok');
        loadSkills();
      }).catch(function(e){ alert("Erreur: "+e); });
    }
  }

  return {
    init: init,
    enter: enter,
    leave: leave,
    loadSkills: loadSkills,
    loadProposedSkills: loadProposedSkills,
    newSkill: newSkill
  };
})();
LaRuche.App.register('skills', LaRuche.Skills);
"""

# We'll inject the LaRuche.Skills module right before `/* 🐝 App Init 🐝 */`
c = c.replace('/* 🐝 App Init 🐝 */', skills_js + '\n/* 🐝 App Init 🐝 */')

# Now we need to modify handleEvent to support skill_applied and skill_proposed
handle_event_updates = """
      case 'skill_applied':
        var chip = document.createElement('div');
        chip.style.cssText = 'display:inline-block; background:rgba(245,158,11,0.2); color:var(--amber); padding:2px 8px; border-radius:12px; font-size:11px; margin-top:4px; margin-bottom:4px;';
        chip.innerHTML = '🧠 Skill appliqué : <strong>' + data.name + '</strong>';
        if (currentAssistantMsg) {
          currentAssistantMsg.appendChild(chip);
        } else {
          var r = addMessage('assistant','');
          r.msgEl.appendChild(chip);
        }
        break;
      case 'skill_proposed':
        LaRuche.Toast.show('✨ Skill né : ' + data.name, 'ok');
        if (location.hash === '#skills') {
          LaRuche.Skills.loadProposedSkills();
        }
        break;
"""

c = c.replace("case 'thinking':", handle_event_updates + "\n      case 'thinking':")

f = open('laruche-dashboard/src/templates/spa.html', 'w', encoding='utf-8')
f.write(c)
f.close()
