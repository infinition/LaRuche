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

  var copy={
    fr:{score:'Score',best:'Record',undo:'Annuler',newGame:'Nouvelle partie',continue:'Continuer',restart:'Rejouer',instructions:'Utilise les flèches, ZQSD ou glisse sur la grille.',moves:'coups',move:'coup',saved:'Synchronisé',saving:'Sauvegarde...',offline:'Stockage indisponible',won:'2048 atteint !',wonText:'Tu peux continuer pour viser encore plus haut.',lost:'Partie terminée',lostText:'Plus aucun mouvement possible.'},
    en:{score:'Score',best:'Best',undo:'Undo',newGame:'New game',continue:'Keep going',restart:'Play again',instructions:'Use arrow keys, WASD or swipe the board.',moves:'moves',move:'move',saved:'Synced',saving:'Saving...',offline:'Storage unavailable',won:'You reached 2048!',wonText:'Keep going to chase a higher tile.',lost:'Game over',lostText:'There are no moves left.'}
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
    render();
    persist();
  }

  function newGame(){
    previous=state?clone(state):null;
    state=fresh(state&&state.best);
    render();
    persist();
  }

  function bind(){
    document.addEventListener('keydown',function(event){
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
    document.getElementById('continueGame').addEventListener('click',function(){ state.keepPlaying=true; render(); persist(); });
    document.getElementById('undo').addEventListener('click',function(){
      if(!previous) return;
      var current=clone(state);
      state=previous;
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
    }).catch(function(){
      state=fresh(0);
      render();
      setSaveStatus('offline');
    });
  }

  if(sdk && engine) start();
})();
