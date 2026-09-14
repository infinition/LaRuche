/* Real Chrome layout + scroll events; no account/server or model required.
 * Runs the production scroll controller with chat-sized content in an iframe.
 * CHROME_PATH overrides the browser; CHAT_JS can point to a baseline version.
 * node laruche/laruche-dashboard/test/chat-scroll.test.cjs */
'use strict';
const fs=require('node:fs'), path=require('node:path'), os=require('node:os');
const http=require('node:http'), {spawn}=require('node:child_process');
const source=fs.readFileSync(process.env.CHAT_JS || path.join(__dirname,'../src/templates/js/chat.js'),'utf8');
const start=source.indexOf('  var _chatStick =');
const end=source.indexOf('  // \\u2500\\u2500 Agentic feed:',start);
if(start<0 || end<0) throw Error('Scroll controller markers missing');
const searchStart=source.indexOf('  function scrollToMessageTerm(');
const searchEnd=source.indexOf('  function searchHistory(',searchStart);
const controller=source.slice(start,end)+source.slice(searchStart,searchEnd);
async function scenario(){
  const results=[];
  const sleep=ms=>new Promise(r=>setTimeout(r,ms));
  const c=document.getElementById('chatContainer'), btn=document.getElementById('chatJumpBtn');
  const check=(name,ok)=>results.push({name,ok:!!ok,top:c.scrollTop,height:c.scrollHeight,viewport:c.clientHeight});
  const atBottom=()=>c.scrollHeight-c.scrollTop-c.clientHeight<=6;
  const following=()=>!btn.classList.contains('visible');
  function add(height){ const el=document.createElement('div'); el.className='message-row'; el.style.height=height+'px'; c.appendChild(el); return el; }
  function wheel(d){c.dispatchEvent(new WheelEvent('wheel',{deltaY:d,bubbles:true}));c.scrollTop+=d;c.dispatchEvent(new Event('scroll'));}
  try{
    scrollToBottom(); add(100); await sleep(80);
    check('starts following without a scrollbar',following()&&atBottom());
    // A pending scroll notification runs after content grows, before the
    // MutationObserver's animation frame. Its position alone is ambiguous.
    add(1000);c.dispatchEvent(new Event('scroll'));await sleep(100);
    check('first overflow cannot detach follow',following()&&atBottom());
    for(let i=0;i<25;i++){add(50);await sleep(15);}
    await sleep(80);check('streamed messages stay at bottom',following()&&atBottom());
    c.style.height='220px';c.dispatchEvent(new Event('scroll'));await sleep(100);
    check('shrinking viewport retains follow',following()&&atBottom());
    const last=c.lastElementChild;last.style.height='400px';await sleep(150);
    check('late content resize retains follow',following()&&atBottom());
    wheel(-300);await sleep(50);check('wheel pauses following',!following());
    const top=c.scrollTop;add(200);await sleep(100);
    check('new messages respect reader position',!following()&&Math.abs(c.scrollTop-top)<2);
    btn.click();await sleep(400);check('arrow resumes and reaches bottom',following()&&atBottom());
    // Pause and click the arrow, then interrupt before its return completes.
    wheel(-800);btn.click();await sleep(40);wheel(-100);
    const interrupted=c.scrollTop;await sleep(300);
    check('wheel interrupts return animation immediately',!following()&&Math.abs(c.scrollTop-interrupted)<2);
    scrollToBottom(true);await sleep(100);
    check('sending a message forces follow again',following()&&atBottom());
    c.dispatchEvent(new KeyboardEvent('keydown',{key:'PageUp',bubbles:true}));
    c.scrollTop-=200;c.dispatchEvent(new Event('scroll'));await sleep(30);
    check('keyboard pauses following',!following());
    c.scrollTop=c.scrollHeight;c.dispatchEvent(new Event('scroll'));await sleep(30);
    check('reader returning to bottom reattaches',following());
    const nested=document.createElement('pre');nested.style.cssText='height:60px;overflow:auto';
    nested.textContent='Nested output\n'.repeat(80);last.appendChild(nested);await sleep(80);
    nested.scrollTop=100;nested.dispatchEvent(new WheelEvent('wheel',{deltaY:-20,bubbles:true}));
    await sleep(30);check('scrolling a nested output does not detach chat',following());
    // Un clic dans le vide du fil n'est pas une navigation. Il etait lu comme
    // tel, et coupait le suivi du flux en pleine reponse sans rien dire.
    c.dispatchEvent(new PointerEvent('pointerdown',{button:0,bubbles:true}));
    await sleep(30);
    check('a click in the empty thread does not detach',following());
    add(120);await sleep(120);
    check('streaming continues after a click',following()&&atBottom());
    // Tirer la barre n'emet ni molette, ni tactile, ni touche. Son seul signe
    // est qu'un bouton est ENFONCE pendant que la position bouge.
    c.dispatchEvent(new PointerEvent('pointerdown',{button:0,bubbles:true}));
    c.scrollTop=100;c.dispatchEvent(new Event('scroll'));await sleep(30);
    check('scrollbar navigation pauses following',!following());
    window.dispatchEvent(new PointerEvent('pointerup',{bubbles:true}));
    btn.click();await sleep(400);
    /* Un agent qui utilise un outil, ecrit, puis en utilise un autre fait
       VARIER la hauteur du fil: une carte d'outil en cours est remplacee par
       son resultat, souvent plus court. Le navigateur recadre alors scrollTop
       tout seul, et l'evenement qui suit rapporte une position que le fil n'a
       pas ecrite. La prendre pour un geste du lecteur decrochait le suivi en
       plein travail de l'agent, sans que rien ne l'explique. */
    const grosse=add(900);await sleep(150);
    check('following after a tall tool card',following()&&atBottom());
    grosse.style.height='80px';c.dispatchEvent(new Event('scroll'));await sleep(120);
    check('a tool card shrinking does not detach',following());
    add(60);await sleep(150);
    check('following resumes after the shrink',following()&&atBottom());
    // Un recadrage du navigateur, sans aucun bouton enfonce, n'est pas un geste.
    c.scrollTop-=150;c.dispatchEvent(new Event('scroll'));await sleep(60);
    check('an unasked scroll does not detach',following());
    add(80);await sleep(150);
    check('following catches up after an unasked scroll',following()&&atBottom());
    /* Et la meme chose en pire: plusieurs remplacements d'affilee. */
    for(let i=0;i<4;i++){
      const carte=add(500);await sleep(60);
      carte.style.height='40px';c.dispatchEvent(new Event('scroll'));await sleep(60);
      add(120);await sleep(60);
    }
    check('following survives repeated tool cycles',following()&&atBottom());
    c.dispatchEvent(new PointerEvent('pointerdown',{button:0,bubbles:true}));
    window.dispatchEvent(new PointerEvent('pointerup',{bubbles:true}));
    c.scrollTop-=200;c.dispatchEvent(new Event('scroll'));await sleep(60);
    check('a released pointer does not keep detaching',following());
    add(80);await sleep(150);
    c.dispatchEvent(new Event('touchmove',{bubbles:true}));
    c.scrollTop-=200;c.dispatchEvent(new Event('scroll'));await sleep(30);
    check('touch navigation pauses following',!following());
    // Emulate a background webview with suspended animation callbacks.
    const raf=window.requestAnimationFrame;
    Object.defineProperty(document,'visibilityState',{configurable:true,get:()=> 'hidden'});
    window.requestAnimationFrame=()=>0;
    scrollToBottom(true);add(200);await sleep(50);
    check('hidden page follows without animation frames',following()&&atBottom());
    window.requestAnimationFrame=raf;delete document.visibilityState;
    document.dispatchEvent(new Event('visibilitychange'));await sleep(80);
    check('showing the page retains follow',following()&&atBottom());
    c.firstElementChild.classList.add('message');c.firstElementChild.textContent='Message à retrouver';
    scrollToMessageTerm('retrouver');await sleep(800);add(100);await sleep(100);
    check('history search pauses follow at the selected message',!following()&&c.scrollTop<200);
  }catch(e){results.push({name:e.stack,ok:false});}
  await fetch('/report',{method:'POST',body:JSON.stringify(results)});
}
(async()=>{
  const browser=[process.env.CHROME_PATH,'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome','/usr/bin/google-chrome','/usr/bin/chromium'].find(p=>p&&fs.existsSync(p));
  if(!browser)throw Error('Chrome required; set CHROME_PATH');
  let resolve;
  const report=new Promise(r=>resolve=r);
  const server=http.createServer((req,res)=>{
    if(req.url==='/report'){let body='';req.on('data',d=>body+=d);req.on('end',()=>{res.end('ok');resolve(JSON.parse(body));});return;}
    res.setHeader('Content-Type','text/html; charset=utf-8');
    if(req.url==='/frame')return res.end('<iframe src="/" style="width:100%;height:100%;border:0"></iframe>');
    res.end('<style>body{margin:0}#chatContainer{height:360px;overflow:auto;overflow-anchor:none;display:flex;flex-direction:column;gap:6px}.message-row{flex-shrink:0}#chatJumpBtn{opacity:0}#chatJumpBtn.visible{opacity:1}</style><div id="chatContainer" tabindex="0"></div><button id="chatJumpBtn">Bas</button><script>'+controller+'\n('+scenario.toString()+')()</script>');
  });
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  const profile=fs.mkdtempSync(path.join(os.tmpdir(),'laruche-chat-scroll-'));
  const child=spawn(browser,['--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check','--user-data-dir='+profile,'--window-size=600,800','http://127.0.0.1:'+server.address().port+'/frame'],{stdio:'ignore'});
  const timeout=setTimeout(()=>resolve([{name:'browser timed out',ok:false}]),30000);
  const results=await report;clearTimeout(timeout);child.kill();server.close();
  await new Promise(r=>child.on('exit',r));fs.rmSync(profile,{recursive:true,force:true});
  results.filter(r=>!r.ok).forEach(r=>console.error('FAIL',r));
  console.log('Chat scroll:',results.filter(r=>r.ok).length+'/'+results.length,'passed');
  process.exitCode=results.some(r=>!r.ok)?1:0;
})();
