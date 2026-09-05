(function(root, factory){
  'use strict';
  var engine = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = engine;
  } else {
    root.CheckersEngine = engine;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(){
  'use strict';

  var SIZE = 8;
  var WHITE_MAN = 1;
  var WHITE_KING = 2;
  var BLACK_MAN = -1;
  var BLACK_KING = -2;
  var EMPTY = 0;

  function isDarkSquare(row, col) {
    return (row + col) % 2 === 1;
  }

  function indexToPos(index) {
    return { row: Math.floor(index / SIZE), col: index % SIZE };
  }

  function posToIndex(row, col) {
    return row * SIZE + col;
  }

  function inBounds(row, col) {
    return row >= 0 && row < SIZE && col >= 0 && col < SIZE;
  }

  function createBoard() {
    var board = new Array(SIZE * SIZE).fill(EMPTY);
    for (var r = 0; r < SIZE; r += 1) {
      for (var c = 0; c < SIZE; c += 1) {
        if (isDarkSquare(r, c)) {
          var idx = posToIndex(r, c);
          if (r < 3) {
            board[idx] = BLACK_MAN;
          } else if (r > 4) {
            board[idx] = WHITE_MAN;
          }
        }
      }
    }
    return board;
  }

  function isWhite(piece) {
    return piece === WHITE_MAN || piece === WHITE_KING;
  }

  function isBlack(piece) {
    return piece === BLACK_MAN || piece === BLACK_KING;
  }

  function isKing(piece) {
    return piece === WHITE_KING || piece === BLACK_KING;
  }

  function isSameColor(p1, p2) {
    if (p1 === EMPTY || p2 === EMPTY) return false;
    return (isWhite(p1) && isWhite(p2)) || (isBlack(p1) && isBlack(p2));
  }

  function isOpponent(p1, p2) {
    if (p1 === EMPTY || p2 === EMPTY) return false;
    return (isWhite(p1) && isBlack(p2)) || (isBlack(p1) && isWhite(p2));
  }

  function getMoveDirections(piece) {
    if (piece === WHITE_KING || piece === BLACK_KING) {
      return [[-1, -1], [-1, 1], [1, -1], [1, 1]];
    }
    if (piece === WHITE_MAN) {
      return [[-1, -1], [-1, 1]];
    }
    if (piece === BLACK_MAN) {
      return [[1, -1], [1, 1]];
    }
    return [];
  }

  function findJumps(board, startIndex, currentPos, piece, visitedCaptures, pathSoFar) {
    var pos = currentPos || indexToPos(startIndex);
    var dirs = getMoveDirections(piece);
    var jumps = [];

    for (var i = 0; i < dirs.length; i += 1) {
      var d = dirs[i];
      var midRow = pos.row + d[0];
      var midCol = pos.col + d[1];
      var landRow = pos.row + (d[0] * 2);
      var landCol = pos.col + (d[1] * 2);

      if (inBounds(midRow, midCol) && inBounds(landRow, landCol)) {
        var midIdx = posToIndex(midRow, midCol);
        var landIdx = posToIndex(landRow, landCol);
        var midPiece = board[midIdx];
        var landPiece = board[landIdx];

        var notAlreadyCaptured = visitedCaptures.indexOf(midIdx) === -1;
        var landIsEmpty = (landPiece === EMPTY) || (landIdx === startIndex);

        if (isOpponent(piece, midPiece) && notAlreadyCaptured && landIsEmpty) {
          var nextCaptures = visitedCaptures.concat([midIdx]);
          var nextPath = pathSoFar.concat([landIdx]);

          var promoted = false;
          var nextPiece = piece;
          if (piece === WHITE_MAN && landRow === 0) {
            promoted = true;
            nextPiece = WHITE_KING;
          } else if (piece === BLACK_MAN && landRow === SIZE - 1) {
            promoted = true;
            nextPiece = BLACK_KING;
          }

          var subJumps = [];
          if (!promoted) {
            subJumps = findJumps(board, startIndex, { row: landRow, col: landCol }, nextPiece, nextCaptures, nextPath);
          }

          if (subJumps.length > 0) {
            for (var j = 0; j < subJumps.length; j += 1) {
              jumps.push(subJumps[j]);
            }
          } else {
            jumps.push({
              from: startIndex,
              to: landIdx,
              path: nextPath,
              captures: nextCaptures
            });
          }
        }
      }
    }

    return jumps;
  }

  function findSimpleMoves(board, index, piece) {
    var pos = indexToPos(index);
    var dirs = getMoveDirections(piece);
    var moves = [];

    for (var i = 0; i < dirs.length; i += 1) {
      var d = dirs[i];
      var nextRow = pos.row + d[0];
      var nextCol = pos.col + d[1];

      if (inBounds(nextRow, nextCol)) {
        var targetIdx = posToIndex(nextRow, nextCol);
        if (board[targetIdx] === EMPTY) {
          moves.push({
            from: index,
            to: targetIdx,
            path: [index, targetIdx],
            captures: []
          });
        }
      }
    }
    return moves;
  }

  function legalMoves(board, turn) {
    var turnIsWhite = turn === 'white';
    var allJumps = [];
    var allSimple = [];

    for (var i = 0; i < board.length; i += 1) {
      var p = board[i];
      if (p === EMPTY) continue;
      if (turnIsWhite && !isWhite(p)) continue;
      if (!turnIsWhite && !isBlack(p)) continue;

      var jumps = findJumps(board, i, null, p, [], [i]);
      if (jumps.length > 0) {
        for (var j = 0; j < jumps.length; j += 1) {
          allJumps.push(jumps[j]);
        }
      } else {
        var simples = findSimpleMoves(board, i, p);
        for (var s = 0; s < simples.length; s += 1) {
          allSimple.push(simples[s]);
        }
      }
    }

    if (allJumps.length > 0) {
      return allJumps;
    }
    return allSimple;
  }

  function applyMove(board, move) {
    var next = board.slice();
    var piece = next[move.from];
    next[move.from] = EMPTY;

    for (var i = 0; i < move.captures.length; i += 1) {
      next[move.captures[i]] = EMPTY;
    }

    var targetPos = indexToPos(move.to);
    if (piece === WHITE_MAN && targetPos.row === 0) {
      next[move.to] = WHITE_KING;
    } else if (piece === BLACK_MAN && targetPos.row === SIZE - 1) {
      next[move.to] = BLACK_KING;
    } else {
      next[move.to] = piece;
    }

    return next;
  }

  function countPieces(board) {
    var whiteMen = 0;
    var whiteKings = 0;
    var blackMen = 0;
    var blackKings = 0;

    for (var i = 0; i < board.length; i += 1) {
      var p = board[i];
      if (p === WHITE_MAN) whiteMen += 1;
      else if (p === WHITE_KING) whiteKings += 1;
      else if (p === BLACK_MAN) blackMen += 1;
      else if (p === BLACK_KING) blackKings += 1;
    }

    return {
      white: whiteMen + whiteKings,
      whiteMen: whiteMen,
      whiteKings: whiteKings,
      black: blackMen + blackKings,
      blackMen: blackMen,
      blackKings: blackKings
    };
  }

  function gameStatus(board, turn) {
    var counts = countPieces(board);
    if (counts.white === 0) {
      return { over: true, winner: 'black', reason: 'no_pieces' };
    }
    if (counts.black === 0) {
      return { over: true, winner: 'white', reason: 'no_pieces' };
    }

    var available = legalMoves(board, turn);
    if (available.length === 0) {
      var winner = turn === 'white' ? 'black' : 'white';
      return { over: true, winner: winner, reason: 'no_moves' };
    }

    return { over: false, winner: null, reason: 'in_progress' };
  }

  return Object.freeze({
    SIZE: SIZE,
    WHITE_MAN: WHITE_MAN,
    WHITE_KING: WHITE_KING,
    BLACK_MAN: BLACK_MAN,
    BLACK_KING: BLACK_KING,
    EMPTY: EMPTY,
    createBoard: createBoard,
    isDarkSquare: isDarkSquare,
    indexToPos: indexToPos,
    posToIndex: posToIndex,
    legalMoves: legalMoves,
    applyMove: applyMove,
    countPieces: countPieces,
    gameStatus: gameStatus,
    isWhite: isWhite,
    isBlack: isBlack,
    isKing: isKing
  });
});
