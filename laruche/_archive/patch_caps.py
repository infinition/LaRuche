import re

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    html = f.read()

# 1. Update fetchModels
old_fetch_models = """  async function fetchModels(){
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
          var isPreferred = preferredModels[cap] ? (m.name === preferredModels[cap]) : m.is_default;"""

new_fetch_models = """  async function fetchModels(){
    try{
      var selData = await fetch(LaRuche.API.base+'/api/capabilities/selection').then(r=>r.json()).catch(e=>({selection:{}}));
      var serverPreferred = selData.selection || {};
      
      var recapText = "Pour cette discussion : ";
      var caps = Object.keys(serverPreferred);
      if(caps.length > 0) {
        recapText += caps.map(function(c) {
          return c.toUpperCase() + '=' + serverPreferred[c].model;
        }).join(' · ');
      } else {
        recapText += "Auto (aucun modele force)";
      }
      var recapEl = document.getElementById('sessionCapsRecap');
      if(recapEl) recapEl.textContent = recapText;

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
      
      var groups={};
      
      d.models.forEach(function(m){
        var cap=(m.capability||'llm').toLowerCase();
        if(!groups[cap]){groups[cap]=[];}
        groups[cap].push(m);
      });
      
      var allCaps = Object.keys(groups);
      var orderedCaps = ['llm', 'code', 'stt', 'tts', 'vlm', 'vla'];
      var groupOrder = orderedCaps.filter(c => allCaps.includes(c));
      allCaps.forEach(c => { if(!groupOrder.includes(c)) groupOrder.push(c); });
      
      if(LaRuche.Voice && LaRuche.Voice.updateVoiceSelectors) {
        LaRuche.Voice.updateVoiceSelectors(d.models);
      }
      
      groupOrder.forEach(function(cap){
        var section = document.createElement('div');
        section.className = 'mesh-cap-section';
        if(cap === 'code') section.style.border = '1px solid var(--green-dim)';
        
        var hdr = document.createElement('div');
        hdr.className = 'mesh-cap-hdr';
        hdr.innerHTML = '<span>' + cap + (cap==='code'?' <span style="color:var(--green)">[DISTINCT]</span>':'') + '</span> <span>' + groups[cap].length + '</span>';
        section.appendChild(hdr);
        
        var grid = document.createElement('div');
        grid.className = 'mesh-grid';
        
        groups[cap].forEach(function(m){
          var isPreferred = serverPreferred[cap] ? (m.name === serverPreferred[cap].model && m.host === serverPreferred[cap].backend) : m.is_default;"""

html = html.replace(old_fetch_models, new_fetch_models)

# 2. Add Voice Selectors in UI
# TTS
old_tts = """<button class="auto-tts-toggle" id="autoTtsToggle" onclick="LaRuche.Voice.toggleAutoTts()" title="Lecture automatique des reponses">
      <span class="toggle-icon">&#x1F50A;</span>
      <span>Auto-TTS</span>
    </button>"""
new_tts = old_tts + """
    <select id="ttsSelect" class="model-select" onchange="LaRuche.Voice.selectTTS(this.value)" style="max-width:120px;" title="Modele TTS">
      <option value="">Auto (premier detecte)</option>
    </select>"""
html = html.replace(old_tts, new_tts)

# STT
old_stt = """<button class="dictation-btn" id="dictationBtn" onclick="LaRuche.Chat.toggleDictation()" title="Cliquer pour enregistrer / arreter">&#x1F3A4;</button>"""
new_stt = old_stt + """
          <select id="sttSelect" class="model-select" onchange="LaRuche.Voice.selectSTT(this.value)" style="margin-right:8px;max-width:120px;" title="Modele de dictee (STT)">
            <option value="">Auto (premier detecte)</option>
          </select>"""
html = html.replace(old_stt, new_stt)

# G3 Recap
old_recap = """  <div class="page" id="page-chat">
    <div class="chat-layout">"""
new_recap = """  <div class="page" id="page-chat">
    <div id="sessionCapsRecap" style="text-align:center;font-size:10px;color:var(--text-dim);background:var(--bg-panel);padding:2px 0;border-bottom:1px solid var(--border);">Pour cette discussion : Chargement...</div>
    <div class="chat-layout">"""
html = html.replace(old_recap, new_recap)

# 3. Add Voice functions
old_voice_export = """return {
    init:init, speakText:speakText, toggleMic:toggleMic, toggleAutoTts:toggleAutoTts,
    isAutoTts:function(){return autoTtsEnabled;}, cleanTextForTTS:cleanTextForTTS
  };"""
new_voice_export = """return {
    init:init, speakText:speakText, toggleMic:toggleMic, toggleAutoTts:toggleAutoTts,
    isAutoTts:function(){return autoTtsEnabled;}, cleanTextForTTS:cleanTextForTTS,
    selectTTS: function(v){ if(!v)return; var p=v.split('|'); LaRuche.Dashboard.useMeshModel(p[0],p[1],'tts'); },
    selectSTT: function(v){ if(!v)return; var p=v.split('|'); LaRuche.Dashboard.useMeshModel(p[0],p[1],'stt'); },
    updateVoiceSelectors: function(models) {
      var tts = document.getElementById('ttsSelect');
      var stt = document.getElementById('sttSelect');
      if(!tts || !stt || !models) return;
      var currentTts = tts.value; var currentStt = stt.value;
      tts.innerHTML = '<option value="">Auto (premier detecte)</option>';
      stt.innerHTML = '<option value="">Auto (premier detecte)</option>';
      models.forEach(function(m){
        var cap=(m.capability||'llm').toLowerCase();
        if(cap==='tts') tts.innerHTML += '<option value="'+m.host+'|'+m.name+'">'+m.name+'</option>';
        if(cap==='stt') stt.innerHTML += '<option value="'+m.host+'|'+m.name+'">'+m.name+'</option>';
      });
      fetch('/api/voice/status').then(function(r){return r.json();}).then(function(d){
         if(d.tts && d.tts.is_selected) tts.value = d.tts.selected_host + '|' + d.tts.selected_model;
         if(d.stt && d.stt.is_selected) stt.value = d.stt.selected_host + '|' + d.stt.selected_model;
      }).catch(function(){});
    }
  };"""
html = html.replace(old_voice_export, new_voice_export)

# 4. Expose useMeshModel
html = html.replace("return { init:init, openBlueprintForm:openBlueprintForm, instanciateBlueprint:instanciateBlueprint, enter:enter, leave:leave, toggleNodeExpand:toggleNodeExpand };", "return { init:init, openBlueprintForm:openBlueprintForm, instanciateBlueprint:instanciateBlueprint, enter:enter, leave:leave, toggleNodeExpand:toggleNodeExpand, useMeshModel:useMeshModel };")

with open('laruche-dashboard/src/templates/spa.html', 'w', encoding='utf-8') as f:
    f.write(html)
