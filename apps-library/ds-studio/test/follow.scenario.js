/* Browser regression scenario, run through DS_FOLLOW_ONLY=1 browser.test.cjs.
 * Output events use the real notebook model with fixture outputs, so these
 * navigation checks do not depend on a Python download or kernel execution. */
(async function(){
  var results = [];
  function check(name, ok, detail){ results.push({name: name, ok: !!ok, detail: String(detail || '')}); }
  function sleep(ms){ return new Promise(function(resolve){ setTimeout(resolve, ms); }); }
  async function call(name, args){ return (await window.__harness.call(name, args)).result; }
  async function mutate(name, args){
    var state = await call('notebook.state');
    return call(name, Object.assign({revision: state.revision}, args));
  }
  window.addEventListener('error', function(e){ check('no browser error', false, e.message); });
  try {
    for (var tries = 0; tries < 200 && window.__harness.status().state !== 'ready'; tries++) await sleep(50);
    check('browser is visible and ready', document.visibilityState === 'visible' && window.__harness.status().state === 'ready');
    // Capture the real model when the next action adds a cell.
    var model;
    var original = StudioNotebook.Notebook.prototype.addCell;
    StudioNotebook.Notebook.prototype.addCell = function(){ model = this; return original.apply(this, arguments); };
    var ids = [];
    for (var i = 0; i < 16; i++) {
      ids.push((await mutate('cell.add', {type:'markdown', source: 'Bloc ' + i + '\n\n' + 'Contenu de la cellule. '.repeat(50)})).cellId);
      await sleep(35);
    }
    StudioNotebook.Notebook.prototype.addCell = original;
    await sleep(650);
    var pane = document.querySelector('.notebook-pane');
    var button = document.getElementById('followBtn');
    check('overflow stays in follow mode', pane.scrollHeight > pane.clientHeight * 2 && button.hidden);
    async function focus(id){ await mutate('cell.update', {cellId:id, source:'Texte modifié '.repeat(60)}); await sleep(650); }
    await focus(ids[7]);
    // Content growing ABOVE the viewport is the real non-user scroll. It must
    // never detach, and it is tested by growing a cell rather than by poking
    // scrollTop: a bare position change with no navigation event is what the
    // reader produces by dragging the scrollbar, and that one must detach.
    document.querySelector('[data-cell-id="' + ids[0] + '"]').style.minHeight = '900px';
    await sleep(400);
    check('a non-user scroll cannot detach follow', button.hidden);
    document.querySelector('[data-cell-id="' + ids[0] + '"]').style.minHeight = '';
    await sleep(400);
    await focus(ids[8]);
    function aligned(target){
      var scale = pane.getBoundingClientRect().width / pane.offsetWidth;
      return Math.abs((target.getBoundingClientRect().top - pane.getBoundingClientRect().top) / scale - 12) < 4;
    }
    check('next agent update is aligned at the top', aligned(document.querySelector('[data-cell-id="' + ids[8] + '"]')));
    // Emit multiple real output events, then redraw the complete list.
    var outId = (await mutate('cell.add', {type:'code', index:5, source:'print(1)'})).cellId;
    var outputs = [{kind:'stream', text:'Resultat\n'.repeat(90)}, {kind:'chart', chart:'bar', title:'Graphique de test', labels:['A','B'], series:[{name:'Total', values:[20,13]}]}];
    model.setOutputs(outId, outputs);
    await sleep(650);
    var outputSelector = '[data-cell-id="' + outId + '"] [data-role="outputs"] > :last-child';
    check('chart output renders as SVG', document.querySelector(outputSelector + ' svg'));
    check('follows the last output sub-block', aligned(document.querySelector(outputSelector)));
    model.moveCell(ids[15], 14);
    await sleep(650);
    check('full redraw retains the output and halo', aligned(document.querySelector(outputSelector)) && document.querySelector(outputSelector).classList.contains('is-focus-bloc'));
    // Delayed layout growth before the target, like an image loading.
    document.querySelector('[data-cell-id="' + ids[1] + '"]').style.minHeight = '1500px';
    await sleep(750);
    check('layout growth retains following and alignment', button.hidden && aligned(document.querySelector(outputSelector)));
    document.getElementById('zoomOutBtn').click();
    document.getElementById('zoomOutBtn').click();
    await sleep(750);
    check('zoom keeps the output aligned', button.hidden && aligned(document.querySelector(outputSelector)));
    pane.dispatchEvent(new WheelEvent('wheel', {deltaY:-100, bubbles:true}));
    pane.scrollTop -= 100;
    pane.dispatchEvent(new Event('scroll'));
    check('wheel navigation pauses following', !button.hidden);
    var top = pane.scrollTop;
    model.setOutputs(outId, outputs.concat({kind:'stream',text:'Nouveau resultat'}));
    await sleep(650);
    check('agent output respects reader position', !button.hidden && Math.abs(pane.scrollTop - top) < 4);
    button.click();
    await sleep(650);
    check('resume returns to the latest output', button.hidden && aligned(document.querySelector(outputSelector)));
    await mutate('cell.update', {cellId: ids[2], source:'Ancienne cible'});
    await mutate('cell.update', {cellId: ids[10], source:'Cible finale'});
    await sleep(650);
    check('latest target wins over pending frames', document.querySelector('.cell.is-active').dataset.cellId === ids[10] && aligned(document.querySelector('[data-cell-id="' + ids[10] + '"]')));
    pane.dispatchEvent(new KeyboardEvent('keydown', {key:'PageUp', bubbles:true}));
    pane.scrollTop -= 100;
    pane.dispatchEvent(new Event('scroll'));
    check('keyboard navigation pauses following', !button.hidden);
    pane.scrollTop = pane.scrollHeight;
    pane.dispatchEvent(new Event('scroll'));
    check('returning to bottom resumes following', button.hidden);
    // Dragging the scrollbar emits no wheel, no touch and no key: the position
    // arriving is all there is, and it is not one the notebook wrote.
    pane.scrollTop = 0;
    pane.dispatchEvent(new Event('scroll'));
    check('scrollbar navigation pauses following', !button.hidden);
    await mutate('notebook.new', {title:'Nouveau carnet'});
    await sleep(200);
    check('new notebook resets following', button.hidden && !document.querySelector('.is-focus-bloc'));
  } catch(e){ check('scenario completed', false, e.stack || e); }
  await fetch('/report', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({results:results})});
})();
