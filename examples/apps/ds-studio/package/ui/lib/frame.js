/* Columnar table engine for the DS Studio notebook.
 *
 * Pure data structure, no DOM and no host SDK, so the same file runs under
 * Node for the test suite and inside the App sandbox. Columns are stored as
 * parallel arrays: aggregation walks one array instead of one object per row.
 */
(function(root, factory){
  'use strict';
  var api = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioFrame = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(){
  'use strict';

  var MISSING = null;
  var SEPARATOR = String.fromCharCode(31);

  function FrameError(message) {
    var error = new Error(message);
    error.name = 'FrameError';
    return error;
  }

  function isMissing(value) {
    return value === null || value === undefined || (typeof value === 'number' && isNaN(value));
  }

  function isNumeric(value) {
    return typeof value === 'number' && isFinite(value);
  }

  function uniqueNames(names) {
    var seen = Object.create(null);
    return names.map(function(name){
      var base = String(name);
      if (!seen[base]) { seen[base] = 1; return base; }
      var next = base + '_' + seen[base];
      seen[base] += 1;
      while (seen[next]) { next = base + '_' + (seen[base]++); }
      seen[next] = 1;
      return next;
    });
  }

  /* ---------------------------------------------------------------- Frame */

  function Frame(columns, data, length) {
    this.columns = columns;
    this.data = data;
    this.length = length;
  }

  Frame.prototype.has = function(name) {
    return Object.prototype.hasOwnProperty.call(this.data, name);
  };

  Frame.prototype.require = function(name) {
    if (!this.has(name)) {
      throw FrameError('unknown column "' + name + '" (available: ' + this.columns.join(', ') + ')');
    }
    return this.data[name];
  };

  Frame.prototype.col = function(name) {
    return this.require(name).slice();
  };

  Frame.prototype.row = function(index) {
    var out = {};
    for (var i = 0; i < this.columns.length; i += 1) {
      var name = this.columns[i];
      out[name] = this.data[name][index];
    }
    return out;
  };

  Frame.prototype.rows = function(limit) {
    var count = limit == null ? this.length : Math.min(limit, this.length);
    var out = new Array(count);
    for (var i = 0; i < count; i += 1) out[i] = this.row(i);
    return out;
  };

  Frame.prototype.dtype = function(name) {
    var values = this.require(name);
    var kinds = Object.create(null);
    var seen = 0;
    for (var i = 0; i < values.length; i += 1) {
      var value = values[i];
      if (isMissing(value)) continue;
      seen += 1;
      var kind = value instanceof Date ? 'date' : typeof value;
      kinds[kind] = true;
      if (seen > 500) break;
    }
    var found = Object.keys(kinds);
    if (found.length === 0) return 'empty';
    if (found.length === 1) return found[0] === 'object' ? 'mixed' : found[0];
    return 'mixed';
  };

  Frame.prototype.schema = function() {
    var self = this;
    return this.columns.map(function(name){
      return { name: name, dtype: self.dtype(name), missing: self.missingCount(name) };
    });
  };

  Frame.prototype.missingCount = function(name) {
    var values = this.require(name);
    var total = 0;
    for (var i = 0; i < values.length; i += 1) if (isMissing(values[i])) total += 1;
    return total;
  };

  /* ------------------------------------------------------ Reshaping verbs */

  Frame.prototype.select = function(names) {
    var self = this;
    var picked = names.map(String);
    var data = {};
    picked.forEach(function(name){ data[name] = self.require(name).slice(); });
    return new Frame(picked, data, this.length);
  };

  Frame.prototype.drop = function(names) {
    var removed = {};
    names.forEach(function(name){ removed[String(name)] = true; });
    return this.select(this.columns.filter(function(name){ return !removed[name]; }));
  };

  Frame.prototype.rename = function(mapping) {
    var self = this;
    var columns = [];
    var data = {};
    this.columns.forEach(function(name){
      var next = Object.prototype.hasOwnProperty.call(mapping, name) ? String(mapping[name]) : name;
      columns.push(next);
      data[next] = self.data[name].slice();
    });
    return new Frame(uniqueNames(columns), data, this.length);
  };

  Frame.prototype.take = function(indices) {
    var self = this;
    var data = {};
    this.columns.forEach(function(name){
      var source = self.data[name];
      var target = new Array(indices.length);
      for (var i = 0; i < indices.length; i += 1) target[i] = source[indices[i]];
      data[name] = target;
    });
    return new Frame(this.columns.slice(), data, indices.length);
  };

  Frame.prototype.filter = function(predicate) {
    var keep = [];
    for (var i = 0; i < this.length; i += 1) {
      if (predicate(this.row(i), i)) keep.push(i);
    }
    return this.take(keep);
  };

  Frame.prototype.head = function(n) {
    var count = Math.max(0, Math.min(n == null ? 10 : n, this.length));
    var indices = new Array(count);
    for (var i = 0; i < count; i += 1) indices[i] = i;
    return this.take(indices);
  };

  Frame.prototype.tail = function(n) {
    var count = Math.max(0, Math.min(n == null ? 10 : n, this.length));
    var start = this.length - count;
    var indices = new Array(count);
    for (var i = 0; i < count; i += 1) indices[i] = start + i;
    return this.take(indices);
  };

  Frame.prototype.slice = function(start, end) {
    var from = Math.max(0, start == null ? 0 : start);
    var to = Math.min(this.length, end == null ? this.length : end);
    var indices = [];
    for (var i = from; i < to; i += 1) indices.push(i);
    return this.take(indices);
  };

  function compareValues(a, b) {
    var aMissing = isMissing(a);
    var bMissing = isMissing(b);
    if (aMissing && bMissing) return 0;
    if (aMissing) return 1;
    if (bMissing) return -1;
    if (a instanceof Date) a = a.getTime();
    if (b instanceof Date) b = b.getTime();
    if (typeof a === 'number' && typeof b === 'number') return a - b;
    if (typeof a === 'boolean' && typeof b === 'boolean') return (a ? 1 : 0) - (b ? 1 : 0);
    var left = String(a);
    var right = String(b);
    return left < right ? -1 : left > right ? 1 : 0;
  }

  Frame.prototype.sort = function(names, descending) {
    var self = this;
    var keys = (Array.isArray(names) ? names : [names]).map(String);
    keys.forEach(function(name){ self.require(name); });
    var order = new Array(this.length);
    for (var i = 0; i < this.length; i += 1) order[i] = i;
    order.sort(function(left, right){
      for (var k = 0; k < keys.length; k += 1) {
        var column = self.data[keys[k]];
        var result = compareValues(column[left], column[right]);
        if (result !== 0) return descending ? -result : result;
      }
      return left - right;
    });
    return this.take(order);
  };

  Frame.prototype.sortByKey = function(keyOf, descending) {
    var order = new Array(this.length);
    var keys = new Array(this.length);
    for (var i = 0; i < this.length; i += 1) {
      order[i] = i;
      keys[i] = keyOf(this.row(i), i);
    }
    order.sort(function(left, right){
      var result = compareValues(keys[left], keys[right]);
      if (result !== 0) return descending ? -result : result;
      return left - right;
    });
    return this.take(order);
  };

  Frame.prototype.withColumn = function(name, compute) {
    var key = String(name);
    var values = new Array(this.length);
    for (var i = 0; i < this.length; i += 1) values[i] = compute(this.row(i), i);
    var columns = this.columns.slice();
    if (columns.indexOf(key) === -1) columns.push(key);
    var data = {};
    var self = this;
    columns.forEach(function(column){
      data[column] = column === key ? values : self.data[column].slice();
    });
    return new Frame(columns, data, this.length);
  };

  Frame.prototype.distinct = function(names) {
    var self = this;
    var keys = names && names.length ? names.map(String) : this.columns.slice();
    keys.forEach(function(name){ self.require(name); });
    var seen = Object.create(null);
    var keep = [];
    for (var i = 0; i < this.length; i += 1) {
      var signature = keys.map(function(name){ return keyOfValue(self.data[name][i]); }).join(SEPARATOR);
      if (!seen[signature]) { seen[signature] = true; keep.push(i); }
    }
    return this.take(keep);
  };

  Frame.prototype.dropna = function(names) {
    var self = this;
    var keys = names && names.length ? names.map(String) : this.columns.slice();
    keys.forEach(function(name){ self.require(name); });
    var keep = [];
    for (var i = 0; i < this.length; i += 1) {
      var complete = true;
      for (var k = 0; k < keys.length; k += 1) {
        if (isMissing(self.data[keys[k]][i])) { complete = false; break; }
      }
      if (complete) keep.push(i);
    }
    return this.take(keep);
  };

  Frame.prototype.fillna = function(replacement, names) {
    var self = this;
    var target = Object.create(null);
    (names && names.length ? names.map(String) : this.columns).forEach(function(name){
      self.require(name);
      target[name] = true;
    });
    var data = {};
    this.columns.forEach(function(name){
      var source = self.data[name];
      data[name] = target[name]
        ? source.map(function(value){ return isMissing(value) ? replacement : value; })
        : source.slice();
    });
    return new Frame(this.columns.slice(), data, this.length);
  };

  Frame.prototype.concat = function(other) {
    var columns = this.columns.slice();
    other.columns.forEach(function(name){ if (columns.indexOf(name) === -1) columns.push(name); });
    var self = this;
    var data = {};
    columns.forEach(function(name){
      var left = self.has(name) ? self.data[name] : new Array(self.length).fill(MISSING);
      var right = other.has(name) ? other.data[name] : new Array(other.length).fill(MISSING);
      data[name] = left.concat(right);
    });
    return new Frame(columns, data, this.length + other.length);
  };

  /* ---------------------------------------------------------- Aggregation */

  /* Group keys are concatenated one signature per column, so each signature
   * carries its own length: without it ("a", "b") and ("ab", "") collide. */
  function keyOfValue(value) {
    if (value === null || value === undefined) return 'n:0:';
    var text = value instanceof Date ? String(value.getTime()) : String(value);
    var kind = value instanceof Date ? 'd'
      : typeof value === 'number' ? 'f'
      : typeof value === 'boolean' ? 'b'
      : 's';
    return kind + ':' + text.length + ':' + text;
  }

  var AGGREGATIONS = {
    count: function(values){ return values.length; },
    countValid: function(values){
      var total = 0;
      for (var i = 0; i < values.length; i += 1) if (!isMissing(values[i])) total += 1;
      return total;
    },
    countDistinct: function(values){
      var seen = Object.create(null);
      var total = 0;
      for (var i = 0; i < values.length; i += 1) {
        var key = keyOfValue(values[i]);
        if (!seen[key]) { seen[key] = true; total += 1; }
      }
      return total;
    },
    sum: function(values){
      var total = 0;
      var seen = false;
      for (var i = 0; i < values.length; i += 1) {
        if (isNumeric(values[i])) { total += values[i]; seen = true; }
      }
      return seen ? total : MISSING;
    },
    mean: function(values){
      var total = 0;
      var count = 0;
      for (var i = 0; i < values.length; i += 1) {
        if (isNumeric(values[i])) { total += values[i]; count += 1; }
      }
      return count ? total / count : MISSING;
    },
    min: function(values){
      var best = MISSING;
      for (var i = 0; i < values.length; i += 1) {
        if (isMissing(values[i])) continue;
        if (isMissing(best) || compareValues(values[i], best) < 0) best = values[i];
      }
      return best;
    },
    max: function(values){
      var best = MISSING;
      for (var i = 0; i < values.length; i += 1) {
        if (isMissing(values[i])) continue;
        if (isMissing(best) || compareValues(values[i], best) > 0) best = values[i];
      }
      return best;
    },
    median: function(values){
      var numbers = values.filter(isNumeric).sort(function(a, b){ return a - b; });
      if (!numbers.length) return MISSING;
      var middle = Math.floor(numbers.length / 2);
      return numbers.length % 2 ? numbers[middle] : (numbers[middle - 1] + numbers[middle]) / 2;
    },
    quantile: function(values, ratio){
      var numbers = values.filter(isNumeric).sort(function(a, b){ return a - b; });
      if (!numbers.length) return MISSING;
      var position = (numbers.length - 1) * Math.max(0, Math.min(1, ratio == null ? 0.5 : ratio));
      var lower = Math.floor(position);
      var upper = Math.ceil(position);
      if (lower === upper) return numbers[lower];
      return numbers[lower] + (numbers[upper] - numbers[lower]) * (position - lower);
    },
    variance: function(values){
      var numbers = values.filter(isNumeric);
      if (numbers.length < 2) return MISSING;
      var mean = AGGREGATIONS.mean(numbers);
      var total = 0;
      for (var i = 0; i < numbers.length; i += 1) total += (numbers[i] - mean) * (numbers[i] - mean);
      return total / (numbers.length - 1);
    },
    std: function(values){
      var variance = AGGREGATIONS.variance(values);
      return isMissing(variance) ? MISSING : Math.sqrt(variance);
    },
    first: function(values){ return values.length ? values[0] : MISSING; },
    last: function(values){ return values.length ? values[values.length - 1] : MISSING; },
    mode: function(values){
      var counts = Object.create(null);
      var best = MISSING;
      var bestCount = 0;
      for (var i = 0; i < values.length; i += 1) {
        if (isMissing(values[i])) continue;
        var key = keyOfValue(values[i]);
        counts[key] = (counts[key] || 0) + 1;
        if (counts[key] > bestCount) { bestCount = counts[key]; best = values[i]; }
      }
      return best;
    }
  };

  function Grouped(frame, keys, groups) {
    this.frame = frame;
    this.keys = keys;
    this.groups = groups;
  }

  Grouped.prototype.size = function() {
    return this.agg([{ out: 'count', fn: 'count', values: null }]);
  };

  /* Each spec is {out, fn, values} where `values` maps a group's row indices
   * to the array the aggregation consumes. A null `values` counts rows. */
  Grouped.prototype.agg = function(specs) {
    var self = this;
    var columns = this.keys.slice();
    var data = {};
    columns.forEach(function(name){ data[name] = []; });
    specs.forEach(function(spec){
      if (columns.indexOf(spec.out) !== -1) {
        throw FrameError('duplicate output column "' + spec.out + '" in agg');
      }
      columns.push(spec.out);
      data[spec.out] = [];
    });

    this.groups.forEach(function(group){
      for (var k = 0; k < self.keys.length; k += 1) {
        data[self.keys[k]].push(group.key[k]);
      }
      specs.forEach(function(spec){
        var reduce = AGGREGATIONS[spec.fn];
        if (!reduce) throw FrameError('unknown aggregation "' + spec.fn + '"');
        var values = spec.values ? spec.values(group.indices) : group.indices;
        data[spec.out].push(reduce(values, spec.arg));
      });
    });

    return new Frame(columns, data, this.groups.length);
  };

  Frame.prototype.groupby = function(names) {
    var self = this;
    var keys = (Array.isArray(names) ? names : [names]).map(String);
    keys.forEach(function(name){ self.require(name); });
    var index = Object.create(null);
    var groups = [];
    for (var i = 0; i < this.length; i += 1) {
      var key = new Array(keys.length);
      var signature = '';
      for (var k = 0; k < keys.length; k += 1) {
        key[k] = this.data[keys[k]][i];
        signature += keyOfValue(key[k]) + SEPARATOR;
      }
      var group = index[signature];
      if (!group) {
        group = { key: key, indices: [] };
        index[signature] = group;
        groups.push(group);
      }
      group.indices.push(i);
    }
    return new Grouped(this, keys, groups);
  };

  Frame.prototype.aggregate = function(name, fn, arg) {
    var reduce = AGGREGATIONS[fn];
    if (!reduce) throw FrameError('unknown aggregation "' + fn + '"');
    return reduce(this.require(name).slice(), arg);
  };

  Frame.prototype.valueCounts = function(name, descending) {
    var values = this.require(name);
    var index = Object.create(null);
    var order = [];
    for (var i = 0; i < values.length; i += 1) {
      var key = keyOfValue(values[i]);
      if (!index[key]) { index[key] = { value: values[i], count: 0 }; order.push(index[key]); }
      index[key].count += 1;
    }
    var sorted = order.slice().sort(function(a, b){
      return descending === false ? a.count - b.count : b.count - a.count;
    });
    var label = name === 'count' ? 'value' : name;
    var data = {};
    data[label] = sorted.map(function(entry){ return entry.value; });
    data.count = sorted.map(function(entry){ return entry.count; });
    return new Frame([label, 'count'], data, sorted.length);
  };

  Frame.prototype.unique = function(name) {
    var values = this.require(name);
    var seen = Object.create(null);
    var out = [];
    for (var i = 0; i < values.length; i += 1) {
      var key = keyOfValue(values[i]);
      if (!seen[key]) { seen[key] = true; out.push(values[i]); }
    }
    return out;
  };

  Frame.prototype.pivot = function(indexName, columnName, valueName, fn) {
    var self = this;
    this.require(indexName);
    this.require(columnName);
    this.require(valueName);
    var reduce = AGGREGATIONS[fn || 'sum'];
    if (!reduce) throw FrameError('unknown aggregation "' + fn + '"');

    var rowOrder = [];
    var rowIndex = Object.create(null);
    var columnOrder = [];
    var columnIndex = Object.create(null);
    var buckets = Object.create(null);

    for (var i = 0; i < this.length; i += 1) {
      var rowKey = this.data[indexName][i];
      var colKey = this.data[columnName][i];
      var rowSignature = keyOfValue(rowKey);
      var colSignature = keyOfValue(colKey);
      if (!rowIndex[rowSignature]) { rowIndex[rowSignature] = { value: rowKey }; rowOrder.push(rowSignature); }
      if (!columnIndex[colSignature]) { columnIndex[colSignature] = { value: colKey }; columnOrder.push(colSignature); }
      var cell = rowSignature + SEPARATOR + colSignature;
      if (!buckets[cell]) buckets[cell] = [];
      buckets[cell].push(this.data[valueName][i]);
    }

    columnOrder.sort(function(a, b){ return compareValues(columnIndex[a].value, columnIndex[b].value); });

    var headers = columnOrder.map(function(signature){ return String(columnIndex[signature].value); });
    var columns = [indexName].concat(uniqueNames(headers));
    var data = {};
    columns.forEach(function(name){ data[name] = []; });

    rowOrder.forEach(function(rowSignature){
      data[indexName].push(rowIndex[rowSignature].value);
      columnOrder.forEach(function(colSignature, position){
        var bucket = buckets[rowSignature + SEPARATOR + colSignature];
        data[columns[position + 1]].push(bucket ? reduce(bucket) : MISSING);
      });
    });

    return new Frame(columns, data, rowOrder.length);
  };

  Frame.prototype.join = function(other, on, how) {
    var self = this;
    var keys = (Array.isArray(on) ? on : [on]).map(String);
    keys.forEach(function(name){ self.require(name); other.require(name); });
    var mode = how || 'inner';
    if (['inner', 'left', 'right', 'outer'].indexOf(mode) === -1) {
      throw FrameError('unknown join type "' + mode + '" (inner, left, right, outer)');
    }
    if (mode === 'right') return other.join(this, keys, 'left');

    var rightIndex = Object.create(null);
    for (var r = 0; r < other.length; r += 1) {
      var signature = keys.map(function(name){ return keyOfValue(other.data[name][r]); }).join(SEPARATOR);
      if (!rightIndex[signature]) rightIndex[signature] = [];
      rightIndex[signature].push(r);
    }

    var rightColumns = other.columns.filter(function(name){ return keys.indexOf(name) === -1; });
    var suffixed = rightColumns.map(function(name){
      return self.has(name) ? name + '_right' : name;
    });
    var columns = this.columns.concat(uniqueNames(suffixed));
    var data = {};
    columns.forEach(function(name){ data[name] = []; });

    var matchedRight = Object.create(null);
    for (var i = 0; i < this.length; i += 1) {
      var key = keys.map(function(name){ return keyOfValue(self.data[name][i]); }).join(SEPARATOR);
      var matches = rightIndex[key];
      if (matches && matches.length) {
        matches.forEach(function(rightRow){
          matchedRight[rightRow] = true;
          self.columns.forEach(function(name){ data[name].push(self.data[name][i]); });
          rightColumns.forEach(function(name, position){
            data[columns[self.columns.length + position]].push(other.data[name][rightRow]);
          });
        });
      } else if (mode === 'left' || mode === 'outer') {
        self.columns.forEach(function(name){ data[name].push(self.data[name][i]); });
        rightColumns.forEach(function(name, position){
          data[columns[self.columns.length + position]].push(MISSING);
        });
      }
    }

    if (mode === 'outer') {
      for (var right = 0; right < other.length; right += 1) {
        if (matchedRight[right]) continue;
        self.columns.forEach(function(name){
          data[name].push(keys.indexOf(name) !== -1 ? other.data[name][right] : MISSING);
        });
        rightColumns.forEach(function(name, position){
          data[columns[self.columns.length + position]].push(other.data[name][right]);
        });
      }
    }

    return new Frame(columns, data, data[columns[0]].length);
  };

  Frame.prototype.describe = function() {
    var self = this;
    var stats = ['count', 'missing', 'mean', 'std', 'min', 'q25', 'median', 'q75', 'max'];
    var columns = ['column', 'dtype'].concat(stats);
    var data = {};
    columns.forEach(function(name){ data[name] = []; });

    this.columns.forEach(function(name){
      var values = self.data[name];
      var dtype = self.dtype(name);
      data.column.push(name);
      data.dtype.push(dtype);
      data.count.push(AGGREGATIONS.countValid(values));
      data.missing.push(self.missingCount(name));
      if (dtype === 'number') {
        data.mean.push(AGGREGATIONS.mean(values));
        data.std.push(AGGREGATIONS.std(values));
        data.min.push(AGGREGATIONS.min(values));
        data.q25.push(AGGREGATIONS.quantile(values, 0.25));
        data.median.push(AGGREGATIONS.median(values));
        data.q75.push(AGGREGATIONS.quantile(values, 0.75));
        data.max.push(AGGREGATIONS.max(values));
      } else {
        data.mean.push(MISSING);
        data.std.push(MISSING);
        data.min.push(MISSING);
        data.q25.push(MISSING);
        data.median.push(AGGREGATIONS.countDistinct(values));
        data.q75.push(MISSING);
        data.max.push(AGGREGATIONS.mode(values));
      }
    });

    return new Frame(columns, data, this.columns.length);
  };

  Frame.prototype.histogram = function(name, bins) {
    var values = this.require(name).filter(isNumeric);
    if (!values.length) throw FrameError('column "' + name + '" holds no numeric value');
    var count = Math.max(1, Math.min(bins == null ? 12 : Math.round(bins), 200));
    var min = Math.min.apply(null, values);
    var max = Math.max.apply(null, values);
    if (min === max) { max = min + 1; }
    var width = (max - min) / count;
    var labels = new Array(count);
    var counts = new Array(count).fill(0);
    for (var b = 0; b < count; b += 1) {
      labels[b] = min + b * width;
    }
    for (var i = 0; i < values.length; i += 1) {
      var slot = Math.min(count - 1, Math.floor((values[i] - min) / width));
      counts[slot] += 1;
    }
    return new Frame(['bin', 'count'], { bin: labels, count: counts }, count);
  };

  Frame.prototype.toRows = function(limit) {
    return this.rows(limit);
  };

  Frame.prototype.toCsv = function(delimiter) {
    var separator = delimiter || ',';
    var self = this;
    var lines = [this.columns.map(function(name){ return escapeCsv(name, separator); }).join(separator)];
    for (var i = 0; i < this.length; i += 1) {
      lines.push(this.columns.map(function(name){
        return escapeCsv(self.data[name][i], separator);
      }).join(separator));
    }
    return lines.join('\n');
  };

  function escapeCsv(value, separator) {
    if (isMissing(value)) return '';
    var text = value instanceof Date ? value.toISOString() : String(value);
    if (text.indexOf(separator) !== -1 || text.indexOf('"') !== -1 || /[\r\n]/.test(text)) {
      return '"' + text.replace(/"/g, '""') + '"';
    }
    return text;
  }

  /* ------------------------------------------------------- Constructors */

  function fromColumns(source, order) {
    var columns = order ? order.map(String) : Object.keys(source);
    var length = columns.length ? source[columns[0]].length : 0;
    var data = {};
    columns.forEach(function(name){
      var values = source[name];
      if (!Array.isArray(values)) throw FrameError('column "' + name + '" is not an array');
      if (values.length !== length) {
        throw FrameError('column "' + name + '" has ' + values.length + ' values, expected ' + length);
      }
      data[name] = values.slice();
    });
    return new Frame(columns, data, length);
  }

  function fromRows(rows, order) {
    if (!Array.isArray(rows)) throw FrameError('expected an array of rows');
    var columns = order ? order.map(String) : [];
    if (!columns.length) {
      var seen = Object.create(null);
      rows.forEach(function(row){
        Object.keys(row || {}).forEach(function(name){
          if (!seen[name]) { seen[name] = true; columns.push(name); }
        });
      });
    }
    var data = {};
    columns.forEach(function(name){ data[name] = new Array(rows.length); });
    for (var i = 0; i < rows.length; i += 1) {
      var row = rows[i] || {};
      for (var c = 0; c < columns.length; c += 1) {
        var value = row[columns[c]];
        data[columns[c]][i] = value === undefined ? MISSING : value;
      }
    }
    return new Frame(columns, data, rows.length);
  }

  function empty() {
    return new Frame([], {}, 0);
  }

  return Object.freeze({
    Frame: Frame,
    Grouped: Grouped,
    fromRows: fromRows,
    fromColumns: fromColumns,
    empty: empty,
    aggregations: AGGREGATIONS,
    isMissing: isMissing,
    isNumeric: isNumeric,
    compareValues: compareValues,
    error: FrameError
  });
});
