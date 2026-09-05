// Test a DS Studio package against the real sandbox and disk-backed App SDK.
// Uses a private temporary hive, no LLM, no existing account and no production data.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
const net=require('node:net');
const {spawn}=require('node:child_process');
const {randomUUID}=require('node:crypto');
const root=path.resolve(__dirname,'../../..');
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function until(fn,label){const end=Date.now()+45000;while(Date.now()<end){if(await fn())return;await sleep(250);}throw new Error('Timed out: '+label);}

(async()=>{
  const archive=process.argv[2];
  if(!archive)throw new Error('Usage: node ds-persistence.test.cjs <DS Studio .laruche-app archive>');
  const dataHome=fs.mkdtempSync(path.join(os.tmpdir(),'laruche-ds-persistence-'));
  const probe=net.createServer();await new Promise(r=>probe.listen(0,'127.0.0.1',r));
  const port=probe.address().port;await new Promise(r=>probe.close(r));
  const base='http://127.0.0.1:'+port;
  const binary=process.env.LARUCHE_TEST_BINARY||path.join(root,'laruche/target/debug/laruche-node'+(process.platform==='win32'?'.exe':''));
  let child,browser,context,api,page,instance;
  const errors=[];
  const appId='dev.laruche.ds-studio';
  async function start(){
    child=spawn(binary,['--no-tui'],{cwd:root,windowsHide:true,env:{...process.env,LARUCHE_DATA_DIR:dataHome,LARUCHE_PORT:String(port),LARUCHE_BIND_LAN:'0',LARUCHE_NO_BROWSER:'1'},stdio:'ignore'});
    let bootError;child.on('error',e=>{bootError=e;});
    await until(async()=>{if(bootError)throw bootError;try{return (await fetch(base+'/health')).ok;}catch{return false;}},'node boot');
  }
  async function stop(){if(child&&child.exitCode===null){const exited=new Promise(r=>child.once('exit',r));child.kill();await exited;}child=null;}
  async function post(endpoint,data){const r=await api.post(base+endpoint,{data});const value=await r.json();assert(r.ok(),endpoint+' '+JSON.stringify(value));return value;}
  async function view(cookies){
    context=await browser.newContext({viewport:{width:1400,height:1000}});api=context.request;
    await context.addInitScript(()=>{if(window===window.top)localStorage.setItem('laruche_accueil_vu','1');});
    if(cookies)await context.addCookies(cookies);
    else await post('/api/auth/enroll',{display_name:'Persistence test',password:randomUUID()});
    page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
    await page.goto(base+'/#apps/overview');
  }
  async function open(){
    // Registration is asynchronous after the native page loads.
    await sleep(1500);
    const opened=await post('/api/apps/command',{kind:'app_open',appId});instance=opened.instanceId;
    const ready=await post('/api/apps/command',{kind:'app_wait',appId,instanceId:instance});assert(ready.ready,'runtime must become Ready');
  }
  const call=(action,args={})=>post('/api/apps/command',{appId,action,arguments:args,instanceId:instance});
  try{
    await start();
    browser=await chromium.launch({headless:true,...(process.env.CHROME_PATH?{executablePath:process.env.CHROME_PATH}:{})});
    await view();
    const installed=await api.post(base+'/api/apps/install',{data:fs.readFileSync(path.resolve(archive)),headers:{'Content-Type':'application/zip'}});
    assert(installed.ok(),'package accepted by real manifest/schema validator');
    const app=await(await api.get(base+'/api/apps/'+appId)).json();assert.equal(app.enabled,false);
    await post('/api/apps/'+appId+'/enable',{grantedPermissions:app.manifest.permissions.required});
    const actions=Object.fromEntries(app.manifest.actions.map(a=>[a.name,true]));
    await post('/api/apps/access',{kind:'policy',appId,policy:{principals:{laruche:{discover:true,open:true,actions}},invokeAgents:[]}});
    await open();
    const frame=page.frames().find(f=>f.url().includes('/apps-assets/'+appId+'/'));
    assert(frame,'real sandbox iframe exists');
    assert(await frame.evaluate(()=>{try{void localStorage;return false;}catch{return true;}}),'opaque origin forbids localStorage');
    const kernel=await call('kernel.status');assert(kernel.ready);
    let state=await call('notebook.state');
    const data=await call('data.add',{name:'sales_test',text:'year,sales\n2023,10\n2023,15\n2024,30',revision:state.revision});
    state=await call('notebook.state');
    const source='v = load("'+data.name+'")\nt = v.groupby("year").agg(total = sum(sales))\nshow(t)\nbar(t, x = "year", y = "total")';
    const cell=await call('cell.add',{source,type:'code',revision:state.revision});
    const job=await call('cell.run',{cellId:cell.cellId,revision:cell.revision});
    await until(async()=>{const s=await call('job.status',{jobId:job.jobId});if(s.status==='error')throw new Error(JSON.stringify(s));return s.status==='ok';},'cell computation');
    state=await call('notebook.state',{includeOutputs:true});
    const result=state.cells.find(c=>c.id===cell.cellId);assert(result.outputs.some(o=>o.kind==='table'));assert(result.outputs.some(o=>o.kind==='chart'));
    assert.deepEqual(result.outputs.find(o=>o.kind==='table').rows,[[2023,25],[2024,30]],'yearly sales aggregation');
    // Confirm the write reached the real storage API before closing the browser.
    await until(async()=>{
      const r=await post('/api/apps/'+appId+'/storage',{op:'list',prefix:''});
      if(!r.keys.includes('studio.lib.v1'))return false;
      for(const key of r.keys.filter(k=>k.startsWith('studio.nb.'))){
        const stored=await post('/api/apps/'+appId+'/storage',{op:'get',key});
        if(typeof stored.value==='string'&&stored.value.includes(cell.cellId)&&stored.value.includes('"kind":"chart"'))return true;
      }
      return false;
    },'durable cell and chart save');
    await page.screenshot({path:path.join(dataHome,'notebook.png')});
    const cookies=await context.cookies();await context.close();await stop();
    await start();await view(cookies);await open();
    const restored=await call('notebook.state',{includeOutputs:true});
    assert.equal(restored.notebookId,state.notebookId,'same saved notebook');
    const saved=restored.cells.find(c=>c.id===cell.cellId);assert(saved,'cell restored');assert.equal(saved.source,source);
    assert(saved.outputs.some(o=>o.kind==='table'),'table restored');assert(saved.outputs.some(o=>o.kind==='chart'),'chart restored');
    assert.deepEqual(saved.outputs.find(o=>o.kind==='table').rows,[[2023,25],[2024,30]],'saved table retains calculated values');
    const datasets=await call('data.list');assert(datasets.datasets.some(d=>d.name===data.name),'small dataset restored');
    assert.deepEqual(errors,[]);
    console.log('PASS: real sandbox, Ready, dataset, cell/job, table/chart, disk persistence after node restart and a fresh browser context.');
    console.log('Test artifacts: '+dataHome);
  }finally{if(browser)await browser.close();await stop();}
})().catch(e=>{console.error(e);process.exitCode=1;});
