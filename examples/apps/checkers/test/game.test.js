'use strict';
var assert = require('node:assert/strict');
var engine = require('../package/ui/game.js');

var board = engine.createBoard();
var counts = engine.countPieces(board);

assert.equal(counts.white, 12);
assert.equal(counts.black, 12);
assert.equal(board.length, 64);

var whiteMoves = engine.legalMoves(board, 'white');
assert.equal(whiteMoves.length, 7);
assert.ok(whiteMoves.every(function(m){ return m.captures.length === 0; }));

var jumpBoard = new Array(64).fill(0);
jumpBoard[engine.posToIndex(5, 2)] = engine.WHITE_MAN;
jumpBoard[engine.posToIndex(4, 3)] = engine.BLACK_MAN;
var forcedJumpMoves = engine.legalMoves(jumpBoard, 'white');
assert.equal(forcedJumpMoves.length, 1);
assert.equal(forcedJumpMoves[0].from, engine.posToIndex(5, 2));
assert.equal(forcedJumpMoves[0].to, engine.posToIndex(3, 4));
assert.deepEqual(forcedJumpMoves[0].captures, [engine.posToIndex(4, 3)]);

var jumpedBoard = engine.applyMove(jumpBoard, forcedJumpMoves[0]);
assert.equal(jumpedBoard[engine.posToIndex(5, 2)], engine.EMPTY);
assert.equal(jumpedBoard[engine.posToIndex(4, 3)], engine.EMPTY);
assert.equal(jumpedBoard[engine.posToIndex(3, 4)], engine.WHITE_MAN);

var doubleJumpBoard = new Array(64).fill(0);
doubleJumpBoard[engine.posToIndex(5, 2)] = engine.WHITE_MAN;
doubleJumpBoard[engine.posToIndex(4, 3)] = engine.BLACK_MAN;
doubleJumpBoard[engine.posToIndex(2, 5)] = engine.BLACK_MAN;
var doubleMoves = engine.legalMoves(doubleJumpBoard, 'white');
assert.equal(doubleMoves.length, 1);
assert.equal(doubleMoves[0].from, engine.posToIndex(5, 2));
assert.equal(doubleMoves[0].to, engine.posToIndex(1, 6));
assert.equal(doubleMoves[0].captures.length, 2);
assert.deepEqual(doubleMoves[0].captures, [engine.posToIndex(4, 3), engine.posToIndex(2, 5)]);

var doubleApplied = engine.applyMove(doubleJumpBoard, doubleMoves[0]);
assert.equal(doubleApplied[engine.posToIndex(4, 3)], engine.EMPTY);
assert.equal(doubleApplied[engine.posToIndex(2, 5)], engine.EMPTY);
assert.equal(doubleApplied[engine.posToIndex(1, 6)], engine.WHITE_MAN);

var promoBoard = new Array(64).fill(0);
promoBoard[engine.posToIndex(1, 2)] = engine.WHITE_MAN;
var promoMoves = engine.legalMoves(promoBoard, 'white');
var appliedPromo = engine.applyMove(promoBoard, promoMoves[0]);
assert.equal(appliedPromo[promoMoves[0].to], engine.WHITE_KING);

var kingBoard = new Array(64).fill(0);
kingBoard[engine.posToIndex(3, 3)] = engine.WHITE_KING;
var kingMoves = engine.legalMoves(kingBoard, 'white');
assert.equal(kingMoves.length, 4);

var emptyStatus = engine.gameStatus(new Array(64).fill(0), 'white');
assert.equal(emptyStatus.over, true);

console.log('Checkers engine: all tests passed successfully.');
