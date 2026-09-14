'use strict';
const assert=require('node:assert/strict');
const checkers=require('../checkers/package/ui/game.js'),checkersAI=require('../checkers/package/ui/ai.js');
const game=require('../2048/package/ui/game.js'),ai=require('../2048/package/ui/ai.js');
for(const side of ['white','black']){
  const board=Array(64).fill(0);board[side==='white'?42:17]=side==='white'?1:-1;board[side==='white'?33:26]=side==='white'?-1:1;
  const report=checkersAI.analyze(checkers,board,side);
  assert(report.depth>0);assert.equal(report.candidates[0].score,10000);
  assert.equal(checkers.gameStatus(checkers.applyMove(board,report.candidates[0]),side==='white'?'black':'white').over,true);
  assert.deepEqual(checkers.resolveMove(board,side,report.candidates[0]).path,report.candidates[0].path);
}
function random(seed){return ()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;};}
function play(seed,smart){
  const rng=random(seed),choose=random(seed+99);let board=game.createBoard(rng),score=0,moves=0,maxMs=0;
  while(game.canMove(board)&&moves<800){
    let d;
    if(smart){const start=performance.now(),report=ai.analyze(game,board);maxMs=Math.max(maxMs,performance.now()-start);
      const legal=game.describe(board,true).legalMoves;
      assert.equal(report.candidates.length,legal.length);report.candidates.forEach(c=>assert(legal.includes(c.direction)));
      d=report.candidates[0].direction;
      assert.deepEqual(report.candidates[0].boardBeforeSpawn,game.move(board,d).board);
    }else{const legal=game.describe(board,true).legalMoves;d=legal[Math.floor(choose()*legal.length)];}
    const next=game.move(board,d);score+=next.gained;board=game.addRandom(next.board,rng);moves++;
  }
  return {score,moves,tile:Math.max(...board),maxMs:Math.round(maxMs)};
}
let smart=0,baseline=0;
for(const seed of [7,31,101]){const a=play(seed,true),b=play(seed,false);smart+=a.score;baseline+=b.score;console.log('2048 seeded comparison',seed,'search',a,'random',b);}
assert(smart>baseline*2,'bounded search should materially outperform random play over fixed seeds');
console.log('Game strategy: legal-only choices, winning checkers captures for both sides, exact 2048 slide predictions and seeded quality benchmark passed.');
