/* Built-in kernel: the notebook language interpreted in the page.
 *
 * Nothing to download and nothing to install, so it reaches Ready in one
 * frame. It runs on the main thread like the Python kernel does, because the
 * App CSP has no worker-src and blocks Worker construction outright.
 */
(function(root, factory){
  'use strict';
  var api = factory(
    typeof module === 'object' && module.exports ? require('../lib/lang.js') : root.StudioLang,
    typeof module === 'object' && module.exports ? require('../lib/frame.js') : root.StudioFrame
  );
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioKernelJs = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(Lang, Frames){
  'use strict';

  function JsKernel(store) {
    this.id = 'js';
    this.language = 'studio';
    this.ready = false;
    this.store = store;
    this.scope = Object.create(null);
  }

  /* Returned by kernel.status and, in short form, by notebook.state. An agent
   * that reads this never has to guess the syntax or go looking through the
   * package for the grammar. */
  var REFERENCE = {
    language: 'studio',
    summary: 'One statement per line, assignment is "name = expression", comments start with #. ' +
      'Variables persist across cells in the same notebook. This is NOT JavaScript and NOT Python: ' +
      'free-form code from either fails to parse.',
    loading: 'v = load("dataset_name")   # the names come from data.list',
    reshaping: [
      'filter(condition)', 'select("a", "b")', 'drop("a")', 'rename(old = "new")',
      'sort("col", desc = true)', 'head(n)', 'tail(n)', 'slice(start, end)',
      'assign(newCol = expression)', 'withColumn("name", expression)',
      'distinct()', 'dropna()', 'fillna(0)', 'concat(other)'
    ],
    aggregating: [
      'groupby("col").agg(total = sum(amount), n = count())',
      'pivot(index = "a", columns = "b", values = "c", agg = "sum")',
      'join(other, on = "key", how = "left")',
      'describe()', 'valueCounts("col")', 'unique("col")', 'histogram("col", bins = 12)'
    ],
    aggregations: ['sum', 'mean', 'min', 'max', 'count', 'countDistinct', 'median',
      'quantile', 'std', 'variance', 'first', 'last', 'mode'],
    outputs: ['show(table)', 'print(value)', 'md("text")', 'save("name", table)'],
    charts3d: 'scatter3d(t, x = "a", y = "b", z = "c", series = "g", size = "s", label = "name", title = "", color = "#ff7a18") draws a rotatable point cloud. Depth is a weak cue on a flat screen: use it to see shape and clusters, and a 2D chart when a value has to be read.',
    charts: [
      'bar(t, x = "col", y = "col", title = "", series = "col", stacked = true, horizontal = true, color = "#ccff00")',
      'line(t, x = "col", y = "col", title = "")',
      'area(t, x = "col", y = "col", title = "")',
      'scatter(t, x = "col", y = "col", title = "")',
      'pie(t, labels = "col", values = "col", title = "")',
      'hist(t, column = "col", bins = 12, title = "", color = "#ccff00")'
    ],
    colour: 'Every chart takes color = "#rrggbb" to override the default palette. Hex only.',
    rowScope: 'Inside filter, assign, withColumn and agg, a bare word is a column of the current row: ' +
      'v.filter(annee >= 2020 and region == "Nord")',
    example: 'v = load("ventes")\n' +
      'top = v.filter(annee >= 2020).groupby("produit").agg(total = sum(montant), n = count())\n' +
      'show(top.sort("total", desc = true).head(10))\n' +
      'bar(top, x = "produit", y = "total", title = "Top produits")',
    notSupported: [
      'free JavaScript or Python expressions',
      'network access from a cell',
      'file system access from a cell',
      'importing libraries'
    ]
  };

  var SYNTAX_HINT = 'The notebook language is "studio", not JavaScript or Python. ' +
    'Example: v = load("name") then show(v.head(10)). Call kernel.status for the full reference.';

  JsKernel.prototype.describe = function() {
    return {
      id: this.id,
      language: this.language,
      labelKey: 'kernelJsLabel',
      versionKey: 'kernelJsVersion',
      ready: this.ready,
      reference: REFERENCE,
      syntaxHint: SYNTAX_HINT
    };
  };

  JsKernel.prototype.init = function(onProgress) {
    var self = this;
    if (onProgress) onProgress(40, 'kernelStarting');
    return Promise.resolve().then(function(){
      self.ready = true;
      if (onProgress) onProgress(100, 'kernelReady');
    });
  };

  /* Cells share one namespace, so a variable assigned in cell 1 is visible in
   * cell 2, exactly like a notebook kernel. */
  JsKernel.prototype.execute = function(code, options) {
    var self = this;
    var settings = options || {};
    return new Promise(function(resolve){
      var result = Lang.execute(code, {
        datasets: self.store.all(),
        variables: self.scope,
        token: settings.token,
        timeoutMs: settings.timeoutMs || 12000,
        onOutput: settings.onOutput
      });

      self.scope = result.scope;
      self.store.sync(result.datasets);

      /* A cell written in another language fails deep in the parser, where the
       * message is about a token. Say what the language actually is, so the
       * caller corrects the syntax instead of going looking for a grammar. */
      var error = result.error || null;
      if (error && !error.cancelled && !error.timeout && !error.hint) {
        error.hint = SYNTAX_HINT;
      }

      resolve({ ok: result.ok, outputs: result.outputs, error: error });
    });
  };

  JsKernel.prototype.variables = function() {
    return Lang.describeVariables(this.scope);
  };

  /* The extension lets you edit a variable from the sidebar; the same door,
   * opened to the agent. */
  JsKernel.prototype.setVariable = function(name, value) {
    this.scope[name] = value;
    return true;
  };

  JsKernel.prototype.deleteVariable = function(name) {
    if (!Object.prototype.hasOwnProperty.call(this.scope, name)) return false;
    delete this.scope[name];
    return true;
  };

  JsKernel.prototype.reset = function() {
    this.scope = Object.create(null);
    return Promise.resolve();
  };

  JsKernel.prototype.interrupt = function() {
    return Promise.resolve();
  };

  JsKernel.prototype.completions = function() {
    return Lang.builtins.slice();
  };

  return Object.freeze({
    create: function(store){ return new JsKernel(store); },
    available: function(){ return true; },
    reference: REFERENCE,
    syntaxHint: SYNTAX_HINT
  });
});
