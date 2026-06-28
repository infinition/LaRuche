LaRuche.i18n.add({
  'dashboard.swarmConnected':        {fr:'Swarm connecté : {n} nœud(s)', en:'Swarm connected: {n} node(s)'},
  'dashboard.connectionEstablished': {fr:'Connexion au Swarm établie',   en:'Swarm connection established'},
  'dashboard.newNode':               {fr:'Nouveau nœud! Total: {n}',      en:'New node! Total: {n}'},
  'dashboard.nodeJoined':            {fr:'Nœud rejoint le Swarm',         en:'Node joined the Swarm'},
  'dashboard.nodeDisconnected':      {fr:'Nœud déconnecté. Total: {n}',   en:'Node disconnected. Total: {n}'},
  'dashboard.nodeLeft':              {fr:'Nœud quitte le Swarm',          en:'Node left the Swarm'},
  'dashboard.connectionLost':        {fr:'Connexion perdue',              en:'Connection lost'},
  'dashboard.nodeUnreachable':       {fr:'Nœud inaccessible',             en:'Node unreachable'},
  'dashboard.modelActiveFor':        {fr:'{name} actif pour {cap}',       en:'{name} active for {cap}'},
  'dashboard.error':                 {fr:'Erreur: {msg}',                 en:'Error: {msg}'},
  'dashboard.noService':             {fr:'Aucun service',                 en:'No service'},
  'dashboard.modelCount':            {fr:'{count} modèles sur {src} sources', en:'{count} models from {src} sources'},
  'dashboard.capLlm':                {fr:'💬 Conversation (LLM)',         en:'💬 Conversation (LLM)'},
  'dashboard.capAgent':              {fr:'🤖 Agent',                      en:'🤖 Agent'},
  'dashboard.capCode':               {fr:'💻 Code',                       en:'💻 Code'},
  'dashboard.capStt':                {fr:'🎙️ Voix → Texte',              en:'🎙️ Voice → Text'},
  'dashboard.capTts':                {fr:'🔊 Texte → Voix',               en:'🔊 Text → Voice'},
  'dashboard.capVlm':                {fr:'👁️ Vision',                     en:'👁️ Vision'},
  'dashboard.capVla':                {fr:'🦾 Vision-Action',               en:'🦾 Vision-Action'},
  'dashboard.myLocalModel':          {fr:'Mon modèle local',               en:'My local model'},
  'dashboard.remoteModel':           {fr:'Modèle distant (ruche {name})',  en:'Remote model (ruche {name})'},
  'dashboard.selected':              {fr:'● SÉLECTIONNÉ',                 en:'● SELECTED'},
  'dashboard.myLocal':               {fr:'🖥️ à moi (local)',               en:'🖥️ mine (local)'},
  'dashboard.used':                  {fr:'✓ Utilisé',                     en:'✓ In use'},
  'dashboard.use':                   {fr:'Utiliser',                      en:'Use'},
  'dashboard.waitingData':           {fr:'En attente de données...',       en:'Waiting for data...'},
  'dashboard.initialized':           {fr:'Dashboard initialisé',           en:'Dashboard initialized'},
  'dashboard.noBlueprintAvailable':  {fr:'Aucun blueprint disponible',     en:'No blueprint available'},
  'dashboard.selectBlueprint':       {fr:"Sélectionnez un blueprint pour l'instancier en tant que tâche cron.", en:"Select a blueprint to instantiate it as a cron task."},
  'dashboard.instantiate':           {fr:'Instancier',                    en:'Instantiate'},
  'dashboard.blueprintSuccess':      {fr:'Blueprint instancié avec succès', en:'Blueprint instantiated successfully'},
  'dashboard.blueprintError':        {fr:"Erreur d'instanciation",         en:'Instantiation error'},
  'dashboard.remoteBadge':           {fr:'🐝 {name} · distant',            en:'🐝 {name} · remote'},
});

LaRuche.Dashboard = (function(){
  var pollTimers = [];
  var connected = false;
  var maxTps = 1;
  var lastNodeCount = -1;
  var lastModelCount = -1;
  var lastActivityTs = '';
  var activityInitialized = false;
  var logCount = 0;
  var logUserScrolled = false;
  var expandedNodes = new Set();
  var stableNodes = new Map();
  var NODE_GRACE_POLLS = 10;
  var pendingNodeCount = -1;
  var pendingNodeCountHits = 0;
  var NODE_COLORS = ['#22c55e','#3b82f6','#f59e0b','#a855f7','#06b6d4','#ef4444','#ec4899','#14b8a6','#f97316','#8b5cf6'];

  /* Infer detail popover state */
  var inferDetailMap = new Map();
  var inferDetailCounter = 0;
  var activePopover = null;
  var popoverHoverTimer = null;
  var popoverPinned = false;

  /* Stats chart */
  var statsCardOpen = false;
  var statsMetric = 'all';
  var metricsHistory = [];
  var nodeEvents = [];
  var statsRefreshTimer = null;
  var chartCrosshairX = -1;
  var viewTMin = null;
  var viewTMax = null;
  var isPanning = false;
  var panStartX = 0;
  var panStartTMin = 0;
  var panStartTMax = 0;
  var lastPinchDist = 0;
  var preferredModels = (function(){try{return JSON.parse(localStorage.getItem('laruche_preferred_models')||'{}');}catch(e){return {};}})();
  var modelPriority = (function(){try{return JSON.parse(localStorage.getItem('laruche_model_priority')||'[]');}catch(e){return [];}})();

  var CHART_COLORS = {cpu:{line:'#22c55e',fill:'rgba(34,197,94,.10)'},ram:{line:'#f59e0b',fill:'rgba(245,158,11,.10)'},tps:{line:'#fbbf24',fill:'rgba(251,191,36,.10)'},queue:{line:'#a855f7',fill:'rgba(168,85,247,.10)'},gpu:{line:'#c084fc',fill:'rgba(192,132,252,.10)'},vram:{line:'#e879f9',fill:'rgba(232,121,249,.10)'}};
  var METRIC_KEYS = ['cpu','ram','gpu','vram','tps','queue'];
  var METRIC_LABELS = {cpu:'CPU %',ram:'RAM %',gpu:'GPU %',vram:'VRAM %',tps:'Tokens/s',queue:'Queue'};
  var METRIC_FIELDS = {cpu:'cpu_pct',ram:'ram_pct',gpu:'gpu_pct',vram:'vram_pct',tps:'tokens_per_sec',queue:'queue_depth'};

  function esc(s){return LaRuche.Utils.esc(s);}
  function clamp(v,lo,hi){return LaRuche.Utils.clamp(v,lo,hi);}
  function fmtMB(mb){return LaRuche.Utils.fmtMB(mb);}

  function capBadge(cap){var c=LaRuche.Utils.normalizeCap(cap);if(c.includes('vlm')||c.includes('vla'))return'b-vlm';if(c.includes('code'))return'b-code';if(c.includes('audio')||c==='stt')return'b-audio';if(c==='tts')return'b-tts';if(c.includes('image'))return'b-image';if(c==='embed')return'b-embed';if(c.includes('rag'))return'b-rag';if(c==='agent')return'b-agent';return'b-llm';}
  function setGauge(id,pct,color){var el=document.getElementById(id);if(!el)return;var outer=el.querySelector('.hex-outer');if(outer){outer.style.setProperty('--gauge',clamp(pct,0,100));if(color)outer.style.setProperty('--gauge-color',color);}}
  function gaugeColor(pct){if(pct>80)return'var(--red)';if(pct>50)return'var(--amber)';return'var(--green)';}
  function tempColor(temp){if(temp>85)return'fill-red';if(temp>65)return'fill-amber';return'fill-cyan';}
  function estimateSpeedup(nodes){if(nodes.length<=1)return 1.0;var best=0,total=0;for(var i=0;i<nodes.length;i++){var t=nodes[i].tokens_per_sec||0;total+=t;if(t>best)best=t;}if(best<=0)return 1+(nodes.length-1)*0.85;return total/best;}

  function updateStableNodes(incomingNodes){
    var incomingIds=new Set();
    for(var i=0;i<incomingNodes.length;i++){var n=incomingNodes[i];var id=n.node_id||(n.host+':'+(n.port||8419));incomingIds.add(id);stableNodes.set(id,{node:n,missedPolls:0});}
    for(var entry of stableNodes){if(!incomingIds.has(entry[0])){entry[1].missedPolls++;if(entry[1].missedPolls>=NODE_GRACE_POLLS)stableNodes.delete(entry[0]);}}
    var result=[];for(var e of stableNodes.values())result.push(e.node);return result;
  }

  function toggleNodeExpand(nodeId){
    if(expandedNodes.has(nodeId))expandedNodes.delete(nodeId);else expandedNodes.add(nodeId);
    var el=document.querySelector('[data-node-id="'+CSS.escape(nodeId)+'"]');if(el)el.classList.toggle('expanded');
  }

  function buildNodeDetail(n){
    var html='';
    if(n.cpu_usage_pct!=null){var cpuPct=clamp(n.cpu_usage_pct,0,100);var cpuFill=cpuPct>80?'fill-red':cpuPct>50?'fill-amber':'fill-green';html+='<div class="n-detail-row"><span class="n-detail-label">CPU</span><div class="n-detail-track"><div class="n-detail-fill '+cpuFill+'" style="width:'+cpuPct+'%"></div></div><span class="n-detail-val">'+cpuPct.toFixed(0)+'%</span></div>';}
    if(n.memory_usage_pct!=null){var memPct=clamp(n.memory_usage_pct,0,100);var memFill=memPct>80?'fill-red':memPct>50?'fill-amber':'fill-green';var memLabel=n.memory_used_mb?fmtMB(n.memory_used_mb)+'/'+fmtMB(n.memory_total_mb):memPct.toFixed(0)+'%';html+='<div class="n-detail-row"><span class="n-detail-label">RAM</span><div class="n-detail-track"><div class="n-detail-fill '+memFill+'" style="width:'+memPct+'%"></div></div><span class="n-detail-val">'+esc(memLabel)+'</span></div>';}
    else if(n.memory_total_mb!=null){html+='<div class="n-detail-stat"><span class="n-detail-stat-label">RAM Total</span><span class="n-detail-stat-val">'+fmtMB(n.memory_total_mb)+'</span></div>';}
    if(n.vram_usage_pct!=null){var vramPct=clamp(n.vram_usage_pct,0,100);var vramFill=vramPct>80?'fill-red':vramPct>50?'fill-amber':'fill-purple';var vramLabel=n.vram_used_mb?fmtMB(n.vram_used_mb)+'/'+fmtMB(n.vram_total_mb):vramPct.toFixed(0)+'%';html+='<div class="n-detail-row"><span class="n-detail-label">VRAM</span><div class="n-detail-track"><div class="n-detail-fill '+vramFill+'" style="width:'+vramPct+'%"></div></div><span class="n-detail-val">'+esc(vramLabel)+'</span></div>';}
    else if(n.vram_total_mb!=null&&n.vram_total_mb>0){html+='<div class="n-detail-stat"><span class="n-detail-stat-label">VRAM Total</span><span class="n-detail-stat-val">'+fmtMB(n.vram_total_mb)+'</span></div>';}
    if(n.temperature_c!=null){var temp=n.temperature_c;var tFill=tempColor(temp);var tPct=clamp((temp/100)*100,0,100);html+='<div class="n-detail-row"><span class="n-detail-label">Temp</span><div class="n-detail-track"><div class="n-detail-fill '+tFill+'" style="width:'+tPct+'%"></div></div><span class="n-detail-val">'+temp.toFixed(0)+'&deg;C</span></div>';}
    else if(n.gpu_temperature_c!=null){var gTemp=n.gpu_temperature_c;var gtFill=tempColor(gTemp);var gtPct=clamp((gTemp/100)*100,0,100);html+='<div class="n-detail-row"><span class="n-detail-label">GPU</span><div class="n-detail-track"><div class="n-detail-fill '+gtFill+'" style="width:'+gtPct+'%"></div></div><span class="n-detail-val">'+gTemp.toFixed(0)+'&deg;C</span></div>';}
    var tps=n.tokens_per_sec?n.tokens_per_sec.toFixed(1):'0.0';
    html+='<div class="n-detail-stat"><span class="n-detail-stat-label">Tokens/sec</span><span class="n-detail-stat-val">'+tps+' t/s</span></div>';
    html+='<div class="n-detail-stat"><span class="n-detail-stat-label">Queue</span><span class="n-detail-stat-val">'+(n.queue_depth||0)+'</span></div>';
    if(!html)html='<div style="font-size:9px;color:var(--text-dim);padding:2px 0">No detailed metrics</div>';
    return html;
  }

  function renderSharding(nodes){
    var section=document.getElementById('shard-section');
    if(nodes.length<2){section.classList.remove('active');return;}
    section.classList.add('active');
    var speedup=estimateSpeedup(nodes);
    document.getElementById('shard-speedup').textContent=speedup.toFixed(1)+'x with '+nodes.length+' nodes';
    var nodeWeights=[],totalWeight=0;
    for(var i=0;i<nodes.length;i++){var w=nodes[i].vram_total_mb||nodes[i].memory_total_mb||1024;nodeWeights.push(w);totalWeight+=w;}
    var nodePcts=nodeWeights.map(function(w){return(w/totalWeight)*100;});
    var layerGroups=['Attention','FFN','Embedding','Output'];
    var layersEl=document.getElementById('shard-layers');layersEl.innerHTML='';
    for(var g=0;g<layerGroups.length;g++){
      var row=document.createElement('div');row.className='shard-layer-row';
      var segmentsHtml='';
      for(var j=0;j<nodes.length;j++){var color=NODE_COLORS[j%NODE_COLORS.length];segmentsHtml+='<div class="shard-segment" style="width:'+nodePcts[j].toFixed(1)+'%;background:'+color+';opacity:'+(0.6+(g%2)*0.15)+'"></div>';}
      row.innerHTML='<span class="shard-layer-label">'+layerGroups[g]+'</span><div class="shard-layer-bar">'+segmentsHtml+'</div>';
      layersEl.appendChild(row);
    }
    var legendEl=document.getElementById('shard-legend');legendEl.innerHTML='';
    for(var k=0;k<nodes.length;k++){var color=NODE_COLORS[k%NODE_COLORS.length];var name=nodes[k].name||('Node '+(k+1));legendEl.innerHTML+='<div class="shard-legend-item"><div class="shard-legend-dot" style="background:'+color+'"></div>'+esc(name)+' ('+nodePcts[k].toFixed(0)+'%)</div>';}
  }

  /* Log */
  function highlightMsg(raw){
    var s=esc(raw);
    s=s.replace(/\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}(?::\d+)?)\b/g,'<span class="hl-ip">$1</span>');
    s=s.replace(/\b(\d+)\s*(tokens?\b)/gi,'<span class="hl-tok">$1 $2</span>');
    s=s.replace(/(\d+\.?\d*)\s*(t\/s)/g,'<span class="hl-tok">$1 $2</span>');
    s=s.replace(/\b(in\s+\d+m?s)/gi,'<span class="hl-tok">$1</span>');
    s=s.replace(/(via\s+)([^\s,]+)/g,'$1<span class="hl-model">$2</span>');
    s=s.replace(/(\[System\]:?\s*)([^|]*)/g,'<span class="hl-sys">$1</span><span class="hl-prompt">$2</span>');
    return s;
  }

  function addLog(tag,cls,msg,serverTs,detail){
    var log=document.getElementById('log');
    var ts;
    if(serverTs){try{ts=LaRuche.Utils.fmtTime(new Date(serverTs));}catch(e){ts=LaRuche.Utils.fmtTime(new Date());}}
    else ts=LaRuche.Utils.fmtTime(new Date());
    var row=document.createElement('div');row.className='log-row';
    row.setAttribute('data-tag', (tag || '').toLowerCase());
    if(detail&&detail.full_prompt){var id=++inferDetailCounter;inferDetailMap.set(id,detail);row.setAttribute('data-infer-id',id);}
    row.innerHTML='<span class="log-t">'+ts+'</span><span class="log-tag '+cls+'">'+esc(tag)+'</span><span class="log-msg">'+highlightMsg(msg)+'</span>';
    log.appendChild(row);
    while(log.children.length>200){var removed=log.firstChild;if(removed&&removed.dataset&&removed.dataset.inferId)inferDetailMap.delete(parseInt(removed.dataset.inferId));log.removeChild(removed);}
    logCount=Math.min(logCount+1,200);
    document.getElementById('log-badge').textContent=logCount;
    var logBody=document.getElementById('log-body');
    if(!logUserScrolled)logBody.scrollTop=logBody.scrollHeight;
  }

  /* Popover for INFER log entries */
  function showInferPopover(anchorEl,detail){
    hideInferPopover();
    var pop=document.createElement('div');pop.className='log-popover';
    pop.innerHTML='<div class="log-popover-hdr"><div class="pop-meta">'+(detail.model_used?'<span class="pop-model">'+esc(detail.model_used)+'</span>':'')+(detail.tokens_generated?'<span class="pop-tok">'+detail.tokens_generated+' tokens</span>':'')+(detail.latency_ms?'<span>'+detail.latency_ms+'ms</span>':'')+'</div><button class="log-popover-close">&times;</button></div><div class="log-popover-body"><div class="log-popover-section"><div class="pop-label">PROMPT</div><div class="pop-content">'+esc(detail.full_prompt||'')+'</div></div><div class="log-popover-section"><div class="pop-label">RESPONSE</div><div class="pop-content">'+esc(detail.full_response||'(empty)')+'</div></div></div>';
    document.body.appendChild(pop);activePopover=pop;
    var rect=anchorEl.getBoundingClientRect();var popH=300,popW=Math.min(520,window.innerWidth*0.9);
    var top=rect.bottom+4;if(top+popH>window.innerHeight)top=rect.top-popH-4;if(top<4)top=4;
    var left=Math.min(rect.left,window.innerWidth-popW-8);if(left<4)left=4;
    pop.style.top=top+'px';pop.style.left=left+'px';
    requestAnimationFrame(function(){pop.classList.add('visible');});
    pop.querySelector('.log-popover-close').addEventListener('click',function(e){e.stopPropagation();hideInferPopover();});
    pop.addEventListener('mouseenter',function(){clearTimeout(popoverHoverTimer);});
    pop.addEventListener('mouseleave',function(){if(!popoverPinned)popoverHoverTimer=setTimeout(hideInferPopover,200);});
  }
  function hideInferPopover(){if(activePopover){activePopover.remove();activePopover=null;popoverPinned=false;}clearTimeout(popoverHoverTimer);}

  /* Fetchers */
  async function fetchSwarm(){
    try{
      var r=await fetch(LaRuche.API.base+'/swarm');if(!r.ok)throw new Error('HTTP '+r.status);var d=await r.json();
      var stableNodeList=updateStableNodes(d.nodes);var stableCount=stableNodeList.length;
      document.getElementById('kpi-nodes').textContent=stableCount;
      document.getElementById('kpi-tps').textContent=d.collective_tps.toFixed(1);
      document.getElementById('kpi-queue').textContent=d.collective_queue;
      setGauge('hg-nodes',Math.min(stableCount*33,100),'var(--green)');
      if(d.collective_tps>maxTps)maxTps=d.collective_tps;
      setGauge('hg-tps',(d.collective_tps/maxTps)*100,'var(--amber)');
      setGauge('hg-queue',Math.min(d.collective_queue*10,100),d.collective_queue>5?'var(--red)':'var(--blue)');
      document.getElementById('s-ram').textContent=d.total_ram_mb>0?fmtMB(d.total_ram_mb):'\u2014';
      document.getElementById('s-vram').textContent=d.total_vram_mb>0?fmtMB(d.total_vram_mb):'\u2014';
      document.getElementById('s-tps').textContent=d.collective_tps.toFixed(1)+' t/s';
      document.getElementById('s-queue').textContent=d.collective_queue;
      document.getElementById('swarm-badge').textContent=stableCount+' node'+(stableCount>1?'s':'');
      var pillSwarm=document.getElementById('pill-swarm');
      if(stableCount>1){var speedup=estimateSpeedup(stableNodeList);document.getElementById('pill-swarm-text').textContent='Swarm: '+stableCount+' nodes \u00B7 '+speedup.toFixed(1)+'x';pillSwarm.classList.add('active');}
      else pillSwarm.classList.remove('active');
      if(!connected){connected=true;LaRuche.Toast.show(LaRuche.i18n.t('dashboard.swarmConnected',{n:stableCount}),'ok');addLog('NET','log-ok',LaRuche.i18n.t('dashboard.connectionEstablished'));lastNodeCount=stableCount;}
      if(lastNodeCount>=0&&stableCount!==lastNodeCount){
        if(stableCount===pendingNodeCount)pendingNodeCountHits++;else{pendingNodeCount=stableCount;pendingNodeCountHits=1;}
        if(pendingNodeCountHits>=2){
          if(stableCount>lastNodeCount){LaRuche.Toast.show(LaRuche.i18n.t('dashboard.newNode',{n:stableCount}),'ok');addLog('Miel','log-ok',LaRuche.i18n.t('dashboard.nodeJoined'));}
          else{LaRuche.Toast.show(LaRuche.i18n.t('dashboard.nodeDisconnected',{n:stableCount}),'warn');addLog('Miel','log-warn',LaRuche.i18n.t('dashboard.nodeLeft'));}
          lastNodeCount=stableCount;pendingNodeCount=-1;pendingNodeCountHits=0;
        }
      } else{pendingNodeCount=-1;pendingNodeCountHits=0;}
      /* Render node list */
      var list=document.getElementById('node-list');list.innerHTML='';
      for(var i=0;i<stableNodeList.length;i++){
        var n=stableNodeList[i];var nodeId=n.node_id||(n.host+':'+(n.port||8419));
        var tps=n.tokens_per_sec?n.tokens_per_sec.toFixed(1):'0.0';var q=n.queue_depth||0;
        var normalizedCaps=[];(n.capabilities||[]).forEach(function(c){var cc=LaRuche.Utils.normalizeCap(c);if(cc&&normalizedCaps.indexOf(cc)===-1)normalizedCaps.push(cc);});
        var caps=normalizedCaps.map(function(c){return'<span class="badge '+capBadge(c)+'">'+esc(c)+'</span>';}).join('');
        if(!caps)caps='<span class="badge b-rag">none</span>';
        var modelHtml=n.model?'<div class="n-model">\u25C8 '+esc(n.model)+'</div>':'';
        var hostLabel=n.host+((n.port&&n.port!==8419)?(':'+n.port):'');
        var isExpanded=expandedNodes.has(nodeId);
        var item=document.createElement('div');item.className='n-item'+(isExpanded?' expanded':'');item.setAttribute('data-node-id',nodeId);
        item.innerHTML='<div class="n-item-row" onclick="LaRuche.Dashboard.toggleNodeExpand(\''+esc(nodeId.replace(/'/g,"\\'"))+'\')">'+'<span class="expand-icon">\u25B6</span><div class="n-hex '+(q>3?'busy':'ok')+'"></div><div class="n-info"><div class="n-name">'+esc(n.name||'Unknown')+'</div>'+modelHtml+'<div class="n-meta">'+esc(hostLabel)+' \u00B7 '+tps+' t/s \u00B7 Q:'+q+'</div></div><div class="n-right"><div class="badges">'+caps+'</div></div></div><div class="n-detail">'+buildNodeDetail(n)+'</div>';
        list.appendChild(item);
      }
      renderSharding(stableNodeList);
    } catch(e){
      if(connected){connected=false;LaRuche.Toast.show(LaRuche.i18n.t('dashboard.connectionLost'),'err');addLog('NET','log-err',LaRuche.i18n.t('dashboard.nodeUnreachable'));}
    }
  }

  async function fetchStatus(){
    try{
      var r=await fetch(LaRuche.API.base+'/api/status');if(!r.ok)return;var d=await r.json();
      var cpu=d.cpu_usage_pct||0;
      document.getElementById('kpi-cpu').textContent=cpu.toFixed(0)+'%';
      document.getElementById('r-cpu').textContent=cpu.toFixed(0)+'%';
      document.getElementById('b-cpu').style.width=clamp(cpu,0,100)+'%';
      document.getElementById('b-cpu').className='res-fill '+(cpu>80?'fill-red':cpu>50?'fill-amber':'fill-green');
      setGauge('hg-cpu',cpu,gaugeColor(cpu));
      var mem=d.memory_usage_pct||0;
      var memLabel=d.memory_used_mb?fmtMB(d.memory_used_mb)+'/'+fmtMB(d.memory_total_mb):mem.toFixed(0)+'%';
      document.getElementById('kpi-ram').textContent=mem.toFixed(0)+'%';
      document.getElementById('r-mem').textContent=memLabel;
      document.getElementById('b-mem').style.width=clamp(mem,0,100)+'%';
      document.getElementById('b-mem').className='res-fill '+(mem>80?'fill-red':mem>50?'fill-amber':'fill-green');
      setGauge('hg-ram',mem,gaugeColor(mem));
      // GPU bar
      var gpuRow=document.getElementById('gpu-row');
      if(d.gpu_usage_pct!=null){
        gpuRow.style.display='';
        var gpuPct=clamp(d.gpu_usage_pct,0,100);
        document.getElementById('r-gpu').textContent=gpuPct.toFixed(0)+'%';
        document.getElementById('b-gpu').style.width=gpuPct+'%';
        document.getElementById('b-gpu').className='res-fill '+(gpuPct>80?'fill-red':gpuPct>50?'fill-amber':'fill-purple');
      } else gpuRow.style.display='none';
      var vramRow=document.getElementById('vram-row');
      if(d.vram_usage_pct!=null||d.vram_used_mb!=null||d.vram_total_mb!=null){
        vramRow.style.display='';
        var vramPct=d.vram_usage_pct||(d.vram_total_mb>0?((d.vram_used_mb||0)/d.vram_total_mb)*100:0);
        var vramLabel=d.vram_used_mb!=null&&d.vram_total_mb!=null?fmtMB(d.vram_used_mb)+'/'+fmtMB(d.vram_total_mb):vramPct.toFixed(0)+'%';
        document.getElementById('r-vram').textContent=vramLabel;
        document.getElementById('b-vram').style.width=clamp(vramPct,0,100)+'%';
        document.getElementById('b-vram').className='res-fill '+(vramPct>80?'fill-red':vramPct>50?'fill-amber':'fill-purple');
      } else vramRow.style.display='none';
      // GPU hex gauge
      var hgGpu=document.getElementById('hg-gpu');
      if(d.gpu_usage_pct!=null||d.accelerator_usage_pct!=null){
        var gpuPct=d.gpu_usage_pct||d.accelerator_usage_pct||0;
        hgGpu.style.display='';
        document.getElementById('kpi-gpu').textContent=gpuPct.toFixed(0)+'%';
        setGauge('hg-gpu',gpuPct,gpuPct>80?'var(--red)':gpuPct>50?'var(--amber)':'var(--purple)');
      }
      // VRAM hex gauge
      var hgVram=document.getElementById('hg-vram');
      if(d.vram_used_mb!=null&&d.vram_total_mb!=null&&d.vram_total_mb>0){
        hgVram.style.display='';
        var vPct2=(d.vram_used_mb/d.vram_total_mb)*100;
        document.getElementById('kpi-vram').textContent=fmtMB(d.vram_used_mb);
        setGauge('hg-vram',vPct2,vPct2>80?'var(--red)':vPct2>50?'var(--amber)':'var(--purple)');
      }
      var tempRow=document.getElementById('temp-row');
      var tempVal=d.temperature_c!=null?d.temperature_c:(d.gpu_temperature_c!=null?d.gpu_temperature_c:null);
      if(tempVal!=null){tempRow.style.display='';var tPct=clamp((tempVal/100)*100,0,100);document.getElementById('r-temp').textContent=tempVal.toFixed(0)+'\u00B0C';document.getElementById('b-temp').style.width=tPct+'%';document.getElementById('b-temp').className='res-fill '+tempColor(tempVal);}
      else tempRow.style.display='none';
      var tps=d.tokens_per_sec||0;if(tps>maxTps)maxTps=tps;
      document.getElementById('r-tps').textContent=tps.toFixed(1)+' t/s';
      document.getElementById('b-tps').style.width=clamp((tps/maxTps)*100,0,100)+'%';
      var q=d.queue_depth||0;document.getElementById('r-queue').textContent=q;document.getElementById('b-queue').style.width=clamp(q*10,0,100)+'%';
      document.getElementById('kpi-uptime').textContent=LaRuche.Utils.fmtDuration(d.uptime_secs*1000);
      setGauge('hg-uptime',Math.min((d.uptime_secs/86400)*100,100),'var(--cyan)');
    } catch(e){}
  }

  
  function useMeshModel(host, name, capability, node_id, base_url) {
    fetch('/api/models/use', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({
      host: host, name: name, capability: capability, node_id: node_id, base_url: base_url
    })}).then(function(r){return r.json();}).then(function(d){
      if(d.status==='ok') {
        LaRuche.Toast.show(LaRuche.i18n.t('dashboard.modelActiveFor',{name:name,cap:capability}), 'ok');
        // P7: re-fetch + re-render the top dropdown, the mesh, the capabilities recap and the voice
        LaRuche.refreshAll();
      } else {
        LaRuche.Toast.show(LaRuche.i18n.t('dashboard.error',{msg:(d.error||'?')}), 'err');
        fetchModels();
      }
    }).catch(function(e){LaRuche.Toast.show(LaRuche.i18n.t('dashboard.error',{msg:e}), 'err'); fetchModels();});
  }

  async function fetchModels(){
    try{
      var r=await fetch(LaRuche.API.base+'/swarm/models');if(!r.ok)return;var d=await r.json();
      var selData = await fetch('/api/capabilities/selection').then(r=>r.json()).catch(e=>({selection:{}}));
      var serverPreferred = selData.selection || {};
      var list=document.getElementById('model-list');list.innerHTML='';
      
      var sourcesSet = new Set();
      if(d.models) {
        d.models.forEach(function(m) {
           sourcesSet.add(m.is_local ? 'local' : m.node_id);
        });
      }
      var sourcesCount = sourcesSet.size;
      
      if(!d.models||d.models.length===0){
        list.innerHTML='<div style="font-size:10px;color:var(--text-dim);text-align:center;padding:8px">'+LaRuche.i18n.t('dashboard.noService')+'</div>';
        document.getElementById('model-badge').textContent='0 sources';
        return;
      }
      
      document.getElementById('model-badge').textContent=LaRuche.i18n.t('dashboard.modelCount',{count:d.models.length,src:sourcesCount});
      
      var groups={};
      
      d.models.forEach(function(m){
        var cap=(m.capability||'llm').toLowerCase();
        if(!groups[cap]){groups[cap]=[];}
        // DEDUP: the same model (name + source) must appear only once (otherwise "Used
        // everywhere" and duplicate cards when a model comes from multiple sources).
        var dup = groups[cap].some(function(x){ return x.name===m.name && x.host===m.host && x.node_id===m.node_id; });
        if(!dup) groups[cap].push(m);
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
        
        var CAP_LABEL = { llm:LaRuche.i18n.t('dashboard.capLlm'), agent:LaRuche.i18n.t('dashboard.capAgent'), code:LaRuche.i18n.t('dashboard.capCode'), stt:LaRuche.i18n.t('dashboard.capStt'), tts:LaRuche.i18n.t('dashboard.capTts'), vlm:LaRuche.i18n.t('dashboard.capVlm'), vla:LaRuche.i18n.t('dashboard.capVla') };
        var hdr = document.createElement('div');
        hdr.className = 'mesh-cap-hdr';
        hdr.innerHTML = '<span>' + (CAP_LABEL[cap]||cap) + '</span> <span>' + groups[cap].length + '</span>';
        section.appendChild(hdr);
        
        var grid = document.createElement('div');
        grid.className = 'mesh-grid';
        
        groups[cap].forEach(function(m){
          // Selected = matches EXACTLY the server choice for this capability (name + source).
          // No fallback on is_default (which could mark several cards as "Used").
          var _sp = serverPreferred[cap];
          var isPreferred = !!(_sp && m.name === _sp.model && (_sp.backend == null || m.host === _sp.backend));
          var card = document.createElement('div');
          card.className = 'mesh-card';
          if(isPreferred){ card.style.borderColor = 'var(--amber)'; card.style.background = 'rgba(245,158,11,0.06)'; }
          card.title = m.is_local ? LaRuche.i18n.t('dashboard.myLocalModel') : LaRuche.i18n.t('dashboard.remoteModel',{name:(m.node_name||'pair')});

          var nameDiv = document.createElement('div');
          nameDiv.className = 'mesh-card-name';
          nameDiv.textContent = m.name;
          if(isPreferred) nameDiv.innerHTML += ' <span style="background:var(--amber);color:var(--bg);font-size:9px;font-weight:700;padding:0 5px;border-radius:7px;margin-left:4px;vertical-align:middle;">'+LaRuche.i18n.t('dashboard.selected')+'</span>';

          var metaDiv = document.createElement('div');
          metaDiv.className = 'mesh-card-meta';

          var sizeSpan = document.createElement('span');
          sizeSpan.textContent = m.size_gb > 0 ? m.size_gb.toFixed(1) + ' GB' : '';

          // CLEAR provenance: 🖥️ my local · 🐝 remote (which ruche).
          var badgeSpan = document.createElement('span');
          badgeSpan.className = 'mesh-badge ' + (m.is_local ? 'mesh-local' : 'mesh-remote');
          badgeSpan.textContent = m.is_local ? LaRuche.i18n.t('dashboard.myLocal') : LaRuche.i18n.t('dashboard.remoteBadge',{name:(m.node_name||'pair')});

          metaDiv.appendChild(sizeSpan);
          metaDiv.appendChild(badgeSpan);

          card.appendChild(nameDiv);
          card.appendChild(metaDiv);

          var useBtn = document.createElement('button');
          useBtn.style.cssText = 'background:rgba(245,158,11,0.1); border:1px solid var(--amber); color:var(--amber); border-radius:4px; padding:4px 8px; font-size:10px; cursor:pointer; margin-top:4px; transition:var(--transition-fast); text-align:center;';
          if(isPreferred){
            useBtn.textContent = LaRuche.i18n.t('dashboard.used');
            useBtn.style.background = 'var(--amber)'; useBtn.style.color = 'var(--bg)'; useBtn.disabled = true; useBtn.style.cursor = 'default';
          } else {
            useBtn.textContent = LaRuche.i18n.t('dashboard.use');
            useBtn.onmouseover = function() { this.style.background = 'rgba(245,158,11,0.2)'; };
            useBtn.onmouseout = function() { this.style.background = 'rgba(245,158,11,0.1)'; };
            useBtn.onclick = function(e){
              e.stopPropagation();
              useBtn.disabled = true;
              useBtn.textContent = '...';
              useMeshModel(m.host, m.name, cap, m.node_id, m.base_url);
            };
          }
          card.appendChild(useBtn);
          
          card.addEventListener('click', function(){ setPreferredModel(m.name, cap); });
          
          grid.appendChild(card);
        });
        
        section.appendChild(grid);
        list.appendChild(section);
      });
      
      lastModelCount=d.models.length;
    } catch(e){}
  }

  function setPreferredModel(name,capability){
    capability=capability||'llm';preferredModels[capability]=name;localStorage.setItem('laruche_preferred_models',JSON.stringify(preferredModels));
    LaRuche.Toast.show('Default '+capability+' model: '+name,'ok');addLog('MODEL','log-ok','Default '+capability+' set to '+name);fetchModels();
    fetch('/config/default_model',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({model:name,capability:capability})}).catch(function(){});
  }
  function moveModelPriority(name,dir){
    var idx=modelPriority.indexOf(name);if(idx<0)return;var newIdx=idx+dir;if(newIdx<0||newIdx>=modelPriority.length)return;
    var tmp=modelPriority[newIdx];modelPriority[newIdx]=modelPriority[idx];modelPriority[idx]=tmp;
    localStorage.setItem('laruche_model_priority',JSON.stringify(modelPriority));fetchModels();
  }

  async function fetchActivity(){
    try{
      var r=await fetch(LaRuche.API.base+'/activity');if(!r.ok)return;var d=await r.json();
      if(!activityInitialized){var recent=(d.logs||[]).slice(-30);recent.forEach(function(log){var detail=(log.full_prompt||log.full_response)?log:null;addLog(log.tag,log.level,log.message,log.timestamp,detail);});if(d.logs&&d.logs.length>0)lastActivityTs=d.logs[d.logs.length-1].timestamp;activityInitialized=true;return;}
      d.logs.forEach(function(log){if(log.timestamp>lastActivityTs){var detail=(log.full_prompt||log.full_response)?log:null;addLog(log.tag,log.level,log.message,log.timestamp,detail);lastActivityTs=log.timestamp;}});
    } catch(e){}
  }

  /* Inference */
  var inferRunning = false;
  async function runInference(){
    var prompt=document.getElementById('infer-input').value.trim();if(!prompt||inferRunning)return;
    inferRunning=true;var btn=document.getElementById('infer-btn');btn.disabled=true;btn.classList.add('running');btn.textContent='\u2026';
    var result=document.getElementById('infer-result');result.textContent='';result.classList.add('active');
    document.getElementById('infer-meta').classList.remove('active');
    var t0=performance.now();
    try{
      var payload={prompt:prompt,capability:'llm'};if(preferredModels['llm'])payload.model=preferredModels['llm'];
      var r=await fetch(LaRuche.API.base+'/infer',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)});
      var elapsed=performance.now()-t0;
      if(!r.ok){result.textContent='Error '+r.status;result.style.color='var(--red)';addLog('INFER','log-err','Failed: HTTP '+r.status);return;}
      var d=await r.json();var response=d.response||d.text||d.content||d.message||JSON.stringify(d);
      result.textContent=response;result.style.color='var(--text-mid)';
      document.getElementById('infer-latency').textContent='\u23F1 '+(elapsed/1000).toFixed(2)+'s';
      if(d.tokens_generated||d.eval_count){var tokCount=d.tokens_generated||d.eval_count;var tokRate=d.tokens_per_sec||d.eval_rate||(tokCount/(elapsed/1000));document.getElementById('infer-tokens').textContent=tokCount+' tok @ '+tokRate.toFixed(1)+' t/s';}else document.getElementById('infer-tokens').textContent='';
      if(d.model)document.getElementById('infer-model').textContent='\u25C8 '+d.model;else document.getElementById('infer-model').textContent='';
      document.getElementById('infer-meta').classList.add('active');
      addLog('INFER','log-ok','Response in '+(elapsed/1000).toFixed(2)+'s'+(d.model?' via '+d.model:''));
    } catch(e){result.textContent='Network error: '+e.message;result.style.color='var(--red)';addLog('INFER','log-err','Error: '+e.message);}
    finally{inferRunning=false;btn.disabled=false;btn.classList.remove('running');btn.textContent='Send';}
  }

  /* Chart rendering */
  function renderChart(){
    var canvas=document.getElementById('stats-canvas');
    var rect=canvas.parentElement.getBoundingClientRect();var dpr=window.devicePixelRatio||1;
    canvas.width=rect.width*dpr;canvas.height=rect.height*dpr;
    var ctx=canvas.getContext('2d');ctx.scale(dpr,dpr);var W=rect.width,H=rect.height;ctx.clearRect(0,0,W,H);
    if(metricsHistory.length<2){ctx.fillStyle='#71717a';ctx.font='11px sans-serif';ctx.textAlign='center';ctx.fillText(LaRuche.i18n.t('dashboard.waitingData'),W/2,H/2);return;}
    var pad={top:8,right:10,bottom:22,left:38};var cW=W-pad.left-pad.right;var cH=H-pad.top-pad.bottom;
    var dataMin=metricsHistory[0].epoch_ms;var dataMax=metricsHistory[metricsHistory.length-1].epoch_ms;
    var tMin=viewTMin!==null?viewTMin:dataMin;var tMax=viewTMax!==null?viewTMax:dataMax;var tRange=tMax-tMin||1;
    var resetBtn=document.getElementById('stats-reset-zoom');if(resetBtn)resetBtn.className='stats-reset-zoom'+(viewTMin!==null?' visible':'');
    var keys=statsMetric==='all'?METRIC_KEYS:[statsMetric];
    var yMax=1;keys.forEach(function(k){var field=METRIC_FIELDS[k];metricsHistory.forEach(function(s){if(s.epoch_ms<tMin||s.epoch_ms>tMax)return;var v=s[field]||0;if(v>yMax)yMax=v;});});
    yMax=Math.ceil(yMax*1.15);if(yMax<10)yMax=10;
    ctx.strokeStyle='rgba(255,255,255,.06)';ctx.lineWidth=1;
    for(var gi=0;gi<=4;gi++){var gy=pad.top+cH-(gi/4)*cH;ctx.beginPath();ctx.moveTo(pad.left,gy);ctx.lineTo(pad.left+cW,gy);ctx.stroke();ctx.fillStyle='#71717a';ctx.font='9px monospace';ctx.textAlign='right';ctx.fillText(Math.round((gi/4)*yMax),pad.left-4,gy+3);}
    ctx.textAlign='center';ctx.fillStyle='#71717a';ctx.font='9px monospace';
    for(var ti=0;ti<=6;ti++){var tt=tMin+(ti/6)*tRange;var tx=pad.left+(ti/6)*cW;var d=new Date(tt);ctx.fillText(('0'+d.getHours()).slice(-2)+':'+('0'+d.getMinutes()).slice(-2),tx,H-4);}
    nodeEvents.forEach(function(ev){var ex=pad.left+((ev.epoch_ms-tMin)/tRange)*cW;if(ex<pad.left||ex>pad.left+cW)return;ctx.strokeStyle=ev.event_type==='connected'?'rgba(34,197,94,.5)':'rgba(239,68,68,.5)';ctx.lineWidth=1;ctx.setLineDash([3,3]);ctx.beginPath();ctx.moveTo(ex,pad.top);ctx.lineTo(ex,pad.top+cH);ctx.stroke();ctx.setLineDash([]);ctx.fillStyle=ev.event_type==='connected'?'#22c55e':'#ef4444';ctx.font='8px sans-serif';ctx.textAlign='left';ctx.fillText((ev.event_type==='connected'?'+ ':'- ')+ev.node_name,ex+2,pad.top+10);});
    ctx.save();ctx.beginPath();ctx.rect(pad.left,pad.top,cW,cH);ctx.clip();
    keys.forEach(function(k){var col=CHART_COLORS[k];var field=METRIC_FIELDS[k];ctx.strokeStyle=col.line;ctx.lineWidth=1.5;ctx.lineJoin='round';ctx.beginPath();var pts=[];metricsHistory.forEach(function(s,i){var x=pad.left+((s.epoch_ms-tMin)/tRange)*cW;var v=s[field]||0;var y=pad.top+cH-(v/yMax)*cH;pts.push({x:x,y:y,v:v});if(i===0)ctx.moveTo(x,y);else ctx.lineTo(x,y);});ctx.stroke();ctx.fillStyle=col.fill;ctx.beginPath();pts.forEach(function(p,i){i===0?ctx.moveTo(p.x,p.y):ctx.lineTo(p.x,p.y);});ctx.lineTo(pts[pts.length-1].x,pad.top+cH);ctx.lineTo(pts[0].x,pad.top+cH);ctx.closePath();ctx.fill();});
    ctx.restore();
    if(!isPanning&&chartCrosshairX>=pad.left&&chartCrosshairX<=pad.left+cW){
      ctx.strokeStyle='rgba(255,255,255,.2)';ctx.lineWidth=1;ctx.beginPath();ctx.moveTo(chartCrosshairX,pad.top);ctx.lineTo(chartCrosshairX,pad.top+cH);ctx.stroke();
      var hoverT=tMin+((chartCrosshairX-pad.left)/cW)*tRange;var closest=metricsHistory[0],minDist=Infinity;
      metricsHistory.forEach(function(s){var dist=Math.abs(s.epoch_ms-hoverT);if(dist<minDist){minDist=dist;closest=s;}});
      var lines=[];keys.forEach(function(k){var v=closest[METRIC_FIELDS[k]]||0;var unit=(k==='cpu'||k==='ram')?'%':'';lines.push(METRIC_LABELS[k]+': '+(typeof v==='number'?v.toFixed(1):v)+unit);});
      var tt2=new Date(closest.epoch_ms);lines.unshift(('0'+tt2.getHours()).slice(-2)+':'+('0'+tt2.getMinutes()).slice(-2)+':'+('0'+tt2.getSeconds()).slice(-2));
      ctx.font='9px monospace';var maxW=0;lines.forEach(function(l){var m=ctx.measureText(l).width;if(m>maxW)maxW=m;});
      var boxW=maxW+12,boxH=lines.length*13+8;var bx=chartCrosshairX+8;if(bx+boxW>W-4)bx=chartCrosshairX-boxW-8;var by=pad.top+4;
      ctx.fillStyle='rgba(9,9,11,.9)';ctx.strokeStyle='rgba(245,158,11,.4)';ctx.lineWidth=1;ctx.beginPath();
      if(ctx.roundRect)ctx.roundRect(bx,by,boxW,boxH,4);else ctx.rect(bx,by,boxW,boxH);ctx.fill();ctx.stroke();
      ctx.textAlign='left';lines.forEach(function(l,i){ctx.fillStyle=i===0?'#fbbf24':'#a1a1aa';ctx.fillText(l,bx+6,by+12+i*13);});
    }
  }

  function chartDataRange(){if(metricsHistory.length<2)return{min:0,max:1};return{min:metricsHistory[0].epoch_ms,max:metricsHistory[metricsHistory.length-1].epoch_ms};}
  function chartResetZoom(){viewTMin=null;viewTMax=null;if(metricsHistory.length>=2)renderChart();}
  function chartClampView(){var d=chartDataRange();var fullRange=d.max-d.min;if(fullRange<=0)return;var range=viewTMax-viewTMin;if(range>=fullRange*0.98){viewTMin=null;viewTMax=null;return;}if(range<5000){var mid=(viewTMin+viewTMax)/2;viewTMin=mid-2500;viewTMax=mid+2500;}if(viewTMin<d.min){viewTMax+=(d.min-viewTMin);viewTMin=d.min;}if(viewTMax>d.max){viewTMin-=(viewTMax-d.max);viewTMax=d.max;}if(viewTMin<d.min)viewTMin=d.min;}

  function toggleStatsCard(metric){
    if(statsCardOpen&&statsMetric===metric){closeStatsCard();return;}
    statsMetric=metric||'all';statsCardOpen=true;document.getElementById('stats-card').classList.add('active');
    document.querySelectorAll('.hex-gauge').forEach(function(h){h.classList.remove('active-chart');});
    var hexMap={cpu:'hg-cpu',ram:'hg-ram',tps:'hg-tps',queue:'hg-queue',nodes:'hg-nodes'};
    if(hexMap[metric]){var el=document.getElementById(hexMap[metric]);if(el)el.classList.add('active-chart');}
    document.querySelectorAll('#stats-pills .stats-pill').forEach(function(p){p.classList.toggle('active',p.dataset.metric===statsMetric);});
    fetchMetricsHistory();
    if(statsRefreshTimer)clearInterval(statsRefreshTimer);
    statsRefreshTimer=setInterval(fetchMetricsHistory,5000);
  }
  function closeStatsCard(){statsCardOpen=false;document.getElementById('stats-card').classList.remove('active');document.querySelectorAll('.hex-gauge').forEach(function(h){h.classList.remove('active-chart');});if(statsRefreshTimer){clearInterval(statsRefreshTimer);statsRefreshTimer=null;}viewTMin=null;viewTMax=null;}

  async function fetchMetricsHistory(){
    try{var res=await fetch(LaRuche.API.base+'/metrics/history');if(!res.ok)return;var data=await res.json();metricsHistory=data.snapshots||[];nodeEvents=data.events||[];renderChart();}catch(e){}
  }

  function startPolling(){
    fetchSwarm();fetchStatus();fetchModels();fetchActivity();
    pollTimers.push(setInterval(fetchSwarm,3000));
    pollTimers.push(setInterval(fetchStatus,5000));
    pollTimers.push(setInterval(fetchActivity,2000));
    pollTimers.push(setInterval(fetchModels,15000));
    addLog('SYS','log-info',LaRuche.i18n.t('dashboard.initialized'));
  }
  function stopPolling(){pollTimers.forEach(function(t){clearInterval(t);});pollTimers=[];if(statsRefreshTimer){clearInterval(statsRefreshTimer);statsRefreshTimer=null;}}

  function init(){
    // Log scroll tracking
    var logBody=document.getElementById('log-body');
    logBody.addEventListener('scroll',function(){logUserScrolled=!(logBody.scrollHeight-logBody.scrollTop-logBody.clientHeight<20);});
    // Mobile tabs
    document.getElementById('dash-mob-tabs').addEventListener('click',function(e){var btn=e.target.closest('button');if(!btn)return;var idx=parseInt(btn.dataset.tab);document.querySelectorAll('#dash-mob-tabs button').forEach(function(b){b.classList.remove('active');});btn.classList.add('active');document.querySelectorAll('.dash-mob-panel').forEach(function(p,i){p.classList.toggle('mob-active',i===idx);});});
    // Hex click handlers
    document.getElementById('hg-cpu').addEventListener('click',function(){toggleStatsCard('cpu');});
    document.getElementById('hg-ram').addEventListener('click',function(){toggleStatsCard('ram');});
    document.getElementById('hg-tps').addEventListener('click',function(){toggleStatsCard('tps');});
    document.getElementById('hg-queue').addEventListener('click',function(){toggleStatsCard('queue');});
    document.getElementById('hg-nodes').addEventListener('click',function(){toggleStatsCard('all');});
    document.getElementById('hg-uptime').addEventListener('click',function(){toggleStatsCard('all');});
    document.getElementById('stats-close').addEventListener('click',closeStatsCard);
    document.getElementById('stats-pills').addEventListener('click',function(e){var pill=e.target.closest('.stats-pill');if(!pill)return;statsMetric=pill.dataset.metric;document.querySelectorAll('#stats-pills .stats-pill').forEach(function(p){p.classList.toggle('active',p.dataset.metric===statsMetric);});renderChart();});
    document.getElementById('stats-reset-zoom').addEventListener('click',function(e){e.stopPropagation();chartResetZoom();});
    // Chart interactions
    var statsCanvas=document.getElementById('stats-canvas');
    statsCanvas.addEventListener('wheel',function(e){e.preventDefault();if(metricsHistory.length<2)return;var d=chartDataRange();var rect=statsCanvas.getBoundingClientRect();var cW=rect.width-48;var mx=e.clientX-rect.left-38;var ratio=Math.max(0,Math.min(1,mx/cW));var curMin=viewTMin!==null?viewTMin:d.min;var curMax=viewTMax!==null?viewTMax:d.max;var curRange=curMax-curMin;var factor=e.deltaY>0?1.25:0.8;var newRange=curRange*factor;if(newRange>=(d.max-d.min)*0.98){chartResetZoom();return;}if(newRange<5000)newRange=5000;var pivot=curMin+ratio*curRange;viewTMin=pivot-ratio*newRange;viewTMax=pivot+(1-ratio)*newRange;chartClampView();renderChart();},{passive:false});
    statsCanvas.addEventListener('mousedown',function(e){if(e.button!==0||metricsHistory.length<2)return;var d=chartDataRange();isPanning=true;panStartX=e.clientX;panStartTMin=viewTMin!==null?viewTMin:d.min;panStartTMax=viewTMax!==null?viewTMax:d.max;statsCanvas.style.cursor='grabbing';e.preventDefault();});
    statsCanvas.addEventListener('mousemove',function(e){var rect=statsCanvas.getBoundingClientRect();if(isPanning){var cW=rect.width-48;var panRange=panStartTMax-panStartTMin;var dx=e.clientX-panStartX;var dt=-(dx/cW)*panRange;viewTMin=panStartTMin+dt;viewTMax=panStartTMax+dt;chartClampView();if(metricsHistory.length>=2)renderChart();}else{chartCrosshairX=e.clientX-rect.left;if(metricsHistory.length>=2)renderChart();}});
    window.addEventListener('mouseup',function(){if(isPanning){isPanning=false;document.getElementById('stats-canvas').style.cursor='';}});
    statsCanvas.addEventListener('mouseleave',function(){chartCrosshairX=-1;if(!isPanning&&metricsHistory.length>=2)renderChart();});
    statsCanvas.addEventListener('dblclick',function(e){e.preventDefault();chartResetZoom();});
    // Touch interactions
    statsCanvas.addEventListener('touchstart',function(e){if(metricsHistory.length<2)return;if(e.touches.length===1){var d=chartDataRange();isPanning=true;panStartX=e.touches[0].clientX;panStartTMin=viewTMin!==null?viewTMin:d.min;panStartTMax=viewTMax!==null?viewTMax:d.max;}else if(e.touches.length===2){isPanning=false;var dx=e.touches[0].clientX-e.touches[1].clientX;var dy=e.touches[0].clientY-e.touches[1].clientY;lastPinchDist=Math.sqrt(dx*dx+dy*dy);}e.preventDefault();},{passive:false});
    statsCanvas.addEventListener('touchmove',function(e){if(metricsHistory.length<2)return;if(e.touches.length===1&&isPanning){var rect=statsCanvas.getBoundingClientRect();var cW=rect.width-48;var panRange=panStartTMax-panStartTMin;var dx=e.touches[0].clientX-panStartX;var dt=-(dx/cW)*panRange;viewTMin=panStartTMin+dt;viewTMax=panStartTMax+dt;chartClampView();renderChart();}else if(e.touches.length===2){var dx2=e.touches[0].clientX-e.touches[1].clientX;var dy2=e.touches[0].clientY-e.touches[1].clientY;var dist=Math.sqrt(dx2*dx2+dy2*dy2);if(lastPinchDist>0){var d=chartDataRange();var curMin=viewTMin!==null?viewTMin:d.min;var curMax=viewTMax!==null?viewTMax:d.max;var factor=lastPinchDist/dist;var mid=(curMin+curMax)/2;var halfNew=((curMax-curMin)*factor)/2;viewTMin=mid-halfNew;viewTMax=mid+halfNew;chartClampView();renderChart();}lastPinchDist=dist;}e.preventDefault();},{passive:false});
    statsCanvas.addEventListener('touchend',function(){isPanning=false;lastPinchDist=0;});
    // Popover delegation
    var logBodyEl=document.getElementById('log-body');
    logBodyEl.addEventListener('mouseenter',function(e){var row=e.target.closest('.log-row[data-infer-id]');if(!row||popoverPinned)return;clearTimeout(popoverHoverTimer);popoverHoverTimer=setTimeout(function(){var id=parseInt(row.dataset.inferId);var detail=inferDetailMap.get(id);if(detail)showInferPopover(row,detail);},300);},true);
    logBodyEl.addEventListener('mouseleave',function(e){var row=e.target.closest('.log-row[data-infer-id]');if(!row||popoverPinned)return;clearTimeout(popoverHoverTimer);popoverHoverTimer=setTimeout(hideInferPopover,200);},true);
    logBodyEl.addEventListener('click',function(e){var row=e.target.closest('.log-row[data-infer-id]');if(!row){hideInferPopover();return;}var id=parseInt(row.dataset.inferId);var detail=inferDetailMap.get(id);if(!detail)return;if(popoverPinned&&activePopover){hideInferPopover();}else{showInferPopover(row,detail);popoverPinned=true;if(activePopover)activePopover.classList.add('pinned');}});
    document.addEventListener('click',function(e){if(activePopover&&popoverPinned&&!activePopover.contains(e.target)&&!e.target.closest('.log-row[data-infer-id]'))hideInferPopover();});
    // Sync preferred models from backend
    fetch('/config/default_model').then(function(r){return r.json();}).then(function(d){if(d.default_models){preferredModels=d.default_models;localStorage.setItem('laruche_preferred_models',JSON.stringify(preferredModels));}else if(d.default_model){preferredModels['llm']=d.default_model;localStorage.setItem('laruche_preferred_models',JSON.stringify(preferredModels));}}).catch(function(){});
  }

  function enter() { startPolling(); }
  function leave() { stopPolling(); closeStatsCard(); }

    async function loadBlueprints(el) {
    var bps=[];try{bps=await fetch('/api/blueprints').then(function(r){return r.json();});}catch(e){}
    if(!bps.length){el.innerHTML='<div style="text-align:center;color:var(--text-muted);padding:20px">'+LaRuche.i18n.t('dashboard.noBlueprintAvailable')+'</div>';return;}
    
    window._blueprints = bps;
    el.innerHTML = '<div style="margin-bottom:12px;color:var(--amber);font-size:12px;">'+LaRuche.i18n.t('dashboard.selectBlueprint')+'</div>' +
      bps.map(function(b, idx) {
        return '<div class="settings-card" style="margin-bottom:12px;cursor:pointer;" onclick="LaRuche.Settings.openBlueprintForm('+idx+')">' +
          '<div class="settings-card-title">'+LaRuche.Utils.esc(b.title||b.id)+'</div>' +
          '<div style="font-size:12px;color:var(--text-dim);margin-top:4px;">'+LaRuche.Utils.esc(b.description||'')+'</div>' +
          '<div id="bpForm_'+idx+'" style="display:none;margin-top:12px;padding-top:12px;border-top:1px solid var(--border);" onclick="event.stopPropagation()">' +
            (b.slots||[]).map(function(slot){
              return '<div style="margin-bottom:8px"><label style="font-size:10px;color:var(--text-dim)">'+LaRuche.Utils.esc(slot.label||slot.name)+'</label><input id="bpInput_'+idx+'_'+slot.name+'" class="form-input" placeholder="'+LaRuche.Utils.esc(slot.placeholder||'')+'"></div>';
            }).join('') +
            '<button class="settings-save-btn" style="margin-top:8px" onclick="LaRuche.Settings.instanciateBlueprint('+idx+')">'+LaRuche.i18n.t('dashboard.instantiate')+'</button>' +
          '</div>' +
        '</div>';
      }).join('');
  }

  function openBlueprintForm(idx) {
    var form = document.getElementById('bpForm_'+idx);
    if (form.style.display === 'none') {
      form.style.display = 'block';
    } else {
      form.style.display = 'none';
    }
  }

  function instanciateBlueprint(idx) {
    var b = window._blueprints[idx];
    var slotsData = {};
    (b.slots||[]).forEach(function(slot){
      slotsData[slot.name] = document.getElementById('bpInput_'+idx+'_'+slot.name).value;
    });
    fetch('/api/blueprints/'+b.id+'/instancier', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(slotsData)
    }).then(function(res) {
      if(res.ok) {
        LaRuche.Toast.show(LaRuche.i18n.t('dashboard.blueprintSuccess'), 'ok');
        document.getElementById('bpForm_'+idx).style.display = 'none';
      } else {
        LaRuche.Toast.show(LaRuche.i18n.t('dashboard.blueprintError'), 'err');
      }
    }).catch(function(e){ LaRuche.Toast.show(LaRuche.i18n.t('dashboard.error',{msg:e}), 'err'); });
  }

  return { init:init, openBlueprintForm:openBlueprintForm, instanciateBlueprint:instanciateBlueprint, enter:enter, leave:leave, toggleNodeExpand:toggleNodeExpand, useMeshModel:useMeshModel, fetchModels:fetchModels, FetchModels:fetchModels };
})();

/* ── Settings Page Module ─────────────────────────────────────── */
/* --- Inline Markdown rendering (taken from scriptorium, no dependencies) ----- */
LaRuche.MD = (function(){
  function esc(s){ return String(s==null?'':s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];}); }

  function inline(s){
    var stash = [];
    function stashPush(html){ var tok='\x00X'+stash.length+'\x00'; stash.push(html); return tok; }
    // Wikilinks: [[node_id]] or [[node_id|alias]]  -> memory navigation
    s = s.replace(/\[\[([^\[\]\n|]+)(?:\|([^\[\]\n]+))?\]\]/g, function(m, name, alias){
      var label = (alias || name).trim();
      var target = name.trim();
      return stashPush('<span class="mem2-wikilink" data-wikilink="'+esc(target)+'">'+esc(label)+'</span>');
    });
    // Images
    s = s.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, function(m, alt, url){
      return stashPush('<img alt="'+esc(alt)+'" src="'+esc(url)+'"/>');
    });
    // Links
    s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, function(m, text, url){
      return stashPush('<a href="'+esc(url)+'" target="_blank" rel="noopener">'+esc(text)+'</a>');
    });
    s = esc(s);
    s = s.replace(/\x00IC(\d+)\x00/g, function(m){ return m; });
    s = s.replace(/==([^=\n]+)==/g, '<mark>$1</mark>');
    s = s.replace(/\*\*([^\*\n]+)\*\*/g, '<strong>$1</strong>');
    s = s.replace(/__([^_\n]+)__/g, '<strong>$1</strong>');
    s = s.replace(/(^|[^\*])\*([^\*\n]+)\*(?!\*)/g, '$1<em>$2</em>');
    s = s.replace(/(^|[^_])_([^_\n]+)_(?!_)/g, '$1<em>$2</em>');
    s = s.replace(/~~([^~\n]+)~~/g, '<del>$1</del>');
    s = s.replace(/\n/g, '<br/>');
    s = s.replace(/\x00X(\d+)\x00/g, function(m, n){ return stash[+n] || ''; });
    return s;
  }

  function render(src){
    if(!src) return '';
    src = String(src).replace(/\r\n?/g, '\n'); // normalize CRLF -> LF (otherwise code blocks
    // are not extracted and their content passes through the inline regexes -> browser freeze)
    var planBlocks = [];
    src = String(src).replace(/<plan>\s*([\s\S]*?)\s*<\/plan>/gi, function(m, jsonText){
      try {
        var plan = JSON.parse(jsonText);
        var html = '<div class="mem2-plan" style="margin: 10px 0; border: 1px solid var(--border); border-radius: 4px; padding: 8px; background: var(--bg2);">';
        plan.forEach(function(t){
           var icon = t.status === 'done' ? '✅' : (t.status === 'in_progress' ? '⏳' : '⬜');
           var color = t.status === 'done' ? 'var(--text)' : (t.status === 'in_progress' ? 'var(--gold)' : 'var(--text-dim)');
           html += '<div style="margin: 4px 0; color: '+color+'; display: flex; gap: 8px;"><span style="flex-shrink:0;">' + icon + '</span><span>' + esc(t.task) + '</span></div>';
        });
        html += '</div>';
        planBlocks.push(html);
        return '\n\x00PB'+(planBlocks.length-1)+'\x00\n';
      } catch(e) {
        return m;
      }
    });
    var codeBlocks = [];
    src = src.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, function(m, lang, code){
      codeBlocks.push({lang:lang, code:code});
      return '\n\x00CB'+(codeBlocks.length-1)+'\x00\n';
    });
    var inlineCodes = [];
    src = src.replace(/`([^`\n]+)`/g, function(m, c){ inlineCodes.push(c); return '\x00IC'+(inlineCodes.length-1)+'\x00'; });
    var lines = src.split('\n'), html='', i=0;
    while(i < lines.length){
      var line = lines[i];
      var h = line.match(/^(#{1,6})\s+(.*)$/);
      if(h){ html += '<h'+h[1].length+'>'+inline(h[2])+'</h'+h[1].length+'>'; i++; continue; }
      if(/^(---+|\*\*\*+|___+)\s*$/.test(line)){ html += '<hr/>'; i++; continue; }
      if(/^>\s?/.test(line)){
        var bq=[]; while(i<lines.length && /^>\s?/.test(lines[i])){ bq.push(lines[i].replace(/^>\s?/,'')); i++; }
        html += '<blockquote>'+render(bq.join('\n'))+'</blockquote>'; continue;
      }
      if(/^\s*[\-\*\+]\s+/.test(line)){
        var ul=[]; while(i<lines.length && /^\s*[\-\*\+]\s+/.test(lines[i])){ ul.push(lines[i].replace(/^\s*[\-\*\+]\s+/,'')); i++; }
        html += '<ul>'+ul.map(function(b){return '<li>'+inline(b)+'</li>';}).join('')+'</ul>'; continue;
      }
      if(/^\s*\d+\.\s+/.test(line)){
        var ol=[]; while(i<lines.length && /^\s*\d+\.\s+/.test(lines[i])){ ol.push(lines[i].replace(/^\s*\d+\.\s+/,'')); i++; }
        html += '<ol>'+ol.map(function(b){return '<li>'+inline(b)+'</li>';}).join('')+'</ol>'; continue;
      }
      var pb = line.match(/^\x00PB(\d+)\x00$/);
      if(pb){ html += planBlocks[+pb[1]]; i++; continue; }
      var cb = line.match(/^\x00CB(\d+)\x00$/);
      if(cb){ html += '<pre><code>'+esc(codeBlocks[+cb[1]].code)+'</code></pre>'; i++; continue; }
      if(i+1 < lines.length && /\|/.test(line) && /^[\s\|:\-]+$/.test(lines[i+1]) && /\|/.test(lines[i+1])){
        var hcells = line.split('|').slice(1,-1).map(function(s){return s.trim();});
        var rows=[]; i+=2;
        while(i<lines.length && /\|/.test(lines[i])){ rows.push(lines[i].split('|').slice(1,-1).map(function(s){return s.trim();})); i++; }
        html += '<table><thead><tr>'+hcells.map(function(c){return '<th>'+inline(c)+'</th>';}).join('')+'</tr></thead><tbody>'+
          rows.map(function(r){return '<tr>'+r.map(function(c){return '<td>'+inline(c)+'</td>';}).join('')+'</tr>';}).join('')+'</tbody></table>';
        continue;
      }
      if(line.trim()===''){ i++; continue; }
      var buf=[];
      while(i<lines.length && lines[i].trim()!=='' && !/^(#{1,6}\s|>\s?|[\-\*\+]\s+|\d+\.\s+|---+|\*\*\*+|___+|\x00CB|\x00PB)/.test(lines[i])){ buf.push(lines[i]); i++; }
      html += '<p>'+inline(buf.join('\n'))+'</p>';
    }
    html = html.replace(/\x00IC(\d+)\x00/g, function(m, n){ return '<code>'+esc(inlineCodes[+n])+'</code>'; });
    return html;
  }

  // Wires the [[node]] links of an already-rendered container to a navigation callback.
  function wireWikilinks(container, onNav){
    if(!container) return;
    container.querySelectorAll('.mem2-wikilink').forEach(function(el){
      el.onclick = function(e){ e.preventDefault(); e.stopPropagation(); onNav(el.dataset.wikilink); };
    });
  }

  return { render:render, wireWikilinks:wireWikilinks };
})();

/* --- Memory Page Module (drives mem2-*: Obsidian tree + markdown + CRUD) - */
