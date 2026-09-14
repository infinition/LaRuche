(function(){
  'use strict';
  var sdk=window.LaRucheApp;
  var engine=window.Game2048Engine;
  var storageKey='game.state.v1';
  var state=null;
  var previous=null;
  var saveChain=Promise.resolve();
  var saveRevision=0;
  var locale='fr';
  var touchStart=null;
  var revision=0, agentBusy=false, agentAuto=false, agentTimer=null;
  var seat='game-'+Date.now().toString(36);

  var copy={
    fr:{score:'Score',best:'Record',undo:'Annuler',newGame:'Nouvelle partie',continue:'Continuer',restart:'Rejouer',instructions:'Utilise les flèches, ZQSD ou glisse sur la grille.',moves:'coups',move:'coup',saved:'Synchronisé',saving:'Sauvegarde...',offline:'Stockage indisponible',won:'2048 atteint !',wonText:'Tu peux continuer pour viser encore plus haut.',lost:'Partie terminée',lostText:'Plus aucun mouvement possible.',autoStopped:'Mode auto arrete.'},
    en:{score:'Score',best:'Best',undo:'Undo',newGame:'New game',continue:'Keep going',restart:'Play again',instructions:'Use arrow keys, WASD or swipe the board.',moves:'moves',move:'move',saved:'Synced',saving:'Saving...',offline:'Storage unavailable',won:'You reached 2048!',wonText:'Keep going to chase a higher tile.',lost:'Game over',lostText:'There are no moves left.',autoStopped:'Auto mode stopped.'}
  };

  function text(key){ return (copy[locale]&&copy[locale][key])||copy.fr[key]||key; }
  function fresh(best){ return {board:engine.createBoard(),score:0,best:best||0,moves:0,won:false,keepPlaying:false}; }
  function clone(value){ return JSON.parse(JSON.stringify(value)); }

  function validState(value){
    return value && engine.validBoard(value.board) && Number.isFinite(value.score) && value.score>=0 && Number.isFinite(value.best) && value.best>=0 && Number.isInteger(value.moves) && value.moves>=0;
  }

  function applyLocale(){
    document.documentElement.lang=locale;
    document.querySelectorAll('[data-i18n]').forEach(function(node){ node.textContent=text(node.dataset.i18n); });
    document.getElementById('instructions').textContent=text('instructions');
  }

  function tileClass(value){ return value<=8192?' value-'+value:' value-super'; }

  function render(){
    var board=document.getElementById('board');
    board.innerHTML='';
    state.board.forEach(function(value,index){
      var tile=document.createElement('div');
      tile.className='tile'+(value?tileClass(value):' empty');
      tile.setAttribute('role','gridcell');
      tile.setAttribute('aria-label',value?String(value):'vide');
      tile.textContent=value?String(value):'';
      tile.style.setProperty('--delay',(index*7)+'ms');
      board.appendChild(tile);
    });
    document.getElementById('score').textContent=String(state.score);
    document.getElementById('best').textContent=String(state.best);
    document.getElementById('moveCount').textContent=state.moves+' '+text(state.moves===1?'move':'moves');
    document.getElementById('undo').disabled=!previous;

    var won=engine.hasWon(state.board) && !state.keepPlaying;
    var lost=!engine.canMove(state.board);
    var overlay=document.getElementById('overlay');
    overlay.hidden=!(won||lost);
    if(won||lost){
      document.getElementById('overlayTitle').textContent=text(won?'won':'lost');
      document.getElementById('overlayText').textContent=text(won?'wonText':'lostText');
      document.getElementById('continueGame').hidden=!won;
    }
  }

  function setSaveStatus(kind){
    var node=document.getElementById('saveStatus');
    node.className='save-status '+kind;
    node.textContent=text(kind==='saved'?'saved':kind==='saving'?'saving':'offline');
  }

  function persist(){
    var revision=++saveRevision;
    var snapshot=clone(state);
    setSaveStatus('saving');
    sdk.ui.setDirty(true).catch(function(){});
    saveChain=saveChain.catch(function(){}).then(function(){
      return sdk.storage.set(storageKey,snapshot);
    }).then(function(){
      if(revision===saveRevision){ setSaveStatus('saved'); sdk.ui.setDirty(false).catch(function(){}); }
    }).catch(function(){
      if(revision===saveRevision){ setSaveStatus('offline'); sdk.ui.setDirty(false).catch(function(){}); }
    });
  }

  function play(direction){
    if(!state || (!state.keepPlaying && engine.hasWon(state.board)) || !engine.canMove(state.board)) return;
    var result=engine.move(state.board,direction);
    if(!result.moved){
      var board=document.getElementById('board');
      board.classList.remove('blocked');
      void board.offsetWidth;
      board.classList.add('blocked');
      return;
    }
    previous=clone(state);
    state.board=engine.addRandom(result.board);
    state.score+=result.gained;
    state.best=Math.max(state.best,state.score);
    state.moves+=1;
    revision+=1;
    render();
    persist();
  }

  function newGame(){
    agentAuto=false; clearTimeout(agentTimer); agentTimer=null;
    previous=state?clone(state):null;
    state=fresh(state&&state.best);
    revision+=1;seat='game-'+Date.now().toString(36);
    render();
    persist();
  }

  function bind(){
    document.addEventListener('keydown',function(event){
      if(/INPUT|TEXTAREA|SELECT/.test(event.target.tagName))return;
      var directions={ArrowLeft:'left',ArrowRight:'right',ArrowUp:'up',ArrowDown:'down',a:'left',q:'left',d:'right',w:'up',z:'up',s:'down'};
      var direction=directions[event.key];
      if(!direction) return;
      event.preventDefault();
      play(direction);
    });
    document.querySelectorAll('[data-direction]').forEach(function(button){
      button.addEventListener('click',function(){ play(button.dataset.direction); });
    });
    var board=document.getElementById('board');
    board.addEventListener('pointerdown',function(event){ touchStart={x:event.clientX,y:event.clientY,id:event.pointerId}; board.setPointerCapture(event.pointerId); });
    board.addEventListener('pointerup',function(event){
      if(!touchStart || touchStart.id!==event.pointerId) return;
      var dx=event.clientX-touchStart.x;
      var dy=event.clientY-touchStart.y;
      touchStart=null;
      if(Math.max(Math.abs(dx),Math.abs(dy))<28) return;
      play(Math.abs(dx)>Math.abs(dy)?(dx>0?'right':'left'):(dy>0?'down':'up'));
    });
    board.addEventListener('pointercancel',function(){ touchStart=null; });
    document.getElementById('newGame').addEventListener('click',newGame);
    document.getElementById('restartGame').addEventListener('click',newGame);
    document.getElementById('continueGame').addEventListener('click',function(){ state.keepPlaying=true; revision++; render(); persist(); });
    document.getElementById('undo').addEventListener('click',function(){
      if(!previous) return;
      var current=clone(state);
      state=previous;
      revision+=1;
      previous=current;
      render();
      persist();
    });
  }

  function start(){
    bind();
    sdk.ready().then(function(context){
      locale=context.locale==='en'?'en':'fr';
      document.documentElement.dataset.hostTheme=context.theme||'default';
      applyLocale();
      return sdk.storage.get(storageKey);
    }).then(function(saved){
      var restored=validState(saved);
      state=restored?saved:fresh(0);
      state.won=!!state.won;
      state.keepPlaying=!!state.keepPlaying;
      render();
      if(restored) setSaveStatus('saved');
      else persist();
      return sdk.ui.setTitle('2048');
    }).then(function(){
      sdk.actions.register('game.state',snapshot);
      sdk.actions.register('game.move',async function(args){
        if(args.revision!==revision)throw new Error('Stale revision: read game.state again');
        if(snapshot().legalMoves.indexOf(args.direction)===-1)throw new Error('Illegal move');
        play(args.direction);await saveChain;return snapshot();
      });
      sdk.actions.register('game.new',async function(args){if(args.revision!==revision)throw new Error('Stale revision');newGame();await saveChain;return snapshot();});
      bindAgent();
      return sdk.ui.setStatus('ready','2048 prêt',100);
    }).catch(function(error){
      state=fresh(0);
      render();
      setSaveStatus('offline');
      sdk.ui.setStatus('error',String(error.message||error).slice(0,180)).catch(function(){});
    });
  }

  function snapshot(){return Object.assign({board:state.board.slice(),score:state.score,best:state.best,moves:state.moves,revision:revision},engine.describe(state.board,state.keepPlaying));}
  function bindAgent(){
    var select=document.getElementById('agentPlayer'),info=document.getElementById('agentStatus');
    function refresh(){sdk.agents.list().then(function(agents){select.innerHTML='';agents.forEach(function(a){var o=document.createElement('option');o.value=a.id;o.textContent=(a.avatar||'')+' '+a.name;select.appendChild(o);});document.getElementById('agentTurn').disabled=!agents.length;document.getElementById('agentAuto').disabled=!agents.length;info.textContent=agents.length?(locale==='en'?'Ready for an agent turn.':'Prêt pour un tour agent.'):(locale==='en'?'Authorize an agent in App Permissions.':'Autorise un agent dans les permissions de l’App.');}).catch(function(e){info.textContent=e.message;});}
    async function turn(){
      if(agentBusy||!select.value)return;
      if(!snapshot().legalMoves.length){agentAuto=false;info.textContent=text(snapshot().won?'wonText':'lostText');return;}
      agentBusy=true;
      document.getElementById('agentTurn').disabled=true;document.getElementById('agentAuto').disabled=true;document.getElementById('agentPause').disabled=false;
      info.textContent=locale==='en'?'Agent thinking…':'L’agent réfléchit…';
      try{var result=await sdk.agents.act(select.value,seat,'game.state','Read the supplied guide, goal and rules. Reach a 2048 tile, then pursue higher score only after the human continues. Select ONE direction from the current legalMoves, using the exact revision. Preserve empty squares and keep large tiles organized. Return only game.move action JSON; do not reset or invent a future random tile.');info.textContent=result.model+' · '+result.text;}
      catch(e){var etaitAuto=agentAuto;agentAuto=false;info.textContent=e.message+(etaitAuto?' '+text('autoStopped'):'');}
      finally{agentBusy=false;document.getElementById('agentTurn').disabled=false;document.getElementById('agentAuto').disabled=false;document.getElementById('agentPause').disabled=!agentAuto;}
      if(agentAuto&&snapshot().legalMoves.length)agentTimer=setTimeout(function(){agentTimer=null;if(agentAuto)turn();},2200);else agentAuto=false;
    }
    document.getElementById('refreshAgents').onclick=refresh;
    document.getElementById('agentTurn').onclick=turn;
    document.getElementById('agentAuto').onclick=function(){agentAuto=true;turn();};
    document.getElementById('agentPause').onclick=function(){agentAuto=false;clearTimeout(agentTimer);agentTimer=null;this.disabled=true;info.textContent=agentBusy?(locale==='en'?'Pause after this turn.':'Pause après le tour en cours.'):(locale==='en'?'Paused.':'En pause.');};
    if(locale==='en'){document.getElementById('agentTurn').textContent='One turn';document.getElementById('agentPlayer').options[0].textContent='Permission required';}
    refresh();
  }

  if(sdk && engine) start();
})();
