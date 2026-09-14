/* UI + real game engine + turn scheduler, controlled model decisions.
 * No provider credentials or live user data. NODE_PATH must include Playwright. */
'use strict';
const {chromium}=require('playwright');
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const root=path.resolve(__dirname,'..');
const stub=`
window.test={actions:{},storage:{},calls:0,failures:0,hold:false,release:null};
window.LaRucheApp={
 ready:async()=>({locale:'fr',theme:'default'}),
 storage:{get:async k=>test.storage[k],set:async(k,v)=>{test.storage[k]=JSON.parse(JSON.stringify(v));}},
 ui:{setTitle:async()=>{},setDirty:async()=>{},setStatus:async s=>{test.ready=s;}},
 actions:{register:(n,f)=>{test.actions[n]=f;}},
 agents:{list:async()=>[{id:'test',name:'Test model'}],act:async(id,seat,action,prompt,options)=>{
  test.calls++;var s=test.actions[action]();
  if(options.expectedRevision!==s.revision)throw Error('Stale revision');
  if(test.hold)await new Promise(r=>{test.release=r;});
  if(test.failures){test.failures--;throw Error('Temporary provider failure');}
  var move=s.analysis.candidates[0];
  var args=move.direction?{direction:move.direction,revision:s.revision}:{from:move.from,to:move.to,path:move.path,revision:s.revision};
  return {model:'fixture',text:JSON.stringify(args),result:await test.actions['game.move'](args)};
 }}
};
`;
(async()=>{
 const server=http.createServer((req,res)=>{
  if(req.url==='/apps-runtime/v1.js'){res.setHeader('Content-Type','text/javascript');res.end(stub);return;}
  const parts=req.url.split('/').filter(Boolean),game=parts.shift();
  if(!['checkers','2048'].includes(game)){res.writeHead(404).end();return;}
  const base=path.join(root,game,'package/ui'),file=path.resolve(base,parts.join('/')||'index.html');
  if(!file.startsWith(base+path.sep)||!fs.existsSync(file)){res.writeHead(404).end();return;}
  res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.json')?'application/json':file.endsWith('.css')?'text/css':'text/html');res.end(fs.readFileSync(file));
 });
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const browser=await chromium.launch({headless:true,executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});
 const errors=[];
 async function pageFor(game,saved){
  const page=await browser.newPage({viewport:{width:700,height:1000},reducedMotion:'reduce'});page.on('pageerror',e=>errors.push(e.message));
  if(saved)await page.route('**/apps-runtime/v1.js',r=>r.fulfill({contentType:'text/javascript',body:stub+'test.storage='+JSON.stringify(saved)+';'}));
  await page.goto('http://127.0.0.1:'+server.address().port+'/'+game+'/index.html');
  await page.waitForFunction(()=>window.test&&test.ready==='ready');return page;
 }
 async function state(p){return p.evaluate(()=>test.actions['game.state']());}
 async function start(p,goal){return p.evaluate(async goal=>test.actions['game.agent']({mode:'start',agentId:'test',revision:test.actions['game.state']().revision,...(goal?{goal}: {})}),goal);}
 try{
  const e=require('../checkers/package/ui/game.js');const winning=Array(64).fill(0);winning[42]=1;winning[33]=-1;
  const p=await pageFor('checkers',{'checkers.state.v1':{board:winning,turn:'white',humanSide:'white',opponentMode:'agent',moves:0,over:false,winner:null}});
  await start(p);await p.waitForFunction(()=>test.actions['game.state']().agent.status==='waiting');
  assert.equal(await p.evaluate(()=>test.calls),0,'no LLM calls during human turn');
  await p.locator('[data-index="42"]').click();await p.locator('[data-index="24"]').click();
  await p.waitForFunction(()=>test.actions['game.state']().agent.status==='finished');
  assert.equal((await state(p)).agent.active,false,'human victory closes the waiting game');await p.close();
  const q=await pageFor('checkers');await q.evaluate(()=>{test.failures=1;});await start(q);
  for(let i=0;i<4;i++){
   const before=await state(q);assert.equal(before.waitingFor,'human');
   const move=e.legalMoves(before.board,before.humanSide)[0];
   await q.locator('[data-index="'+move.from+'"]').click();await q.locator('[data-index="'+move.to+'"]').click();
   await q.waitForFunction(n=>{const s=test.actions['game.state']();return s.moves===n+2&&s.agent.status==='waiting';},before.moves);
   assert((await state(q)).agent.active);
  }
  assert.equal(await q.evaluate(()=>test.calls),5,'one failed attempt, then exactly four agent moves');
  await q.evaluate(()=>{test.hold=true;});let before=await state(q),move=e.legalMoves(before.board,before.humanSide)[0];
  await q.locator('[data-index="'+move.from+'"]').click();await q.locator('[data-index="'+move.to+'"]').click();
  await q.waitForFunction(()=>!!test.release);await q.locator('#agentPauseBtn').click();
  before=await state(q);await q.evaluate(()=>test.release());await q.waitForFunction(()=>!test.actions['game.state']().agent.busy);
  assert.equal((await state(q)).moves,before.moves,'late move after Pause must be refused');await q.close();
  const board=Array(16).fill(0);board[0]=2048;board[1]=2;
  const r=await pageFor('2048',{'game.state.v1':{board,score:20480,best:20480,moves:100,won:true,keepPlaying:false}});
  await r.evaluate(()=>{test.hold=true;});await start(r,'max_score');assert.equal((await state(r)).keepPlaying,true);
  await r.waitForFunction(()=>!!test.release);await r.locator('#agentPause').click();await r.evaluate(()=>test.release());await r.waitForFunction(()=>!test.actions['game.state']().agent.busy);
  await start(r,'2048');await r.waitForFunction(()=>test.actions['game.state']().agent.status==='waiting');
  assert.equal((await state(r)).keepPlaying,false);assert.equal((await state(r)).needsContinue,true);assert.equal(await r.locator('#maximizeScore').isChecked(),false);
  await r.locator('#agentPause').click();await r.locator('#maximizeScore').check();await r.locator('#agentAuto').click();
  assert.equal((await state(r)).keepPlaying,true);await r.locator('#agentPause').click();
  await r.locator('#maximizeScore').uncheck();await r.locator('#agentAuto').click();
  assert.equal((await state(r)).keepPlaying,false,'unchecked UI goal must reset extended play');
  await r.close();assert.deepEqual(errors,[]);
  console.log('Game loops: human victory, four alternating turns, transient failure recovery, late-answer cancellation, 2048 goal changes via action and UI passed.');
 }finally{await browser.close();server.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
