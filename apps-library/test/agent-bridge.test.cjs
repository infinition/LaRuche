// Integration test: isolated hive, controlled local provider, real browser/SDK.
// Requires Playwright and a built laruche-node. No real model or account is used.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');const path=require('node:path');const os=require('node:os');
const http=require('node:http');const net=require('node:net');const {spawn}=require('node:child_process');const {randomUUID}=require('node:crypto');
const root=path.resolve(__dirname,'../..');
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function until(fn,label){for(let i=0;i<160;i++){if(await fn())return;await sleep(250);}throw new Error('Timed out: '+label);}
(async()=>{
  const home=fs.mkdtempSync(path.join(os.tmpdir(),'laruche-app-bridge-'));
  let captures=[];
  const provider=http.createServer(async(req,res)=>{
    let raw='';for await(const b of req)raw+=b;
    if(!req.url.endsWith('/chat/completions')){res.writeHead(200,{'Content-Type':'application/json'});res.end('{"data":[{"id":"test-model"}]}');return;}
    const body=JSON.parse(raw);captures.push(body);
    const last=body.messages.at(-1).content;
    let answer='seen='+body.messages.length;
    if(last.includes('Choose ONE legal action')){
      const state=JSON.parse(last.match(/\nState: (.+)\nAllowed actions:/)[1]);
      const move=state.legalMoves[0];
      answer=JSON.stringify({action:'game.move',arguments:typeof move==='string'?{direction:move,revision:state.revision}:{from:move.from,to:move.to,path:move.path,revision:state.revision}});
    }
    if(last.includes('SLOW'))await sleep(5000);
    res.writeHead(200,{'Content-Type':'text/event-stream'});
    res.end('data: '+JSON.stringify({choices:[{delta:{content:answer}}]})+'\n\ndata: [DONE]\n\n');
  });
  await new Promise(r=>provider.listen(0,'127.0.0.1',r));
  const portProbe=net.createServer();await new Promise(r=>portProbe.listen(0,'127.0.0.1',r));const port=portProbe.address().port;await new Promise(r=>portProbe.close(r));
  const base='http://127.0.0.1:'+port;
  fs.writeFileSync(path.join(home,'provider-profiles.json'),JSON.stringify({version:1,profiles:{test:{provider:'openai',name:'Local test provider',base_url:'http://127.0.0.1:'+provider.address().port,api_key:'',models:['test-model']}},active_model:{profile_id:'test',model:'test-model'},channel_overrides:{}}));
  const binary=process.env.LARUCHE_TEST_BINARY||path.join(root,'laruche/target/debug/laruche-node'+(process.platform==='win32'?'.exe':''));
  const child=spawn(binary,['--no-tui'],{cwd:root,windowsHide:true,env:{...process.env,LARUCHE_DATA_DIR:home,LARUCHE_PORT:String(port),LARUCHE_NO_BROWSER:'1',LARUCHE_MEMOIRE_ARBITRE:'0'},stdio:['ignore','pipe','pipe']});
  const log=fs.createWriteStream(path.join(home,'test.log'));child.stdout.pipe(log);child.stderr.pipe(log);
  let browser;
  try{
    await until(async()=>{try{return (await fetch(base+'/health')).ok;}catch{return false;}},'server boot');
    browser=await chromium.launch({headless:true,...(process.env.CHROME_PATH?{executablePath:process.env.CHROME_PATH}:{})});
    const context=await browser.newContext({viewport:{width:1400,height:1000},reducedMotion:'reduce'});
    await context.addInitScript(()=>{if(window===window.top)localStorage.setItem('laruche_accueil_vu','1');});
    const api=context.request;
    async function post(endpoint,data){const r=await api.post(base+endpoint,{data});const value=await r.json();assert(r.ok(),endpoint+' '+JSON.stringify(value));return value;}
    await post('/api/auth/enroll',{display_name:'App owner',password:randomUUID()});
    const gameVersion=require('../2048/package/app.json').version;
    const archive=path.join(root,'apps-library/2048/dist/laruche-2048-'+gameVersion+'.laruche-app');
    const install=await api.post(base+'/api/apps/install',{data:fs.readFileSync(archive),headers:{'Content-Type':'application/zip'}});assert(install.ok());
    const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.goto(base+'/#apps/overview');
    await page.locator('[data-app-rights="dev.laruche.2048"]').click();
    await page.locator('#packageGrants input[value="agents.invoke"]').check();
    await page.locator('#saveAppGrants').click();
    await until(async()=>((await (await api.get(base+'/api/apps/dev.laruche.2048')).json()).enabled),'app enable');
    await page.locator('[data-rule="open"]').selectOption('true');
    await page.locator('[data-rule="game.state"]').selectOption('true');
    await page.locator('[data-rule="game.move"]').selectOption('true');
    await until(async()=>{const d=await(await api.get(base+'/api/apps/access')).json();return d.config.policies['dev.laruche.2048']?.principals.laruche?.actions['game.move'];},'live permission save');
    await page.locator('[data-close]').click();
    await page.locator('#appsAgents').click();
    await page.locator('#appAgentForm [name=name]').fill('Player A');
    await page.locator('#appAgentForm [name=model]').selectOption({label:'Local test provider / test-model'});
    await page.locator('#appAgentForm [name=instructions]').fill('Choose a legal move. Return the required JSON only.');
    await page.locator('#appAgentForm button[type=submit], #appAgentForm button:not([type])').first().click();
    await until(async()=>((await(await api.get(base+'/api/apps/access')).json()).config.agents.length===1),'agent creation');
    let access=(await(await api.get(base+'/api/apps/access')).json()).config;
    const agent=access.agents[0];
    await post('/api/apps/access',{kind:'agent',agent:{...agent,id:randomUUID(),name:'Player B'}});
    const policy=access.policies['dev.laruche.2048'];policy.invokeAgents=[agent.id];
    await post('/api/apps/access',{kind:'policy',appId:'dev.laruche.2048',policy});
    await page.locator('[data-close]').click();
    const opened=await post('/api/apps/command',{kind:'app_open',appId:'dev.laruche.2048'});
    const ready=await post('/api/apps/command',{kind:'app_wait',appId:'dev.laruche.2048',instanceId:opened.instanceId});assert(ready.ready);
    const call=(action,args={})=>post('/api/apps/command',{appId:'dev.laruche.2048',action,arguments:args,instanceId:opened.instanceId});
    let state=await call('game.state');
    await call('game.move',{direction:state.legalMoves[0],revision:state.revision});
    const stale=await api.post(base+'/api/apps/command',{data:{appId:'dev.laruche.2048',action:'game.move',arguments:{direction:'left',revision:state.revision},instanceId:opened.instanceId}});assert(!stale.ok(),'stale move must fail');
    const game=page.frameLocator('#lrDock iframe');
    await game.locator('#refreshAgents').click();
    await until(async()=>game.locator('#agentPlayer option').count(),'agent list');
    state=await call('game.state');
    await game.locator('#agentTurn').click();
    await until(async()=>{const s=await call('game.state');return s.revision>state.revision;},'model plays through SDK and action broker');
    const frame=page.frames().find(f=>f.parentFrame()&&f.url().includes('/apps-assets/dev.laruche.2048/'));
    await until(async()=>!(await game.locator('#agentTurn').isDisabled()),'turn finished');
    const first=await frame.evaluate(async id=>(await LaRucheApp.agents.run(id,'white','WHITE')).text,agent.id);
    const second=await frame.evaluate(async id=>(await LaRucheApp.agents.run(id,'black','BLACK')).text,agent.id);
    const again=await frame.evaluate(async id=>(await LaRucheApp.agents.run(id,'white','WHITE AGAIN')).text,agent.id);
    assert.equal(first,'seen=2');assert.equal(second,'seen=2');assert.equal(again,'seen=4');
    const other=await browser.newContext();await other.request.post(base+'/api/auth/enroll',{data:{display_name:'Other user',password:randomUUID()}});
    const denied=await other.request.post(base+'/api/apps/command',{data:{appId:'dev.laruche.2048',action:'game.state',instanceId:opened.instanceId}});assert(!denied.ok());
    const otherConfig=await(await other.request.get(base+'/api/apps/access')).json();assert.equal(otherConfig.config.agents.length,0);
    await frame.evaluate(id=>{window.cancelResult=null;LaRucheApp.agents.run(id,'cancel','SLOW').then(()=>window.cancelResult='unexpected success').catch(e=>window.cancelResult=e.message);},agent.id);
    await until(async()=>captures.some(c=>c.messages.at(-1).content==='SLOW'),'pending generation');
    policy.invokeAgents=[];await post('/api/apps/access',{kind:'policy',appId:'dev.laruche.2048',policy});
    await until(async()=>frame.evaluate(()=>window.cancelResult!==null),'generation cancellation');
    assert.match(await frame.evaluate(()=>window.cancelResult),/revoked/i);
    policy.principals.laruche.actions['game.move']=false;await post('/api/apps/access',{kind:'policy',appId:'dev.laruche.2048',policy});
    state=await call('game.state');const revoked=await api.post(base+'/api/apps/command',{data:{appId:'dev.laruche.2048',action:'game.move',arguments:{direction:state.legalMoves[0],revision:state.revision},instanceId:opened.instanceId}});assert(!revoked.ok());
    await page.evaluate(()=>LaRuche.AppAccess.open('dev.laruche.2048'));
    await page.locator('#agentRights').waitFor();
    await page.screenshot({path:path.join(home,'permissions.png')});
    await page.locator('[data-close]').click();

    // The human selected black, so the model must be white and may play first.
    const checkers=require('../checkers/package/app.json');
    const checkersArchive=path.join(root,'apps-library/checkers/dist/'+checkers.id+'-'+checkers.version+'.laruche-app');
    const ci=await api.post(base+'/api/apps/install',{data:fs.readFileSync(checkersArchive),headers:{'Content-Type':'application/zip'}});assert(ci.ok());
    await post('/api/apps/'+checkers.id+'/enable',{grantedPermissions:checkers.permissions.required.concat(['agents.invoke'])});
    await post('/api/apps/'+checkers.id+'/storage',{op:'set',key:'checkers.state.v1',value:{board:require('../checkers/package/ui/game.js').createBoard(),turn:'white',humanSide:'black',opponentMode:'agent',moves:0,over:false,winner:null}});
    await post('/api/apps/access',{kind:'policy',appId:checkers.id,policy:{principals:{laruche:{discover:true,open:true,actions:{'game.state':true,'game.move':true}}},invokeAgents:[agent.id]}});
    const co=await post('/api/apps/command',{kind:'app_open',appId:checkers.id});
    assert((await post('/api/apps/command',{kind:'app_wait',appId:checkers.id,instanceId:co.instanceId})).ready);
    const cc=(action,args={})=>post('/api/apps/command',{appId:checkers.id,instanceId:co.instanceId,action,arguments:args});
    let cs=await cc('game.state');assert.equal(cs.agentSide,'white');assert.equal(cs.waitingFor,'agent');assert(cs.rules.captures.includes('compulsory'));
    const cf=page.frames().find(f=>f.url().includes('/apps-assets/'+checkers.id+'/'));
    await cf.evaluate(async id=>LaRucheApp.agents.act(id,'white-test','game.state','Follow the complete rules and select one legal move.'),agent.id);
    cs=await cc('game.state');assert.equal(cs.turn,'black');assert.equal(cs.waitingFor,'human');assert.deepEqual(cs.legalMoves,[]);
    const forbidden=await api.post(base+'/api/apps/command',{data:{appId:checkers.id,instanceId:co.instanceId,action:'game.move',arguments:{from:1,to:10,revision:cs.revision}}});assert(!forbidden.ok(),'model cannot play for human');
    await page.screenshot({path:path.join(home,'checkers.png')});
    assert.deepEqual(errors,[]);
    console.log('PASS: install consent UI, live grants, agent library UI, discovery/open/Ready/actions, valid/stale 2048 moves, isolated contexts/users, live revocation, checkers agent-as-white and enforced human turns.');
    console.log('Test artifacts: '+home);
  }finally{
    if(browser)await browser.close();child.stdout.unpipe(log);child.stderr.unpipe(log);child.kill();provider.closeAllConnections();provider.close();log.end();
  }
})().catch(e=>{console.error(e);process.exitCode=1;});
