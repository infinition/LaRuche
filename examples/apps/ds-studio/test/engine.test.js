'use strict';
/* Engine tests: table verbs, delimited reading, notebook language, charts.
 * Run with: node examples/apps/ds-studio/test/engine.test.js */

var assert = require('node:assert/strict');
var Frames = require('../package/ui/lib/frame.js');
var Csv = require('../package/ui/lib/csv.js');
var Lang = require('../package/ui/lib/lang.js');
var Chart = require('../package/ui/lib/chart.js');

var checks = 0;

// CSV pages must never be cut inside a UTF-8 value, quoted field or record.
var exportRows=Array.from({length:100},(_,i)=>({id:i,text:'漢字😀,"\n'.repeat(180)}));
var exportFrame=Frames.fromRows(exportRows), offset=0, restoredRows=[];
while(offset!==null){
  var page=Csv.exportPage(exportFrame,'unicode',offset,100,',');
  assert(Buffer.byteLength(JSON.stringify(page),'utf8')<=48000);
  var parsedPage=Csv.parse(page.csv).rows();
  assert.equal(parsedPage.length,page.rows);restoredRows.push(...parsedPage);offset=page.nextOffset;
}
assert(JSON.stringify(restoredRows)===JSON.stringify(exportRows),'CSV pages preserve complete Unicode/quoted fields including their trailing newline');
assert.throws(()=>Csv.exportPage(Frames.fromRows([{text:'x'.repeat(50000)}]),'large',0,1,','),/exceeds/);
function section(name) { process.stdout.write('  ' + name + '\n'); }
function ok(condition, message) {
  checks += 1;
  assert.ok(condition, message);
}

/* ------------------------------------------------------------------ Frame */

section('table engine');

var sales = Frames.fromRows([
  { product: 'A', year: 2020, region: 'north', amount: 10 },
  { product: 'B', year: 2020, region: 'south', amount: 4 },
  { product: 'A', year: 2021, region: 'north', amount: 7 },
  { product: 'B', year: 2021, region: 'south', amount: 9 },
  { product: 'A', year: 2021, region: 'south', amount: 3 }
]);

assert.equal(sales.length, 5);
assert.deepEqual(sales.columns, ['product', 'year', 'region', 'amount']);
assert.equal(sales.dtype('amount'), 'number');
assert.equal(sales.dtype('product'), 'string');
ok(true, 'construction');

var byProduct = sales.groupby('product').agg([
  { out: 'total', fn: 'sum', values: function(indices){ return indices.map(function(i){ return sales.data.amount[i]; }); } },
  { out: 'n', fn: 'count', values: null }
]);
assert.deepEqual(byProduct.col('product'), ['A', 'B']);
assert.deepEqual(byProduct.col('total'), [20, 13]);
assert.deepEqual(byProduct.col('n'), [3, 2]);
ok(true, 'groupby and agg');

/* A composite key must not collide when parts concatenate ambiguously. */
var ambiguous = Frames.fromRows([
  { a: 'x', b: 'yz', v: 1 },
  { a: 'xy', b: 'z', v: 2 }
]);
var grouped = ambiguous.groupby(['a', 'b']);
assert.equal(grouped.groups.length, 2, 'composite keys must stay distinct');
ok(true, 'composite key separation');

var sorted = sales.sort('amount', true);
assert.deepEqual(sorted.col('amount'), [10, 9, 7, 4, 3]);
assert.deepEqual(sales.col('amount'), [10, 4, 7, 9, 3], 'sort must not mutate the source');
ok(true, 'sort is pure');

var pivoted = sales.pivot('product', 'year', 'amount', 'sum');
assert.deepEqual(pivoted.columns, ['product', '2020', '2021']);
assert.deepEqual(pivoted.col('2021'), [10, 9]);
ok(true, 'pivot');

var labels = Frames.fromRows([
  { product: 'A', label: 'Alpha' },
  { product: 'C', label: 'Gamma' }
]);
assert.equal(sales.join(labels, 'product', 'inner').length, 3);
assert.equal(sales.join(labels, 'product', 'left').length, 5);
assert.equal(sales.join(labels, 'product', 'outer').length, 6);
ok(true, 'joins');

var missing = Frames.fromRows([{ a: 1, b: null }, { a: null, b: 2 }, { a: 3, b: 4 }]);
assert.equal(missing.dropna().length, 1);
assert.equal(missing.dropna(['a']).length, 2);
assert.deepEqual(missing.fillna(0).col('a'), [1, 0, 3]);
assert.equal(missing.missingCount('a'), 1);
ok(true, 'missing values');

var stats = sales.describe();
var amountRow = stats.rows().find(function(row){ return row.column === 'amount'; });
assert.equal(amountRow.count, 5);
assert.equal(amountRow.min, 3);
assert.equal(amountRow.max, 10);
assert.equal(amountRow.median, 7);
ok(true, 'describe');

var counts = sales.valueCounts('product');
assert.deepEqual(counts.col('product'), ['A', 'B']);
assert.deepEqual(counts.col('count'), [3, 2]);
ok(true, 'value counts');

var histogram = sales.histogram('amount', 4);
assert.equal(histogram.length, 4);
assert.equal(histogram.col('count').reduce(function(a, b){ return a + b; }, 0), 5);
ok(true, 'histogram bins keep every row');

assert.equal(Frames.aggregations.median([1, 2, 3, 4]), 2.5);
assert.equal(Frames.aggregations.quantile([1, 2, 3, 4], 0.25), 1.75);
assert.equal(Frames.aggregations.std([2, 4, 4, 4, 5, 5, 7, 9]).toFixed(4), '2.1381');
assert.equal(Frames.aggregations.sum([]), null);
ok(true, 'aggregations');

/* -------------------------------------------------------------------- CSV */

section('delimited reader');

var french = Csv.parse('produit;annee;montant\nA;2020;10,5\nB;2020;4\nA;2021;7,5');
assert.deepEqual(french.columns, ['produit', 'annee', 'montant']);
assert.deepEqual(french.col('montant'), [10.5, 4, 7.5]);
assert.equal(french.dtype('annee'), 'number');
ok(true, 'semicolon file with comma decimals');

var quoted = Csv.parse('name,note\n"Durand, Paul","said ""yes"""\n"multi\nline",ok');
assert.equal(quoted.length, 2);
assert.equal(quoted.col('name')[0], 'Durand, Paul');
assert.equal(quoted.col('note')[0], 'said "yes"');
assert.equal(quoted.col('name')[1], 'multi\nline');
ok(true, 'quoting, embedded delimiter and newline');

var dated = Csv.parse('day,value\n2024-03-01,5\n2024-03-02,6');
ok(dated.col('day')[0] instanceof Date, 'ISO dates are typed');
assert.equal(dated.col('day')[0].getUTCFullYear(), 2024);
ok(true, 'date inference');

var booleans = Csv.parse('flag,other\nvrai,1\nfaux,2');
assert.deepEqual(booleans.col('flag'), [true, false]);
assert.deepEqual(booleans.col('other'), [1, 2]);
ok(true, 'boolean inference stays out of numeric columns');

assert.equal(Csv.sniffDelimiter('a\tb\n1\t2'), '\t');
assert.equal(Csv.sniffDelimiter('a|b\n1|2'), '|');
assert.throws(function(){ Csv.parse('   '); }, /empty/);
ok(true, 'delimiter sniffing and empty input');

var noHeader = Csv.parse('1,2,3\n4,5,6');
assert.deepEqual(noHeader.columns, ['column_1', 'column_2', 'column_3']);
assert.equal(noHeader.length, 2);
ok(true, 'headerless file');

/* --------------------------------------------------------------- Language */

section('notebook language');

function run(code, datasets) {
  return Lang.execute(code, { datasets: datasets || {}, timeoutMs: 5000 });
}

var data = { sales: { frame: sales } };

var basic = run('1 + 2 * 3', {});
assert.ok(basic.ok, basic.ok ? '' : basic.error && basic.error.message);
assert.equal(basic.outputs[0].text, '7');
ok(true, 'operator precedence');

var chained = run(
  'total = load("sales").filter(year == 2021).groupby("product").agg(amount = sum(amount))\n' +
  'show(total.sort("amount", desc = true))',
  data
);
assert.ok(chained.ok, chained.ok ? '' : chained.error && chained.error.message);
var table = chained.outputs[chained.outputs.length - 1];
assert.equal(table.kind, 'table');
assert.deepEqual(table.columns, ['product', 'amount']);
assert.deepEqual(table.rows, [['A', 10], ['B', 9]]);
ok(true, 'filter, groupby, agg, sort');

var multiline = run(
  'r = load("sales")\n' +
  '  .filter(amount > 3)\n' +
  '  .sort("amount", desc = true)\n' +
  '  .head(2)\n' +
  'print(r.count())',
  data
);
assert.ok(multiline.ok, multiline.ok ? '' : multiline.error && multiline.error.message);
assert.equal(multiline.outputs[0].text, '2');
ok(true, 'method chain across lines');

var derived = run(
  'x = load("sales").assign(doubled = amount * 2, big = amount > 5)\n' +
  'print(x.sum("doubled"))\n' +
  'print(x.filter(big).count())',
  data
);
assert.ok(derived.ok, derived.ok ? '' : derived.error && derived.error.message);
assert.equal(derived.outputs[0].text, '66');
assert.equal(derived.outputs[1].text, '3');
ok(true, 'row-scoped assign and filter');

var comment = run('# a comment\nprint("ok") # trailing\n', {});
assert.ok(comment.ok);
assert.equal(comment.outputs[0].text, 'ok');
ok(true, 'comments');

var strings = run('print("A" + "-" + upper("b"))\nprint("hello".slice(0, 2))', {});
assert.equal(strings.outputs[0].text, 'A-B');
assert.equal(strings.outputs[1].text, 'he');
ok(true, 'text operations');

var branch = run('print(ifElse(2 > 1, "yes", "no"))\nprint(coalesce(null, null, 3))', {});
assert.equal(branch.outputs[0].text, 'yes');
assert.equal(branch.outputs[1].text, '3');
ok(true, 'conditionals');

var pivotCell = run('show(load("sales").pivot(index = "product", columns = "year", values = "amount"))', data);
assert.ok(pivotCell.ok, pivotCell.ok ? '' : pivotCell.error && pivotCell.error.message);
assert.deepEqual(pivotCell.outputs[0].columns, ['product', '2020', '2021']);
ok(true, 'pivot from a cell');

var described = run('show(load("sales").describe())', data);
assert.ok(described.ok);
assert.equal(described.outputs[0].kind, 'table');
ok(true, 'describe from a cell');

/* Errors carry a position and never throw out of execute. */
var unknownColumn = run('load("sales").filter(nope > 1)', data);
assert.equal(unknownColumn.ok, false);
assert.match(unknownColumn.error.message, /not defined/);
assert.ok(unknownColumn.error.line >= 1);
ok(true, 'unknown identifier reports a position');

var unknownDataset = run('load("absent")', data);
assert.equal(unknownDataset.ok, false);
assert.match(unknownDataset.error.message, /no dataset/);
assert.match(unknownDataset.error.hint, /sales/);
ok(true, 'unknown dataset lists what exists');

var syntax = run('load("sales"', data);
assert.equal(syntax.ok, false);
assert.match(syntax.error.message, /expected/);
ok(true, 'syntax error is reported, not thrown');

var badMethod = run('load("sales").nope()', data);
assert.equal(badMethod.ok, false);
assert.match(badMethod.error.message, /has no method/);
ok(true, 'unknown method');

var badArgument = run('load("sales").sort()', data);
assert.equal(badArgument.ok, false);
assert.match(badArgument.error.message, /required/);
ok(true, 'missing required argument');

/* No eval anywhere: a JavaScript payload is a syntax error, not code. */
var injection = run('constructor.constructor("return 1")()', {});
assert.equal(injection.ok, false);
ok(true, 'javascript payloads do not execute');

var cancelToken = { cancelled: true };
var cancelled = Lang.execute('r = range(200000)\nprint(r.count())', { datasets: {}, token: cancelToken });
assert.equal(cancelled.ok, false);
assert.equal(cancelled.error.cancelled, true);
ok(true, 'cancellation');

var timedOut = Lang.execute('r = range(500000)\nprint(r.count())', { datasets: {}, timeoutMs: -1 });
assert.equal(timedOut.ok, false);
assert.equal(timedOut.error.timeout, true);
ok(true, 'time budget');

var saved = run('save("subset", load("sales").filter(year == 2020))\nprint(load("subset").count())', data);
assert.ok(saved.ok, saved.ok ? '' : saved.error && saved.error.message);
assert.equal(saved.outputs[1].text, '2');
ok(true, 'save then load inside one cell');

var variables = Lang.describeVariables(run('a = load("sales")\nb = 42\nc = [1, 2, 3]', data).scope);
assert.equal(variables.length, 3);
assert.equal(variables[0].type, 'table');
assert.equal(variables[0].summary, '5 rows x 4 columns');
assert.equal(variables[2].type, 'list');
ok(true, 'variable inspection');

/* ------------------------------------------------------------------ Chart */

section('charts');

var chartCell = run('bar(load("sales").groupby("product").agg(total = sum(amount)), x = "product", y = "total", title = "Sales")', data);
assert.ok(chartCell.ok, chartCell.ok ? '' : chartCell.error && chartCell.error.message);
var spec = chartCell.outputs[0];
assert.equal(spec.kind, 'chart');
assert.equal(spec.chart, 'bar');
assert.deepEqual(spec.labels, ['A', 'B']);
assert.deepEqual(spec.series[0].values, [20, 13]);
ok(true, 'chart specification');

['dark', 'light'].forEach(function(theme){
  var rendered = Chart.render(spec, { theme: theme });
  ok(rendered && rendered.svg.indexOf('<svg') === 0, theme + ' svg');
  ok(rendered.svg.indexOf('</svg>') > 0, theme + ' svg closes');
  ok(rendered.svg.indexOf('viewBox') !== -1, theme + ' svg is responsive');
});

var seriesSpec = run(
  'line(load("sales").pivot(index = "year", columns = "product", values = "amount"), x = "year", title = "By product")',
  data
).outputs[0];
var lineSvg = Chart.render(seriesSpec, { theme: 'dark' }).svg;
ok(lineSvg.indexOf('stroke-width="2"') !== -1, 'lines are 2px');
ok(seriesSpec.series.length === 2, 'two series');
ok(lineSvg.indexOf('<rect x=') !== -1, 'legend swatches present for two series');
ok(true, 'multi series line');

var single = Chart.render(spec, { theme: 'dark' }).svg;
var swatchCount = (single.match(/rx="2"/g) || []).length;
assert.equal(swatchCount, 0, 'a single series carries no legend box');
ok(true, 'no legend for one series');

var pieSpec = run('pie(load("sales").groupby("product").agg(total = sum(amount)), labels = "product", values = "total")', data).outputs[0];
var pieRendered = Chart.render(pieSpec, { theme: 'light' });
ok(pieRendered.svg.indexOf('<path') !== -1, 'pie slices');
ok(true, 'pie');

var wide = { chart: 'bar', labels: ['a'], series: [] };
for (var s = 0; s < 12; s += 1) wide.series.push({ name: 'S' + s, values: [s + 1] });
var capped = Chart.render(wide, { theme: 'dark', otherLabel: 'Other' });
ok(capped.svg.indexOf('Other') !== -1, 'past eight series the tail folds into Other');
ok(true, 'series cap');

var tableView = Chart.toTable(spec);
assert.deepEqual(tableView.columns, ['product', 'total']);
assert.deepEqual(tableView.rows, [['A', 20], ['B', 13]]);
ok(true, 'every chart has a table view');

var ticks = Chart.niceTicks(0, 97, 5);
assert.equal(ticks.min, 0);
assert.ok(ticks.max >= 97);
assert.ok(ticks.ticks.every(function(value){ return Number.isFinite(value); }));
ok(true, 'axis ticks are round numbers');

process.stdout.write('\nDS Studio engine: ' + checks + ' checks passed.\n');
