/* Delimited text reader for the DS Studio notebook.
 *
 * Sniffs the separator, honours RFC 4180 quoting, then infers one type per
 * column. Files exported from a French locale use ";" with a comma decimal
 * mark, so the numeric pass is delimiter-aware instead of assuming a dot.
 */
(function(root, factory){
  'use strict';
  var api = factory(
    typeof module === 'object' && module.exports ? require('./frame.js') : root.StudioFrame
  );
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioCsv = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(Frames){
  'use strict';

  var DELIMITERS = [',', ';', '\t', '|'];
  var TRUE_WORDS = ['true', 'vrai', 'yes', 'oui', 'y', 'o'];
  var FALSE_WORDS = ['false', 'faux', 'no', 'non', 'n'];
  var ISO_DATE = /^\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?)?$/;
  var EURO_DATE = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})$/;

  function CsvError(message) {
    var error = new Error(message);
    error.name = 'CsvError';
    return error;
  }

  function stripBom(text) {
    return text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  }

  /* Counts a candidate delimiter only outside quoted spans, so a separator
   * that appears inside a quoted field cannot win the vote. */
  function scoreDelimiter(text, delimiter) {
    var counts = [];
    var current = 0;
    var quoted = false;
    var lines = 0;
    for (var i = 0; i < text.length && lines < 20; i += 1) {
      var char = text[i];
      if (quoted) {
        if (char === '"') {
          if (text[i + 1] === '"') { i += 1; } else { quoted = false; }
        }
        continue;
      }
      if (char === '"') { quoted = true; continue; }
      if (char === delimiter) { current += 1; continue; }
      if (char === '\n') {
        counts.push(current);
        current = 0;
        lines += 1;
      }
    }
    if (current > 0 || counts.length === 0) counts.push(current);
    var populated = counts.filter(function(value){ return value > 0; });
    if (!populated.length) return { fields: 0, stable: false };
    var first = populated[0];
    var stable = populated.every(function(value){ return value === first; });
    return { fields: first, stable: stable };
  }

  function sniffDelimiter(text) {
    var best = null;
    DELIMITERS.forEach(function(delimiter){
      var score = scoreDelimiter(text, delimiter);
      if (!score.fields) return;
      if (!best) { best = { delimiter: delimiter, score: score }; return; }
      if (score.stable && !best.score.stable) { best = { delimiter: delimiter, score: score }; return; }
      if (score.stable === best.score.stable && score.fields > best.score.fields) {
        best = { delimiter: delimiter, score: score };
      }
    });
    return best ? best.delimiter : ',';
  }

  function splitRecords(text, delimiter) {
    var records = [];
    var field = '';
    var record = [];
    var quoted = false;
    var i = 0;

    while (i < text.length) {
      var char = text[i];
      if (quoted) {
        if (char === '"') {
          if (text[i + 1] === '"') { field += '"'; i += 2; continue; }
          quoted = false;
          i += 1;
          continue;
        }
        field += char;
        i += 1;
        continue;
      }
      if (char === '"' && field === '') { quoted = true; i += 1; continue; }
      if (char === delimiter) { record.push(field); field = ''; i += 1; continue; }
      if (char === '\r') { i += 1; continue; }
      if (char === '\n') {
        record.push(field);
        records.push(record);
        record = [];
        field = '';
        i += 1;
        continue;
      }
      field += char;
      i += 1;
    }

    if (quoted) throw CsvError('unterminated quoted field');
    if (field !== '' || record.length) {
      record.push(field);
      records.push(record);
    }
    return records.filter(function(entry){
      return entry.length > 1 || (entry.length === 1 && entry[0].trim() !== '');
    });
  }

  function parseNumber(text, commaDecimal) {
    var cleaned = text.trim();
    if (!cleaned) return null;
    if (commaDecimal) {
      cleaned = cleaned.replace(/ |\s/g, '').replace(/\./g, '').replace(',', '.');
    } else {
      cleaned = cleaned.replace(/ |\s/g, '').replace(/,/g, '');
    }
    if (!/^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i.test(cleaned)) return null;
    var value = Number(cleaned);
    return isFinite(value) ? value : null;
  }

  function parseDate(text) {
    var cleaned = text.trim();
    if (ISO_DATE.test(cleaned)) {
      var iso = new Date(cleaned.length === 10 ? cleaned + 'T00:00:00Z' : cleaned);
      return isNaN(iso.getTime()) ? null : iso;
    }
    var euro = EURO_DATE.exec(cleaned);
    if (euro) {
      var day = Number(euro[1]);
      var month = Number(euro[2]);
      if (month > 12 || day > 31 || month < 1 || day < 1) return null;
      var value = new Date(Date.UTC(Number(euro[3]), month - 1, day));
      return isNaN(value.getTime()) ? null : value;
    }
    return null;
  }

  function inferColumn(values, commaDecimal) {
    var filled = values.filter(function(value){ return value.trim() !== ''; });
    if (!filled.length) {
      return values.map(function(){ return null; });
    }

    var allNumbers = filled.every(function(value){ return parseNumber(value, commaDecimal) !== null; });
    if (allNumbers) {
      return values.map(function(value){
        return value.trim() === '' ? null : parseNumber(value, commaDecimal);
      });
    }

    var allBooleans = filled.every(function(value){
      var lower = value.trim().toLowerCase();
      return TRUE_WORDS.indexOf(lower) !== -1 || FALSE_WORDS.indexOf(lower) !== -1;
    });
    if (allBooleans) {
      return values.map(function(value){
        var lower = value.trim().toLowerCase();
        if (!lower) return null;
        return TRUE_WORDS.indexOf(lower) !== -1;
      });
    }

    var allDates = filled.every(function(value){ return parseDate(value) !== null; });
    if (allDates) {
      return values.map(function(value){
        return value.trim() === '' ? null : parseDate(value);
      });
    }

    return values.map(function(value){
      var trimmed = value.trim();
      return trimmed === '' ? null : value;
    });
  }

  function headerLooksLikeData(header, commaDecimal) {
    var numeric = header.filter(function(name){ return parseNumber(name, commaDecimal) !== null; });
    return header.length > 1 && numeric.length === header.length;
  }

  function normaliseHeader(header) {
    var seen = Object.create(null);
    return header.map(function(name, position){
      var base = String(name || '').trim() || ('column_' + (position + 1));
      if (!seen[base]) { seen[base] = 1; return base; }
      var next = base + '_' + seen[base];
      seen[base] += 1;
      return next;
    });
  }

  /* Returns a Frame. `options.delimiter` and `options.header` override the
   * sniffing pass when the caller already knows the shape of the file. */
  function parse(text, options) {
    var settings = options || {};
    var source = stripBom(String(text || ''));
    if (!source.trim()) throw CsvError('the file is empty');

    var delimiter = settings.delimiter || sniffDelimiter(source);
    var commaDecimal = settings.commaDecimal;
    if (commaDecimal === undefined) commaDecimal = delimiter === ';';

    var records = splitRecords(source, delimiter);
    if (!records.length) throw CsvError('no record found');

    var width = 0;
    records.forEach(function(record){ width = Math.max(width, record.length); });

    var hasHeader = settings.header;
    if (hasHeader === undefined) hasHeader = !headerLooksLikeData(records[0], commaDecimal);

    var header = hasHeader
      ? normaliseHeader(records[0])
      : normaliseHeader(new Array(width).fill(''));
    while (header.length < width) header.push('column_' + (header.length + 1));

    var body = hasHeader ? records.slice(1) : records;
    var raw = {};
    header.forEach(function(name){ raw[name] = new Array(body.length); });
    for (var i = 0; i < body.length; i += 1) {
      for (var c = 0; c < header.length; c += 1) {
        var cell = body[i][c];
        raw[header[c]][i] = cell === undefined ? '' : cell;
      }
    }

    var data = {};
    header.forEach(function(name){
      data[name] = inferColumn(raw[name], commaDecimal);
    });

    return Frames.fromColumns(data, header);
  }

  function exportPage(frame, name, offset, limit, delimiter) {
    var start = Math.min(frame.length, Math.max(0, offset || 0));
    var count = Math.min(frame.length - start, Math.max(1, Math.min(limit || 500, 2000)));
    function page(size) {
      var next = start + size;
      return {name:name,offset:start,rows:size,total:frame.length,
        csv:frame.slice(start,next).toCsv(delimiter || ','),
        nextOffset:next < frame.length ? next : null,truncated:next < frame.length};
    }
    function fits(value) { return new TextEncoder().encode(JSON.stringify(value)).length <= 48000; }
    var low = 0, high = count, best = null;
    while (low <= high) {
      var mid = Math.floor((low + high) / 2), candidate = page(mid);
      if (fits(candidate)) { best = candidate; low = mid + 1; } else high = mid - 1;
    }
    if (!best || (count > 0 && best.rows === 0)) throw CsvError('A single CSV row/header exceeds the export byte budget. Select fewer columns or export from the UI.');
    return best;
  }

  return Object.freeze({
    parse: parse,
    exportPage: exportPage,
    sniffDelimiter: sniffDelimiter,
    splitRecords: splitRecords,
    parseNumber: parseNumber,
    parseDate: parseDate,
    error: CsvError
  });
});
