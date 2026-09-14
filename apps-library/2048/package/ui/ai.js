(function(root,factory){
  if(typeof module==='object'&&module.exports)module.exports=factory();
  else root.Game2048AI=factory();
})(typeof globalThis!=='undefined'?globalThis:this,function(){
  'use strict';
  // Evaluate space, smoothness, merge potential and monotonic rows/columns.
  // Scores are heuristic utilities, never predicted final game scores.
  function evaluate(board){
    var logs=board.map(function(v){return v?Math.log2(v):0;}),empty=0,smooth=0,merges=0,mono=0;
    for(var i=0;i<16;i++){
      if(!board[i])empty++;
      [i%4<3?i+1:-1,i<12?i+4:-1].forEach(function(j){
        if(j<0||!logs[i]||!logs[j])return;
        smooth+=Math.abs(logs[i]-logs[j]);
        if(logs[i]===logs[j])merges+=logs[i];
      });
    }
    for(var axis=0;axis<2;axis++)for(var line=0;line<4;line++){
      var up=0,down=0;
      for(var x=0;x<3;x++){
        var a=logs[axis?x*4+line:line*4+x],b=logs[axis?(x+1)*4+line:line*4+x+1];
        up+=Math.max(0,b-a);down+=Math.max(0,a-b);
      }
      mono+=Math.min(up,down);
    }
    var max=Math.max.apply(null,logs),corner=Math.max(logs[0],logs[3],logs[12],logs[15])===max;
    return empty*270 + merges*20 - smooth*12 - mono*75 + max*35 + (corner?max*90:-max*100);
  }
  function analyze(engine,board,settings){
    settings=settings||{};
    var nodes=0,limit=settings.nodeLimit||12000,cache=new Map(),cutoff={};
    function count(){if(++nodes>limit)throw cutoff;}
    function player(b,depth){
      count();if(!depth)return evaluate(b);
      var key=depth+':'+b.join(',');if(cache.has(key))return cache.get(key);
      var best=-1000000;
      ['left','right','up','down'].forEach(function(d){
        var moved=engine.move(b,d);
        if(moved.moved)best=Math.max(best,chance(moved.board,depth-1)+Math.log2(moved.gained+1)*6);
      });
      cache.set(key,best);return best;
    }
    function chance(b,depth){
      count();var empty=[];b.forEach(function(v,i){if(!v)empty.push(i);});
      if(!empty.length)return player(b,depth);
      var value=0;
      empty.forEach(function(i){
        var next=b.slice();next[i]=2;value+=0.9*player(next,depth);
        next[i]=4;value+=0.1*player(next,depth);
      });
      return value/empty.length;
    }
    var candidates=['left','right','up','down'].map(function(d){
      var m=engine.move(board,d);
      return m.moved?{direction:d,boardBeforeSpawn:m.board,gained:m.gained,
        emptyAfterSlide:m.board.filter(function(v){return !v;}).length,utility:evaluate(m.board)}:null;
    }).filter(Boolean);
    var completed=0;
    for(var depth=1;depth<=(settings.depth||2);depth++){
      try{
        var ranked=candidates.map(function(c){return Object.assign({},c,{utility:Math.round(chance(c.boardBeforeSpawn,depth-1)+Math.log2(c.gained+1)*6)});});
        candidates=ranked.sort(function(a,b){return b.utility-a.utility;});completed=depth;
      }catch(e){if(e!==cutoff)throw e;break;}
    }
    candidates.sort(function(a,b){return b.utility-a.utility;});
    return {method:'Expectimax over 2 (90%) / 4 (10%) spawns; higher utility is better, not a guaranteed score.',depth:completed,nodes:nodes,candidates:candidates};
  }
  return {analyze:analyze,evaluate:evaluate};
});
