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

assert(engine.validBoard(engine.createBoard()));
var corrupt = engine.createBoard(); corrupt[0] = 1;
assert.equal(engine.validBoard(corrupt), false, 'pieces cannot occupy light squares');
corrupt = engine.createBoard(); corrupt[1] = 99;
assert.equal(engine.validBoard(corrupt), false, 'unknown piece values are rejected');

// This variant does not permit backward captures by men or flying kings.
var backwards = Array(64).fill(0); backwards[26] = 1; backwards[35] = -1;
assert(engine.legalMoves(backwards,'white').every(m=>m.captures.length===0));
var shortKing = Array(64).fill(0); shortKing[26] = 2;
assert(engine.legalMoves(shortKing,'white').every(m=>Math.abs(Math.floor(m.to/8)-3)===1));

// Captures on any piece prohibit every simple move, without a longest-chain rule.
var choices = Array(64).fill(0); choices[42]=1; choices[46]=1; choices[33]=-1; choices[17]=-1; choices[37]=-1;
var captures = engine.legalMoves(choices,'white');
assert(captures.every(m=>m.captures.length>0));
assert(captures.some(m=>m.captures.length===1));assert(captures.some(m=>m.captures.length===2));

var promotion = Array(64).fill(0); promotion[17]=1; promotion[10]=-1; promotion[12]=-1;
var crowned = engine.legalMoves(promotion,'white').find(m=>m.to===3);
assert(crowned);assert.equal(crowned.captures.length,1,'promotion ends a capture turn');
assert.equal(engine.applyMove(promotion,crowned)[3],2);

var diamond = Array(64).fill(0); diamond[42]=2; [33,17,19,35].forEach(i=>diamond[i]=-1);
var loops = engine.legalMoves(diamond,'white').filter(m=>m.to===42);
assert(loops.length>1,'several paths share the same endpoints');
assert.throws(()=>engine.resolveMove(diamond,'white',{from:42,to:42}),/Ambiguous/);
assert.deepEqual(engine.resolveMove(diamond,'white',{from:42,to:42,path:loops[0].path}),loops[0]);
assert.throws(()=>engine.resolveMove(diamond,'white',{from:42,to:42,path:[42,42]}),/Illegal/);
assert.equal(engine.countPieces(engine.applyMove(diamond,loops[0])).black,0);
console.log('Checkers contract: orientation, mandatory/complete captures, promotion, path ambiguity and stored boards passed.');
