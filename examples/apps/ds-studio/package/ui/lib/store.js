/* Dataset registry shared by the notebook, the kernels and the agent actions.
 *
 * Datasets live in memory for the session. Private storage holds 1 MiB per App
 * and user in total, so only datasets under the configured budget are written
 * back; the rest stay session-only and say so in the interface. Nothing is put
 * in localStorage: the App iframe has an opaque origin and browser storage
 * throws there.
 */
(function(root, factory){
  'use strict';
  var api = factory(
    typeof module === 'object' && module.exports ? require('./frame.js') : root.StudioFrame,
    typeof module === 'object' && module.exports ? require('./csv.js') : root.StudioCsv
  );
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioStore = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(Frames, Csv){
  'use strict';

  var MAX_ROWS = 200000;
  var MAX_COLUMNS = 512;
  var NAME_PATTERN = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;

  function StoreError(message, code) {
    var error = new Error(message);
    error.name = 'StoreError';
    error.code = code || 'invalid';
    return error;
  }

  /* Combining diacritics, built without escapes to keep the file plain ASCII. */
  var COMBINING = new RegExp('[' + String.fromCharCode(0x300) + '-' + String.fromCharCode(0x36f) + ']', 'g');

  /* Dataset names double as identifiers inside cell code, so they are
   * restricted to what the language can actually reference: "Ventes 2024.csv"
   * becomes "Ventes_2024". */
  function normaliseName(input) {
    var base = String(input || '').trim();
    if (!base) throw StoreError('a dataset name is required', 'name');
    var cleaned = base.replace(/\.[A-Za-z0-9]+$/, '');
    if (typeof cleaned.normalize === 'function') {
      cleaned = cleaned.normalize('NFD').replace(COMBINING, '');
    }
    cleaned = cleaned.replace(/[^A-Za-z0-9_]+/g, '_').replace(/^_+|_+$/g, '');
    if (!cleaned) cleaned = 'dataset';
    if (/^[0-9]/.test(cleaned)) cleaned = 'd_' + cleaned;
    cleaned = cleaned.slice(0, 64);
    if (!NAME_PATTERN.test(cleaned)) throw StoreError('invalid dataset name', 'name');
    return cleaned;
  }

  function Store(options) {
    var settings = options || {};
    this.items = Object.create(null);
    this.order = [];
    this.persistBudget = settings.persistBudget || 24 * 1024;
    this.maxRows = settings.maxRows || MAX_ROWS;
  }

  Store.prototype.uniqueName = function(name) {
    var base = normaliseName(name);
    if (!this.items[base]) return base;
    var index = 2;
    function candidate() { var suffix = '_' + index; return base.slice(0, 64 - suffix.length) + suffix; }
    while (this.items[candidate()]) index += 1;
    return candidate();
  };

  Store.prototype.add = function(name, frame, source) {
    if (!(frame instanceof Frames.Frame)) throw StoreError('expected a table', 'type');
    if (frame.length > this.maxRows) {
      throw StoreError('this dataset has ' + frame.length + ' rows, the limit is ' + this.maxRows, 'size');
    }
    if (frame.columns.length > MAX_COLUMNS) {
      throw StoreError('this dataset has too many columns', 'size');
    }
    var key = normaliseName(name);
    if (this.order.indexOf(key) === -1) this.order.push(key);
    this.items[key] = {
      frame: frame,
      source: source || 'memory',
      importedAt: Date.now(),
      bytes: estimateBytes(frame)
    };
    return key;
  };

  Store.prototype.addCsv = function(name, text, options) {
    return this.add(this.uniqueName(name), Csv.parse(text, options), (options && options.source) || 'file');
  };

  Store.prototype.addRows = function(name, rows, source) {
    return this.add(this.uniqueName(name), Frames.fromRows(rows), source || 'agent');
  };

  Store.prototype.remove = function(name) {
    var key = String(name);
    if (!this.items[key]) return false;
    delete this.items[key];
    this.order = this.order.filter(function(entry){ return entry !== key; });
    return true;
  };

  Store.prototype.get = function(name) {
    return this.items[String(name)] || null;
  };

  Store.prototype.all = function() {
    return this.items;
  };

  /* A cell can create datasets through save(); this folds them back in. */
  Store.prototype.sync = function(datasets) {
    var self = this;
    if (!datasets) return;
    Object.keys(datasets).forEach(function(key){
      if (self.items[key] === datasets[key]) return;
      var entry = datasets[key];
      if (!entry || !(entry.frame instanceof Frames.Frame)) return;
      if (self.order.indexOf(key) === -1) self.order.push(key);
      self.items[key] = {
        frame: entry.frame,
        source: entry.source || 'cell',
        importedAt: entry.importedAt || Date.now(),
        bytes: estimateBytes(entry.frame)
      };
    });
  };

  Store.prototype.list = function() {
    var self = this;
    return this.order.filter(function(key){ return !!self.items[key]; }).map(function(key){
      var entry = self.items[key];
      return {
        name: key,
        rows: entry.frame.length,
        columns: entry.frame.columns.length,
        schema: entry.frame.schema(),
        source: entry.source,
        bytes: entry.bytes,
        persistable: entry.bytes <= self.persistBudget
      };
    });
  };

  Store.prototype.bytes = function() {
    var self = this;
    return this.order.reduce(function(total, key){
      return total + (self.items[key] ? self.items[key].bytes : 0);
    }, 0);
  };

  /* Only datasets that fit the budget are serialised. The caller reports which
   * ones were left behind rather than silently dropping them. */
  Store.prototype.serialize = function() {
    var self = this;
    var kept = [];
    var skipped = [];
    this.order.forEach(function(key){
      var entry = self.items[key];
      if (!entry) return;
      if (entry.bytes > self.persistBudget) {
        skipped.push({ name: key, bytes: entry.bytes });
        return;
      }
      kept.push({
        name: key,
        source: entry.source,
        importedAt: entry.importedAt,
        columns: entry.frame.columns,
        rows: entry.frame.rows().map(function(row){
          return entry.frame.columns.map(function(column){ return encodeValue(row[column]); });
        })
      });
    });
    return { datasets: kept, skipped: skipped };
  };

  Store.prototype.restore = function(payload) {
    var self = this;
    if (!payload || !Array.isArray(payload.datasets)) return 0;
    var restored = 0;
    payload.datasets.forEach(function(entry){
      if (!entry || !Array.isArray(entry.columns) || !Array.isArray(entry.rows)) return;
      try {
        var data = {};
        entry.columns.forEach(function(column, index){
          data[column] = entry.rows.map(function(row){ return decodeValue(row[index]); });
        });
        var frame = Frames.fromColumns(data, entry.columns);
        self.add(entry.name, frame, entry.source || 'restored');
        if (self.items[entry.name]) self.items[entry.name].importedAt = entry.importedAt || Date.now();
        restored += 1;
      } catch (error) {
        /* A dataset that no longer decodes is skipped, not fatal. */
      }
    });
    return restored;
  };

  function encodeValue(value) {
    if (value instanceof Date) return { d: value.toISOString() };
    if (value === undefined) return null;
    if (typeof value === 'number' && !isFinite(value)) return null;
    return value;
  }

  function decodeValue(value) {
    if (value && typeof value === 'object' && typeof value.d === 'string') {
      var parsed = new Date(value.d);
      return isNaN(parsed.getTime()) ? null : parsed;
    }
    return value === undefined ? null : value;
  }

  /* Cheap size estimate, used for the quota gauge and the persist decision. */
  function estimateBytes(frame) {
    var total = 0;
    frame.columns.forEach(function(name){
      total += name.length + 4;
      var values = frame.data[name];
      var sampleSize = Math.min(values.length, 200);
      var sampled = 0;
      for (var i = 0; i < sampleSize; i += 1) {
        sampled += sizeOf(values[i]);
      }
      var average = sampleSize ? sampled / sampleSize : 0;
      total += Math.round(average * values.length);
    });
    return total;
  }

  function sizeOf(value) {
    if (value === null || value === undefined) return 2;
    if (typeof value === 'number') return 8;
    if (typeof value === 'boolean') return 2;
    if (value instanceof Date) return 26;
    return new TextEncoder().encode(JSON.stringify(String(value))).length + 1;
  }

  return Object.freeze({
    create: function(options){ return new Store(options); },
    normaliseName: normaliseName,
    estimateBytes: estimateBytes,
    error: StoreError,
    maxRows: MAX_ROWS
  });
});
