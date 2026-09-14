(function(){
  'use strict';

  var sdk = window.LaRucheApp;
  var engine = window.CheckersEngine;
  var ai = window.CheckersAI;
  var storageKey = 'checkers.state.v1';

  var state = null;
  var previous = null;
  var selectedSquare = null;
  var validMovesForSelected = [];
  var saveChain = Promise.resolve();
  var saveRevision = 0;
  var revision = 0;
  var locale = 'fr';
  var translations = {};

  var agentBusy = false;
  var player = null, analysisRevision = -1, analysis = null;
  var agentTimer = null;
  var seat = 'checkers-' + Date.now().toString(36);

  var fallbackCopy = {
    fr: {
      title: "Jeu de Dames",
      eyebrow: "LARUCHE APP",
      white: "Blancs",
      black: "Noirs",
      you: "Vous",
      agent: "Agent",
      aiLocal: "IA locale",
      score: "Pieces restantes",
      turn: "Tour",
      yourTurn: "A votre tour",
      agentTurn: "Tour de l'adversaire",
      thinking: "L'adversaire reflechit...",
      waitingUser: "En attente de votre coup.",
      newGame: "Nouvelle partie",
      undo: "Annuler",
      playAs: "Jouer",
      opponent: "Adversaire",
      agentPlayer: "Agent",
      oneTurn: "Tour agent",
      autoPlay: "Reponse auto",
      pause: "Pause",
      refresh: "Actualiser",
      instructions: "Selectionnez un pion puis la case cible surlignee pour jouer.",
      statusReady: "Partie prete.",
      statusWon: "Victoire ! Vous avez gagne la partie.",
      statusLost: "Defaite. L'adversaire a remporte la partie.",
      statusDraw: "Match nul.",
      mandatoryCapture: "Prise obligatoire.",
      saved: "Synchronise",
      saving: "Sauvegarde...",
      offline: "Stockage local",
      moves: "coups",
      move: "coup",
      permissionRequired: "Autorisation requise",
      noAgentFound: "Aucun agent disponible"
    },
    en: {
      title: "Checkers",
      eyebrow: "LARUCHE APP",
      white: "White",
      black: "Black",
      you: "You",
      agent: "Agent",
      aiLocal: "Local AI",
      score: "Pieces remaining",
      turn: "Turn",
      yourTurn: "Your turn",
      agentTurn: "Opponent turn",
      thinking: "Opponent is thinking...",
      waitingUser: "Waiting for your move.",
      newGame: "New game",
      undo: "Undo",
      playAs: "Play as",
      opponent: "Opponent",
      agentPlayer: "Agent",
      oneTurn: "Agent turn",
      autoPlay: "Auto response",
      pause: "Pause",
      refresh: "Refresh",
      instructions: "Select a piece and then the highlighted destination square to move.",
      statusReady: "Game ready.",
      statusWon: "Victory! You won the game.",
      statusLost: "Defeat. Opponent won the game.",
      statusDraw: "Draw game.",
      mandatoryCapture: "Capture is mandatory.",
      saved: "Synced",
      saving: "Saving...",
      offline: "Local storage",
      moves: "moves",
      move: "move",
      permissionRequired: "Permission required",
      noAgentFound: "No agent available"
    }
  };

  function t(key) {
    if (translations[key]) return translations[key];
    var dict = fallbackCopy[locale] || fallbackCopy.fr;
    return dict[key] || key;
  }

  function clone(val) {
    return JSON.parse(JSON.stringify(val));
  }

  function createInitialState(humanSide, opponentMode) {
    var side = humanSide || 'white';
    var opp = opponentMode || 'agent';
    return {
      board: engine.createBoard(),
      turn: 'white',
      humanSide: side,
      opponentMode: opp,
      moves: 0,
      over: false,
      winner: null
    };
  }

  function validStoredState(s) {
    return s && engine.validBoard(s.board) &&
      (s.turn === 'white' || s.turn === 'black') &&
      (s.humanSide === 'white' || s.humanSide === 'black') &&
      (s.opponentMode === 'agent' || s.opponentMode === 'aiLocal') &&
      Number.isInteger(s.moves) && s.moves >= 0;
  }

  async function loadTranslations(lang) {
    try {
      var res = await fetch('./locales/' + lang + '.json');
      if (res.ok) {
        translations = await res.json();
      }
    } catch (err) {
      translations = fallbackCopy[lang] || fallbackCopy.fr;
    }
  }

  function applyLocale() {
    document.documentElement.lang = locale;
    document.querySelectorAll('[data-i18n]').forEach(function(node) {
      var k = node.dataset.i18n;
      node.textContent = t(k);
    });
  }

  function setSaveStatus(kind) {
    var node = document.getElementById('saveStatus');
    if (!node) return;
    node.className = 'save-status ' + kind;
    node.textContent = t(kind === 'saved' ? 'saved' : kind === 'saving' ? 'saving' : 'offline');
  }

  function persist() {
    var rev = ++saveRevision;
    var snap = clone(state);
    setSaveStatus('saving');
    if (sdk && sdk.ui && sdk.ui.setDirty) {
      sdk.ui.setDirty(true).catch(function(){});
    }
    saveChain = saveChain.catch(function(){}).then(function(){
      if (sdk && sdk.storage) {
        return sdk.storage.set(storageKey, snap);
      }
    }).then(function(){
      if (rev === saveRevision) {
        setSaveStatus('saved');
        if (sdk && sdk.ui && sdk.ui.setDirty) {
          sdk.ui.setDirty(false).catch(function(){});
        }
      }
    }).catch(function(){
      if (rev === saveRevision) {
        setSaveStatus('offline');
        if (sdk && sdk.ui && sdk.ui.setDirty) {
          sdk.ui.setDirty(false).catch(function(){});
        }
      }
    });
  }

  function updateHeaderScores() {
    var counts = engine.countPieces(state.board);
    var scoreWhite = document.getElementById('scoreWhite');
    var scoreBlack = document.getElementById('scoreBlack');
    if (scoreWhite) scoreWhite.textContent = String(counts.white);
    if (scoreBlack) scoreBlack.textContent = String(counts.black);

    var moveCount = document.getElementById('moveCount');
    if (moveCount) {
      moveCount.textContent = state.moves + ' ' + t(state.moves === 1 ? 'move' : 'moves');
    }

    var undoBtn = document.getElementById('undoBtn');
    if (undoBtn) undoBtn.disabled = !previous || state.over;
  }

  function updateTurnBanner() {
    var badge = document.getElementById('turnBadge');
    var label = document.getElementById('turnLabel');
    var details = document.getElementById('turnDetails');

    var isHumanTurn = state.turn === state.humanSide;
    var legal = engine.legalMoves(state.board, state.turn);
    var hasCaptures = legal.some(function(m){ return m.captures.length > 0; });

    if (badge) {
      if (isHumanTurn) {
        badge.className = 'turn-indicator';
      } else {
        badge.className = 'turn-indicator agent-turn';
      }
    }

    if (label) {
      label.textContent = isHumanTurn ? t('yourTurn') : t('agentTurn');
    }

    if (details) {
      if (state.over) {
        details.textContent = state.winner === state.humanSide ? t('statusWon') : t('statusLost');
      } else if (!isHumanTurn) {
        details.textContent = agentBusy || state.opponentMode === 'aiLocal' ? t('thinking') : t('agentTurn');
      } else if (hasCaptures) {
        details.textContent = t('mandatoryCapture');
      } else {
        details.textContent = t('instructions');
      }
    }

    var agentTurnBtn = document.getElementById('agentTurnBtn');
    if (agentTurnBtn) {
      agentTurnBtn.disabled = state.over || isHumanTurn || agentBusy;
    }
  }

  function crownSvg() {
    return '<svg class="crown-icon" viewBox="0 0 24 24"><path d="M5 16L3 7l5 4 4-7 4 7 5-4-2 9H5zm14 3H5v-1h14v1z"/></svg>';
  }

  function renderBoard() {
    var boardNode = document.getElementById('board');
    if (!boardNode) return;
    boardNode.innerHTML = '';

    var isHumanTurn = state.turn === state.humanSide && !state.over;
    var allLegal = isHumanTurn ? engine.legalMoves(state.board, state.turn) : [];
    var movableFroms = allLegal.map(function(m){ return m.from; });

    var targetSquares = validMovesForSelected.map(function(m){ return m.to; });

    for (var i = 0; i < 64; i += 1) {
      var row = Math.floor(i / 8);
      var col = i % 8;
      var isDark = engine.isDarkSquare(row, col);

      var square = document.createElement('div');
      square.className = 'square ' + (isDark ? 'dark' : 'light');
      square.dataset.index = String(i);

      if (selectedSquare === i) {
        square.classList.add('selected');
      }

      if (targetSquares.indexOf(i) !== -1) {
        square.classList.add('target');
      }

      var piece = state.board[i];
      if (piece !== engine.EMPTY) {
        var pieceEl = document.createElement('div');
        var isW = engine.isWhite(piece);
        pieceEl.className = 'piece ' + (isW ? 'white-piece' : 'black-piece');

        if (engine.isKing(piece)) {
          pieceEl.innerHTML = crownSvg();
        }

        if (isHumanTurn && movableFroms.indexOf(i) !== -1) {
          square.classList.add('clickable');
        }

        square.appendChild(pieceEl);
      }

      boardNode.appendChild(square);
    }
  }

  function checkGameOver() {
    var status = engine.gameStatus(state.board, state.turn);
    if (status.over) {
      state.over = true;
      state.winner = status.winner;
      var overlay = document.getElementById('overlay');
      var title = document.getElementById('overlayTitle');
      var text = document.getElementById('overlayText');

      if (overlay && title && text) {
        overlay.hidden = false;
        if (status.winner === state.humanSide) {
          title.textContent = t('statusWon');
        } else {
          title.textContent = t('statusLost');
        }
        text.textContent = t('statusReady');
      }
    }
  }

  function render() {
    updateHeaderScores();
    updateTurnBanner();
    renderBoard();
    checkGameOver();
  }

  function handleSquareClick(index) {
    if (state.over) return;
    if (state.turn !== state.humanSide) return;

    var piece = state.board[index];
    var isOwnPiece = state.humanSide === 'white' ? engine.isWhite(piece) : engine.isBlack(piece);

    if (isOwnPiece) {
      var moves = engine.legalMoves(state.board, state.turn);
      var pieceMoves = moves.filter(function(m){ return m.from === index; });
      if (pieceMoves.length > 0) {
        selectedSquare = index;
        validMovesForSelected = pieceMoves;
        render();
        return;
      }
    }

    if (selectedSquare !== null) {
      var matchingMove = validMovesForSelected.find(function(m){ return m.to === index; });
      if (matchingMove) {
        selectedSquare = null;
        validMovesForSelected = [];
        executeMove(matchingMove);
        return;
      }
    }

    selectedSquare = null;
    validMovesForSelected = [];
    render();
  }

  function executeMove(move) {
    previous = clone(state);
    state.board = engine.applyMove(state.board, move);
    state.moves += 1;
    state.turn = state.turn === 'white' ? 'black' : 'white';
    revision += 1;

    render();
    persist();

    if (!state.over && state.turn !== state.humanSide) {
      scheduleOpponentMove();
    }
  }

  function scheduleOpponentMove() {
    clearTimeout(agentTimer);
    var expectedRevision = revision;
    var agentStatus = document.getElementById('agentStatus');
    if (agentStatus) agentStatus.textContent = t('thinking');

    if (state.opponentMode === 'aiLocal') {
      agentTimer = setTimeout(function(){
        agentTimer = null;
        if (revision !== expectedRevision || state.opponentMode !== 'aiLocal' || state.over || state.turn === state.humanSide) return;
        var best = ai.chooseBestMove(engine, state.board, state.turn, 3);
        if (best) {
          executeMove(best);
        }
      }, 400);
    } else if (state.opponentMode === 'agent') {
      if (player && player.view().active) {
        player.wake();
      } else if (agentStatus) agentStatus.textContent = t('agentTurn');
    }
  }

  function snapshot() {
    var isAgentTurn = !state.over && state.opponentMode === 'agent' && state.turn !== state.humanSide;
    var legal = engine.legalMoves(state.board, state.turn);

    if(isAgentTurn && analysisRevision!==revision){analysis=ai.analyze(engine,state.board,state.turn);analysisRevision=revision;}
    return {
      agent:player?player.view():null,
      analysis:isAgentTurn?analysis:null,
      board: state.board.slice(),
      turn: state.turn,
      humanSide: state.humanSide,
      agentSide: state.humanSide === 'white' ? 'black' : 'white',
      opponentMode: state.opponentMode,
      waitingFor: state.over ? 'finished' : state.turn === state.humanSide ? 'human' : state.opponentMode === 'aiLocal' ? 'localAI' : 'agent',
      rules: engine.rules(),
      piecesRemaining: engine.countPieces(state.board),
      moves: state.moves,
      revision: revision,
      /* null: ce n'est pas votre tour, la question ne se pose pas.
         [] quand c'est votre tour: vous n'avez aucun coup, vous avez perdu.
         Les deux rendaient [], et un agent qui lit une liste vide alors qu'il
         voit des pions sur le plateau conclut que l'etat lui arrive tronque.
         Il est alle chercher le vrai etat dans le stockage prive de l'App. */
      legalMoves: isAgentTurn ? legal : null,
      over: state.over,
      winner: state.winner
    };
  }

  function newGame() {
    clearTimeout(agentTimer); agentTimer = null; if (player) player.reset();
    previous = null;
    selectedSquare = null;
    validMovesForSelected = [];
    var humanSide = document.getElementById('userColor') ? document.getElementById('userColor').value : 'white';
    var oppMode = document.getElementById('opponentMode') ? document.getElementById('opponentMode').value : 'agent';

    state = createInitialState(humanSide, oppMode);
    revision += 1;
    seat = 'checkers-' + Date.now().toString(36);

    var overlay = document.getElementById('overlay');
    if (overlay) overlay.hidden = true;

    render();
    persist();

    if (state.turn !== state.humanSide) {
      scheduleOpponentMove();
    }
  }

  function undo() {
    if (!previous || state.over) return;
    clearTimeout(agentTimer); agentTimer = null; if (player) player.reset();
    state = previous;
    previous = null;
    selectedSquare = null;
    validMovesForSelected = [];
    revision += 1;

    var overlay = document.getElementById('overlay');
    if (overlay) overlay.hidden = true;

    render();
    persist();
  }

  function playerLabel(view) {
    var en=locale==='en';
    var labels=en?{paused:'Paused',ready:'Game active',thinking:'Agent thinking…',waiting:'Game active · waiting for your move',
      retry:'Temporary error · retrying automatically',permission:'Waiting for agent access · Apps → Permissions → App → Agents',finished:'Game finished'}:
      {paused:'En pause',ready:'Partie active',thinking:'L’agent réfléchit…',waiting:'Partie active · en attente de ton coup',
      retry:'Erreur temporaire · reprise automatique',permission:'En attente d’autorisation · Apps → Permissions → App → Agents',finished:'Partie terminée'};
    return (labels[view.status]||view.status)+(view.detail?' · '+view.detail:'');
  }

  function createPlayer() {
    seat=state.agentSeat || seat;state.agentSeat=seat;
    player=window.GameAgent.create({
      state:function(){return {revision:revision,over:state.over,turn:state.turn,humanSide:state.humanSide,opponentMode:state.opponentMode};},
      canPlay:function(s){return s.opponentMode==='agent'&&s.turn!==s.humanSide;},
      list:function(){return sdk.agents.list();},
      invalidate:function(){revision++;},
      save:function(settings){state.agent=settings;persist();},
      changed:function(view){
        agentBusy=view.busy;
        var info=document.getElementById('agentStatus');if(info)info.textContent=playerLabel(view);
        var pause=document.getElementById('agentPauseBtn');if(pause)pause.disabled=!view.active;
        var auto=document.getElementById('agentAutoBtn');if(auto){auto.setAttribute('aria-pressed',String(view.active&&view.continuous));auto.textContent=view.active&&view.continuous?(locale==='en'?'Game active':'Partie active'):t('autoPlay');}
        updateTurnBanner();
      },
      act:function(id,lastError){
        var prompt='Play the agent side in this LaRuche 8x8 checkers game. The engine supplies the exact variant rules, complete legal paths and bounded tactical analysis. Compare the ranked alternatives: score is from your side, not a proof of victory. Prefer favorable exchanges, safe promotion and avoid forced losses. Copy ONE full legal path with from, to and revision. Never play for the human or reset. Return only the game.move action JSON.';
        if(lastError)prompt+=' Previous attempt failed: '+lastError.slice(0,300)+'. Reassess this NEW state.';
        return sdk.agents.act(id,seat,'game.state',prompt,{allowedActions:['game.move'],freshState:true,expectedRevision:revision});
      }
    });
  }

  function triggerAgentTurn(){
    var select=document.getElementById('agentSelect');
    if(player&&select.value)player.start(select.value,false);
  }

  function bindEvents() {
    var boardNode = document.getElementById('board');
    if (boardNode) {
      boardNode.addEventListener('click', function(e) {
        var square = e.target.closest('.square');
        if (square && square.dataset.index) {
          handleSquareClick(parseInt(square.dataset.index, 10));
        }
      });
    }

    var newGameBtn = document.getElementById('newGameBtn');
    if (newGameBtn) newGameBtn.addEventListener('click', newGame);

    var restartBtn = document.getElementById('restartBtn');
    if (restartBtn) restartBtn.addEventListener('click', newGame);

    var undoBtn = document.getElementById('undoBtn');
    if (undoBtn) undoBtn.addEventListener('click', undo);

    var userColor = document.getElementById('userColor');
    if (userColor) {
      userColor.addEventListener('change', function() {
        newGame();
      });
    }

    var opponentMode = document.getElementById('opponentMode');
    if (opponentMode) {
      opponentMode.addEventListener('change', function() {
        clearTimeout(agentTimer); agentTimer = null; if (player) player.reset();
        state.opponentMode = opponentMode.value;
        revision += 1;
        var agentSection = document.getElementById('agentSection');
        if (agentSection) {
          agentSection.style.display = state.opponentMode === 'agent' ? 'flex' : 'none';
        }
        persist();
        if (!state.over && state.turn !== state.humanSide) {
          scheduleOpponentMove();
        }
      });
    }

    var refreshAgentsBtn = document.getElementById('refreshAgentsBtn');
    if (refreshAgentsBtn) refreshAgentsBtn.addEventListener('click', refreshAgentList);

    var agentTurnBtn = document.getElementById('agentTurnBtn');
    if (agentTurnBtn) agentTurnBtn.addEventListener('click', triggerAgentTurn);

    var agentAutoBtn = document.getElementById('agentAutoBtn');
    if(agentAutoBtn)agentAutoBtn.addEventListener('click',function(){
      var id=document.getElementById('agentSelect').value;if(id&&player)player.start(id,true);
    });
    document.getElementById('agentPauseBtn').addEventListener('click',function(){if(player)player.pause();});
    document.getElementById('agentSelect').addEventListener('change',function(){if(player)player.pause();});
  }

  function refreshAgentList() {
    var select = document.getElementById('agentSelect');
    var info = document.getElementById('agentStatus');
    if (!sdk || !sdk.agents || !sdk.agents.list) {
      if (info) info.textContent = t('offline');
      return;
    }

    return sdk.agents.list().then(function(agents) {
      var selected=(player&&player.view().agentId)||select.value||(state.agent&&state.agent.agentId)||'laruche';
      if (!select) return;
      select.innerHTML = '';
      if (!agents || agents.length === 0) {
        var opt = document.createElement('option');
        opt.value = '';
        opt.textContent = t('noAgentFound');
        select.appendChild(opt);
        document.getElementById('agentAutoBtn').disabled=true;
        if (info) info.textContent = locale==='en'?'Apps → Permissions: enable agents.invoke and authorize LaRuche or another agent.':'Apps → Permissions : active agents.invoke et autorise LaRuche ou un autre agent.';
        return;
      }
      agents.forEach(function(a) {
        var o = document.createElement('option');
        o.value = a.id;
        o.textContent = (a.avatar || '') + ' ' + a.name;
        select.appendChild(o);
      });
      if(agents.some(function(a){return a.id===selected;}))select.value=selected;
      document.getElementById('agentAutoBtn').disabled=false;
      if (info && (!player||!player.view().active)) info.textContent = t('statusReady');
    }).catch(function(err) {
      if (info) info.textContent = err.message;
    });
  }

  /* Un refus qui ne dit pas la valeur courante oblige a relire l'etat avant
     de pouvoir reessayer, et le tour d'apres la revision a encore bouge. */
  function perimee(recue) {
    return 'Stale revision ' + recue + ', current is ' + revision +
      '. Read game.state again and use the revision it returns.';
  }

  function registerSdkActions() {
    if (!sdk || !sdk.actions || !sdk.actions.register) return;

    sdk.actions.register('game.state', function() {
      return snapshot();
    });

    sdk.actions.register('game.move', async function(args) {
      if (state.over) {
        throw new Error('Game is already over');
      }
      if (args.revision !== revision) {
        throw new Error(perimee(args.revision));
      }
      if (state.turn === state.humanSide) {
        throw new Error('Not agent turn: waiting for human move');
      }
      if (state.opponentMode !== 'agent') throw new Error('Local AI controls this side. Ask the human to select Agent mode.');
      var matching = engine.resolveMove(state.board, state.turn, args);

      executeMove(matching);
      await saveChain;
      return snapshot();
    });

    sdk.actions.register('game.agent',async function(args){
      if(args.revision!==revision)throw new Error(perimee(args.revision));
      if(args.mode==='pause'){player.pause();await saveChain;return snapshot();}
      if(state.opponentMode!=='agent')throw new Error('Select Agent opponent mode first.');
      var id=args.agentId||'laruche';
      var agents=await sdk.agents.list();
      if(!agents.some(function(a){return a.id===id;}))throw new Error('Authorize this agent in Apps → Permissions → App → Agents first.');
      if(args.revision!==revision)throw new Error(perimee(args.revision));
      revision++;document.getElementById('agentSelect').value=id;player.start(id,true);
      await saveChain;return snapshot();
    });

    sdk.actions.register('game.new', async function(args) {
      if (args.revision !== revision) {
        throw new Error(perimee(args.revision));
      }
      newGame();
      await saveChain;
      return snapshot();
    });
  }

  async function start() {
    bindEvents();

    if (!sdk) {
      state = createInitialState('white', 'aiLocal');
      render();
      setSaveStatus('offline');
      return;
    }

    try {
      var context = await sdk.ready();
      locale = context.locale === 'en' ? 'en' : 'fr';
      document.documentElement.dataset.hostTheme = context.theme || 'default';
      await loadTranslations(locale);
      applyLocale();

      var saved = await sdk.storage.get(storageKey);
      if (validStoredState(saved)) {
        state = saved;
        var restoredStatus = engine.gameStatus(state.board, state.turn);
        state.over = restoredStatus.over; state.winner = restoredStatus.winner;
        setSaveStatus('saved');
      } else {
        state = createInitialState('white', 'agent');
        persist();
      }

      var userColorEl = document.getElementById('userColor');
      if (userColorEl) userColorEl.value = state.humanSide;

      var opponentModeEl = document.getElementById('opponentMode');
      if (opponentModeEl) opponentModeEl.value = state.opponentMode;

      var agentSection = document.getElementById('agentSection');
      if (agentSection) {
        agentSection.style.display = state.opponentMode === 'agent' ? 'flex' : 'none';
      }

      render();
      await sdk.ui.setTitle(t('title'));
      registerSdkActions();
      createPlayer();
      await refreshAgentList();
      if(state.agent&&state.agent.active&&state.agent.continuous)player.start(state.agent.agentId,true);
      await sdk.ui.setStatus('ready', t('statusReady'), 100);

      if (!state.over && state.turn !== state.humanSide) {
        scheduleOpponentMove();
      }
    } catch (e) {
      state = createInitialState('white', 'aiLocal');
      render();
      setSaveStatus('offline');
      sdk.ui.setStatus('error', String(e.message || e).slice(0, 180)).catch(function(){});
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start);
  } else {
    start();
  }
})();
