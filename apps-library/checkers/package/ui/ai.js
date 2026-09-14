(function(root, factory){
  'use strict';
  var ai = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = ai;
  } else {
    root.CheckersAI = ai;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(){
  'use strict';

  function evaluateBoard(engine, board) {
    var score = 0;
    for (var i = 0; i < board.length; i += 1) {
      var p = board[i];
      if (p === engine.EMPTY) continue;
      var pos = engine.indexToPos(i);

      if (p === engine.WHITE_MAN) {
        score += 100 + (7 - pos.row) * 10;
        if (pos.col > 1 && pos.col < 6) score += 5;
      } else if (p === engine.WHITE_KING) {
        score += 300;
        if (pos.row > 1 && pos.row < 6 && pos.col > 1 && pos.col < 6) score += 15;
      } else if (p === engine.BLACK_MAN) {
        score -= 100 + (pos.row) * 10;
        if (pos.col > 1 && pos.col < 6) score -= 5;
      } else if (p === engine.BLACK_KING) {
        score -= 300;
        if (pos.row > 1 && pos.row < 6 && pos.col > 1 && pos.col < 6) score -= 15;
      }
    }
    return score;
  }

  function minimax(engine, board, depth, alpha, beta, isMaximizing, budget) {
    if (budget && ++budget.nodes > budget.limit) throw budget;
    var turn = isMaximizing ? 'white' : 'black';
    var moves = engine.legalMoves(board, turn);

    if (depth === 0 || moves.length === 0) {
      if (moves.length === 0) {
        return isMaximizing ? -10000 : 10000;
      }
      return evaluateBoard(engine, board);
    }

    if (isMaximizing) {
      var maxEval = -Infinity;
      for (var i = 0; i < moves.length; i += 1) {
        var nextBoard = engine.applyMove(board, moves[i]);
        var evaluation = minimax(engine, nextBoard, depth - 1, alpha, beta, false, budget);
        maxEval = Math.max(maxEval, evaluation);
        alpha = Math.max(alpha, evaluation);
        if (beta <= alpha) break;
      }
      return maxEval;
    } else {
      var minEval = Infinity;
      for (var j = 0; j < moves.length; j += 1) {
        var nextBoard2 = engine.applyMove(board, moves[j]);
        var evaluation2 = minimax(engine, nextBoard2, depth - 1, alpha, beta, true, budget);
        minEval = Math.min(minEval, evaluation2);
        beta = Math.min(beta, evaluation2);
        if (beta <= alpha) break;
      }
      return minEval;
    }
  }

  function chooseBestMove(engine, board, turn, depth) {
    var searchDepth = depth || 3;
    var moves = engine.legalMoves(board, turn);
    if (moves.length === 0) return null;
    if (moves.length === 1) return moves[0];

    var isMaximizing = turn === 'white';
    var bestMove = moves[0];
    var bestValue = isMaximizing ? -Infinity : Infinity;

    for (var i = 0; i < moves.length; i += 1) {
      var nextBoard = engine.applyMove(board, moves[i]);
      var value = minimax(engine, nextBoard, searchDepth - 1, -Infinity, Infinity, !isMaximizing);

      if (isMaximizing) {
        if (value > bestValue) {
          bestValue = value;
          bestMove = moves[i];
        }
      } else {
        if (value < bestValue) {
          bestValue = value;
          bestMove = moves[i];
        }
      }
    }

    return bestMove;
  }

  // Iterative deepening: retain only fully evaluated root iterations so a
  // budget cutoff cannot favor whichever move happened to be searched first.
  function analyze(engine, board, turn, settings) {
    settings = settings || {};
    var moves = engine.legalMoves(board, turn), ranked = [], completed = 0;
    var budget = {nodes:0, limit:settings.nodeLimit || 10000};
    var sign = turn === 'white' ? 1 : -1;
    ranked = moves.map(function(move){
      var next = engine.applyMove(board, move);
      return Object.assign({}, move, {score:sign * evaluateBoard(engine, next),
        promotes: !engine.isKing(board[move.from]) && engine.isKing(next[move.to])});
    });
    for (var depth = 1; depth <= (settings.depth || 5); depth++) {
      try {
        var iteration = ranked.map(function(move){
          var next = engine.applyMove(board, move);
          return Object.assign({}, move, {score:sign * minimax(engine, next, depth - 1,
            -Infinity, Infinity, turn !== 'white', budget)});
        });
        ranked = iteration.sort(function(a,b){return b.score-a.score;});
        completed = depth;
      } catch (cutoff) { if(cutoff !== budget) throw cutoff; break; }
    }
    return {method:'Bounded alpha-beta; higher score favors the agent. Not a proof of a forced win.',
      depth:completed,nodes:budget.nodes,candidates:ranked};
  }

  return Object.freeze({
    analyze: analyze,
    chooseBestMove: chooseBestMove
  });
});
