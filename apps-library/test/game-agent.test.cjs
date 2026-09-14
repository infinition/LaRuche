'use strict';
const assert=require('node:assert/strict');
const {create}=require('../shared/game-agent.js');
const flush=async()=>{for(let i=0;i<12;i++)await Promise.resolve();};
(async()=>{
  let queue=[],id=0,calls=0,state={revision:0,over:false,turn:'human'},allowed=true,next=null;
  const controller=create({state:()=>state,canPlay:s=>s.turn==='agent',list:async()=>allowed?[{id:'test'}]:[],
    act:async()=>{calls++;if(next)return next();state.revision++;state.turn='human';return {model:'test',text:'legal'};},
    invalidate:()=>state.revision++,setTimeout:(f,ms)=>{const t={id:++id,f,ms};queue.push(t);return t.id;},clearTimeout:id=>queue=queue.filter(t=>t.id!==id)});
  async function tick(){const t=queue.shift();if(t)t.f();await flush();}
  controller.start('test',true);await tick();assert.equal(calls,0);assert.equal(controller.view().status,'waiting');assert.equal(queue.length,0);
  state.turn='agent';state.revision++;controller.wake();controller.wake();await tick();assert.equal(calls,1);assert(controller.view().active);assert.equal(controller.view().status,'waiting');
  for(let i=0;i<20;i++){state.turn='agent';state.revision++;controller.wake();await tick();assert.equal(state.turn,'human');}
  assert.equal(calls,21);assert(controller.view().active);
  state.turn='agent';next=async()=>{throw Error('Provider unavailable');};controller.wake();await tick();assert.equal(controller.view().status,'retry');assert.equal(queue[0].ms,1000);assert(controller.view().active);
  next=null;await tick();assert.equal(controller.view().status,'waiting');assert.equal(controller.view().failures,0);
  state.turn='agent';allowed=false;controller.wake();const before=calls;await tick();assert.equal(calls,before);assert.equal(controller.view().status,'permission');assert.equal(queue[0].ms,15000);
  allowed=true;await tick();assert.equal(calls,before+1);
  // A lost reply after an applied action must not duplicate that action.
  state.turn='agent';next=async()=>{state.turn='human';state.revision++;throw Error('Lost reply');};controller.wake();await tick();assert.equal(queue.length,0);assert.equal(controller.view().status,'waiting');
  let finish;state.turn='agent';next=()=>new Promise(r=>finish=r);controller.wake();await tick();assert(controller.view().busy);
  const old=state.revision;controller.pause();assert(state.revision>old);finish({text:'late'});await flush();assert(!controller.view().active);assert.equal(queue.length,0);
  next=null;controller.start('test',true);await tick();state.over=true;controller.wake();await tick();assert.equal(controller.view().status,'finished');assert(!controller.view().active);
  console.log('Game agent: 20 human/agent alternations, retry, permission recovery, lost reply reconciliation, pause cancellation and game over passed.');
})().catch(e=>{console.error(e);process.exitCode=1;});
