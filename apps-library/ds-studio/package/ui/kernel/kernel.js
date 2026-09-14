/* Kernel selection.
 *
 * The notebook talks to one interface: describe, init, execute, variables,
 * reset, interrupt. Which implementation answers depends on whether the
 * Python runtime was vendored into the package before the build. Detection
 * happens once at startup and never silently falls back mid-session: if the
 * user asked for Python and Python fails to start, that is an error the
 * interface reports, not a swap behind their back.
 */
(function(root, factory){
  'use strict';
  var api = factory(
    typeof module === 'object' && module.exports ? require('./kernel-js.js') : root.StudioKernelJs,
    typeof module === 'object' && module.exports ? require('./kernel-python.js') : root.StudioKernelPython
  );
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioKernel = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(JsKernel, PythonKernel){
  'use strict';

  /* Returns which kernels this build can actually run. */
  function detect() {
    return PythonKernel.available().then(function(pythonAvailable){
      return {
        js: true,
        python: !!pythonAvailable,
        preferred: pythonAvailable ? 'python' : 'js'
      };
    }).catch(function(){
      return { js: true, python: false, preferred: 'js' };
    });
  }

  function create(id, store) {
    if (id === 'python') return PythonKernel.create(store);
    return JsKernel.create(store);
  }

  return Object.freeze({
    detect: detect,
    create: create
  });
});
