/* End-to-end smoke test in a real browser.
 *
 * Serves package/ui over HTTP with a stand-in for /apps-runtime/v1.js that
 * implements the parts of the SDK this App uses, then drives the App the way
 * an agent would: wait for ready, read the kernel, load data, add a cell, run
 * it, poll the job, and check that a chart and a table actually rendered.
 *
 * Needs a Chrome or Edge binary. Set CHROME_PATH to point at one.
 * Run with: node apps-library/ds-studio/test/browser.test.cjs
 */

'use strict';

const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { spawn } = require('node:child_process');

const UI = path.join(__dirname, '..', 'package', 'ui');

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.md': 'text/plain; charset=utf-8'
};

function findBrowser() {
  const candidates = [
    process.env.CHROME_PATH,
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
    '/usr/bin/google-chrome',
    '/usr/bin/chromium',
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
  ];
  return candidates.find((entry) => entry && fs.existsSync(entry)) || null;
}

/* The SDK stand-in plus the scenario. Both are served from the App's own
 * origin, so the App's Content-Security-Policy accepts them unchanged. */
const RUNTIME_STUB = `
(function(global){
  'use strict';
  var actions = new Map();
  var storage = new Map();
  var status = { state: 'loading', message: '', progress: 0 };
  var log = [];

  function record(entry) { log.push(entry); }

  global.LaRucheApp = Object.freeze({
    version: 'test',
    ready: function(){
      return Promise.resolve({
        sessionId: 'test-session',
        appId: 'dev.laruche.ds-studio',
        viewId: 'notebook',
        grantedCapabilities: ['storage.private', 'ui.locale.read', 'ui.theme.read'],
        locale: 'fr',
        theme: 'default',
        limits: { messageBytes: 65536, storageBytes: 1048576 }
      });
    },
    actions: Object.freeze({
      register: function(name, handler){
        if (!/^[A-Za-z0-9._-]{1,80}$/.test(name)) throw new Error('bad action name ' + name);
        actions.set(name, handler);
        record({ kind: 'register', name: name });
        return function(){ actions.delete(name); };
      }
    }),
    agents: Object.freeze({
      list: function(){ return Promise.resolve([]); },
      run: function(){ return Promise.reject(new Error('no agent in the harness')); },
      act: function(){ return Promise.reject(new Error('no agent in the harness')); },
      reset: function(){ return Promise.resolve({}); }
    }),
    storage: Object.freeze({
      get: function(key){ return Promise.resolve(storage.has(key) ? storage.get(key) : undefined); },
      set: function(key, value){
        var bytes = new TextEncoder().encode(JSON.stringify(value)).length;
        if (bytes > 64 * 1024) return Promise.reject(new Error('value exceeds 64 KiB: ' + key));
        storage.set(key, JSON.parse(JSON.stringify(value)));
        return Promise.resolve({});
      },
      delete: function(key){ storage.delete(key); return Promise.resolve({}); },
      list: function(prefix){
        return Promise.resolve(Array.from(storage.keys()).filter(function(key){
          return key.indexOf(prefix || '') === 0;
        }));
      }
    }),
    ui: Object.freeze({
      setStatus: function(state, message, progress){
        if (['loading','ready','error'].indexOf(state) === -1) return Promise.reject(new Error('bad state'));
        if (typeof message !== 'string' || message.length > 200) return Promise.reject(new Error('bad message'));
        status = { state: state, message: message, progress: progress };
        record({ kind: 'status', state: state, progress: progress });
        return Promise.resolve({});
      },
      setTitle: function(title){
        if (typeof title !== 'string' || !title.trim() || title.length > 80) {
          return Promise.reject(new Error('invalid title'));
        }
        return Promise.resolve({});
      },
      setDirty: function(){ return Promise.resolve({}); },
      requestDetach: function(){ return Promise.resolve({}); },
      close: function(){ return Promise.resolve({}); }
    })
  });

  global.__harness = {
    status: function(){ return status; },
    log: function(){ return log; },
    storageKeys: function(){ return Array.from(storage.keys()); },
    storageBytes: function(){
      var total = 0;
      storage.forEach(function(value, key){ total += key.length + JSON.stringify(value).length; });
      return total;
    },
    /* Actions are invoked the way the host does: the declared name, a plain
     * arguments object, and a JSON result that must fit 60 KiB. */
    call: function(name, args){
      var handler = actions.get(name);
      if (!handler) return Promise.reject(new Error('action not registered: ' + name));
      return Promise.resolve().then(function(){
        return handler(args || {});
      }).then(function(result){
        var payload = result == null ? {} : result;
        var bytes = JSON.stringify(payload).length;
        if (bytes > 60 * 1024) throw new Error(name + ' result is ' + bytes + ' bytes, over the 60 KiB cap');
        return { result: payload, bytes: bytes };
      });
    },
    registered: function(){ return Array.from(actions.keys()); }
  };
})(window);
`;

const SCENARIO = `
(function(){
  'use strict';
  var results = [];
  var failed = false;

  function check(name, condition, detail) {
    results.push({ name: name, ok: !!condition, detail: detail === undefined ? '' : String(detail) });
    if (!condition) failed = true;
  }

  function sleep(ms) { return new Promise(function(r){ setTimeout(r, ms); }); }

  function waitFor(predicate, label, timeout) {
    var deadline = Date.now() + (timeout || 15000);
    return (function poll(){
      return Promise.resolve(predicate()).then(function(value){
        if (value) return value;
        if (Date.now() > deadline) throw new Error('timed out: ' + label);
        return sleep(60).then(poll);
      });
    })();
  }

  function report() {
    return fetch('/report', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ failed: failed, results: results })
    });
  }

  window.addEventListener('error', function(event){
    check('no uncaught error', false, event.message + ' @ ' + event.filename + ':' + event.lineno);
  });
  window.addEventListener('unhandledrejection', function(event){
    check('no unhandled rejection', false, String(event.reason && event.reason.message || event.reason));
  });

  (async function(){
    try {
      /* 1. The view must reach ready, and only then expose its actions. */
      await waitFor(function(){ return window.__harness.status().state === 'ready'; }, 'ready status');
      check('reports ready to the host', true);

      var statuses = window.__harness.log().filter(function(entry){ return entry.kind === 'status'; });
      check('reports loading before ready', statuses.length > 1 && statuses[0].state === 'loading',
        statuses.map(function(s){ return s.state + ':' + s.progress; }).join(' '));

      var registered = window.__harness.registered();
      ['kernel.status','notebook.state','cell.add','cell.update','cell.delete','cell.run',
       'job.status','job.cancel','data.list','data.preview','data.add','data.remove','notebook.new']
        .forEach(function(name){
          check('action registered: ' + name, registered.indexOf(name) !== -1);
        });

      /* The hidden attribute has to actually remove the boot card from the
       * layout: a CSS display would otherwise beat it and push the notebook
       * below the fold. */
      var boot = document.getElementById('boot');
      check('boot screen hidden', boot.hidden);
      check('boot screen takes no space', boot.getBoundingClientRect().height === 0,
        getComputedStyle(boot).display + ' / ' + boot.getBoundingClientRect().height);
      check('shell visible', !document.getElementById('shell').hidden);
      check('drop hint takes no space',
        document.getElementById('dropHint').getBoundingClientRect().height === 0);
      check('the page itself does not scroll',
        document.documentElement.scrollHeight <= window.innerHeight + 2,
        document.documentElement.scrollHeight + ' vs ' + window.innerHeight);

      /* 2. The interface is translated, not left with markup placeholders. */
      check('locale applied', document.documentElement.lang === 'fr', document.documentElement.lang);
      check('toolbar translated', document.getElementById('addCodeBtn').textContent.trim().length > 0);

      /* 3. kernel.status names the language to write in. */
      var kernel = (await window.__harness.call('kernel.status')).result;
      check('kernel is ready', kernel.ready === true);
      check('kernel names a language', kernel.language === 'studio' || kernel.language === 'python', kernel.language);

      /* 4. Data enters through the action, as an agent would do it. */
      var state = (await window.__harness.call('notebook.state')).result;
      var added = (await window.__harness.call('data.add', {
        name: 'ventes',
        text: 'annee;produit;montant\\n2021;Ruche;1200,50\\n2021;Cadre;300\\n2022;Ruche;1800,25\\n2022;Cadre;450\\n2023;Ruche;2100\\n2023;Cadre;510',
        revision: state.revision
      })).result;
      check('dataset loaded', added.rows === 6, JSON.stringify(added.columns));
      check('comma decimals parsed as numbers',
        added.schema.filter(function(c){ return c.name === 'montant' && c.dtype === 'number'; }).length === 1,
        JSON.stringify(added.schema));

      var listed = (await window.__harness.call('data.list')).result;
      check('dataset listed', listed.datasets.length === 1 && listed.datasets[0].name === 'ventes');

      var preview = (await window.__harness.call('data.preview', { name: 'ventes', limit: 3 })).result;
      check('preview returns rows', preview.preview.length === 3);

      check('dataset panel rendered', document.querySelectorAll('#datasetList .dataset').length === 1);

      /* 5. A stale revision must be refused, not silently applied. */
      var current = (await window.__harness.call('notebook.state')).result;
      var refused = false;
      try {
        await window.__harness.call('cell.add', { source: 'print(1)', revision: current.revision - 1 });
      } catch (error) {
        refused = /[Ss]tale/.test(String(error.message));
      }
      check('stale revision refused', refused);

      /* 6. Add, stream into, and run a cell. */
      var cell = (await window.__harness.call('cell.add', {
        source: 'v = load("ventes")',
        revision: current.revision
      })).result;
      check('cell added', typeof cell.cellId === 'string');

      await window.__harness.call('cell.update', {
        cellId: cell.cellId,
        source: '\\ntotal = v.groupby("annee").agg(ca = sum(montant))',
        mode: 'append',
        revision: cell.revision
      });
      var afterAppend = (await window.__harness.call('notebook.state')).result;
      var appended = afterAppend.cells.filter(function(c){ return c.id === cell.cellId; })[0];
      check('append streams into the cell', appended.source.indexOf('groupby') !== -1, appended.source);
      check('agent authorship recorded', appended.author === 'agent', appended.author);

      var editor = document.querySelector('[data-cell-id="' + cell.cellId + '"] [data-role="editor"]');
      check('editor shows the agent code', editor && editor.value.indexOf('groupby') !== -1);

      await window.__harness.call('cell.update', {
        cellId: cell.cellId,
        source: 'v = load("ventes")\\ntotal = v.groupby("annee").agg(ca = sum(montant))\\nshow(total)\\nbar(total, x = "annee", y = "ca", title = "CA par annee")',
        mode: 'replace',
        revision: afterAppend.revision
      });

      var beforeRun = (await window.__harness.call('notebook.state')).result;
      var job = (await window.__harness.call('cell.run', {
        cellId: cell.cellId,
        revision: beforeRun.revision
      })).result;
      check('run returns a job without waiting', typeof job.jobId === 'string' &&
        (job.status === 'queued' || job.status === 'running'), job.status);

      /* 7. Poll the job the way the guide tells the agent to. */
      var finished = await waitFor(function(){
        return window.__harness.call('job.status', { jobId: job.jobId }).then(function(response){
          var snapshot = response.result;
          return (snapshot.status === 'ok' || snapshot.status === 'error' || snapshot.status === 'cancelled')
            ? snapshot : null;
        });
      }, 'job to finish');

      check('job succeeded', finished.status === 'ok', JSON.stringify(finished.error));
      check('job returns a table', finished.outputs.some(function(o){ return o.kind === 'table'; }));
      check('job returns a chart', finished.outputs.some(function(o){ return o.kind === 'chart'; }));

      var table = finished.outputs.filter(function(o){ return o.kind === 'table'; })[0];
      check('aggregation is correct', table && table.rows.length === 3 &&
        Math.abs(table.rows[0][1] - 1500.5) < 0.001, table && JSON.stringify(table.rows));

      /* 8. What the user sees: a real table and a real SVG in the page. */
      var article = document.querySelector('[data-cell-id="' + cell.cellId + '"]');
      check('cell marked ok', article.classList.contains('is-ok'), article.className);
      /* Scoped to the direct child: a chart carries its own table view, which
       * would otherwise be counted here too. */
      var shownTable = article.querySelector('.outputs > .output-table');
      check('table rendered in the page', shownTable && shownTable.querySelectorAll('tbody tr').length === 3,
        shownTable ? shownTable.querySelectorAll('tbody tr').length : 'no table');

      var svg = article.querySelector('.chart-holder svg');
      check('chart rendered as svg', !!svg);
      check('chart is responsive', svg && svg.getAttribute('viewBox'), svg && svg.getAttribute('viewBox'));
      check('chart has marks', svg && svg.querySelectorAll('.mark').length > 0,
        svg ? svg.querySelectorAll('.mark').length : 0);
      check('chart offers its table view', !!article.querySelector('.chart-table'));

      /* 9. Errors surface with a position instead of throwing. */
      var stateForError = (await window.__harness.call('notebook.state')).result;
      var badCell = (await window.__harness.call('cell.add', {
        source: 'load("ventes").filter(inconnu > 1)',
        revision: stateForError.revision
      })).result;
      var badJob = (await window.__harness.call('cell.run', {
        cellId: badCell.cellId,
        revision: badCell.revision
      })).result;
      var badResult = await waitFor(function(){
        return window.__harness.call('job.status', { jobId: badJob.jobId }).then(function(response){
          var snapshot = response.result;
          return snapshot.status === 'error' || snapshot.status === 'ok' ? snapshot : null;
        });
      }, 'failing job');
      check('bad cell fails visibly', badResult.status === 'error', badResult.status);
      check('error carries a line', badResult.error && badResult.error.line >= 1,
        badResult.error && JSON.stringify(badResult.error));
      check('error shown in the page',
        !!document.querySelector('[data-cell-id="' + badCell.cellId + '"] .output-error'));

      /* 10. The notebook follows the agent, and lets go when the reader scrolls
       * away. */
      var pane = document.querySelector('.notebook-pane');
      var distance = function(){ return pane.scrollHeight - pane.scrollTop - pane.clientHeight; };

      for (var f = 0; f < 8; f += 1) {
        var filler = (await window.__harness.call('notebook.state')).result;
        await window.__harness.call('cell.add', {
          source: '# remplissage ' + f + '\\n' + 'print("' + f + '")',
          revision: filler.revision
        });
      }
      await sleep(150);
      check('pane actually overflows', pane.scrollHeight > pane.clientHeight + 50,
        pane.scrollHeight + ' vs ' + pane.clientHeight);
      /* Diagnostic, restored immediately: the pane and not the document must
       * be the element that scrolls. */
      var resting = pane.scrollTop;
      pane.scrollTop = 0;
      var movedUp = pane.scrollTop === 0 && resting > 0;
      pane.scrollTop = resting;
      check('the pane is the scroll container', movedUp,
        'resting at ' + resting + ', after scrolling to top ' + pane.scrollTop);

      check('follows the agent to the bottom', distance() < 60, distance());

      /* Reader scrolls up: the follow must stop. */
      pane.scrollTop = 0;
      pane.dispatchEvent(new Event('scroll'));
      await sleep(60);
      var beforeUnstuck = pane.scrollTop;
      var away = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('cell.add', { source: 'print("pendant lecture")', revision: away.revision });
      await sleep(200);
      check('a cell added while reading does not yank the view',
        Math.abs(pane.scrollTop - beforeUnstuck) < 20, pane.scrollTop + ' vs ' + beforeUnstuck);

      /* Back to the bottom: the follow resumes. */
      pane.scrollTop = pane.scrollHeight;
      pane.dispatchEvent(new Event('scroll'));
      await sleep(60);
      var back = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('cell.add', { source: 'print("retour en bas")', revision: back.revision });
      await sleep(250);
      check('following resumes at the bottom', distance() < 60, distance());

      /* 11. Persistence: chunks written, none over the per-value cap. */
      await waitFor(function(){
        return window.__harness.storageKeys().some(function(key){ return key.indexOf('studio.nb.') === 0; });
      }, 'a save to happen', 8000);
      check('notebook persisted', window.__harness.storageKeys().indexOf('studio.lib.v1') !== -1,
        window.__harness.storageKeys().join(','));
      check('storage stays under 1 MiB', window.__harness.storageBytes() < 1024 * 1024,
        window.__harness.storageBytes());

      /* 12. Snapshot size: the notebook.state reply must fit the bridge. */
      var big = (await window.__harness.call('notebook.state'));
      check('notebook.state fits the bridge budget', big.bytes < 60 * 1024, big.bytes + ' bytes');

      /* 13. Interface controls respond. */
      document.getElementById('tabVars').click();
      check('variables tab shows the namespace',
        document.querySelectorAll('#variableList li').length >= 2,
        document.querySelectorAll('#variableList li').length);

      document.getElementById('addNoteBtn').click();
      check('note cell added from the toolbar',
        document.querySelectorAll('.cell.kind-markdown').length >= 1);

      /* 14. Argument aliases: callers reliably send content and id. */
      var aliasState = (await window.__harness.call('notebook.state')).result;
      var aliasCell = (await window.__harness.call('cell.add', {
        content: 'print("alias")',
        language: 'studio',
        revision: aliasState.revision
      })).result;
      check('content is accepted for source', typeof aliasCell.cellId === 'string');

      var aliasJob = (await window.__harness.call('cell.run', {
        id: aliasCell.cellId,
        revision: aliasCell.revision
      })).result;
      check('id is accepted for cellId', typeof aliasJob.jobId === 'string');
      var aliasResult = await waitFor(function(){
        return window.__harness.call('job.status', { jobId: aliasJob.jobId }).then(function(response){
          return response.result.status === 'ok' || response.result.status === 'error' ? response.result : null;
        });
      }, 'the alias job');
      check('the aliased cell ran', aliasResult.status === 'ok', JSON.stringify(aliasResult.error));

      var missing = '';
      try {
        var badState = (await window.__harness.call('notebook.state')).result;
        await window.__harness.call('cell.add', { revision: badState.revision });
      } catch (error) {
        missing = String(error.message);
      }
      check('a missing source names the expected argument', /source/.test(missing), missing);

      /* 15. An explicit colour overrides the palette. */
      var colourState = (await window.__harness.call('notebook.state')).result;
      var colourCell = (await window.__harness.call('cell.add', {
        source: 'hist(load("ventes"), column = "montant", bins = 5, color = "#ccff00", title = "Acide")',
        revision: colourState.revision
      })).result;
      var colourJob = (await window.__harness.call('cell.run', {
        cellId: colourCell.cellId, revision: colourCell.revision
      })).result;
      await waitFor(function(){
        return window.__harness.call('job.status', { jobId: colourJob.jobId }).then(function(response){
          return response.result.status === 'ok' || response.result.status === 'error' ? response.result : null;
        });
      }, 'the coloured chart');
      var colouredSvg = document.querySelector('[data-cell-id="' + colourCell.cellId + '"] .chart-holder svg');
      check('an explicit colour reaches the svg',
        colouredSvg && colouredSvg.innerHTML.indexOf('#ccff00') !== -1);

      var refusedColour = '';
      var badColourState = (await window.__harness.call('notebook.state')).result;
      var badColourCell = (await window.__harness.call('cell.add', {
        source: 'bar(load("ventes"), x = "produit", y = "montant", color = "vert acide")',
        revision: badColourState.revision
      })).result;
      var badColourJob = (await window.__harness.call('cell.run', {
        cellId: badColourCell.cellId, revision: badColourCell.revision
      })).result;
      var badColourResult = await waitFor(function(){
        return window.__harness.call('job.status', { jobId: badColourJob.jobId }).then(function(response){
          return response.result.status === 'ok' || response.result.status === 'error' ? response.result : null;
        });
      }, 'the bad colour');
      refusedColour = badColourResult.error ? badColourResult.error.message : '';
      check('a colour that is not hex is refused', /hex/.test(refusedColour), refusedColour);

      /* 16. Results are actually written to storage, so a reopen restores them. */
      await sleep(1200);
      var libraryValue = await window.LaRucheApp.storage.get('studio.lib.v1');
      check('the library index is stored', libraryValue && Array.isArray(libraryValue.notebooks));

      var storedParts = [];
      var activeEntry = libraryValue.notebooks.filter(function(entry){ return entry.id === libraryValue.activeId; })[0];
      for (var c = 0; c < (activeEntry.chunks || 1); c += 1) {
        storedParts.push(await window.LaRucheApp.storage.get('studio.nb.' + activeEntry.id + '.' + c));
      }
      var storedPayload = window.StudioNotebook.unchunk(storedParts);
      var storedWithOutputs = storedPayload.cells.filter(function(cell){ return cell.outputs && cell.outputs.length; });
      check('results are saved with the code', storedWithOutputs.length > 0,
        storedWithOutputs.length + ' cells carry results');
      var storedChart = null;
      storedWithOutputs.forEach(function(cell){
        cell.outputs.forEach(function(output){ if (output.kind === 'chart') storedChart = output; });
      });
      check('a chart is saved as its specification', storedChart && storedChart.series.length > 0);

      var revived = window.StudioNotebook.deserialize(storedPayload);
      check('a reload brings the results back',
        revived.cells.some(function(cell){ return cell.outputs.length > 0 && cell.status === 'ok'; }));

      /* 17. The notebook library. */
      var beforeNew = (await window.__harness.call('notebook.state')).result;
      var created = (await window.__harness.call('notebook.new', {
        title: 'Deuxieme carnet', revision: beforeNew.revision
      })).result;
      check('a new notebook is created', created.notebookId !== beforeNew.notebookId);
      check('the new notebook is named', created.title === 'Deuxieme carnet');

      var listed2 = (await window.__harness.call('notebook.list')).result;
      check('both notebooks are listed', listed2.notebooks.length >= 2, listed2.notebooks.length);
      check('the active one is flagged',
        listed2.notebooks.filter(function(entry){ return entry.active; }).length === 1);
      check('the picker shows them', document.querySelectorAll('#notebookSelect option').length >= 2);

      var renameState = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('notebook.rename', { title: 'Renomme', revision: renameState.revision });
      check('rename reaches the title field', document.getElementById('notebookTitle').value === 'Renomme');

      var first = listed2.notebooks.filter(function(entry){ return !entry.active; })[0];
      var openState = (await window.__harness.call('notebook.state')).result;
      var opened = (await window.__harness.call('notebook.open', {
        notebookId: first.id, revision: openState.revision
      })).result;
      check('the earlier notebook reopens', opened.notebookId === first.id);
      check('its cells are back', opened.cells > 0, opened.cells);
      check('the reply warns that the kernel was reset', /kernel was reset/i.test(opened.note || ''));

      var deleteState = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('notebook.delete', {
        notebookId: created.notebookId, revision: deleteState.revision
      });
      var afterDelete = (await window.__harness.call('notebook.list')).result;
      check('the deleted notebook is gone',
        afterDelete.notebooks.every(function(entry){ return entry.id !== created.notebookId; }));

      /* 18. Three axes: a rotatable point cloud. */
      var newline = String.fromCharCode(10);
      var cloudRows = 'x,y,z,groupe' + newline;
      for (var g = 0; g < 30; g += 1) {
        cloudRows += g + ',' + (g * 1.5 + 2) + ',' + (30 - g) + ',' +
          (g % 3 === 0 ? 'Ruche' : 'Cadre') + newline;
      }
      var cloudDataState = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('data.add', {
        name: 'nuage', text: cloudRows, revision: cloudDataState.revision
      });

      var cloudState = (await window.__harness.call('notebook.state')).result;
      var cloudCell = (await window.__harness.call('cell.add', {
        source: 'scatter3d(load("nuage"), x = "x", y = "y", z = "z", ' +
                'series = "groupe", title = "Nuage", color = "#ff7a18")',
        revision: cloudState.revision
      })).result;
      var cloudJob = (await window.__harness.call('cell.run', {
        cellId: cloudCell.cellId, revision: cloudCell.revision
      })).result;
      var cloudResult = await waitFor(function(){
        return window.__harness.call('job.status', { jobId: cloudJob.jobId }).then(function(response){
          return response.result.status === 'ok' || response.result.status === 'error' ? response.result : null;
        });
      }, 'the 3d cell');
      check('a 3d cloud renders', cloudResult.status === 'ok', JSON.stringify(cloudResult.error));

      var cloudOutput = cloudResult.outputs.filter(function(o){ return o.kind === 'chart'; })[0];
      check('the cloud is summarised, not sent whole',
        cloudOutput && typeof cloudOutput.points === 'number' && cloudOutput.sample.length <= 10,
        JSON.stringify(cloudOutput).length + ' bytes');

      var cloudHolder = document.querySelector('[data-cell-id="' + cloudCell.cellId + '"] .chart-holder');
      var cloudSvg = cloudHolder && cloudHolder.querySelector('svg');
      check('the cloud is drawn', cloudSvg && cloudSvg.querySelectorAll('.mark').length > 10,
        cloudSvg ? cloudSvg.querySelectorAll('.mark').length : 'no svg');
      check('the orange override reaches the cloud',
        cloudSvg && cloudSvg.innerHTML.indexOf('#ff7a18') !== -1);

      var beforeRotation = cloudHolder.innerHTML;
      cloudHolder.dispatchEvent(new PointerEvent('pointerdown', { clientX: 300, clientY: 200, pointerId: 1, bubbles: true }));
      cloudHolder.dispatchEvent(new PointerEvent('pointermove', { clientX: 380, clientY: 230, pointerId: 1, bubbles: true }));
      cloudHolder.dispatchEvent(new PointerEvent('pointerup', { clientX: 380, clientY: 230, pointerId: 1, bubbles: true }));
      check('dragging turns the cloud', cloudHolder.innerHTML !== beforeRotation);

      /* 19. Table sorting, filtering and paging, in the page. */
      var tableState = (await window.__harness.call('notebook.state')).result;
      var tableCell = (await window.__harness.call('cell.add', {
        source: 'show(load("nuage"), limit = 40)',
        revision: tableState.revision
      })).result;
      var tableJob = (await window.__harness.call('cell.run', {
        cellId: tableCell.cellId, revision: tableCell.revision
      })).result;
      await waitFor(function(){
        return window.__harness.call('job.status', { jobId: tableJob.jobId }).then(function(response){
          return response.result.status === 'ok' || response.result.status === 'error' ? response.result : null;
        });
      }, 'the table cell');

      var tableNode = document.querySelector('[data-cell-id="' + tableCell.cellId + '"] .outputs > .output-table');
      check('a long table is paginated', tableNode.querySelectorAll('tbody tr').length === 25,
        tableNode.querySelectorAll('tbody tr').length);

      var pagerButtons = tableNode.querySelectorAll('.pager .link-button');
      check('the pager is shown', !tableNode.querySelector('.pager').hidden);
      pagerButtons[1].click();
      check('paging moves through the rows',
        tableNode.querySelector('.pager span').textContent.indexOf('2') !== -1,
        tableNode.querySelector('.pager span').textContent);

      var header = tableNode.querySelectorAll('th')[1];
      var firstBefore = tableNode.querySelector('tbody td:last-child').textContent;
      header.click();
      check('a column sorts on click', tableNode.querySelectorAll('th.sorted-asc').length === 1);
      header.click();
      check('a second click reverses it', tableNode.querySelectorAll('th.sorted-desc').length === 1);
      void firstBefore;

      var searchBox = tableNode.querySelector('.table-search');
      searchBox.value = 'Ruche';
      searchBox.dispatchEvent(new Event('input', { bubbles: true }));
      var filtered = tableNode.querySelectorAll('tbody tr').length;
      check('searching narrows the rows', filtered > 0 && filtered <= 25, filtered);
      check('the match count is shown', /\\d/.test(tableNode.querySelector('.table-controls .muted').textContent));

      /* 20. Variables and kernel, the extension's sidebar as actions. */
      var varsState = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('vars.set', { name: 'seuil', value: '100', revision: varsState.revision });
      var listedVars = (await window.__harness.call('vars.list')).result;
      check('a variable can be defined without running a cell',
        listedVars.variables.some(function(entry){ return entry.name === 'seuil'; }),
        JSON.stringify(listedVars.variables));
      check('the variables panel shows it',
        document.querySelectorAll('#variableList li').length >= 1);

      var deleteVarState = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('vars.delete', { name: 'seuil', revision: deleteVarState.revision });
      var afterDeleteVars = (await window.__harness.call('vars.list')).result;
      check('a variable can be removed',
        !afterDeleteVars.variables.some(function(entry){ return entry.name === 'seuil'; }));

      var resetState = (await window.__harness.call('notebook.state')).result;
      var reset = (await window.__harness.call('kernel.reset', { revision: resetState.revision })).result;
      check('the kernel resets', reset.reset === true);
      check('the reset explains what is gone', /namespace/i.test(reset.note || ''));
      check('the namespace is empty after a reset',
        (await window.__harness.call('vars.list')).result.variables.length === 0);
      check('datasets survive a reset',
        (await window.__harness.call('data.list')).result.datasets.length === 2);

      /* 21. A dataset can be read back out. */
      var exported = (await window.__harness.call('data.export', { name: 'nuage', limit: 5 })).result;
      check('a dataset exports as delimited text',
        exported.csv.split(newline).length === 6 && exported.rows === 5 && exported.total === 30,
        exported.rows + ' of ' + exported.total + ', ' + exported.csv.split(newline).length + ' lines');
      check('the export says it is partial', exported.truncated === true);

      /* 22. Cleanup. */
      var cleanup = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('data.remove', { name: 'ventes', revision: cleanup.revision });
      var afterFirst = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('data.remove', { name: 'nuage', revision: afterFirst.revision });
      check('datasets removed', document.querySelectorAll('#datasetList .dataset').length === 0);

    } catch (error) {
      check('scenario completed', false, String(error && error.stack || error));
    }

    try {
      /* Un carnet neuf, pour partir d'un etat connu quoi qu'aient laisse les
         etapes precedentes. La revision courante est exigee a chaque mutation. */
      var courant = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('notebook.new', { revision: courant.revision, title: 'Suivi' });
      await sleep(150);
      /* Le carnet suit l'agent, et lache prise quand le lecteur s'en mele.
       *
       * La regression corrigee ici: redessiner la liste remet le defilement en
       * haut puis le restaure, et ces ecritures emettaient un evenement lu
       * comme un geste du lecteur. Le suivi se coupait donc tout seul des que
       * le carnet depassait un ecran, et l'agent ecrivait sans que rien bouge. */
      var pane = document.querySelector('.notebook-pane');
      function auBas(){ return pane.scrollHeight - pane.scrollTop - pane.clientHeight <= 48; }
      async function remplir(n){
        for (var i = 0; i < n; i++) {
          var etat = (await window.__harness.call('notebook.state')).result;
          await window.__harness.call('cell.add', {
            type: 'markdown',
            source: 'Remplissage ' + i + ' ' + 'texte de remplissage '.repeat(30),
            revision: etat.revision
          });
        }
        await sleep(200);
      }

      await remplir(14);
      check('le carnet deborde apres remplissage', pane.scrollHeight > pane.clientHeight + 100,
        pane.scrollHeight + ' vs ' + pane.clientHeight);
      var doc = document.scrollingElement;
      check('le carnet suit l agent jusqu au bas', auBas(),
        'volet ' + Math.round(pane.scrollTop) + '/' + pane.scrollHeight + ' visible ' + pane.clientHeight +
        ' | document ' + Math.round(doc.scrollTop) + '/' + doc.scrollHeight + ' visible ' + doc.clientHeight +
        ' | largeur ' + window.innerWidth);

      /* Le lecteur remonte: le suivi doit lacher et ne plus rien imposer. */
      pane.scrollTop = 0;
      pane.dispatchEvent(new Event('scroll'));
      await sleep(80);
      var avant = pane.scrollTop;
      await remplir(2);
      check('remonter coupe le suivi', Math.abs(pane.scrollTop - avant) < 4,
        'scrollTop=' + Math.round(pane.scrollTop) + ' attendu=' + Math.round(avant));

      /* La sequence reelle d'une demonstration: l'agent ajoute plusieurs
       * cellules d'un trait, PUIS les execute depuis la premiere. La vue doit
       * alors remonter sur la cellule qui travaille, et non rester au bas du
       * carnet ou plus rien ne se passe. */
      var etatRun = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('notebook.new', { revision: etatRun.revision, title: 'Execution' });
      await sleep(150);
      var ids = [];
      for (var k = 0; k < 7; k++) {
        var e = (await window.__harness.call('notebook.state')).result;
        var ajoutee = (await window.__harness.call('cell.add', {
          type: 'markdown',
          source: 'Bloc ' + k + ' ' + 'texte de remplissage '.repeat(30),
          revision: e.revision
        })).result;
        ids.push(ajoutee && (ajoutee.cellId || ajoutee.id));
      }
      await sleep(250);
      check('le carnet deborde avant execution', pane.scrollHeight > pane.clientHeight + 200,
        pane.scrollHeight + ' vs ' + pane.clientHeight);

      /* L'agent revient travailler sur la PREMIERE cellule, comme il le fait
       * quand il execute le carnet depuis le debut. */
      var premiere = document.querySelector('[data-cell-id]');
      var idPremiere = premiere.getAttribute('data-cell-id');
      var avantCentrage = pane.scrollTop;
      var etatMaj = (await window.__harness.call('notebook.state')).result;
      await window.__harness.call('cell.update', {
        cellId: idPremiere, mode: 'replace',
        source: 'Bloc 0 repris par l agent ' + 'texte de remplissage '.repeat(30),
        revision: etatMaj.revision
      });
      await sleep(300);
      premiere = document.querySelector('[data-cell-id="' + idPremiere + '"]');
      var boite = premiere.getBoundingClientRect();
      var volet = pane.getBoundingClientRect();
      var visible = boite.bottom > volet.top && boite.top < volet.bottom;
      var halo = document.querySelector('.cell.is-active');
      check('un halo marque le bloc travaille', !!halo && halo.getAttribute('data-cell-id') === idPremiere,
        halo ? halo.getAttribute('data-cell-id') : 'aucun');
      check('un seul bloc porte le halo', document.querySelectorAll('.cell.is-active').length === 1,
        document.querySelectorAll('.cell.is-active').length);
      check('la vue remonte sur la cellule qui travaille', visible,
        'scrollTop ' + Math.round(avantCentrage) + ' -> ' + Math.round(pane.scrollTop) +
        ' | cellule ' + Math.round(boite.top - volet.top) + ' du haut du volet');

      /* Le lecteur redescend au bas: le suivi doit se raccrocher. */
      pane.scrollTop = pane.scrollHeight;
      pane.dispatchEvent(new Event('scroll'));
      await sleep(80);
      await remplir(2);
      check('redescendre raccroche le suivi', auBas(),
        'scrollTop=' + Math.round(pane.scrollTop) + ' hauteur=' + pane.scrollHeight);

      /* Le zoom, le halo et le volet repliable. */
      var racine = document.documentElement;
      var avantZoom = getComputedStyle(document.querySelector('.shell')).zoom;
      document.getElementById('zoomOutBtn').click();
      await sleep(120);
      check('dezoomer change l echelle de la coque',
        getComputedStyle(document.querySelector('.shell')).zoom !== avantZoom,
        avantZoom + ' -> ' + getComputedStyle(document.querySelector('.shell')).zoom);
      var coque = document.querySelector('.shell').getBoundingClientRect();
      check('la coque remplit toujours la hauteur apres dezoom',
        Math.abs(coque.height - window.innerHeight) < 8,
        Math.round(coque.height) + ' pour ' + window.innerHeight);
      /* Dezoomer donne de la largeur logique, et la mise en page doit s'en
         servir. Un seuil mesure a la fenetre ne le voit pas: a 60% dans un
         panneau de 600 px l'App dispose de 1000 px et restait en une colonne. */
      for (var z = 0; z < 4; z++) { document.getElementById('zoomOutBtn').click(); }
      await sleep(220);
      var facteur = parseFloat(getComputedStyle(document.querySelector('.shell')).zoom) || 1;
      var logique = window.innerWidth / facteur;
      var direction = getComputedStyle(document.querySelector('.layout')).flexDirection;
      check('la mise en page suit la largeur logique, pas la fenetre',
        logique >= 900 ? direction === 'row' : direction === 'column',
        Math.round(logique) + 'px logiques -> ' + direction);
      check('la coque remplit toujours la largeur apres dezoom',
        Math.abs(coque.width - window.innerWidth) < 8,
        Math.round(coque.width) + ' pour ' + window.innerWidth);
      document.getElementById('zoomValue').click();
      await sleep(120);
      check('le libelle revient a 100%', document.getElementById('zoomValue').textContent === '100%',
        document.getElementById('zoomValue').textContent);

      var volet = document.getElementById('inspector');
      var deplie = volet.querySelector('.panel.is-active').getBoundingClientRect().height;
      document.getElementById('inspectorToggle').click();
      await sleep(120);
      check('replier cache le contenu du volet', volet.classList.contains('is-collapsed') &&
        volet.querySelector('.panel.is-active').getBoundingClientRect().height === 0, 'deplie ' + Math.round(deplie));
      check('les onglets restent visibles une fois replie',
        volet.querySelector('.tabs').getBoundingClientRect().height > 10,
        Math.round(volet.querySelector('.tabs').getBoundingClientRect().height));
      document.getElementById('tabVars').click();
      await sleep(120);
      check('choisir un onglet deplie le volet', !volet.classList.contains('is-collapsed'));
    } catch (error) {
      check('suivi du carnet teste', false, String(error && error.stack || error));
    }

    await report();
    document.title = failed ? 'SMOKE-FAILED' : 'SMOKE-OK';
  })();
})();
`;

function serve(port) {
  return new Promise((resolve) => {
    let reportPayload = null;
    let onReport = null;

    const server = http.createServer((request, response) => {
      if (request.method === 'POST' && request.url === '/report') {
        let raw = '';
        request.on('data', (chunk) => { raw += chunk; });
        request.on('end', () => {
          try { reportPayload = JSON.parse(raw); } catch (error) { reportPayload = { failed: true, results: [] }; }
          response.writeHead(204).end();
          if (onReport) onReport(reportPayload);
        });
        return;
      }

      if (request.url === '/apps-runtime/v1.js') {
        response.writeHead(200, { 'Content-Type': TYPES['.js'] });
        response.end(RUNTIME_STUB + '\n' + wrapScenario());
        return;
      }

      const clean = decodeURIComponent(request.url.split('?')[0]);
      const target = path.join(UI, clean === '/' ? 'index.html' : clean);
      if (!target.startsWith(UI) || !fs.existsSync(target) || fs.statSync(target).isDirectory()) {
        response.writeHead(404).end();
        return;
      }
      response.writeHead(200, { 'Content-Type': TYPES[path.extname(target)] || 'application/octet-stream' });
      response.end(fs.readFileSync(target));
    });

    server.listen(port, '127.0.0.1', () => {
      resolve({
        server,
        port: server.address().port,
        waitForReport: (timeout) => new Promise((done, fail) => {
          if (reportPayload) return done(reportPayload);
          const timer = setTimeout(() => fail(new Error('the page never reported back')), timeout);
          onReport = (payload) => { clearTimeout(timer); done(payload); };
        })
      });
    });
  });
}

/* The scenario runs after the App's own scripts, which are deferred. */
function wrapScenario() {
  return 'window.addEventListener("load", function(){ setTimeout(function(){' + SCENARIO + '}, 0); });';
}

(async function main() {
  const browser = findBrowser();
  if (!browser) {
    process.stdout.write('DS Studio browser: skipped, no Chrome or Edge found. Set CHROME_PATH to run it.\n');
    return;
  }

  const harness = await serve(0);
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'ds-studio-smoke-'));
  const url = 'http://127.0.0.1:' + harness.port + '/index.html';

  const child = spawn(browser, [
    '--headless=new',
    '--disable-gpu',
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-extensions',
    '--user-data-dir=' + profile,
    '--window-size=' + (process.env.DS_WINDOW || '1280,900'),
    url
  ], { stdio: 'ignore', windowsHide: true });

  let report;
  try {
    report = await harness.waitForReport(60000);
  } catch (error) {
    child.kill();
    harness.server.close();
    process.stderr.write('DS Studio browser: ' + error.message + '\n');
    process.exit(1);
    return;
  }

  child.kill();
  harness.server.close();
  try { fs.rmSync(profile, { recursive: true, force: true }); } catch (error) { /* the profile is temporary */ }

  const failures = report.results.filter((entry) => !entry.ok);
  report.results.forEach((entry) => {
    if (!entry.ok) process.stdout.write('  FAIL  ' + entry.name + (entry.detail ? ' -> ' + entry.detail : '') + '\n');
  });

  if (failures.length) {
    process.stdout.write('\nDS Studio browser: ' + failures.length + ' of ' + report.results.length + ' checks failed.\n');
    process.exit(1);
  }

  process.stdout.write('DS Studio browser: ' + report.results.length + ' checks passed in ' +
    path.basename(browser) + '.\n');
})();
