'use strict';
/* The Python preamble is a JavaScript array of source lines, and nothing else
 * checks it. A broken concatenation or a lost definition only shows when the
 * kernel boots in a browser, long after the commit, as a traceback nobody links
 * back to this file.
 *
 * Run with: node examples/apps/ds-studio/test/preamble.test.js */

var assert = require('node:assert/strict');
var fs = require('node:fs');
var path = require('node:path');

var source = fs.readFileSync(
  path.join(__dirname, '..', 'package', 'ui', 'kernel', 'kernel-python.js'),
  'utf8'
);

var start = source.indexOf('var PREAMBLE = [');
var end = source.indexOf("].join('\\n');", start);
assert.notEqual(start, -1, 'PREAMBLE not found');
assert.notEqual(end, -1, 'end of PREAMBLE not found');

/* The marks are concatenated into the array, so evaluating it needs them in
 * scope. Reading them from the file rather than repeating them here keeps the
 * test honest: a renamed mark must not silently pass. */
function mark(name) {
  var found = new RegExp('var ' + name + " = '([^']+)'").exec(source);
  assert.ok(found, name + ' is not declared in the kernel');
  return found[1];
}
var TABLE_MARK = mark('TABLE_MARK');
var CHART_MARK = mark('CHART_MARK');
var IMAGE_MARK = mark('IMAGE_MARK');
var PLOTLY_MARK = mark('PLOTLY_MARK');
var END_MARK = mark('END_MARK');

var lines = eval(source.slice(start + 'var PREAMBLE = '.length, end + 1));
assert.ok(Array.isArray(lines), 'PREAMBLE is not an array');
assert.ok(lines.length > 100, 'PREAMBLE looks truncated: ' + lines.length + ' lines');
assert.ok(
  lines.every(function (line) { return typeof line === 'string'; }),
  'every PREAMBLE entry must be a string'
);

var code = lines.join('\n');

/* Each entry is a capability the notebook is documented as having. */
[
  'import pandas as pd',
  'def load(name):',
  'def show(data, limit=50, title=""):',
  'def _ds_flush_figures():',
  'import seaborn as sns',
  'import plotly.io as pio',
  'def _ds_plotly(fig):',
  'import pyodide_http',
  'class _DSFiles:',
  'class _DSLaruche:',
  'laruche = _DSLaruche()'
].forEach(function (fragment) {
  assert.ok(code.indexOf(fragment) !== -1, 'missing from the preamble: ' + fragment);
});

/* The marks must survive concatenation: a broken one turns a table into raw
 * text in the output stream, which reads as the cell having printed garbage. */
[TABLE_MARK, CHART_MARK, IMAGE_MARK, PLOTLY_MARK, END_MARK].forEach(function (token) {
  assert.ok(code.indexOf(token) !== -1, 'mark lost in the preamble: ' + token);
});

/* Indentation is the whole syntax in Python: a line that starts with a space
 * outside any block would be a fatal error at kernel start. */
var opener = /(^|\n)(def |class |if |try:|except|for |while |with )/;
assert.ok(opener.test(code), 'no Python block found, the preamble is probably empty');
code.split('\n').forEach(function (line, index) {
  assert.ok(
    line.indexOf('\t') === -1,
    'tab on line ' + (index + 1) + ': Python mixes tabs and spaces badly'
  );
});

/* Every scanned mark must also be decoded, or the payload reaches the notebook
 * as text. */
['TABLE_MARK', 'CHART_MARK', 'IMAGE_MARK', 'PLOTLY_MARK'].forEach(function (name) {
  var scanner = /\[([^\]]*)\]\.forEach\(function\(token\)\{/.exec(source);
  assert.ok(scanner, 'the mark scanner was not found');
  assert.ok(scanner[1].indexOf(name) !== -1, name + ' is not scanned in the output stream');
});

console.log(
  'DS Studio preamble: ' + lines.length + ' lines, ' + code.length +
  ' characters. All checks passed.'
);
