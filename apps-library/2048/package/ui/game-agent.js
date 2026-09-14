/* Shared turn scheduler. Copied into each game package by build.py. */
(function(root,factory){
  if(typeof module==='object'&&module.exports)module.exports=factory();
  else root.GameAgent=factory();
})(typeof globalThis!=='undefined'?globalThis:this,function(){
  'use strict';
  function create(options){
    var active=false,continuous=false,busy=false,agentId='',timer=null,generation=0,failures=0;
    var status='paused',detail='',lastMove=null;
    var later=options.setTimeout||setTimeout,clear=options.clearTimeout||clearTimeout;
    function view(){return {active:active,continuous:continuous,busy:busy,agentId:agentId,status:status,detail:detail,failures:failures,lastMove:lastMove};}
    function paint(next,message){status=next;detail=message||'';if(options.changed)options.changed(view());}
    function save(){if(options.save)options.save({active:active,continuous:continuous,agentId:agentId});}
    function cancelTimer(){if(timer!==null)clear(timer);timer=null;}
    function schedule(ms){cancelTimer();if(active)timer=later(function(){timer=null;tick();},ms);}
    function wake(){if(active&&!busy&&timer===null)schedule(0);}
    async function tick(){
      if(!active||busy)return;
      var current=options.state();
      if(current.over){active=false;save();paint('finished');return;}
      if(!options.canPlay(current)){paint('waiting');return;}
      busy=true;
      var token=generation,revision=current.revision,previousError=detail,started=Date.now();
      paint('thinking');
      try{
        // Re-read permissions each attempt. A pending game can recover when
        // access is granted without issuing model requests while denied.
        var agents=await options.list();
        if(token!==generation||!active)return;
        if(!agents.some(function(a){return a.id===agentId;})){
          paint('permission');schedule(15000);return;
        }
        var result=await options.act(agentId,failures?previousError:'');
        if(token!==generation||!active)return;
        var after=options.state();
        if(after.revision===revision)throw new Error('No move applied; choose an exact legal action from the new state.');
        failures=0;lastMove={model:result&&result.model,text:result&&String(result.text||'').slice(0,500)};detail='';
        if(!continuous){active=false;save();paint('paused');}
        else if(after.over){active=false;save();paint('finished');}
        else if(!options.canPlay(after))paint('waiting');
        else {paint('ready');schedule(Math.max(options.delay===undefined?80:options.delay,(options.minimumInterval||0)-(Date.now()-started)));}
      }catch(error){
        if(token!==generation||!active)return;
        var observed=options.state();
        // The action may have completed before a lost HTTP reply. Never
        // replay cached coordinates: the next act reads a fresh game state.
        if(observed.revision!==revision){
          failures=0;
          if(!continuous){active=false;save();paint('paused');}
          else if(observed.over){active=false;save();paint('finished');}
          else if(!options.canPlay(observed))paint('waiting');
          else schedule(options.delay||80);
        }else{
          failures++;
          var message=String(error.message||error);
          var denied=/permission|authoriz|forbidden|not allowed|révoqu|revoked/i.test(message);
          var wait=denied?15000:/rate|limit|429/i.test(message)?30000:Math.min(30000,1000*Math.pow(2,Math.min(failures-1,5)));
          paint(denied?'permission':'retry',message);
          schedule(wait);
        }
      }finally{
        busy=false;
        if(token!==generation){wake();}
        if(options.changed)options.changed(view());
      }
    }
    return {
      view:view,wake:wake,
      start:function(id,auto){
        if(!id)throw new Error('Select an authorized agent first.');
        if((active||busy)&&options.invalidate)options.invalidate();
        generation++;cancelTimer();agentId=id;continuous=auto!==false;active=true;failures=0;detail='';
        save();paint('ready');wake();
      },
      pause:function(){generation++;cancelTimer();active=false;failures=0;if(options.invalidate)options.invalidate();save();paint('paused');},
      // Invalidate a pending model answer on undo/new game/manual takeover.
      reset:function(){generation++;cancelTimer();active=false;failures=0;if(options.invalidate)options.invalidate();save();paint('paused');}
    };
  }
  return {create:create};
});
