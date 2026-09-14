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

  /* Deux facons d'obtenir un interpreteur, et l'ordre compte.
   *
   * Vendorise: le runtime voyage dans le paquet, l'App n'a besoin d'aucun
   * reseau, et c'est toujours le meilleur choix quand ca rentre. Mais l'archive
   * plafonne a 32 Mio compresses, et le coeur seul en pese une dizaine: numpy et
   * pandas passent encore, matplotlib deja plus.
   *
   * CDN: ce que fait le plugin Obsidian. Il faut alors que le paquet declare
   * `network.fetch` et ses hotes, et que l'utilisateur les ait accordes, sinon
   * la CSP du bac a sable bloque le script et on le dit clairement. */
  var CDN = 'https://cdn.jsdelivr.net/pyodide/v0.26.4/full/';
  var PLOTLY_JS = 'https://cdn.jsdelivr.net/npm/plotly.js-dist-min@2.35.2/plotly.min.js';

  /* Les roues livrees avec Pyodide, chargees d'un bloc. Celles qui n'y sont pas
   * passent par micropip juste apres, en pur Python. */
  var ROUES = ['numpy', 'pandas', 'matplotlib', 'scikit-learn', 'micropip', 'pyodide-http'];
  var VIA_MICROPIP = ['seaborn', 'plotly'];

  var TABLE_MARK = '__DS_TABLE__';
  var CHART_MARK = '__DS_CHART__';
  var IMAGE_MARK = '__DS_IMAGE__';
  var PLOTLY_MARK = '__DS_PLOTLY__';
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
    '_DS_PLOTLY = "' + PLOTLY_MARK + '"',
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
    '# Seul du parfaitement hexadecimal atteint le SVG: le langage studio',
    '# refusait deja le reste, un noyau Python ecrit du Python ordinaire',
    '# et na pas ce filtre. La garde lui manquait.',
    'def _ds_hex(text):',
    '    if len(text) not in (4, 7) or text[0] != "#":',
    '        return False',
    '    return all(c in "0123456789abcdefABCDEF" for c in text[1:])',
    '',
    'def _ds_colors(color):',
    '    if color is None:',
    '        return None',
    '    entries = [color] if isinstance(color, str) else list(color)',
    '    out = []',
    '    for entry in entries:',
    '        text = str(entry).strip()',
    '        if not _ds_hex(text):',
    '            raise ValueError(',
    '                \'color must be a hex value such as \"#ccff00\", got \"%s\"\' % text)',
    '        out.append(text)',
    '    return out',
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
    '        "colors": _ds_colors(color),',
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
    '        "colors": _ds_colors(color),',
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
    '        "colors": _ds_colors(color),',
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
    '        "colors": _ds_colors(color),',
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
    '    plt.show = _ds_flush_figures',
    '',
    'try:',
    '    import seaborn as sns',
    '    _DS_HAS_SNS = True',
    'except ImportError:',
    '    _DS_HAS_SNS = False',
    '',
    'try:',
    '    import plotly.io as pio',
    '    import plotly.graph_objects as go',
    '    import plotly.express as px',
    '    _DS_HAS_PLOTLY = True',
    'except ImportError:',
    '    _DS_HAS_PLOTLY = False',
    '',
    'def _ds_plotly(fig):',
    '    if not _DS_HAS_PLOTLY:',
    '        raise RuntimeError("plotly is not available in this kernel")',
    '    print(_DS_PLOTLY + fig.to_json() + _DS_END)',
    '',
    'if _DS_HAS_PLOTLY:',
    '    go.Figure.show = lambda self, *a, **k: _ds_plotly(self)',
    '    pio.show = lambda fig, *a, **k: _ds_plotly(fig)',
    '',
    '',
    'class _DSFiles:',
    '    """Files of this App, under its own folder and nowhere else.',
    '',
    '    Every call is a coroutine: the write really crosses to the node, which',
    '    checks the path and the quota, and pretending otherwise would let a cell',
    '    carry on as though a file existed before it did.',
    '    """',
    '    def _pont(self):',
    '        import js',
    '        pont = getattr(js, "__dsFiles", None)',
    '        if pont is None:',
    '            raise RuntimeError(',
    '                "File access is not available. Grant this App the laruche.files "',
    '                "capability in its Permissions panel."',
    '            )',
    '        return pont',
    '    async def read(self, path):',
    '        return await self._pont()("read", {"path": path})',
    '    async def write(self, path, content):',
    '        return await self._pont()("write", {"path": path, "content": str(content), "append": False})',
    '    async def append(self, path, content):',
    '        return await self._pont()("write", {"path": path, "content": str(content), "append": True})',
    '    async def delete(self, path):',
    '        return await self._pont()("delete", {"path": path})',
    '    async def exists(self, path):',
    '        return bool(await self._pont()("exists", {"path": path}))',
    '    async def mkdir(self, path):',
    '        return await self._pont()("mkdir", {"path": path})',
    '    async def list(self, path=""):',
    '        entrees = await self._pont()("list", {"path": path})',
    '        return [e.to_py() if hasattr(e, "to_py") else e for e in entrees]',
    '    async def read_csv(self, path):',
    '        return pd.read_csv(io.StringIO(await self.read(path)))',
    '    async def write_csv(self, path, frame, index=False):',
    '        return await self.write(path, frame.to_csv(index=index))',
    '    async def read_json(self, path):',
    '        return json.loads(await self.read(path))',
    '    async def write_json(self, path, value, indent=2):',
    '        return await self.write(path, json.dumps(value, indent=indent, default=str))',
    '',
    'class _DSNotebook:',
    '    """The notebook this cell runs in: its datasets and its cells.',
    '',
    '    No capability guards this one. The kernel runs inside the App, so saving',
    '    a dataset is the App writing its own state, and a checkbox there would',
    '    have nothing to refuse.',
    '    """',
    '    def _pont(self):',
    '        import js',
    '        pont = getattr(js, "__dsNotebook", None)',
    '        if pont is None:',
    '            raise RuntimeError("The notebook bridge is not available in this view.")',
    '        return pont',
    '    async def datasets(self):',
    '        noms = await self._pont()("datasets", {})',
    '        return list(noms)',
    '    async def save(self, name, frame, delimiter=","):',
    '        texte = frame.to_csv(index=False) if hasattr(frame, "to_csv") else str(frame)',
    '        return await self._pont()("save", {"name": name, "text": texte, "delimiter": delimiter})',
    '    async def remove(self, name):',
    '        return await self._pont()("remove", {"name": name})',
    '    async def cell(self, source, type="code"):',
    '        return await self._pont()("cell", {"source": source, "type": type})',
    '    async def state(self):',
    '        etat = await self._pont()("state", {})',
    '        return etat.to_py() if hasattr(etat, "to_py") else etat',
    '',
    'class _DSMemory:',
    '    """LaRuche\'s memory, which does not belong to this App.',
    '',
    '    write() proposes: the fact lands in the review queue and a human decides.',
    '    write_now() goes straight in. The difference matters because a fact',
    '    written here is read months later by something that never saw this',
    '    notebook, and a wrong one is worse than a missing one.',
    '    """',
    '    def _pont(self):',
    '        import js',
    '        pont = getattr(js, "__dsMemory", None)',
    '        if pont is None:',
    '            raise RuntimeError(',
    '                "Memory access is not available. Grant this App the laruche.memory "',
    '                "capability in its Permissions panel."',
    '            )',
    '        return pont',
    '    async def search(self, query, limit=8):',
    '        resultat = await self._pont()("search", {"query": query, "limit": limit})',
    '        return resultat.to_py() if hasattr(resultat, "to_py") else resultat',
    '    async def read(self, node_id):',
    '        resultat = await self._pont()("read", {"nodeId": node_id})',
    '        return resultat.to_py() if hasattr(resultat, "to_py") else resultat',
    '    async def list(self):',
    '        resultat = await self._pont()("list", {})',
    '        return resultat.to_py() if hasattr(resultat, "to_py") else resultat',
    '    async def write(self, node_id, content, tags=None):',
    '        return await self._pont()("propose", {"nodeId": node_id, "content": str(content), "tags": tags or []})',
    '    async def write_now(self, node_id, content, tags=None):',
    '        return await self._pont()("write", {"nodeId": node_id, "content": str(content), "tags": tags or []})',
    '',
    'class _DSLaruche:',
    '    def __init__(self):',
    '        self.files = _DSFiles()',
    '        self.memory = _DSMemory()',
    '        self.notebook = _DSNotebook()',
    '    def datasets(self):',
    '        return sorted(_DS_SOURCES)',
    '    def load(self, name):',
    '        return load(name)',
    '',
    'laruche = _DSLaruche()',
    'sys.modules["laruche"] = laruche',
    '',
    'try:',
    '    import pyodide_http',
    '    pyodide_http.patch_all()',
    '    _DS_HAS_HTTP = True',
    'except Exception:',
    '    _DS_HAS_HTTP = False'
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

  /* Le runtime vendorise s'il existe, le CDN sinon.
   *
   * L'absence du manifeste n'est pas une erreur: c'est l'etat normal d'un paquet
   * qui n'a pas ete vendorise, et il reste alors le chemin reseau. Ce qui est une
   * erreur, c'est de n'avoir ni l'un ni l'autre, et le message le dit avec le nom
   * exact de la permission a accorder plutot qu'un echec de chargement de script
   * que personne ne saurait interpreter. */
  PythonKernel.prototype.init = function(onProgress) {
    var self = this;
    var report = onProgress || function(){};

    report(5, 'kernelReadingManifest');
    return fetch(MANIFEST, { credentials: 'omit' }).then(function(response){
      return response.ok ? response.json() : null;
    }).catch(function(){
      return null;
    }).then(function(manifest){
      self.manifest = manifest;
      self.source = manifest ? 'vendored' : 'cdn';
      var base = manifest ? VENDOR : CDN;
      report(15, 'kernelLoadingRuntime');
      return loadScript(base + 'pyodide.js').catch(function(erreur){
        if (self.source === 'cdn') {
          throw new Error(
            'The Python runtime is neither bundled with this App nor reachable. ' +
            'Grant this App the network.fetch permission so it can load Pyodide from ' +
            'cdn.jsdelivr.net, or vendor the runtime with tools/vendor_pyodide.py and ' +
            'rebuild the package.'
          );
        }
        throw erreur;
      });
    }).then(function(){
      if (typeof window.loadPyodide !== 'function') {
        throw new Error('pyodide.js did not expose loadPyodide');
      }
      report(30, 'kernelStartingInterpreter');
      return window.loadPyodide({ indexURL: self.manifest ? VENDOR : CDN });
    }).then(function(pyodide){
      self.pyodide = pyodide;
      var packages = self.manifest ? (self.manifest.packages || []) : ROUES;
      self.packages = packages.slice();
      if (!packages.length) return null;
      report(45, 'kernelLoadingPackages');
      return pyodide.loadPackage(packages);
    }).then(function(){
      // Pur Python, absent des roues livrees avec Pyodide. Chaque echec est
      // garde et rapporte: un carnet sans seaborn reste utilisable, un carnet
      // qui croit l'avoir ne l'est pas.
      if (self.source !== 'cdn') return null;
      report(65, 'kernelInstallingPackages');
      return self.installer(VIA_MICROPIP);
    }).then(function(){
      report(80, 'kernelPreparing');
      return self.pyodide.runPythonAsync(PREAMBLE);
    }).then(function(){
      // Plotly rend cote JavaScript: sans sa bibliotheque, une figure emise
      // n'aurait rien pour la dessiner. Son absence n'empeche pas le reste.
      if (self.source !== 'cdn') return null;
      report(90, 'kernelLoadingRuntime');
      return loadScript(PLOTLY_JS).then(function(){
        self.plotly = typeof window.Plotly !== 'undefined';
      }).catch(function(){
        self.plotly = false;
      });
    }).then(function(){
      /* Ce que le noyau vient de se donner: le preambule, les modules
       * importes, l'objet du pont. C'est exactement ce qu'un reinitialisation
       * doit rendre, et c'est pour cela qu'on le releve au lieu de le decrire.
       *
       * La liste etait tenue a la main, et elle avait derive de ce que le
       * preambule definit: changer de carnet effacait scatter3d, seaborn,
       * plotly, pyodide_http, micropip et l'objet laruche, c'est-a-dire
       * l'acces aux fichiers et a la memoire que le guide promet. */
      var releve = self.pyodide.runPython(
        '_DS_BASE = frozenset(globals())\n' +
        'json.dumps(sorted(n for n in _DS_BASE if not n.startswith("_")))'
      );
      try { self.baseNames = JSON.parse(releve); } catch (erreur) { self.baseNames = null; }
      return self.pushDatasets(true);
    }).then(function(){
      self.ready = true;
      report(100, 'kernelReady');
    });
  };

  /* Installe des paquets par micropip, et rend ce qui a echoue.
   *
   * Sert au demarrage pour seaborn et plotly, et a la demande depuis l'onglet
   * Packages ou depuis une action d'agent. Un nom refuse n'interrompt jamais la
   * serie: on installe ce qui peut l'etre et on nomme le reste. */
  PythonKernel.prototype.installer = function(noms) {
    var self = this;
    if (!this.pyodide || !noms || !noms.length) return Promise.resolve({ installed: [], failed: [] });
    var restants = noms.slice();
    var installes = [];
    var echecs = [];
    function suivant() {
      if (!restants.length) {
        self.installed = (self.installed || []).concat(installes);
        return Promise.resolve({ installed: installes, failed: echecs });
      }
      var nom = restants.shift();
      return self.pyodide
        .runPythonAsync('import micropip\nawait micropip.install("' + String(nom).replace(/"/g, '') + '")')
        .then(function(){
          installes.push(nom);
        })
        .catch(function(erreur){
          echecs.push({ name: nom, error: String((erreur && erreur.message) || erreur).slice(0, 400) });
        })
        .then(suivant);
    }
    return suivant();
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
      [TABLE_MARK, CHART_MARK, IMAGE_MARK, PLOTLY_MARK].forEach(function(token){
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
      if (token === PLOTLY_MARK) {
        try {
          var figure = JSON.parse(payload);
          emit({
            kind: 'plotly',
            data: figure.data || [],
            layout: figure.layout || {},
            title: (figure.layout && figure.layout.title && figure.layout.title.text) || ''
          });
        } catch (erreur) {
          emit({ kind: 'text', text: 'plotly figure could not be read: ' + erreur.message });
        }
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
      /* Sans instantane on n'efface rien: rendre un noyau amoindri est pire
       * que de laisser des variables en place. */
      '_ds_base = globals().get("_DS_BASE")\n' +
      'if _ds_base:\n' +
      '    for _k in [k for k in list(globals()) if not k.startswith("_") and k not in _ds_base]:\n' +
      '        del globals()[_k]\n'
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
    return this.baseNames ||
      ['load', 'datasets', 'show', 'table', 'bar', 'line', 'area', 'scatter', 'scatter3d',
        'pie', 'hist', 'pd', 'np'];
  };

  /* Y a-t-il un interpreteur a notre portee, sans le charger.
   *
   * Deux reponses possibles et il faut les essayer dans cet ordre, parce que le
   * runtime vendorise ne coute rien et que le CDN coute un aller-retour.
   *
   * Cette fonction n'a longtemps regarde que le paquet. Le chemin CDN existait
   * dans init(), et init() n'est appelee que sur un noyau Python: comme rien
   * n'etait vendorise, available() repondait faux, le noyau Python n'etait
   * jamais cree, et son chemin reseau restait mort. L'App tournait en langage
   * maison avec network.fetch accordee et personne ne comprenait pourquoi.
   *
   * Pour le CDN c'est la CSP qui repond a notre place: sans la capacite, la
   * requete est bloquee et la promesse est rejetee. On ne teste donc pas la
   * permission, on teste ce qu'elle permet, ce qui ne peut pas se desynchroniser
   * d'elle. */
  function available() {
    if (typeof fetch !== 'function' || typeof document === 'undefined') return Promise.resolve(false);
    return fetch(MANIFEST, { credentials: 'omit' })
      .then(function(response){ return response.ok; })
      .catch(function(){ return false; })
      .then(function(vendorise){
        if (vendorise) return true;
        return fetch(CDN + 'pyodide.js', { method: 'HEAD', mode: 'cors', credentials: 'omit' })
          .then(function(response){ return response.ok; })
          .catch(function(){ return false; });
      });
  }

  return Object.freeze({
    create: function(store){ return new PythonKernel(store); },
    available: available,
    manifestUrl: MANIFEST,
    preamble: PREAMBLE
  });
});
