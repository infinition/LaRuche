'use strict';
/* Checks that no user-facing string is hardcoded and that both catalogues
 * cover every key the interface asks for.
 * Run with: node apps-library/ds-studio/test/i18n.test.js */

var assert = require('node:assert/strict');
var fs = require('node:fs');
var path = require('node:path');

var UI = path.join(__dirname, '..', 'package', 'ui');
var LOCALES = path.join(UI, 'locales');

var available = fs.readdirSync(LOCALES).filter(function(name){ return /\.json$/.test(name); });
assert.ok(available.length >= 2, 'at least two catalogues ship');

var catalogs = {};
available.forEach(function(file){
  var tag = file.replace(/\.json$/, '');
  catalogs[tag] = JSON.parse(fs.readFileSync(path.join(LOCALES, file), 'utf8'));
});

var reference = catalogs.fr;
var tags = Object.keys(catalogs);

/* Every catalogue holds exactly the same keys. */
tags.forEach(function(tag){
  var missing = Object.keys(reference).filter(function(key){
    return !Object.prototype.hasOwnProperty.call(catalogs[tag], key);
  });
  var extra = Object.keys(catalogs[tag]).filter(function(key){
    return !Object.prototype.hasOwnProperty.call(reference, key);
  });
  assert.deepEqual(missing, [], tag + ' is missing keys: ' + missing.join(', '));
  assert.deepEqual(extra, [], tag + ' has keys fr does not: ' + extra.join(', '));
});

/* Placeholders must match across catalogues, or an interpolation goes blank. */
Object.keys(reference).forEach(function(key){
  var expected = placeholders(reference[key]);
  tags.forEach(function(tag){
    var actual = placeholders(catalogs[tag][key]);
    assert.deepEqual(actual, expected, 'placeholders differ for "' + key + '" in ' + tag +
      ' (' + actual.join(',') + ' vs ' + expected.join(',') + ')');
  });
});

function placeholders(value) {
  return (String(value).match(/\{(\w+)\}/g) || []).sort();
}

/* Every key the markup and the controller reference must exist. */
var html = fs.readFileSync(path.join(UI, 'index.html'), 'utf8');
var app = fs.readFileSync(path.join(UI, 'app.js'), 'utf8');
var kernelJs = fs.readFileSync(path.join(UI, 'kernel', 'kernel-js.js'), 'utf8');
var kernelPython = fs.readFileSync(path.join(UI, 'kernel', 'kernel-python.js'), 'utf8');

var used = [];
var htmlKeys = html.match(/data-i18n(?:-[a-z-]+)?="([^"]+)"/g) || [];
htmlKeys.forEach(function(match){
  used.push(/="([^"]+)"/.exec(match)[1]);
});

var appKeys = app.match(/\bt\('([A-Za-z0-9_]+)'/g) || [];
appKeys.forEach(function(match){
  used.push(/'([A-Za-z0-9_]+)'/.exec(match)[1]);
});

[kernelJs, kernelPython].forEach(function(source){
  (source.match(/labelKey: '([A-Za-z0-9_]+)'|versionKey: '([A-Za-z0-9_]+)'/g) || []).forEach(function(match){
    used.push(/'([A-Za-z0-9_]+)'/.exec(match)[1]);
  });
  (source.match(/onProgress\([0-9]+, '([A-Za-z0-9_]+)'\)|report\([0-9]+, '([A-Za-z0-9_]+)'\)/g) || []).forEach(function(match){
    used.push(/'([A-Za-z0-9_]+)'/.exec(match)[1]);
  });
});

/* Keys assembled at runtime from a suffix. */
['dtype_number', 'dtype_string', 'dtype_boolean', 'dtype_date', 'dtype_mixed', 'dtype_empty',
 'sampleCellCode_studio', 'sampleCellCode_python',
 'unitByte', 'unitKilobyte', 'unitMegabyte'].forEach(function(key){ used.push(key); });

var unique = used.filter(function(key, index){
  /* A trailing underscore is a prefix built at runtime, such as
   * t('dtype_' + schema.dtype); the concrete keys are listed above. */
  return used.indexOf(key) === index && !/_$/.test(key);
});
var unknown = unique.filter(function(key){
  if (Object.prototype.hasOwnProperty.call(reference, key)) return false;
  /* A plural key lives in the catalogue as key_one and key_other. */
  return !Object.keys(reference).some(function(candidate){
    return candidate.indexOf(key + '_') === 0;
  });
});

assert.deepEqual(unknown, [], 'keys used but never translated: ' + unknown.join(', '));

/* Nothing user-facing may be written directly into the markup or the logic. */
var bodyOnly = html.slice(html.indexOf('<body'));
var suspicious = [];
(bodyOnly.match(/>[^<>{}]{4,}</g) || []).forEach(function(fragment){
  var text = fragment.slice(1, -1).trim();
  if (!text || /^[\s0-9.,:%+-]*$/.test(text)) return;
  suspicious.push(text);
});
/* Placeholder text in the markup is allowed only where data-i18n replaces it. */
var unguarded = suspicious.filter(function(text){
  var index = bodyOnly.indexOf('>' + text);
  var opening = bodyOnly.lastIndexOf('<', index);
  return bodyOnly.slice(opening, index).indexOf('data-i18n') === -1;
});
assert.deepEqual(unguarded, [], 'untranslated markup text: ' + unguarded.join(' | '));

process.stdout.write('DS Studio i18n: ' + tags.length + ' catalogues, ' +
  Object.keys(reference).length + ' keys, ' + unique.length + ' referenced. All checks passed.\n');
