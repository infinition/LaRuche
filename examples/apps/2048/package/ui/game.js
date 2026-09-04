(function(root,factory){
  'use strict';
  var engine=factory();
  if(typeof module==='object' && module.exports) module.exports=engine;
  else root.Game2048Engine=engine;
})(typeof globalThis!=='undefined'?globalThis:this,function(){
  'use strict';
  var SIZE=4;
  var DIRECTIONS=['left','right','up','down'];

  function validBoard(board){
    return Array.isArray(board) && board.length===SIZE*SIZE && board.every(function(value){
      return Number.isInteger(value) && value>=0 && (value===0 || (value&(value-1))===0);
    });
  }

  function slideLine(line){
    var values=line.filter(function(value){ return value!==0; });
    var output=[];
    var gained=0;
    for(var index=0;index<values.length;index+=1){
      if(values[index]===values[index+1]){
        var merged=values[index]*2;
        output.push(merged);
        gained+=merged;
        index+=1;
      }else output.push(values[index]);
    }
    while(output.length<SIZE) output.push(0);
    return {line:output,gained:gained};
  }

  function coordinates(direction,outer){
    var result=[];
    for(var inner=0;inner<SIZE;inner+=1){
      if(direction==='left') result.push(outer*SIZE+inner);
      if(direction==='right') result.push(outer*SIZE+(SIZE-1-inner));
      if(direction==='up') result.push(inner*SIZE+outer);
      if(direction==='down') result.push((SIZE-1-inner)*SIZE+outer);
    }
    return result;
  }

  function move(board,direction){
    if(!validBoard(board)) throw new Error('Invalid 2048 board');
    if(DIRECTIONS.indexOf(direction)===-1) throw new Error('Invalid direction');
    var output=Array(SIZE*SIZE).fill(0);
    var gained=0;
    for(var outer=0;outer<SIZE;outer+=1){
      var indexes=coordinates(direction,outer);
      var shifted=slideLine(indexes.map(function(index){ return board[index]; }));
      gained+=shifted.gained;
      indexes.forEach(function(index,position){ output[index]=shifted.line[position]; });
    }
    return {
      board:output,
      gained:gained,
      moved:output.some(function(value,index){ return value!==board[index]; })
    };
  }

  function addRandom(board,random){
    if(!validBoard(board)) throw new Error('Invalid 2048 board');
    var empty=[];
    board.forEach(function(value,index){ if(value===0) empty.push(index); });
    if(!empty.length) return board.slice();
    var next=board.slice();
    var rng=typeof random==='function'?random:Math.random;
    var slot=Math.min(empty.length-1,Math.floor(Math.max(0,rng())*empty.length));
    next[empty[slot]]=rng()<0.9?2:4;
    return next;
  }

  function createBoard(random){
    return addRandom(addRandom(Array(SIZE*SIZE).fill(0),random),random);
  }

  function canMove(board){
    if(!validBoard(board)) return false;
    if(board.indexOf(0)!==-1) return true;
    for(var row=0;row<SIZE;row+=1){
      for(var column=0;column<SIZE;column+=1){
        var index=row*SIZE+column;
        if(column+1<SIZE && board[index]===board[index+1]) return true;
        if(row+1<SIZE && board[index]===board[index+SIZE]) return true;
      }
    }
    return false;
  }

  function hasWon(board){
    return validBoard(board) && board.some(function(value){ return value>=2048; });
  }

  return Object.freeze({
    size:SIZE,
    validBoard:validBoard,
    move:move,
    addRandom:addRandom,
    createBoard:createBoard,
    canMove:canMove,
    hasWon:hasWon
  });
});
