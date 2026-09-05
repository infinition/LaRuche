/* Notebook model and cell scheduler.
 *
 * Two host limits shape this file. An action handler has a 15 second deadline,
 * so running a cell cannot be one long action: cell.run enqueues a job and
 * returns immediately, and the agent polls job.status. And a bridge message
 * caps at 64 KiB, so every agent-facing snapshot is built through a budget
 * that truncates rows and text rather than overflowing.
 *
 * DOM-free on purpose: the whole file is exercised by the Node test suite.
 */
(function(root, factory){
  'use strict';
  var api = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioNotebook = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(){
  'use strict';

  var MAX_CELLS = 200;
  var MAX_SOURCE_BYTES = 32 * 1024;
  var MAX_OUTPUTS_PER_CELL = 40;
  var SCHEMA_VERSION = 1;

  function NotebookError(message, code) {
    var error = new Error(message);
    error.name = 'NotebookError';
    error.code = code || 'invalid';
    return error;
  }

  var counter = 0;
  function newId(prefix) {
    counter += 1;
    return prefix + '-' + Date.now().toString(36) + '-' + counter.toString(36);
  }

  /* ---------------------------------------------------------- Notebook */

  function Notebook(options) {
    var settings = options || {};
    this.id = settings.id || newId('nb');
    this.title = settings.title || '';
    this.createdAt = settings.createdAt || Date.now();
    this.updatedAt = settings.updatedAt || this.createdAt;
    this.kernelId = settings.kernelId || 'js';
    this.cells = [];
    this.revision = 0;
    this.listeners = [];
  }

  Notebook.prototype.on = function(listener) {
    this.listeners.push(listener);
    var self = this;
    return function(){
      self.listeners = self.listeners.filter(function(entry){ return entry !== listener; });
    };
  };

  Notebook.prototype.touch = function(event) {
    this.revision += 1;
    this.updatedAt = Date.now();
    var payload = event || { type: 'change' };
    payload.revision = this.revision;
    this.listeners.forEach(function(listener){
      try { listener(payload); } catch (error) { /* a listener must not break the model */ }
    });
    return this.revision;
  };

  Notebook.prototype.index = function(cellId) {
    for (var i = 0; i < this.cells.length; i += 1) {
      if (this.cells[i].id === cellId) return i;
    }
    return -1;
  };

  Notebook.prototype.cell = function(cellId) {
    var position = this.index(cellId);
    if (position === -1) throw NotebookError('unknown cell "' + cellId + '"', 'not_found');
    return this.cells[position];
  };

  Notebook.prototype.addCell = function(options) {
    var settings = options || {};
    if (this.cells.length >= MAX_CELLS) {
      throw NotebookError('a notebook holds at most ' + MAX_CELLS + ' cells', 'limit');
    }
    var type = settings.type === 'markdown' ? 'markdown' : 'code';
    var source = checkSource(settings.source || '');
    var cell = {
      id: newId('cell'),
      type: type,
      source: source,
      outputs: [],
      status: 'idle',
      execCount: 0,
      durationMs: 0,
      error: null,
      author: settings.author === 'agent' ? 'agent' : 'user',
      updatedAt: Date.now()
    };
    var position = settings.index === undefined || settings.index === null
      ? this.cells.length
      : Math.max(0, Math.min(Math.trunc(settings.index), this.cells.length));
    this.cells.splice(position, 0, cell);
    this.touch({ type: 'cell.add', cellId: cell.id, index: position });
    return cell;
  };

  /* `append` lets an agent stream a cell in fragments, so the notebook shows
   * the code arriving instead of appearing in one jump. */
  Notebook.prototype.updateCell = function(cellId, options) {
    var settings = options || {};
    var cell = this.cell(cellId);
    if (settings.type === 'markdown' || settings.type === 'code') cell.type = settings.type;
    if (settings.source !== undefined && settings.source !== null) {
      var next = settings.mode === 'append' ? cell.source + String(settings.source) : String(settings.source);
      cell.source = checkSource(next);
      cell.status = cell.status === 'running' ? cell.status : 'stale';
    }
    if (settings.author === 'agent' || settings.author === 'user') cell.author = settings.author;
    cell.updatedAt = Date.now();
    this.touch({ type: 'cell.update', cellId: cell.id, streaming: settings.mode === 'append' });
    return cell;
  };

  Notebook.prototype.deleteCell = function(cellId) {
    var position = this.index(cellId);
    if (position === -1) throw NotebookError('unknown cell "' + cellId + '"', 'not_found');
    this.cells.splice(position, 1);
    this.touch({ type: 'cell.delete', cellId: cellId });
    return true;
  };

  Notebook.prototype.moveCell = function(cellId, index) {
    var position = this.index(cellId);
    if (position === -1) throw NotebookError('unknown cell "' + cellId + '"', 'not_found');
    var target = Math.max(0, Math.min(Math.trunc(index), this.cells.length - 1));
    var cell = this.cells.splice(position, 1)[0];
    this.cells.splice(target, 0, cell);
    this.touch({ type: 'cell.move', cellId: cellId, index: target });
    return target;
  };

  Notebook.prototype.setOutputs = function(cellId, outputs, meta) {
    var cell = this.cell(cellId);
    var settings = meta || {};
    cell.outputs = (outputs || []).slice(0, MAX_OUTPUTS_PER_CELL);
    cell.status = settings.status || 'ok';
    cell.error = settings.error || null;
    cell.durationMs = settings.durationMs || 0;
    if (settings.countExecution) cell.execCount += 1;
    cell.updatedAt = Date.now();
    this.touch({ type: 'cell.outputs', cellId: cellId });
    return cell;
  };

  Notebook.prototype.clearOutputs = function(cellId) {
    var self = this;
    var targets = cellId ? [this.cell(cellId)] : this.cells;
    targets.forEach(function(cell){
      cell.outputs = [];
      cell.error = null;
      cell.status = 'idle';
      cell.durationMs = 0;
    });
    this.touch({ type: 'cell.clear', cellId: cellId || null });
    return targets.length;
  };

  function checkSource(source) {
    var text = String(source);
    if (text.length > MAX_SOURCE_BYTES) {
      throw NotebookError('cell source exceeds ' + Math.round(MAX_SOURCE_BYTES / 1024) + ' KiB', 'limit');
    }
    return text;
  }

  /* ------------------------------------------------------------- Jobs */

  /* One kernel, one main thread, so jobs run strictly one at a time. A queued
   * job that is cancelled before it starts never reaches the kernel. */
  function Runner(notebook, kernel, options) {
    var settings = options || {};
    this.notebook = notebook;
    this.kernel = kernel;
    this.jobs = Object.create(null);
    this.queue = [];
    this.current = null;
    this.pumpScheduled = false;
    this.timeoutMs = settings.timeoutMs || 12000;
    this.maxJobs = settings.maxJobs || 64;
    this.onOutput = settings.onOutput || null;
    this.onJobChange = settings.onJobChange || null;
  }

  Runner.prototype.setKernel = function(kernel) {
    this.kernel = kernel;
  };

  Runner.prototype.submit = function(cellId) {
    var cell = this.notebook.cell(cellId);
    if (cell.type !== 'code') {
      throw NotebookError('only a code cell can run', 'type');
    }
    if (!this.kernel || !this.kernel.ready) {
      throw NotebookError('the kernel is not ready', 'not_ready');
    }

    var existing = this.jobFor(cellId);
    if (existing) return existing;

    var job = {
      id: newId('job'),
      cellId: cellId,
      status: 'queued',
      queuedAt: Date.now(),
      startedAt: 0,
      finishedAt: 0,
      durationMs: 0,
      outputs: [],
      error: null,
      token: { cancelled: false }
    };
    this.jobs[job.id] = job;
    this.queue.push(job.id);
    this.trim();

    cell.status = 'queued';
    this.notebook.touch({ type: 'cell.queued', cellId: cellId });
    this.notify(job);
    this.schedulePump();
    return job;
  };

  /* Execution always starts on a later turn of the event loop. The action
   * handler that submitted the job therefore returns straight away, and a job
   * cancelled before it starts never reaches the kernel at all. */
  Runner.prototype.schedulePump = function() {
    var self = this;
    if (this.pumpScheduled) return;
    this.pumpScheduled = true;
    setTimeout(function(){
      self.pumpScheduled = false;
      self.pump();
    }, 0);
  };

  Runner.prototype.jobFor = function(cellId) {
    var self = this;
    var active = Object.keys(this.jobs).filter(function(id){
      var job = self.jobs[id];
      return job.cellId === cellId && (job.status === 'queued' || job.status === 'running');
    });
    return active.length ? this.jobs[active[0]] : null;
  };

  Runner.prototype.job = function(jobId) {
    return this.jobs[jobId] || null;
  };

  Runner.prototype.trim = function() {
    var ids = Object.keys(this.jobs);
    if (ids.length <= this.maxJobs) return;
    var self = this;
    ids.sort(function(a, b){ return self.jobs[a].queuedAt - self.jobs[b].queuedAt; })
      .slice(0, ids.length - this.maxJobs)
      .forEach(function(id){
        if (self.jobs[id].status === 'queued' || self.jobs[id].status === 'running') return;
        delete self.jobs[id];
      });
  };

  Runner.prototype.notify = function(job) {
    if (this.onJobChange) {
      try { this.onJobChange(job); } catch (error) { /* reporting must not break the run */ }
    }
  };

  Runner.prototype.cancel = function(jobId) {
    var job = this.jobs[jobId];
    if (!job) throw NotebookError('unknown job "' + jobId + '"', 'not_found');
    if (job.status === 'ok' || job.status === 'error' || job.status === 'cancelled') return job;

    job.token.cancelled = true;
    if (job.status === 'queued') {
      this.queue = this.queue.filter(function(id){ return id !== jobId; });
      this.finish(job, 'cancelled', [], { message: 'cancelled before it started', line: 0, column: 0, hint: '' });
    } else if (this.kernel && this.kernel.interrupt) {
      this.kernel.interrupt();
    }
    return job;
  };

  Runner.prototype.cancelAll = function() {
    var self = this;
    Object.keys(this.jobs).forEach(function(id){
      var job = self.jobs[id];
      if (job.status === 'queued' || job.status === 'running') self.cancel(id);
    });
  };

  Runner.prototype.pump = function() {
    var self = this;
    if (this.current || !this.queue.length) return;

    var jobId = this.queue.shift();
    var job = this.jobs[jobId];
    if (!job || job.status !== 'queued') { this.schedulePump(); return; }

    if (job.token.cancelled) {
      this.finish(job, 'cancelled', [], { message: 'cancelled before it started', line: 0, column: 0, hint: '' });
      return;
    }

    var cell;
    try {
      cell = this.notebook.cell(job.cellId);
    } catch (error) {
      this.finish(job, 'error', [], { message: error.message, line: 0, column: 0, hint: '' });
      return;
    }

    this.current = job;
    job.status = 'running';
    job.startedAt = Date.now();
    cell.status = 'running';
    cell.outputs = [];
    cell.error = null;
    this.notebook.touch({ type: 'cell.start', cellId: cell.id });
    this.notify(job);

    var live = [];
    this.kernel.execute(cell.source, {
      token: job.token,
      timeoutMs: this.timeoutMs,
      onOutput: function(output){
        live.push(output);
        try {
          var target = self.notebook.cell(job.cellId);
          target.outputs = live.slice(0, MAX_OUTPUTS_PER_CELL);
          self.notebook.touch({ type: 'cell.output', cellId: job.cellId });
        } catch (error) {
          /* the cell was deleted while running */
        }
        if (self.onOutput) self.onOutput(job, output);
      }
    }).then(function(result){
      var status = result.ok ? 'ok' : (result.error && result.error.cancelled ? 'cancelled' : 'error');
      self.finish(job, status, result.outputs || [], result.error || null);
    }).catch(function(error){
      self.finish(job, 'error', live, { message: String(error && error.message || error), line: 0, column: 0, hint: '' });
    });
  };

  Runner.prototype.finish = function(job, status, outputs, error) {
    job.status = status;
    job.outputs = outputs || [];
    job.error = error || null;
    job.finishedAt = Date.now();
    job.durationMs = job.startedAt ? job.finishedAt - job.startedAt : 0;

    try {
      this.notebook.setOutputs(job.cellId, job.outputs, {
        status: status,
        error: job.error,
        durationMs: job.durationMs,
        countExecution: status === 'ok'
      });
    } catch (failure) {
      /* the cell was deleted while running: the job still completes */
    }

    if (this.current && this.current.id === job.id) this.current = null;
    this.notify(job);
    this.schedulePump();
  };

  Runner.prototype.runAll = function() {
    var self = this;
    var submitted = [];
    this.notebook.cells.forEach(function(cell){
      if (cell.type !== 'code' || !cell.source.trim()) return;
      submitted.push(self.submit(cell.id).id);
    });
    return submitted;
  };

  Runner.prototype.busy = function() {
    return !!this.current || this.queue.length > 0;
  };

  /* -------------------------------------------------- Agent snapshots */

  /* Bridge messages cap at 64 KiB and action results at 60 KiB, so a snapshot
   * is assembled against a byte budget: rows and text are cut, and the result
   * says so, rather than letting the host reject the whole reply. */
  function summarize(notebook, options) {
    var settings = options || {};
    var budget = settings.budget || 40 * 1024;
    var sourceLimit = settings.sourceLimit || 2000;

    var cells = notebook.cells.map(function(cell){
      return {
        id: cell.id,
        type: cell.type,
        status: cell.status,
        author: cell.author,
        execCount: cell.execCount,
        durationMs: cell.durationMs,
        source: clip(cell.source, sourceLimit),
        sourceTruncated: cell.source.length > sourceLimit,
        outputs: cell.outputs.map(function(output){ return summarizeOutput(output, settings); }),
        error: cell.error ? {
          message: clip(cell.error.message, 400),
          line: cell.error.line || 0,
          hint: clip(cell.error.hint || '', 200)
        } : null
      };
    });

    var snapshot = {
      notebookId: notebook.id,
      title: notebook.title,
      revision: notebook.revision,
      kernel: settings.kernel || null,
      cellCount: cells.length,
      datasets: settings.datasets || [],
      cells: cells,
      truncated: false
    };

    /* Drop the oldest cells' outputs first, then the cells themselves. */
    while (bytesOf(snapshot) > budget && snapshot.cells.length) {
      var trimmed = false;
      for (var i = 0; i < snapshot.cells.length; i += 1) {
        if (snapshot.cells[i].outputs.length) {
          snapshot.cells[i].outputs = [];
          snapshot.cells[i].outputsTruncated = true;
          snapshot.truncated = true;
          trimmed = true;
          break;
        }
      }
      if (trimmed) continue;
      snapshot.cells.shift();
      snapshot.truncated = true;
    }
    while (bytesOf(snapshot) > budget && snapshot.datasets.length) {
      snapshot.datasets.pop(); snapshot.datasetsTruncated = true; snapshot.truncated = true;
    }

    return snapshot;
  }

  function summarizeOutput(output, options) {
    var settings = options || {};
    var rowLimit = settings.rowLimit || 20;
    var textLimit = settings.textLimit || 1200;

    if (output.kind === 'table') {
      return {
        kind: 'table',
        title: output.title || '',
        columns: output.columns || [],
        rows: (output.rows || []).slice(0, rowLimit),
        total: output.total || 0,
        truncated: (output.rows || []).length > rowLimit || !!output.truncated
      };
    }
    if (output.kind === 'chart') {
      /* A point cloud is summarised by its size and a handful of samples: the
       * full cloud would not fit the bridge, and the caller does not need it. */
      if (output.chart === 'scatter3d') {
        var cloud = output.points || [];
        return {
          kind: 'chart',
          chart: 'scatter3d',
          title: output.title || '',
          axis: output.axis || null,
          points: cloud.length,
          sample: cloud.slice(0, Math.min(rowLimit, 10)),
          truncated: cloud.length > 10
        };
      }
      return {
        kind: 'chart',
        chart: output.chart,
        title: output.title || '',
        axis: output.axis || null,
        labels: (output.labels || []).slice(0, rowLimit),
        series: (output.series || []).slice(0, 8).map(function(entry){
          return { name: entry.name, values: (entry.values || []).slice(0, rowLimit) };
        }),
        truncated: (output.labels || []).length > rowLimit
      };
    }
    if (output.kind === 'image') {
      /* Image bytes are intentionally absent. Download through the native App
       * UI; there is no image/artifact export action in this SDK yet. */
      return { kind: 'image', title: output.title || '', bytes: (output.source || '').length };
    }
    if (output.kind === 'markdown') {
      return { kind: 'markdown', text: clip(output.text || '', textLimit) };
    }
    return {
      kind: output.kind === 'value' ? 'value' : 'stream',
      stream: output.stream || 'out',
      text: clip(output.text || '', textLimit)
    };
  }

  function clip(text, limit) {
    var value = String(text === null || text === undefined ? '' : text);
    return value.length > limit ? value.slice(0, limit) + '...' : value;
  }

  function bytesOf(value) {
    return new TextEncoder().encode(JSON.stringify(value)).length;
  }

  /* ------------------------------------------------------ Persistence */

  /* Outputs are saved with the code, so closing and reopening the view brings
   * the tables, charts and figures back. They do not fit unconditionally: the
   * App gets 1 MiB in total, and one matplotlib figure can be hundreds of
   * kilobytes. Outputs are therefore kept against a byte budget, newest cell
   * first, and whatever does not fit is reported rather than silently lost. */
  var OUTPUT_TEXT_LIMIT = 8 * 1024;
  var OUTPUT_ROW_LIMIT = 200;
  var OUTPUT_POINT_LIMIT = 500;
  var IMAGE_LIMIT = 96 * 1024;
  var POINT_CLOUD_LIMIT = 4000;

  function persistableOutput(output) {
    if (output.kind === 'image') {
      /* A figure larger than this would eat the whole budget on its own. */
      if (!output.source || output.source.length > IMAGE_LIMIT) return null;
      return { kind: 'image', source: output.source, title: output.title || '' };
    }
    if (output.kind === 'table') {
      var rows = output.rows || [];
      return {
        kind: 'table',
        title: output.title || '',
        columns: output.columns || [],
        schema: output.schema || [],
        rows: rows.slice(0, OUTPUT_ROW_LIMIT),
        total: output.total || rows.length,
        truncated: !!output.truncated || rows.length > OUTPUT_ROW_LIMIT
      };
    }
    if (output.kind === 'chart') {
      /* The specification, not the rendered SVG: it redraws on load, in the
       * theme in force then. */
      if (output.chart === 'scatter3d') {
        return {
          kind: 'chart',
          chart: 'scatter3d',
          title: output.title || '',
          axis: output.axis || null,
          colors: output.colors || null,
          points: (output.points || []).slice(0, POINT_CLOUD_LIMIT)
        };
      }
      return {
        kind: 'chart',
        chart: output.chart,
        title: output.title || '',
        labels: (output.labels || []).slice(0, OUTPUT_POINT_LIMIT),
        series: (output.series || []).slice(0, 8).map(function(entry){
          return { name: entry.name, values: (entry.values || []).slice(0, OUTPUT_POINT_LIMIT) };
        }),
        axis: output.axis || null,
        colors: output.colors || null,
        stacked: !!output.stacked,
        horizontal: !!output.horizontal
      };
    }
    if (output.kind === 'markdown') {
      return { kind: 'markdown', text: clip(output.text || '', OUTPUT_TEXT_LIMIT) };
    }
    return {
      kind: output.kind === 'value' ? 'value' : 'stream',
      stream: output.stream || 'out',
      valueType: output.valueType || '',
      text: clip(output.text || '', OUTPUT_TEXT_LIMIT)
    };
  }

  /* A run in flight has no result worth restoring; it comes back as edited. */
  function restorableStatus(status) {
    return status === 'ok' || status === 'error' ? status : 'stale';
  }

  function serialize(notebook, options) {
    var settings = options || {};
    var budget = settings.outputBudget === undefined ? 420 * 1024 : settings.outputBudget;

    var cells = notebook.cells.map(function(cell){
      return {
        id: cell.id,
        type: cell.type,
        source: cell.source,
        author: cell.author,
        execCount: cell.execCount,
        status: restorableStatus(cell.status),
        durationMs: cell.durationMs || 0,
        error: cell.error ? {
          message: clip(cell.error.message, 400),
          line: cell.error.line || 0,
          column: cell.error.column || 0,
          hint: clip(cell.error.hint || '', 300)
        } : null,
        outputs: []
      };
    });

    var spent = 0;
    var dropped = 0;
    /* Last cell first: the recent results are the ones still being read. */
    for (var i = notebook.cells.length - 1; i >= 0; i -= 1) {
      var outputs = notebook.cells[i].outputs || [];
      if (!outputs.length) continue;
      var kept = outputs.map(persistableOutput).filter(Boolean);
      if (!kept.length) { dropped += 1; continue; }
      var size = JSON.stringify(kept).length;
      if (spent + size > budget) { dropped += 1; continue; }
      spent += size;
      cells[i].outputs = kept;
      if (kept.length !== outputs.length) dropped += 0;
    }

    return {
      version: SCHEMA_VERSION,
      id: notebook.id,
      title: notebook.title,
      kernelId: notebook.kernelId,
      createdAt: notebook.createdAt,
      updatedAt: notebook.updatedAt,
      outputBytes: spent,
      outputsDropped: dropped,
      cells: cells
    };
  }

  function deserialize(payload) {
    if (!payload || payload.version !== SCHEMA_VERSION || !Array.isArray(payload.cells)) {
      throw NotebookError('unreadable notebook payload', 'schema');
    }
    var notebook = new Notebook({
      id: payload.id,
      title: payload.title,
      kernelId: payload.kernelId,
      createdAt: payload.createdAt,
      updatedAt: payload.updatedAt
    });
    payload.cells.slice(0, MAX_CELLS).forEach(function(entry){
      var outputs = Array.isArray(entry.outputs) ? entry.outputs.slice(0, MAX_OUTPUTS_PER_CELL) : [];
      var cell = {
        id: entry.id || newId('cell'),
        type: entry.type === 'markdown' ? 'markdown' : 'code',
        source: String(entry.source || '').slice(0, MAX_SOURCE_BYTES),
        outputs: outputs,
        /* A cell whose results came back keeps the status they were produced
         * with; one saved without them is edited, not silently valid. */
        status: outputs.length || entry.error ? restorableStatus(entry.status) : 'stale',
        execCount: Number(entry.execCount) || 0,
        durationMs: Number(entry.durationMs) || 0,
        error: entry.error || null,
        author: entry.author === 'agent' ? 'agent' : 'user',
        restored: outputs.length > 0,
        updatedAt: Date.now()
      };
      notebook.cells.push(cell);
    });
    return notebook;
  }

  /* Library entry for the notebook picker, small enough to keep them all in
   * one storage value. */
  function describe(notebook) {
    return {
      id: notebook.id,
      title: notebook.title,
      updatedAt: notebook.updatedAt,
      createdAt: notebook.createdAt,
      kernelId: notebook.kernelId,
      cellCount: notebook.cells.length,
      codeCells: notebook.cells.filter(function(cell){ return cell.type === 'code'; }).length
    };
  }

  /* Private storage caps one value at 64 KiB, so a payload is split across
   * numbered keys and reassembled on load. */
  function chunk(payload, maxBytes) {
    var text = JSON.stringify(payload);
    var limit = Math.max(1024, (maxBytes || 48 * 1024));
    var parts = [];
    for (var offset = 0; offset < text.length;) {
      // The host counts UTF-8 bytes of the JSON STRING, including escaping.
      // Character slicing alone overflows on non-ASCII text and backslashes.
      var low = offset + 1, high = Math.min(text.length, offset + limit), end = low;
      while (low <= high) {
        var mid = Math.floor((low + high) / 2);
        if (bytesOf(text.slice(offset, mid)) <= limit) { end = mid; low = mid + 1; }
        else high = mid - 1;
      }
      if (end < text.length && /[\uD800-\uDBFF]/.test(text[end - 1]) && /[\uDC00-\uDFFF]/.test(text[end])) end -= 1;
      parts.push(text.slice(offset, end));
      offset = end;
    }
    return parts.length ? parts : [''];
  }

  function unchunk(parts) {
    if (!parts || !parts.length) return null;
    try {
      return JSON.parse(parts.join(''));
    } catch (error) {
      return null;
    }
  }

  return Object.freeze({
    create: function(options){ return new Notebook(options); },
    Notebook: Notebook,
    Runner: Runner,
    runner: function(notebook, kernel, options){ return new Runner(notebook, kernel, options); },
    summarize: summarize,
    summarizeOutput: summarizeOutput,
    serialize: serialize,
    deserialize: deserialize,
    describe: describe,
    chunk: chunk,
    unchunk: unchunk,
    error: NotebookError,
    limits: {
      cells: MAX_CELLS,
      sourceBytes: MAX_SOURCE_BYTES,
      outputsPerCell: MAX_OUTPUTS_PER_CELL
    }
  });
});
