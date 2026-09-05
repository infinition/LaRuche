'use strict';
/* Notebook model, job scheduler, dataset store and persistence budgets.
 * Run with: node examples/apps/ds-studio/test/notebook.test.js */

var assert = require('node:assert/strict');
var Notebook = require('../package/ui/notebook.js');
var Store = require('../package/ui/lib/store.js');
var KernelJs = require('../package/ui/kernel/kernel-js.js');
var Csv = require('../package/ui/lib/csv.js');

var checks = 0;
function section(name) { process.stdout.write('  ' + name + '\n'); }
function ok(message) { checks += 1; void message; }

function waitFor(condition, label) {
  return new Promise(function(resolve, reject){
    var deadline = Date.now() + 4000;
    (function poll(){
      if (condition()) return resolve();
      if (Date.now() > deadline) return reject(new Error('timed out waiting for ' + label));
      setTimeout(poll, 5);
    })();
  });
}

/* ------------------------------------------------------------------ Store */

section('dataset store');

var store = Store.create({ persistBudget: 4096 });
var csv = 'produit,annee,montant\nA,2020,10\nB,2020,4\nA,2021,7\n';
var name = store.addCsv('Ventes 2024.csv', csv);
assert.equal(name, 'Ventes_2024', 'file names become usable identifiers');
assert.equal(store.get(name).frame.length, 3);
ok('csv import');

assert.equal(store.uniqueName('Ventes 2024.csv'), 'Ventes_2024_2', 'collisions get a suffix');
assert.equal(Store.normaliseName('Chiffre d\'affaires 2024'), 'Chiffre_d_affaires_2024');
assert.equal(Store.normaliseName('2024 ventes'), 'd_2024_ventes', 'an identifier cannot start with a digit');
ok('name normalisation');

assert.throws(function(){ store.addRows('x', 'not rows'); }, /expected an array|expected a table/);
ok('type guard');

var listed = store.list();
assert.equal(listed.length, 1);
assert.equal(listed[0].rows, 3);
assert.equal(listed[0].columns, 3);
assert.ok(listed[0].persistable, 'a small dataset is persistable');
ok('listing');

var big = [];
for (var i = 0; i < 4000; i += 1) big.push({ id: i, label: 'row-' + i, value: i * 1.5 });
store.addRows('large', big);
var largeEntry = store.list().find(function(entry){ return entry.name === 'large'; });
assert.ok(!largeEntry.persistable, 'a dataset past the budget is session-only');
ok('persist budget');

var snapshot = store.serialize();
assert.equal(snapshot.datasets.length, 1, 'only the small dataset is written');
assert.equal(snapshot.skipped.length, 1);
assert.equal(snapshot.skipped[0].name, 'large');
ok('serialize skips oversized datasets, visibly');

var restoredStore = Store.create({ persistBudget: 4096 });
assert.equal(restoredStore.restore(snapshot), 1);
assert.equal(restoredStore.get('Ventes_2024').frame.length, 3);
ok('restore');

var dated = Store.create({ persistBudget: 65536 });
dated.addCsv('days', 'day,value\n2024-03-01,5\n2024-03-02,6');
var roundTrip = Store.create({ persistBudget: 65536 });
roundTrip.restore(dated.serialize());
assert.ok(roundTrip.get('days').frame.col('day')[0] instanceof Date, 'dates survive the round trip');
ok('typed round trip');

/* --------------------------------------------------------------- Notebook */

section('notebook model');

var notebook = Notebook.create({ title: 'Ventes' });
assert.equal(notebook.revision, 0);

var first = notebook.addCell({ source: 'print(1)' });
assert.equal(notebook.revision, 1, 'every mutation bumps the revision');
assert.equal(notebook.cells.length, 1);
assert.equal(first.status, 'idle');
ok('add cell');

var markdown = notebook.addCell({ type: 'markdown', source: '# Titre', index: 0 });
assert.equal(notebook.cells[0].id, markdown.id, 'a cell can be inserted at a position');
ok('insert at index');

var events = [];
var stop = notebook.on(function(event){ events.push(event.type); });
notebook.updateCell(first.id, { source: 'print(2)' });
assert.equal(notebook.cell(first.id).source, 'print(2)');
assert.deepEqual(events, ['cell.update']);
ok('update and listener');

notebook.updateCell(first.id, { source: '\nprint(3)', mode: 'append', author: 'agent' });
assert.equal(notebook.cell(first.id).source, 'print(2)\nprint(3)');
assert.equal(notebook.cell(first.id).author, 'agent', 'streaming marks the author');
ok('append mode streams a cell in');
stop();

notebook.moveCell(first.id, 0);
assert.equal(notebook.cells[0].id, first.id);
ok('move');

assert.throws(function(){ notebook.cell('absent'); }, /unknown cell/);
assert.throws(function(){ notebook.addCell({ source: 'x'.repeat(40000) }); }, /exceeds/);
ok('guards');

notebook.deleteCell(markdown.id);
assert.equal(notebook.cells.length, 1);
ok('delete');

/* ----------------------------------------------------------- Persistence */

section('persistence');

var wide = Notebook.create({ title: 'Long' });
for (var c = 0; c < 12; c += 1) {
  wide.addCell({ source: 'x' + c + ' = ' + c + '\n' + 'y'.repeat(9000) });
}
var payload = Notebook.serialize(wide);
var parts = Notebook.chunk(payload, 48 * 1024);
assert.ok(parts.length > 1, 'a large notebook spans several storage values');
parts.forEach(function(part){
  assert.ok(part.length <= 48 * 1024, 'no chunk exceeds the per-value cap');
});
var reassembled = Notebook.unchunk(parts);
assert.deepEqual(reassembled, payload);
ok('chunking respects the 64 KiB per-value limit');

var unicodePayload = {text:('漢字😀é\\\"\n').repeat(25000)};
var unicodeParts = Notebook.chunk(unicodePayload, 48 * 1024);
unicodeParts.forEach(function(part){assert(Buffer.byteLength(JSON.stringify(part),'utf8')<=48*1024);});
assert.deepEqual(Notebook.unchunk(unicodeParts),unicodePayload);
ok('UTF-8 and JSON-escaped strings fit the actual host byte limit');

var duplicateStore = Store.create();
var longName='x'.repeat(64);
var name1=duplicateStore.addRows(longName,[{n:1}]);
var name2=duplicateStore.addRows(longName,[{n:2}]);
assert.notEqual(name1,name2);assert(name2.length<=64);
assert.equal(duplicateStore.get(name1).frame.rows()[0].n,1);
ok('long duplicate dataset names do not overwrite the earlier dataset');

var reloaded = Notebook.deserialize(payload);
assert.equal(reloaded.cells.length, 12);
assert.equal(reloaded.title, 'Long');
assert.equal(reloaded.cells[0].status, 'stale', 'a cell saved without results is stale, not falsely valid');
assert.equal(reloaded.cells[0].outputs.length, 0);
ok('deserialize');

/* Results survive a close and reopen, within the byte budget. */
var withResults = Notebook.create({ title: 'Resultats' });
var tableCell = withResults.addCell({ source: 'show(t)' });
withResults.setOutputs(tableCell.id, [
  { kind: 'table', columns: ['a', 'b'], rows: [['x', 1], ['y', 2]], total: 2, schema: [] },
  { kind: 'chart', chart: 'bar', labels: ['x', 'y'], series: [{ name: 'b', values: [1, 2] }], title: 'T' },
  { kind: 'stream', stream: 'out', text: 'done' }
], { status: 'ok', durationMs: 42, countExecution: true });

var savedResults = Notebook.deserialize(Notebook.serialize(withResults));
var back = savedResults.cells[0];
assert.equal(back.outputs.length, 3, 'every output kind comes back');
assert.equal(back.outputs[0].rows.length, 2);
assert.equal(back.outputs[1].series[0].values[1], 2, 'the chart keeps its numbers');
assert.equal(back.status, 'ok', 'a restored result keeps the status it was produced with');
assert.equal(back.durationMs, 42);
assert.equal(back.execCount, 1);
assert.equal(back.restored, true, 'the cell is flagged as restored, the kernel namespace is not');
ok('results survive a reload');

var failed = Notebook.create({ title: 'Echec' });
var badCell = failed.addCell({ source: 'load("absent")' });
failed.setOutputs(badCell.id, [], { status: 'error', error: { message: 'no dataset', line: 1, hint: 'h' } });
var failedBack = Notebook.deserialize(Notebook.serialize(failed));
assert.equal(failedBack.cells[0].status, 'error');
assert.equal(failedBack.cells[0].error.message, 'no dataset');
ok('a failure and its position survive a reload');

/* A figure larger than the per-image cap is dropped, not truncated. */
var heavy = Notebook.create({ title: 'Figures' });
var smallImage = heavy.addCell({ source: 'plot()' });
heavy.setOutputs(smallImage.id, [
  { kind: 'image', source: 'data:image/png;base64,' + 'A'.repeat(2000), title: 'petite' }
], { status: 'ok' });
var hugeImage = heavy.addCell({ source: 'plot()' });
heavy.setOutputs(hugeImage.id, [
  { kind: 'image', source: 'data:image/png;base64,' + 'A'.repeat(200000), title: 'enorme' }
], { status: 'ok' });

var heavyPayload = Notebook.serialize(heavy);
assert.equal(heavyPayload.cells[0].outputs.length, 1, 'a small figure is kept');
assert.equal(heavyPayload.cells[1].outputs.length, 0, 'a figure past the cap is dropped');
assert.ok(heavyPayload.outputsDropped >= 1, 'the drop is reported, not silent');
ok('oversized figures are dropped and reported');

/* Under a tight budget the most recent cells keep their results. */
var many = Notebook.create({ title: 'Budget' });
var ids = [];
for (var m = 0; m < 6; m += 1) {
  var one = many.addCell({ source: 'cell ' + m });
  ids.push(one.id);
  many.setOutputs(one.id, [
    { kind: 'stream', stream: 'out', text: String(m) + 'z'.repeat(4000) }
  ], { status: 'ok' });
}
var tight = Notebook.serialize(many, { outputBudget: 9000 });
var keptFlags = tight.cells.map(function(cell){ return cell.outputs.length > 0; });
assert.ok(keptFlags[5] && keptFlags[4], 'the newest cells keep their results');
assert.ok(!keptFlags[0], 'the oldest give way first');
assert.ok(tight.outputBytes <= 9000, tight.outputBytes + ' bytes for a 9000 budget');
ok('the output budget is honoured, newest first');

var noResults = Notebook.serialize(many, { outputBudget: 0 });
assert.ok(noResults.cells.every(function(cell){ return cell.outputs.length === 0; }));
ok('a zero budget saves the code alone');

var described = Notebook.describe(many);
assert.equal(described.cellCount, 6);
assert.equal(described.codeCells, 6);
assert.equal(described.id, many.id);
ok('library entries');

assert.equal(Notebook.unchunk(['{bad json']), null, 'a corrupt payload returns null, it does not throw');
assert.throws(function(){ Notebook.deserialize({ version: 99 }); }, /unreadable/);
ok('corrupt payload handling');

/* --------------------------------------------------------------- Runner */

section('cell scheduler');

var runStore = Store.create({});
runStore.addCsv('sales', 'product,year,amount\nA,2020,10\nB,2020,4\nA,2021,7\nB,2021,9\n');

var kernel = KernelJs.create(runStore);

var live = Notebook.create({ title: 'Run' });
var runner = Notebook.runner(live, kernel, { timeoutMs: 3000 });

assert.throws(function(){ runner.submit(live.addCell({ source: 'print(1)' }).id); }, /not ready/);
ok('no job runs before the kernel is ready');

kernel.init().then(function(){
  assert.equal(kernel.ready, true);

  var cellA = live.cells[0];
  live.updateCell(cellA.id, { source: 'total = load("sales").groupby("product").agg(amount = sum(amount))\nshow(total)' });
  var job = runner.submit(cellA.id);
  /* The point is that submit hands back a job handle without waiting for the
   * result, so the action returns well inside the 15 s host deadline. */
  assert.ok(job.status === 'queued' || job.status === 'running', 'submit must not wait for the result');
  assert.equal(job.finishedAt, 0);
  ok('submit is non blocking');

  return waitFor(function(){ return job.status === 'ok' || job.status === 'error'; }, 'the job to finish')
    .then(function(){
      assert.equal(job.status, 'ok', job.error && job.error.message);
      var cell = live.cell(cellA.id);
      assert.equal(cell.status, 'ok');
      assert.equal(cell.execCount, 1);
      assert.equal(cell.outputs[0].kind, 'table');
      assert.deepEqual(cell.outputs[0].rows, [['A', 17], ['B', 13]]);
      ok('job result lands on the cell');

      /* A variable defined in one cell is visible in the next. */
      var second = live.addCell({ source: 'print(total.count())' });
      var job2 = runner.submit(second.id);
      return waitFor(function(){ return job2.status !== 'queued' && job2.status !== 'running'; }, 'the second job')
        .then(function(){
          assert.equal(job2.status, 'ok', job2.error && job2.error.message);
          assert.equal(live.cell(second.id).outputs[0].text, '2');
          ok('cells share one namespace');
        });
    })
    .then(function(){
      var failing = live.addCell({ source: 'load("absent")' });
      var job3 = runner.submit(failing.id);
      return waitFor(function(){ return job3.status !== 'queued' && job3.status !== 'running'; }, 'the failing job')
        .then(function(){
          assert.equal(job3.status, 'error');
          assert.match(job3.error.message, /no dataset/);
          assert.equal(live.cell(failing.id).status, 'error');
          assert.equal(live.cell(failing.id).execCount, 0, 'a failed run does not count as an execution');
          ok('failures are reported, not thrown');
        });
    })
    .then(function(){
      var slow = live.addCell({ source: 'r = range(400000)\nprint(r.count())' });
      var job4 = runner.submit(slow.id);
      runner.cancel(job4.id);
      return waitFor(function(){ return job4.status === 'cancelled'; }, 'the cancelled job')
        .then(function(){
          assert.equal(live.cell(slow.id).status, 'cancelled');
          ok('cancellation');
        });
    })
    .then(function(){
      var markdownCell = live.addCell({ type: 'markdown', source: '# note' });
      assert.throws(function(){ runner.submit(markdownCell.id); }, /only a code cell/);
      ok('markdown cells do not run');
    })
    .then(function(){
      /* Two submissions for the same cell share one job rather than racing. */
      var target = live.addCell({ source: 'print("once")' });
      var a = runner.submit(target.id);
      var b = runner.submit(target.id);
      assert.equal(a.id, b.id, 'a pending cell is not queued twice');
      ok('idempotent submission');
      return waitFor(function(){ return a.status !== 'queued' && a.status !== 'running'; }, 'the shared job');
    })
    .then(function(){
      return runSnapshotChecks(live, runStore);
    });
}).then(function(){
  process.stdout.write('\nDS Studio notebook: ' + checks + ' checks passed.\n');
}).catch(function(error){
  process.stderr.write('\nFAILED: ' + (error && error.stack || error) + '\n');
  process.exit(1);
});

/* -------------------------------------------------------------- Snapshot */

function runSnapshotChecks(live, runStore) {
  section('agent snapshot');

  var snapshot = Notebook.summarize(live, {
    kernel: { id: 'js', language: 'studio', ready: true },
    datasets: runStore.list().map(function(entry){
      return { name: entry.name, rows: entry.rows, columns: entry.columns };
    })
  });

  assert.equal(snapshot.revision, live.revision);
  assert.equal(snapshot.kernel.language, 'studio', 'the snapshot names the language to write in');
  assert.ok(snapshot.datasets.length >= 1);
  assert.ok(snapshot.cells.every(function(cell){ return typeof cell.id === 'string'; }));
  ok('snapshot shape');

  var withImage = Notebook.create({ title: 'Image' });
  var imageCell = withImage.addCell({ source: 'plot()' });
  withImage.setOutputs(imageCell.id, [
    { kind: 'image', source: 'data:image/png;base64,' + 'A'.repeat(80000), title: 'figure' }
  ], { status: 'ok' });
  var imageSnapshot = Notebook.summarize(withImage, {});
  assert.equal(imageSnapshot.cells[0].outputs[0].kind, 'image');
  assert.ok(!imageSnapshot.cells[0].outputs[0].source, 'image bytes are referenced, never inlined');
  assert.ok(JSON.stringify(imageSnapshot).length < 4000);
  ok('images do not blow the bridge budget');

  /* A notebook far larger than the bridge allows must still produce a reply
   * under the cap, and must say that it was truncated. */
  var huge = Notebook.create({ title: 'Huge' });
  for (var i = 0; i < 40; i += 1) {
    var cell = huge.addCell({ source: 'block' + i + '\n' + 'z'.repeat(3000) });
    huge.setOutputs(cell.id, [
      { kind: 'table', columns: ['a', 'b'], rows: manyRows(200), total: 200 },
      { kind: 'stream', stream: 'out', text: 'w'.repeat(5000) }
    ], { status: 'ok' });
  }
  var budget = 40 * 1024;
  var trimmed = Notebook.summarize(huge, { budget: budget });
  var size = JSON.stringify(trimmed).length;
  assert.ok(size <= budget, 'snapshot is ' + size + ' bytes, budget is ' + budget);
  assert.equal(trimmed.truncated, true, 'truncation is reported');
  ok('snapshot honours the byte budget');

  var wideChart = Notebook.summarizeOutput({
    kind: 'chart', chart: 'bar', labels: manyLabels(500),
    series: [{ name: 's', values: manyLabels(500) }]
  }, { rowLimit: 20 });
  assert.equal(wideChart.labels.length, 20);
  assert.equal(wideChart.truncated, true);
  ok('chart snapshots are capped');

  return Promise.resolve();
}

function manyRows(count) {
  var rows = [];
  for (var i = 0; i < count; i += 1) rows.push(['label-' + i, i]);
  return rows;
}

function manyLabels(count) {
  var out = [];
  for (var i = 0; i < count; i += 1) out.push(i);
  return out;
}
