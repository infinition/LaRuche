'use strict';
var assert=require('node:assert/strict');
var game=require('../package/ui/game.js');

function board(rows){ return rows.flat(); }

var start=board([
  [2,2,2,2],
  [0,0,0,0],
  [0,0,0,0],
  [0,0,0,0]
]);
var left=game.move(start,'left');
assert.deepEqual(left.board,board([[4,4,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]]));
assert.equal(left.gained,8);
assert.equal(left.moved,true);
assert.deepEqual(start.slice(0,4),[2,2,2,2]);

var right=game.move(board([[2,2,4,4],[0,0,0,0],[0,0,0,0],[0,0,0,0]]),'right');
assert.deepEqual(right.board.slice(0,4),[0,0,4,8]);
assert.equal(right.gained,12);

var up=game.move(board([[2,0,0,0],[2,0,0,0],[4,0,0,0],[4,0,0,0]]),'up');
assert.deepEqual([up.board[0],up.board[4],up.board[8],up.board[12]],[4,8,0,0]);

var full=board([[2,4,2,4],[4,2,4,2],[2,4,2,4],[4,2,4,2]]);
assert.equal(game.canMove(full),false);
assert.equal(game.move(full,'left').moved,false);
assert.equal(game.canMove(board([[2,2,4,8],[16,32,64,128],[256,512,1024,2],[4,8,16,32]])),true);
assert.equal(game.hasWon(board([[2048,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]])),true);

var randomValues=[0,0.5];
var added=game.addRandom(Array(16).fill(0),function(){ return randomValues.shift(); });
assert.equal(added[0],2);
assert.equal(added.filter(Boolean).length,1);

console.log('2048 engine: all tests passed');

var victory = Array(16).fill(0); victory[0]=2048;
assert.equal(game.describe(victory,false).waitingFor,'humanContinue');
assert.equal(game.describe(victory,false).won,true);
assert.equal(game.describe(victory,false).over,false);
assert.deepEqual(game.describe(victory,false).legalMoves,[]);
assert.equal(game.describe(victory,true).waitingFor,'move');
assert(game.describe(victory,true).legalMoves.length>0);
assert.equal(game.describe(full,false).waitingFor,'finished');
assert.equal(game.describe(full,false).over,true);
var invalid=Array(16).fill(0);invalid[0]=1;assert.equal(game.validBoard(invalid),false);
invalid[0]=4294967297;assert.equal(game.validBoard(invalid),false,'do not truncate values to 32 bits');
assert.deepEqual(game.move(board([[2,2,4,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]]),'left').board.slice(0,4),[4,4,0,0]);
console.log('2048 contract: victory pause, continuation, terminal state and numeric validation passed.');
