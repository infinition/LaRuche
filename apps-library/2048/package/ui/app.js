(function(){
  'use strict';
  var sdk=window.LaRucheApp;
  var engine=window.Game2048Engine;
  var storageKey='game.state.v1', zoomKey='game.zoom.v1', zoom=null;
  var state=null;
  var previous=null;
  var saveChain=Promise.resolve();
  var saveRevision=0;
  var locale='fr';
  var touchStart=null;
  var revision=0, player=null, analysisRevision=-1, analysis=null;
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
    if(player)player.wake();
  }

  function newGame(){
    if(player)player.reset();
    previous=state?clone(state):null;
    state=fresh(state&&state.best);
    revision+=1;seat='game-'+Date.now().toString(36);
    render();
    persist();
  }

  function humanPlay(direction){if(player&&player.view().active)player.pause();play(direction);}
  function bind(){
    document.addEventListener('keydown',function(event){
      if(/INPUT|TEXTAREA|SELECT/.test(event.target.tagName))return;
      var directions={ArrowLeft:'left',ArrowRight:'right',ArrowUp:'up',ArrowDown:'down',a:'left',q:'left',d:'right',w:'up',z:'up',s:'down'};
      var direction=directions[event.key];
      if(!direction) return;
      event.preventDefault();
      humanPlay(direction);
    });
    document.querySelectorAll('[data-direction]').forEach(function(button){
      button.addEventListener('click',function(){ humanPlay(button.dataset.direction); });
    });
    var board=document.getElementById('board');
    board.addEventListener('pointerdown',function(event){ touchStart={x:event.clientX,y:event.clientY,id:event.pointerId}; board.setPointerCapture(event.pointerId); });
    board.addEventListener('pointerup',function(event){
      if(!touchStart || touchStart.id!==event.pointerId) return;
      var dx=event.clientX-touchStart.x;
      var dy=event.clientY-touchStart.y;
      touchStart=null;
      if(Math.max(Math.abs(dx),Math.abs(dy))<28) return;
      humanPlay(Math.abs(dx)>Math.abs(dy)?(dx>0?'right':'left'):(dy>0?'down':'up'));
    });
    board.addEventListener('pointercancel',function(){ touchStart=null; });
    document.getElementById('newGame').addEventListener('click',newGame);
    document.getElementById('restartGame').addEventListener('click',newGame);
    document.getElementById('continueGame').addEventListener('click',function(){ state.keepPlaying=true; revision++; render(); persist(); if(player)player.wake(); });
    document.getElementById('undo').addEventListener('click',function(){
      if(!previous) return;
      if(player)player.pause();
      var current=clone(state);
      state=previous;
      revision+=1;
      previous=current;
      render();
      persist();
    });
  }

  /* Le plateau et les tuiles suivent la largeur du conteneur, donc dezoomer
     leur donne plus de pixels logiques au lieu de tout rapetisser. */
  function brancherZoom(){
    if(!window.AppZoom)return;
    var etiquette=document.getElementById('zoomValue');
    zoom=window.AppZoom.create({
      load:function(){return sdk.storage.get(zoomKey);},
      save:function(v){return sdk.storage.set(zoomKey,v);},
      label:function(t){if(etiquette)etiquette.textContent=t;}
    });
    document.getElementById('zoomIn').onclick=function(){zoom.decaler(1);};
    document.getElementById('zoomOut').onclick=function(){zoom.decaler(-1);};
    etiquette.onclick=function(){zoom.reinitialiser();};
    return zoom.restaurer();
  }

  function start(){
    bind();
    sdk.ready().then(function(context){
      locale=context.locale==='en'?'en':'fr';
      document.documentElement.dataset.hostTheme=context.theme||'default';
      applyLocale();
      return Promise.resolve(brancherZoom()).then(function(){return sdk.storage.get(storageKey);});
    }).then(function(saved){
      var restored=validState(saved);
      state=restored?saved:fresh(0);
      state.won=!!state.won;
      state.keepPlaying=!!state.keepPlaying;
      render();
      if(restored) setSaveStatus('saved');
      else persist();
      return sdk.ui.setTitle('2048');
    }).then(async function(){
      sdk.actions.register('game.state',snapshot);
      sdk.actions.register('game.move',async function(args){
        if(args.revision!==revision)throw new Error(perimee(args.revision));
        if(snapshot().legalMoves.indexOf(args.direction)===-1)throw new Error('Illegal move');
        play(args.direction);await saveChain;return snapshot();
      });
      sdk.actions.register('game.new',async function(args){if(args.revision!==revision)throw new Error(perimee(args.revision));newGame();await saveChain;return snapshot();});
      await bindAgent();
      return sdk.ui.setStatus('ready','2048 prêt',100);
    }).catch(function(error){
      state=fresh(0);
      render();
      setSaveStatus('offline');
      sdk.ui.setStatus('error',String(error.message||error).slice(0,180)).catch(function(){});
    });
  }

  /* Un refus qui ne dit pas la valeur courante oblige a relire l'etat avant
     de pouvoir reessayer, et le tour d'apres la revision a encore bouge. */
  function perimee(recue) {
    return 'Stale revision ' + recue + ', current is ' + revision +
      '. Read game.state again and use the revision it returns.';
  }

  function snapshot(){
    var description=engine.describe(state.board,state.keepPlaying);
    if(description.legalMoves.length&&analysisRevision!==revision){analysis=window.Game2048AI.analyze(engine,state.board);analysisRevision=revision;}
    return Object.assign({board:state.board.slice(),score:state.score,best:state.best,moves:state.moves,revision:revision,
      agent:player?player.view():null,analysis:description.legalMoves.length?analysis:null},description);
  }
  function setAgentGoal(goal){
    state.agentGoal=goal;
    state.keepPlaying=goal==='max_score';
    document.getElementById('maximizeScore').checked=state.keepPlaying;
  }
  async function bindAgent(){
    var select=document.getElementById('agentPlayer'),info=document.getElementById('agentStatus');
    var saved=state.agent;
    seat=state.agentSeat||seat;state.agentSeat=seat;
    function label(view){
      var dict=locale==='en'?{paused:'Paused',ready:'Playing',thinking:'Agent thinking…',waiting:'Waiting for Continue',retry:'Temporary error · retrying automatically',permission:'Waiting for access · Apps → Permissions → App → Agents',finished:'Game over'}:
        {paused:'En pause',ready:'Partie active',thinking:'L’agent réfléchit…',waiting:'En attente de Continuer',retry:'Erreur temporaire · reprise automatique',permission:'En attente d’autorisation · Apps → Permissions → App → Agents',finished:'Partie terminée'};
      return (dict[view.status]||view.status)+(view.detail?' · '+view.detail:'');
    }
    /* Trois choses portaient le meme mot "agent" sans que rien ne les separe:
       la conversation du chat, l'agent choisi ici, et l'heuristique locale. Ce
       qui suit nomme celui qui joue a cet instant, et dit ce qu'est le menu. */
    var quiJoue=document.getElementById('quiJoue');
    function direQuiJoue(view){
      if(!quiJoue)return;
      var fini=engine.describe(state.board,state.keepPlaying).waitingFor!=='move';
      var agentActif=!!(view&&view.active);
      var pense=!!(view&&view.busy);
      quiJoue.className='qui-joue'+(fini?'':agentActif?' agent':' toi');
      quiJoue.textContent=fini?(locale==='en'?'Game over':'Partie terminée')
        :agentActif?(pense?(locale==='en'?'The agent is choosing its move':'L’agent choisit son coup')
                          :(locale==='en'?'The agent is playing':'L’agent joue'))
        :(locale==='en'?'Your turn':'À toi de jouer');
    }
    var aide=document.getElementById('agentAide');
    if(aide){
      var en=locale==='en';
      aide.textContent=en
        ? '"LaRuche" is your active model, called by this App one move at a time. It is not the chat conversation.'
        : '« LaRuche » est ton modèle actif, appelé par l’App un coup à la fois. Ce n’est pas la conversation du chat.';
      aide.title=en
        ? 'This App calls the chosen agent itself, so closing or stopping the chat does not stop it; the App view must stay open. "One turn" plays a single move. "Auto" keeps playing until the game ends or you pause.'
        : 'L’App appelle elle-même l’agent choisi : fermer ou arrêter le chat ne l’interrompt pas, mais la vue de l’App doit rester ouverte. « Un tour » joue un seul coup. « Auto » enchaîne jusqu’à la fin de la partie ou une pause.';
    }

    player=window.GameAgent.create({
      state:function(){return Object.assign({revision:revision},engine.describe(state.board,state.keepPlaying));},
      canPlay:function(s){return s.waitingFor==='move';},
      list:function(){return sdk.agents.list();},
      delay:60,minimumInterval:520,
      invalidate:function(){revision++;},
      save:function(settings){state.agent=settings;persist();},
      changed:function(view){
        info.textContent=label(view);
        document.getElementById('agentTurn').disabled=view.busy||!select.value;
        document.getElementById('agentAuto').disabled=!select.value;
        document.getElementById('agentAuto').setAttribute('aria-pressed',String(view.active&&view.continuous));
        document.getElementById('agentAuto').textContent=view.active&&view.continuous?(locale==='en'?'Playing':'Partie active'):'Auto';
        document.getElementById('agentPause').disabled=!view.active;
        direQuiJoue(view);
      },
      act:function(id,lastError){
        var prompt='Maximize 2048 score and tile size. Read exact merge rules and current legalMoves. The state includes expectimax analysis of legal directions with actual after-slide boards BEFORE the unknown random tile. Prefer high utility, preserve space and a stable large-tile corner; compare rather than cycling random directions. Utility is a heuristic, not guaranteed future score. Choose exactly ONE legal direction with this revision. Return only game.move action JSON, without commentary. Never reset.';
        if(lastError)prompt+=' Previous attempt: '+lastError.slice(0,300)+'. Reassess the new state.';
        return sdk.agents.act(id,seat,'game.state',prompt,{allowedActions:['game.move'],freshState:true,expectedRevision:revision});
      }
    });
    async function refresh(){
      try{
        var wanted=player.view().agentId||select.value||(saved&&saved.agentId)||'laruche';
        var agents=await sdk.agents.list();select.innerHTML='';
        agents.forEach(function(a){var o=document.createElement('option');o.value=a.id;o.textContent=(a.avatar||'')+' '+a.name;select.appendChild(o);});
        if(agents.some(function(a){return a.id===wanted;}))select.value=wanted;
        document.getElementById('agentTurn').disabled=!agents.length;
        document.getElementById('agentAuto').disabled=!agents.length;
        if(!player.view().active)info.textContent=agents.length?(locale==='en'?'Ready':'Prêt'):(locale==='en'?'Apps → Permissions: authorize LaRuche or another agent.':'Apps → Permissions : autorise LaRuche ou un autre agent.');
      }catch(e){info.textContent=e.message;}
    }
    document.getElementById('refreshAgents').onclick=refresh;
    direQuiJoue(player.view());
    document.getElementById('agentTurn').onclick=function(){if(select.value)player.start(select.value,false);};
    document.getElementById('maximizeScore').checked=state.agentGoal!=='2048';
    document.getElementById('agentAuto').onclick=function(){if(select.value){setAgentGoal(document.getElementById('maximizeScore').checked?'max_score':'2048');revision++;render();player.start(select.value,true);}};
    if(locale==='en')document.getElementById('maximizeLabel').textContent='Maximize score, beyond 2048';
    document.getElementById('agentPause').onclick=function(){player.pause();};
    select.onchange=function(){player.pause();};
    sdk.actions.register('game.agent',async function(args){
      if(args.revision!==revision)throw new Error(perimee(args.revision));
      if(args.mode==='pause'){player.pause();await saveChain;return snapshot();}
      var id=args.agentId||'laruche',agents=await sdk.agents.list();
      if(!agents.some(function(a){return a.id===id;}))throw new Error('Authorize this agent in Apps → Permissions → App → Agents first.');
      if(args.revision!==revision)throw new Error(perimee(args.revision));
      // The request explicitly chooses score maximization, including beyond 2048.
      if(args.goal)setAgentGoal(args.goal);
      revision++;select.value=id;player.start(id,true);render();await saveChain;return snapshot();
    });
    if(locale==='en')document.getElementById('agentTurn').textContent='One turn';
    await refresh();
    if(saved&&saved.active&&saved.continuous)player.start(saved.agentId,true);
  }

  if(sdk && engine) start();
})();
