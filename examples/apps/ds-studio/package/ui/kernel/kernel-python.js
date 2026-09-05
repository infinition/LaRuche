/* Experimental Python kernel: CPython compiled to WebAssembly, through Pyodide.
 *
 * Pyodide is NOT downloaded at runtime. The App CSP restricts script-src and
 * connect-src to this package's own versioned asset directory, so a CDN load
 * is blocked; the runtime has to be vendored into ui/vendor/pyodide/ by
 * tools/vendor_pyodide.py before the build. When the manifest written by that
 * script is absent, this kernel reports itself unavailable and the built-in
 * kernel is used instead.
 *
 * It also runs on the main thread. The same CSP sets default-src 'none' with
 * no worker-src, so Worker construction is blocked: the Worker split used by
 * the Obsidian extension cannot be ported as is. Cell execution therefore
 * holds the UI, which is why the notebook runs cells as bounded jobs.
 */
(function(root, factory){
  'use strict';
  var api = factory(
    typeof module === 'object' && module.exports ? require('../lib/frame.js') : root.StudioFrame
  );
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioKernelPython = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(Frames){
  'use strict';

  var VENDOR = './vendor/pyodide/';
  var MANIFEST = VENDOR + 'studio-manifest.json';

  var TABLE_MARK = '__DS_TABLE__';
  var CHART_MARK = '__DS_CHART__';
  var IMAGE_MARK = '__DS_IMAGE__';
  var END_MARK = '__DS_END__';

  /* Helpers injected once, so a cell can use the same vocabulary as the
   * built-in kernel: show(), bar(), line(), pie(), hist(). Matplotlib figures
   * are still captured as PNG, the way the Obsidian plugin does it. */
  var PREAMBLE = [
    'import sys, io, json, base64',
    'import pandas as pd',
    'import numpy as np',
    '',
    '_DS_TABLE = "' + TABLE_MARK + '"',
    '_DS_CHART = "' + CHART_MARK + '"',
    '_DS_IMAGE = "' + IMAGE_MARK + '"',
    '_DS_END = "' + END_MARK + '"',
    '_DS_SOURCES = {}',
    '',
    'def _ds_emit(mark, payload):',
    '    print(mark + json.dumps(payload, default=str) + _DS_END)',
    '',
    'def _ds_clean(value):',
    '    if value is None:',
    '        return None',
    '    if isinstance(value, (np.integer,)):',
    '        return int(value)',
    '    if isinstance(value, (np.floating,)):',
    '        value = float(value)',
    '        return None if (value != value or value in (float("inf"), float("-inf"))) else value',
    '    if isinstance(value, (np.bool_,)):',
    '        return bool(value)',
    '    if isinstance(value, float) and value != value:',
    '        return None',
    '    if isinstance(value, pd.Timestamp):',
    '        return {"__date": value.isoformat()}',
    '    return value',
    '',
    'def load(name):',
    '    if name not in _DS_SOURCES:',
    '        raise KeyError("no dataset named %r (available: %s)" % (name, ", ".join(sorted(_DS_SOURCES)) or "none"))',
    '    return pd.read_csv(io.StringIO(_DS_SOURCES[name]))',
    '',
    'def datasets():',
    '    return pd.DataFrame([{"name": k, "rows": len(v.splitlines()) - 1} for k, v in _DS_SOURCES.items()])',
    '',
    'def show(data, limit=50, title=""):',
    '    if not isinstance(data, pd.DataFrame):',
    '        data = pd.DataFrame(data)',
    '    head = data.head(limit)',
    '    _ds_emit(_DS_TABLE, {',
    '        "title": title,',
    '        "columns": [str(c) for c in head.columns],',
    '        "rows": [[_ds_clean(v) for v in row] for row in head.itertuples(index=False, name=None)],',
    '        "total": int(len(data)),',
    '        "truncated": bool(len(data) > limit),',
    '        "schema": [{"name": str(c), "dtype": _ds_dtype(data[c]), "missing": int(data[c].isna().sum())} for c in data.columns],',
    '    })',
    '',
    'table = show',
    '',
    'def _ds_dtype(series):',
    '    kind = series.dtype.kind',
    '    if kind in "iufc":',
    '        return "number"',
    '    if kind == "b":',
    '        return "boolean"',
    '    if kind == "M":',
    '        return "date"',
    '    return "string"',
    '',
    'def _ds_chart(kind, data, x=None, y=None, series=None, title="", stacked=False, horizontal=False, color=None):',
    '    if not isinstance(data, pd.DataFrame):',
    '        data = pd.DataFrame(data)',
    '    if data.empty:',
    '        raise ValueError("the chart data is empty")',
    '    xcol = x if x is not None else str(data.columns[0])',
    '    if y is None:',
    '        ycols = [str(c) for c in data.columns if str(c) != xcol and data[c].dtype.kind in "iufc"]',
    '    elif isinstance(y, str):',
    '        ycols = [y]',
    '    else:',
    '        ycols = [str(c) for c in y]',
    '    if not ycols:',
    '        raise ValueError("no numeric column to plot")',
    '    if series is not None:',
    '        wide = data.pivot_table(index=xcol, columns=series, values=ycols[0], aggfunc="sum")',
    '        labels = [str(v) for v in wide.index]',
    '        payload = [{"name": str(c), "values": [_ds_clean(v) for v in wide[c]]} for c in wide.columns]',
    '    else:',
    '        labels = [str(v) for v in data[xcol]]',
    '        payload = [{"name": str(c), "values": [_ds_clean(v) for v in data[c]]} for c in ycols]',
    '    _ds_emit(_DS_CHART, {',
    '        "chart": kind, "title": title, "labels": labels, "series": payload,',
    '        "stacked": bool(stacked), "horizontal": bool(horizontal),',
    '        "colors": ([color] if isinstance(color, str) else color),',
    '        "axis": {"x": xcol, "y": ", ".join(ycols)},',
    '    })',
    '',
    'def scatter3d(data, x, y, z, series=None, size=None, label=None, title="", color=None):',
    '    pts = []',
    '    for _, row in data.iterrows():',
    '        try:',
    '            px, py, pz = float(row[x]), float(row[y]), float(row[z])',
    '        except (TypeError, ValueError):',
    '            continue',
    '        if px != px or py != py or pz != pz:',
    '            continue',
    '        pts.append({',
    '            "x": px, "y": py, "z": pz,',
    '            "series": (None if series is None else str(row[series])),',
    '            "size": (None if size is None else float(row[size])),',
    '            "label": (None if label is None else str(row[label])),',
    '        })',
    '    if not pts:',
    '        raise ValueError("no row has all three coordinates")',
    '    _ds_emit(_DS_CHART, {',
    '        "chart": "scatter3d", "title": title, "points": pts,',
    '        "axis": {"x": str(x), "y": str(y), "z": str(z), "series": series},',
    '        "colors": ([color] if isinstance(color, str) else color),',
    '    })',
    '',
    'def bar(data, **kw): _ds_chart("bar", data, **kw)',
    'def line(data, **kw): _ds_chart("line", data, **kw)',
    'def area(data, **kw): _ds_chart("area", data, **kw)',
    'def scatter(data, **kw): _ds_chart("scatter", data, **kw)',
    '',
    'def pie(data, labels=None, values=None, title="", color=None):',
    '    labcol = labels if labels is not None else str(data.columns[0])',
    '    valcol = values if values is not None else [str(c) for c in data.columns if str(c) != labcol][0]',
    '    _ds_emit(_DS_CHART, {',
    '        "chart": "pie", "title": title,',
    '        "labels": [str(v) for v in data[labcol]],',
    '        "series": [{"name": str(valcol), "values": [_ds_clean(v) for v in data[valcol]]}],',
    '        "axis": {"x": str(labcol), "y": str(valcol)},',
    '        "colors": ([color] if isinstance(color, str) else color),',
    '    })',
    '',
    'def hist(data, column=None, bins=12, title="", color=None):',
    '    col = column if column is not None else [str(c) for c in data.columns if data[c].dtype.kind in "iufc"][0]',
    '    counts, edges = np.histogram(data[col].dropna(), bins=bins)',
    '    _ds_emit(_DS_CHART, {',
    '        "chart": "bar", "title": title or str(col),',
    '        "labels": [str(round(float(e), 3)) for e in edges[:-1]],',
    '        "series": [{"name": "count", "values": [int(c) for c in counts]}],',
    '        "axis": {"x": str(col), "y": "count"},',
    '        "colors": ([color] if isinstance(color, str) else color),',
    '    })',
    '',
    'try:',
    '    import matplotlib',
    '    matplotlib.use("Agg")',
    '    import matplotlib.pyplot as plt',
    '    _DS_HAS_PLT = True',
    'except ImportError:',
    '    _DS_HAS_PLT = False',
    '',
    'def _ds_flush_figures():',
    '    if not _DS_HAS_PLT:',
    '        return',
    '    for number in plt.get_fignums():',
    '        figure = plt.figure(number)',
    '        if not figure.get_axes():',
    '            continue',
    '        buffer = io.BytesIO()',
    '        figure.savefig(buffer, format="png", dpi=100, bbox_inches="tight")',
    '        buffer.seek(0)',
    '        print(_DS_IMAGE + base64.b64encode(buffer.read()).decode("ascii") + _DS_END)',
    '    plt.close("all")',
    '',
    'if _DS_HAS_PLT:',
    '    plt.show = _ds_flush_figures'
  ].join('\n');

  function PythonKernel(store) {
    this.id = 'python';
    this.language = 'python';
    this.ready = false;
    this.store = store;
    this.pyodide = null;
    this.manifest = null;
    this.syncedNames = '';
  }

  var SYNTAX_HINT = 'The notebook language is Python, with pandas as pd and numpy as np. ' +
    'load("name") returns a DataFrame and show(df) renders a table. ' +
    'Call kernel.status for the full reference.';

  PythonKernel.prototype.describe = function() {
    var packages = this.manifest ? this.manifest.packages : [];
    return {
      id: this.id,
      language: this.language,
      labelKey: 'kernelPythonLabel',
      versionKey: 'kernelPythonVersion',
      version: this.manifest ? this.manifest.pyodideVersion : '',
      packages: packages,
      ready: this.ready,
      syntaxHint: SYNTAX_HINT,
      reference: {
        language: 'python',
        summary: 'Ordinary CPython running in WebAssembly through Pyodide. pandas is pd and ' +
          'numpy is np. Names persist across cells in the same notebook. The exact ' +
          'interpreter version is the version field of this same reply, not a number ' +
          'remembered from elsewhere: the vendored runtime decides it.',
        loading: 'v = load("dataset_name")   # returns a pandas DataFrame',
        outputs: ['show(df, limit=50, title="")', 'print(value)'],
        charts: [
          'bar(df, x="col", y="col", title="", series="col", stacked=False, horizontal=False, color="#ccff00")',
          'line(df, x="col", y="col", title="")',
          'area(df, x="col", y="col", title="")',
          'scatter(df, x="col", y="col", title="")',
          'pie(df, labels="col", values="col", title="")',
          'scatter3d(df, x="a", y="b", z="c", series="g", size="s", title="", color="#ff7a18")',
          'hist(df, column="col", bins=12, title="")'
        ],
        figures: packages.indexOf('matplotlib') === -1
          ? 'matplotlib is not bundled in this build: use the chart helpers above.'
          : 'matplotlib figures are captured as images when a cell draws one.',
        packages: packages,
        example: 'v = load("ventes")\n' +
          'top = (v[v["annee"] >= 2020]\n' +
          '       .groupby("produit", as_index=False)\n' +
          '       .agg(total=("montant", "sum")))\n' +
          'show(top.sort_values("total", ascending=False).head(10))\n' +
          'bar(top, x="produit", y="total", title="Top produits")',
        notSupported: [
          'installing packages beyond the ones bundled in this build',
          'network access from a cell',
          'file system access outside the runtime',
          'threads and GPU'
        ]
      }
    };
  };

  function loadScript(url) {
    return new Promise(function(resolve, reject){
      var script = document.createElement('script');
      script.src = url;
      script.onload = function(){ resolve(); };
      script.onerror = function(){ reject(new Error('cannot load ' + url)); };
      document.head.appendChild(script);
    });
  }

  PythonKernel.prototype.init = function(onProgress) {
    var self = this;
    var report = onProgress || function(){};

    report(5, 'kernelReadingManifest');
    return fetch(MANIFEST, { credentials: 'omit' }).then(function(response){
      if (!response.ok) throw new Error('runtime manifest missing');
      return response.json();
    }).then(function(manifest){
      self.manifest = manifest;
      report(15, 'kernelLoadingRuntime');
      return loadScript(VENDOR + 'pyodide.js');
    }).then(function(){
      if (typeof window.loadPyodide !== 'function') {
        throw new Error('pyodide.js did not expose loadPyodide');
      }
      report(30, 'kernelStartingInterpreter');
      return window.loadPyodide({ indexURL: VENDOR });
    }).then(function(pyodide){
      self.pyodide = pyodide;
      var packages = (self.manifest && self.manifest.packages) || [];
      if (!packages.length) return null;
      report(55, 'kernelLoadingPackages');
      return pyodide.loadPackage(packages);
    }).then(function(){
      report(85, 'kernelPreparing');
      return self.pyodide.runPythonAsync(PREAMBLE);
    }).then(function(){
      return self.pushDatasets(true);
    }).then(function(){
      self.ready = true;
      report(100, 'kernelReady');
    });
  };

  /* Datasets cross into Python as CSV text, which pandas reads natively and
   * which survives the JavaScript/Python boundary without a proxy object. */
  PythonKernel.prototype.pushDatasets = function(force) {
    if (!this.pyodide) return Promise.resolve();
    var items = this.store.all();
    var names = Object.keys(items).sort().join(',');
    if (!force && names === this.syncedNames) return Promise.resolve();
    this.syncedNames = names;

    var payload = {};
    Object.keys(items).forEach(function(key){
      payload[key] = items[key].frame.toCsv(',');
    });
    this.pyodide.globals.set('_ds_incoming', this.pyodide.toPy(payload));
    return this.pyodide.runPythonAsync(
      '_DS_SOURCES.clear()\n' +
      '_DS_SOURCES.update({k: v for k, v in _ds_incoming.items()})\n' +
      'del _ds_incoming'
    );
  };

  PythonKernel.prototype.execute = function(code, options) {
    var self = this;
    var settings = options || {};
    var outputs = [];
    var buffer = '';

    function emit(output) {
      outputs.push(output);
      if (settings.onOutput) settings.onOutput(output);
    }

    /* stdout arrives in fragments; markers can straddle two writes, so the
     * buffer is only drained once a complete marker is present. */
    function drain(flush) {
      for (;;) {
        var mark = earliestMark(buffer);
        if (!mark) break;
        if (mark.start > 0) {
          emitText(buffer.slice(0, mark.start));
          buffer = buffer.slice(mark.start);
          continue;
        }
        var end = buffer.indexOf(END_MARK, mark.token.length);
        if (end === -1) return;
        var payload = buffer.slice(mark.token.length, end);
        buffer = buffer.slice(end + END_MARK.length);
        decode(mark.token, payload);
      }
      if (flush && buffer.trim()) {
        emitText(buffer);
        buffer = '';
      }
    }

    function earliestMark(text) {
      var best = null;
      [TABLE_MARK, CHART_MARK, IMAGE_MARK].forEach(function(token){
        var index = text.indexOf(token);
        if (index === -1) return;
        if (!best || index < best.start) best = { start: index, token: token };
      });
      return best;
    }

    function emitText(text) {
      var trimmed = text.replace(/\n+$/, '');
      if (trimmed.trim()) emit({ kind: 'stream', stream: 'out', text: trimmed });
    }

    function decode(token, payload) {
      if (token === IMAGE_MARK) {
        emit({ kind: 'image', source: 'data:image/png;base64,' + payload.trim(), title: '' });
        return;
      }
      try {
        var parsed = JSON.parse(payload);
        if (token === TABLE_MARK) {
          emit({
            kind: 'table',
            title: parsed.title || '',
            columns: parsed.columns || [],
            schema: parsed.schema || [],
            rows: parsed.rows || [],
            total: parsed.total || 0,
            truncated: !!parsed.truncated
          });
        } else {
          emit({
            kind: 'chart',
            chart: parsed.chart || 'bar',
            colors: parsed.colors || null,
            points: parsed.points || null,
            title: parsed.title || '',
            labels: parsed.labels || [],
            series: parsed.series || [],
            axis: parsed.axis || null,
            stacked: !!parsed.stacked,
            horizontal: !!parsed.horizontal
          });
        }
      } catch (error) {
        emitText(payload);
      }
    }

    if (!this.ready) {
      return Promise.resolve({
        ok: false,
        outputs: [],
        error: { message: 'the Python kernel is not ready', line: 0, column: 0, hint: '' }
      });
    }

    return this.pushDatasets(false).then(function(){
      self.pyodide.setStdout({ batched: function(text){ buffer += text + '\n'; drain(false); } });
      self.pyodide.setStderr({ batched: function(text){ emit({ kind: 'stream', stream: 'err', text: text }); } });
      return self.pyodide.runPythonAsync(code);
    }).then(function(){
      return self.pyodide.runPythonAsync('_ds_flush_figures()');
    }).then(function(){
      drain(true);
      return { ok: true, outputs: outputs, error: null };
    }).catch(function(error){
      drain(true);
      return { ok: false, outputs: outputs, error: describeError(error) };
    });
  };

  /* Pyodide reports a full Python traceback; the last line is the useful one
   * and the "line N" frame points back into the cell. */
  function describeError(error) {
    var text = String((error && error.message) || error);
    var lines = text.trim().split('\n');
    var message = lines[lines.length - 1] || text;
    var line = 0;
    var match = /File "<exec>", line (\d+)/.exec(text);
    if (match) line = Number(match[1]);
    return {
      message: message,
      line: line,
      column: 0,
      hint: lines.length > 2 ? lines.slice(0, -1).join('\n').slice(-600) : '',
      cancelled: false,
      timeout: false
    };
  }

  PythonKernel.prototype.variables = function() {
    if (!this.ready) return [];
    try {
      var json = this.pyodide.runPython(
        'json.dumps([\n' +
        '  {"name": k,\n' +
        '   "type": type(v).__name__,\n' +
        '   "summary": ("%d rows x %d columns" % v.shape) if isinstance(v, pd.DataFrame)\n' +
        '              else ("%d values" % len(v)) if isinstance(v, (pd.Series, list, dict))\n' +
        '              else str(v)[:80]}\n' +
        '  for k, v in list(globals().items())\n' +
        '  if not k.startswith("_")\n' +
        '  and isinstance(v, (int, float, str, bool, list, dict, pd.DataFrame, pd.Series))\n' +
        '])'
      );
      return JSON.parse(json);
    } catch (error) {
      return [];
    }
  };

  PythonKernel.prototype.setVariable = function(name, value) {
    if (!this.ready) return false;
    this.pyodide.globals.set(name, this.pyodide.toPy(value));
    return true;
  };

  PythonKernel.prototype.deleteVariable = function(name) {
    if (!this.ready) return false;
    try {
      this.pyodide.runPython('del globals()[' + JSON.stringify(name) + ']');
      return true;
    } catch (error) {
      return false;
    }
  };

  PythonKernel.prototype.reset = function() {
    var self = this;
    if (!this.ready) return Promise.resolve();
    return this.pyodide.runPythonAsync(
      'for _k in [k for k in list(globals()) if not k.startswith("_") and k not in ' +
      '("sys","io","json","base64","pd","np","plt","matplotlib","show","table","bar","line",' +
      '"area","scatter","pie","hist","load","datasets")]:\n' +
      '    del globals()[_k]\n'
    ).then(function(){
      self.syncedNames = '';
      return self.pushDatasets(true);
    });
  };

  PythonKernel.prototype.interrupt = function() {
    /* Pyodide interruption needs a SharedArrayBuffer and cross-origin
     * isolation, which the App sandbox does not provide. */
    return Promise.resolve();
  };

  PythonKernel.prototype.completions = function() {
    return ['load', 'datasets', 'show', 'table', 'bar', 'line', 'area', 'scatter', 'pie', 'hist', 'pd', 'np'];
  };

  /* Probes for the vendored runtime without loading it. */
  function available() {
    if (typeof fetch !== 'function' || typeof document === 'undefined') return Promise.resolve(false);
    return fetch(MANIFEST, { credentials: 'omit' })
      .then(function(response){ return response.ok; })
      .catch(function(){ return false; });
  }

  return Object.freeze({
    create: function(store){ return new PythonKernel(store); },
    available: available,
    manifestUrl: MANIFEST,
    preamble: PREAMBLE
  });
});
