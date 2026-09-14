/* DS Studio: host bridge, interface and agent actions.
 *
 * Startup order matters. The SDK handshake only means the transport is up, so
 * the view reports `loading` with progress while the kernel starts, registers
 * its actions, and only then reports `ready`. The host rejects actions before
 * that, which is what lets an agent call app_wait and trust the answer.
 */
(function(){
  'use strict';

  var sdk = window.LaRucheApp;
  var Notebooks = window.StudioNotebook;
  var Stores = window.StudioStore;
  var Kernels = window.StudioKernel;
  var Charts = window.StudioChart;
  var Csv = window.StudioCsv;
  var Frames = window.StudioFrame;
  var i18n = window.StudioI18n.create();

  var KEY_ZOOM = 'studio.zoom.v1';
  var KEY_INSPECTEUR = 'studio.inspector.v1';
  var KEY_LIBRARY = 'studio.lib.v1';
  var KEY_NOTEBOOK = 'studio.nb.';
  var KEY_DATA = 'studio.data.v1.';
  var KEY_DATA_INDEX = 'studio.data.v1';
  var CHUNK_BYTES = 48 * 1024;
  var MAX_CHUNKS = 12;
  var MAX_NOTEBOOKS = 24;
  var OUTPUT_BUDGET = 420 * 1024;
  var DATASET_PERSIST_BYTES = 24 * 1024;

  var store = Stores.create({ persistBudget: DATASET_PERSIST_BYTES });
  var notebook = null;
  var runner = null;
  var kernel = null;
  /* Ce que la detection au demarrage a trouve, garde pour un carnet neuf.
     Un carnet restaure reste sur SON noyau, ce qui est juste et ce qui
     enfermait: sans cette memoire, un carnet neuf heritait du noyau courant
     et il n'existait aucun chemin pour revenir a Python. */
  var kernelsAvailable = { js: true, python: false, preferred: 'js' };
  var kernelStatus = { state: 'loading', progress: 0, messageKey: 'kernelStarting' };

  var theme = 'dark';
  var storageLimit = 1024 * 1024;
  var saveTimer = null;
  var saveChain = Promise.resolve();
  var saveSequence = 0;
  var saveState = 'idle', saveError = null, savedAt = null, savedRevision = null;
  var library = { activeId: null, notebooks: [] };
  var chunkCounts = Object.create(null);
  var dataChunks = 0;
  var skippedDatasets = [];
  var droppedOutputs = 0;
  var agentSeat = 'studio-' + Date.now().toString(36);
  var agentBusy = false;

  /* The notebook follows the agent as it writes, but only while the reader is
   * already at the bottom. Scrolling up hands control back; coming back within
   * this many pixels of the bottom resumes the follow. */
  var STICK_THRESHOLD = 48;
  var stickToBottom = true;
  var focusNextAdded = false;

  /* Which model events mean "new content arrived at the end". A cell the user
   * is typing into is not one of them: onCellInput never emits an event. */
  var FOLLOW_EVENTS = {
    'cell.add': true,
    'cell.update': true,
    'cell.start': true,
    'cell.queued': true,
    'cell.output': true,
    'cell.outputs': true
  };

  /* ------------------------------------------------------------- Helpers */

  function element(id) { return document.getElementById(id); }

  function t(key, params) { return i18n.t(key, params); }

  function escapeHtml(value) {
    return String(value === null || value === undefined ? '' : value)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function toast(message, kind) {
    var node = element('toast');
    node.textContent = message;
    node.className = 'toast' + (kind ? ' ' + kind : '');
    node.hidden = false;
    clearTimeout(node.dataset.timer);
    node.dataset.timer = setTimeout(function(){ node.hidden = true; }, 4200);
  }

  /* Table cells show the value, never a rounded stand-in: compacting here
   * would turn the year 2021 into "2 k". */
  function formatCell(value) {
    if (value === null || value === undefined) return '';
    if (value && typeof value === 'object' && typeof value.__date === 'string') {
      return i18n.date(new Date(value.__date), { dateStyle: 'short' });
    }
    if (typeof value === 'number') {
      return i18n.number(value, { maximumFractionDigits: Number.isInteger(value) ? 0 : 3 });
    }
    if (typeof value === 'boolean') return t(value ? 'valueTrue' : 'valueFalse');
    return String(value);
  }

  /* Decimals that carry meaning, ignoring binary floating-point residue: a sum
   * of amounts lands on 4856.259999999999, which is two decimals, not twelve. */
  function significantDecimals(value) {
    var text = Number(value.toPrecision(12)).toString();
    if (text.indexOf('e') !== -1) return 2;
    var point = text.indexOf('.');
    return point === -1 ? 0 : Math.min(4, text.length - point - 1);
  }

  /* Decides the display of a numeric column once, from all its values, so a
   * column reads consistently down the page. Thousands grouping is withheld
   * from short integer columns: a year has to read 2021, not 2 021, and the
   * same is true of identifiers and small counts. */
  function columnFormatters(output) {
    var rows = output.rows || [];
    return (output.columns || []).map(function(name, index){
      var numeric = false;
      var allInteger = true;
      var largest = 0;
      var decimals = 0;

      for (var i = 0; i < rows.length; i += 1) {
        var value = rows[i][index];
        if (value === null || value === undefined) continue;
        if (typeof value !== 'number' || !isFinite(value)) return formatCell;
        numeric = true;
        largest = Math.max(largest, Math.abs(value));
        if (!Number.isInteger(value)) {
          allInteger = false;
          decimals = Math.max(decimals, significantDecimals(value));
        }
      }

      if (!numeric) return formatCell;
      var grouping = !(allInteger && largest < 10000);

      return function(value){
        if (value === null || value === undefined) return '';
        if (typeof value !== 'number') return formatCell(value);
        return i18n.number(value, {
          useGrouping: grouping,
          minimumFractionDigits: decimals,
          maximumFractionDigits: decimals
        });
      };
    });
  }

  /* Axis ticks and tooltips: grouped digits while they fit, compact only when
   * a full number would not. */
  function formatChartNumber(value) {
    if (typeof value !== 'number' || !isFinite(value)) return '';
    if (Math.abs(value) >= 100000) return i18n.compact(value);
    return i18n.number(value, {
      maximumFractionDigits: Number.isInteger(value) ? 0 : Math.abs(value) < 10 ? 2 : 1
    });
  }

  /* ------------------------------------------------------------ Startup */

  function setBoot(progress, messageKey, isError) {
    var bar = element('bootBar');
    var message = element('bootMessage');
    var wrapper = element('bootProgress');
    bar.style.width = Math.max(0, Math.min(100, progress)) + '%';
    wrapper.setAttribute('aria-valuenow', String(Math.round(progress)));
    message.textContent = t(messageKey);
    if (isError) element('boot').classList.add('is-error');
  }

  function reportStatus(state, messageKey, progress) {
    kernelStatus = { state: state, progress: progress, messageKey: messageKey };
    paintKernelChip();
    if (!sdk) return Promise.resolve();
    return sdk.ui.setStatus(state, t(messageKey).slice(0, 200), progress).catch(function(){});
  }

  function paintKernelChip() {
    var chip = element('kernelChip');
    var dot = element('kernelDot');
    var label = element('kernelLabel');
    if (!chip) return;
    chip.className = 'chip is-' + kernelStatus.state;
    dot.className = 'dot is-' + kernelStatus.state;
    if (kernelStatus.state === 'ready' && kernel) {
      var description = kernel.describe();
      label.textContent = t(description.labelKey) + (description.version ? ' ' + description.version : '');
      chip.title = t('kernelReadyTitle', { language: description.language });
    } else {
      label.textContent = t(kernelStatus.messageKey);
      chip.title = '';
    }
  }

  async function loadCatalog(locale) {
    var response = await fetch('./locales/' + locale + '.json', { credentials: 'omit' });
    if (!response.ok) throw new Error('missing catalogue ' + locale);
    return response.json();
  }

  async function start() {
    var fallback = await loadCatalog(window.StudioI18n.fallback).catch(function(){ return {}; });
    i18n.use(window.StudioI18n.fallback, fallback, fallback);
    i18n.apply(document);

    var context = null;
    if (sdk) {
      try {
        context = await sdk.ready();
      } catch (error) {
        setBoot(0, 'bridgeUnavailable', true);
        return;
      }
    }

    var locale = i18n.negotiate(context && context.locale);
    if (locale !== window.StudioI18n.fallback) {
      var catalog = await loadCatalog(locale).catch(function(){ return null; });
      if (catalog) i18n.use(locale, catalog, fallback);
    }
    document.documentElement.lang = i18n.locale;
    i18n.apply(document);

    theme = (context && context.theme === 'light') ? 'light' : 'dark';
    document.documentElement.dataset.hostTheme = theme;
    if (context && context.limits && context.limits.storageBytes) {
      storageLimit = context.limits.storageBytes;
    }

    await reportStatus('loading', 'kernelDetecting', 5);
    setBoot(5, 'kernelDetecting');

    var availability = await Kernels.detect();
    kernelsAvailable = availability;
    var restored = await restore();

    /* Un carnet restaure garde SON langage, un carnet neuf prend le meilleur.
     *
     * Les cellules d'un carnet ont ete ecrites pour un noyau precis. Le faire
     * tourner sous l'autre parce que celui-ci vient de devenir disponible
     * casserait chaque cellule, et le bandeau de discordance arriverait apres
     * coup. On ne retrograde que si le noyau demande est absent. */
    var wanted = restored && restored.kernelId
      ? (restored.kernelId === 'python' && !availability.python ? 'js' : restored.kernelId)
      : availability.preferred;

    kernel = Kernels.create(wanted, store);

    try {
      await kernel.init(function(progress, messageKey){
        setBoot(progress, messageKey);
        reportStatus('loading', messageKey, progress);
      });
    } catch (error) {
      /* Python was vendored but did not start. Say so instead of quietly
       * running a different language than the notebook was written for. */
      if (wanted === 'python') {
        toast(t('kernelPythonFailed', { message: String(error.message || error) }), 'error');
        kernel = Kernels.create('js', store);
        await kernel.init(function(progress, messageKey){ setBoot(progress, messageKey); });
      } else {
        setBoot(100, 'kernelFailed', true);
        await reportStatus('error', 'kernelFailed', 100);
        return;
      }
    }

    if (!restored) notebook.kernelId = kernel.id;
    if (notebook.kernelId !== kernel.id) toast(t('kernelLanguageMismatch'), 'error');
    runner = Notebooks.runner(notebook, kernel, {
      timeoutMs: 12000,
      onJobChange: function(){ paintToolbar(); }
    });

    notebook.on(onNotebookEvent);

    bindEvents();
    renderAll();

    if (sdk) {
      await sdk.ui.setTitle(notebook.title || t('appName')).catch(function(){});
      registerActions();
    }

    if (notebook.cells.some(function(cell){ return cell.restored; })) {
      toast(t('restoredResults'));
    }

    /* Relu avant l'affichage, pour que la coque n'apparaisse pas a 100% puis
     * ne saute a la taille voulue sous les yeux du lecteur. */
    if (sdk) {
      var zoomRange = await sdk.storage.get(KEY_ZOOM).catch(function(){ return null; });
      if (typeof zoomRange === 'number') appliquerZoom(zoomRange, false);
      var repliRange = await sdk.storage.get(KEY_INSPECTEUR).catch(function(){ return null; });
      appliquerInspecteur(typeof repliRange === 'boolean' ? repliRange
        : !window.matchMedia('(min-width: 900px)').matches, false);
    }

    element('boot').hidden = true;
    element('shell').hidden = false;
    await reportStatus('ready', 'kernelReady', 100);
  }

  /* -------------------------------------------------------- Persistence

   * The App gets 1 MiB in total, 64 KiB per value and 256 keys, so the library
   * is laid out as: one small index, one chunked value set per notebook, and
   * one shared value set for the datasets. Results are kept for the notebook
   * that is open; switching away rewrites the outgoing one code-only, because
   * a single matplotlib figure can be a fifth of the whole budget. */

  function notebookKey(id, index) {
    return KEY_NOTEBOOK + id + '.' + index;
  }

  async function readChunks(count, keyOf) {
    var parts = [];
    for (var i = 0; i < Math.min(count, MAX_CHUNKS); i += 1) {
      var part = await sdk.storage.get(keyOf(i));
      parts.push(typeof part === 'string' ? part : '');
    }
    return Notebooks.unchunk(parts);
  }

  async function restore() {
    if (!sdk) {
      notebook = starterNotebook();
      library = { activeId: notebook.id, notebooks: [Notebooks.describe(notebook)] };
      return null;
    }

    try {
      var stored = await sdk.storage.get(KEY_LIBRARY);
      if (stored && Array.isArray(stored.notebooks) && stored.notebooks.length) {
        library = {
          activeId: stored.activeId || stored.notebooks[0].id,
          notebooks: stored.notebooks.slice(0, MAX_NOTEBOOKS)
        };
      } else {
        library = { activeId: null, notebooks: [] };
      }

      var datasets = await sdk.storage.get(KEY_DATA_INDEX);
      if (datasets && typeof datasets.chunks === 'number') {
        var datasetPayload = await readChunks(datasets.chunks, function(i){ return KEY_DATA + i; });
        if (datasetPayload) store.restore(datasetPayload);
        dataChunks = datasets.chunks;
      }

      var entry = findEntry(library.activeId) || library.notebooks[0];
      if (entry) {
        var loaded = await loadStored(entry.id);
        if (loaded) {
          notebook = loaded;
          library.activeId = entry.id;
          setSaveStatus('saved');
          return { kernelId: entry.kernelId };
        }
      }

      notebook = starterNotebook();
      registerInLibrary(notebook);
      return null;
    } catch (error) {
      notebook = starterNotebook();
      library = { activeId: notebook.id, notebooks: [Notebooks.describe(notebook)] };
      setSaveStatus('offline');
      return null;
    }
  }

  async function loadStored(id) {
    var entry = findEntry(id);
    if (!entry) return null;
    var payload = await readChunks(entry.chunks || 1, function(i){ return notebookKey(id, i); });
    if (!payload || !Array.isArray(payload.cells)) return null;
    try {
      var loaded = Notebooks.deserialize(payload);
      chunkCounts[id] = entry.chunks || 1;
      return loaded;
    } catch (error) {
      return null;
    }
  }

  function findEntry(id) {
    for (var i = 0; i < library.notebooks.length; i += 1) {
      if (library.notebooks[i].id === id) return library.notebooks[i];
    }
    return null;
  }

  function registerInLibrary(target) {
    var description = Notebooks.describe(target);
    var entry = findEntry(target.id);
    if (entry) {
      Object.keys(description).forEach(function(key){ entry[key] = description[key]; });
    } else {
      library.notebooks.push(description);
    }
    library.activeId = target.id;
    library.notebooks.sort(function(a, b){ return b.updatedAt - a.updatedAt; });
  }

  function starterNotebook() {
    var created = Notebooks.create({ title: '' });
    created.addCell({ type: 'markdown', source: t('starterNote') });
    created.addCell({ source: t('starterCode') });
    return created;
  }

  function schedulePersist() {
    if (!sdk) return;
    clearTimeout(saveTimer);
    setSaveStatus('saving');
    if (sdk.ui.setDirty) sdk.ui.setDirty(true).catch(function(){});
    saveTimer = setTimeout(persist, 700);
  }

  /* Chunks the notebook, shrinking the output budget until it fits the
   * per-notebook chunk allowance rather than failing the whole save. */
  function chunkNotebook(target, withOutputs) {
    var budgets = withOutputs ? [OUTPUT_BUDGET, OUTPUT_BUDGET / 2, 64 * 1024, 0] : [0];
    var parts = null;
    var payload = null;
    for (var i = 0; i < budgets.length; i += 1) {
      payload = Notebooks.serialize(target, { outputBudget: budgets[i] });
      parts = Notebooks.chunk(payload, CHUNK_BYTES);
      if (parts.length <= MAX_CHUNKS) break;
    }
    if (parts.length > MAX_CHUNKS) throw new Error('Notebook code exceeds the storage budget. Export it before removing cells.');
    return { parts: parts, payload: payload };
  }

  async function writeNotebook(target, withOutputs) {
    var result = chunkNotebook(target, withOutputs);
    var parts = result.parts;
    var previous = chunkCounts[target.id] || 0;
    droppedOutputs = result.payload.outputsDropped || 0;

    // Do not flood the bridge's eight-request concurrency limit.
    for (var index = 0; index < parts.length; index += 1) await sdk.storage.set(notebookKey(target.id, index), parts[index]);
    for (var i = parts.length; i < previous; i += 1) await sdk.storage.delete(notebookKey(target.id, i));
    chunkCounts[target.id] = parts.length;
    var entry = findEntry(target.id);
    if (entry) entry.chunks = parts.length;
  }

  async function writeDatasets() {
    var payload = store.serialize();
    skippedDatasets = payload.skipped;
    var parts = Notebooks.chunk(payload, CHUNK_BYTES);

    if (parts.length > MAX_CHUNKS) {
      /* Datasets are the part the user can re-import, so they yield first. */
      skippedDatasets = store.list().map(function(item){ return { name: item.name, bytes: item.bytes }; });
      parts = Notebooks.chunk({ datasets: [], skipped: [] }, CHUNK_BYTES);
    }

    var previous = dataChunks;
    for (var index = 0; index < parts.length; index += 1) await sdk.storage.set(KEY_DATA + index, parts[index]);
    for (var i = parts.length; i < previous; i += 1) await sdk.storage.delete(KEY_DATA + i);
    dataChunks = parts.length;
    await sdk.storage.set(KEY_DATA_INDEX, { chunks: parts.length, updatedAt: Date.now() });
  }

  function persist() {
    if (!sdk) return Promise.resolve();
    var sequence = ++saveSequence;
    var target = notebook, targetRevision = notebook.revision;
    saveError = null;
    setSaveStatus('saving');
    if (sdk.ui.setDirty) sdk.ui.setDirty(true).catch(function(){});

    registerInLibrary(notebook);

    saveChain = saveChain.catch(function(){}).then(function(){
      return writeNotebook(target, true);
    }).then(function(){
      return writeDatasets();
    }).then(function(){
      return sdk.storage.set(KEY_LIBRARY, {
        version: 1,
        activeId: library.activeId,
        notebooks: library.notebooks.slice(0, MAX_NOTEBOOKS)
      });
    }).then(function(){
      if (sequence !== saveSequence) return;
      savedAt = Date.now(); savedRevision = targetRevision;
      setSaveStatus('saved');
      if (sdk.ui.setDirty) sdk.ui.setDirty(false).catch(function(){});
      renderQuota();
      renderLibrary();
    }).catch(function(error){
      if (sequence !== saveSequence) return;
      saveError = String(error.message || error);
      setSaveStatus('offline');
      if (sdk.ui.setDirty) sdk.ui.setDirty(true).catch(function(){});
      toast(t('saveFailed', { message: String(error.message || error) }), 'error');
    });
    return saveChain;
  }

  /* ------------------------------------------------------ Notebook library */

  function adoptNotebook(next) {
    notebook = next;
    if (notebook.kernelId !== kernel.id) toast(t('kernelLanguageMismatch'), 'error');
    notebook.on(onNotebookEvent);
    runner = Notebooks.runner(notebook, kernel, {
      timeoutMs: 12000,
      onJobChange: function(){ paintToolbar(); }
    });
    stickToBottom = true;
    registerInLibrary(notebook);
    element('notebookTitle').value = notebook.title;
    renderAll();
    if (sdk) sdk.ui.setTitle(notebook.title || t('appName')).catch(function(){});
  }

  /* The outgoing notebook is rewritten without its results: only the open one
   * can afford them. Its code, name and history are kept. */
  async function openNotebook(id) {
    if (id === notebook.id) return notebook;
    if (!findEntry(id)) throw new Error('No notebook with id "' + id + '"');

    clearTimeout(saveTimer);
    if (runner) runner.cancelAll();

    if (sdk) {
      registerInLibrary(notebook);
      await saveChain.catch(function(){});
      await writeNotebook(notebook, false);
    }

    var loaded = sdk ? await loadStored(id) : null;
    if (!loaded) throw new Error('Notebook "' + id + '" could not be read back');

    adoptNotebook(loaded);
    if (kernel && kernel.reset) await kernel.reset();
    renderVariables();
    renderPackages();
    schedulePersist();
    return loaded;
  }

  async function newNotebook(title) {
    clearTimeout(saveTimer);
    if (runner) runner.cancelAll();

    if (sdk) {
      registerInLibrary(notebook);
      await saveChain.catch(function(){});
      await writeNotebook(notebook, false);
    }

    if (library.notebooks.length >= MAX_NOTEBOOKS) {
      throw new Error('The library holds at most ' + MAX_NOTEBOOKS + ' notebooks. Delete one first.');
    }

    /* Un carnet neuf n'a pas de cellules, donc aucun langage a preserver: il
       part sur le meilleur noyau disponible. C'est la seule porte de sortie
       quand la session a demarre sur un carnet ecrit dans l'autre langage. */
    if (kernelsAvailable[kernelsAvailable.preferred] && kernel.id !== kernelsAvailable.preferred) {
      await basculerNoyau(kernelsAvailable.preferred);
    }

    var created = Notebooks.create({ title: title || '' });
    created.kernelId = kernel.id;
    created.addCell({ source: '' });
    adoptNotebook(created);
    if (kernel && kernel.reset) await kernel.reset();
    renderVariables();
    renderPackages();
    await persist();
    return created;
  }

  async function deleteNotebook(id) {
    var entry = findEntry(id);
    if (!entry) throw new Error('No notebook with id "' + id + '"');
    if (library.notebooks.length === 1) throw new Error('The last notebook cannot be deleted');

    var count = entry.chunks || chunkCounts[id] || 1;
    library.notebooks = library.notebooks.filter(function(item){ return item.id !== id; });
    delete chunkCounts[id];

    if (sdk) {
      for (var i = 0; i < count; i += 1) {
        await sdk.storage.delete(notebookKey(id, i)).catch(function(){});
      }
    }

    if (id === notebook.id) {
      var next = library.notebooks[0];
      var loaded = sdk ? await loadStored(next.id) : null;
      adoptNotebook(loaded || Notebooks.create({ title: next.title }));
      if (kernel && kernel.reset) await kernel.reset();
    }

    await persist();
    return id;
  }

  function setSaveStatus(kind) {
    saveState = kind;
    var node = element('saveStatus');
    if (!node) return;
    node.className = 'save-status is-' + kind;
    node.textContent = t(kind === 'saved' ? 'saved' : kind === 'saving' ? 'saving' : 'storageLocal');
  }

  /* ------------------------------------------------------------ Rendering */

  function onNotebookEvent(event) {
    if (event.type === 'cell.add' || event.type === 'cell.delete' || event.type === 'cell.move') {
      renderCells();
    } else if (event.type === 'cell.update') {
      renderCell(event.cellId, { keepEditor: !event.streaming });
    } else if (event.type === 'cell.output' || event.type === 'cell.outputs' ||
               event.type === 'cell.start' || event.type === 'cell.queued' || event.type === 'cell.clear') {
      if (event.cellId) renderCell(event.cellId, { keepEditor: true });
      else renderCells();
      renderVariables();
      renderPackages();
    }
    paintToolbar();
    renderSummary();
    if (event.type !== 'cell.output' && event.type !== 'cell.start') schedulePersist();

    /* A cell the user just asked for wins over the follow: put the caret in it. */
    if (event.type === 'cell.add' && focusNextAdded) {
      focusNextAdded = false;
      var target = document.querySelector('[data-cell-id="' + event.cellId + '"]');
      if (target) {
        target.scrollIntoView({ block: 'nearest' });
        var editor = target.querySelector('[data-role="editor"]');
        if (editor) editor.focus();
        return;
      }
    }

    if (stickToBottom && FOLLOW_EVENTS[event.type]) followAgent(event.cellId);
  }

  function notebookPane() {
    return document.querySelector('.notebook-pane');
  }

  /* Le suivi ne doit juger que ce que le LECTEUR a fait.
   *
   * Redessiner la liste vide le conteneur, ce qui ramene le defilement en
   * haut, puis le remet ou il etait. Ces deux ecritures emettent chacune un
   * evenement `scroll`, et ils arrivent AVANT les trames de `followAgent`.
   * Le suivi se coupait donc tout seul des que le carnet depassait un ecran:
   * la position relue etait le haut du carnet, jamais le bas, et l'agent
   * pouvait ecrire dix cellules sans que la vue bouge.
   *
   * Le drapeau couvre nos propres ecritures. Il tombe dans une trame
   * d'animation, et non apres un delai: la specification place la livraison
   * des evenements de defilement avant les rappels de trame, donc ce qui nous
   * appartient est deja passe quand il se leve. */
  var ownScroll = false;
  function releaseOwnScroll() {
    requestAnimationFrame(function(){ ownScroll = false; });
  }

  function updateStickiness() {
    if (ownScroll) return;
    var pane = notebookPane();
    if (!pane) return;
    stickToBottom = pane.scrollHeight - pane.scrollTop - pane.clientHeight <= STICK_THRESHOLD;
  }

  /* Suit l'endroit ou l'agent travaille, et le montre par son debut.
   *
   * Viser le bas du carnet ne marche que si la derniere cellule tient dans le
   * volet. Un graphique ou un grand tableau est plus haut que lui: on se
   * retrouvait alors calle sur sa fin, sans jamais voir ce qui venait
   * d'apparaitre. Centrer une cellule trop haute a le meme defaut, en montrant
   * son milieu.
   *
   * Donc: une cellule qui tient est centree, une cellule trop haute est
   * alignee par son haut, et on ne descend jamais plus bas que necessaire. */
  /* Zoom de l'interface.
   *
   * Un graphique large ne rentre pas dans un panneau etroit, et le reduire
   * seul le rendrait illisible a cote d'un texte reste grand. `zoom` agit sur
   * la mise en page entiere: le graphique recoit plus de pixels logiques, et
   * tout garde ses proportions. Le reglage est range dans le stockage prive de
   * l'App, donc par utilisateur, et relu au demarrage. */
  var ZOOMS = [60, 70, 80, 90, 100, 110, 125, 150];
  var zoomActuel = 100;

  function appliquerZoom(valeur, persister) {
    var proche = ZOOMS.reduce(function(a, b){
      return Math.abs(b - valeur) < Math.abs(a - valeur) ? b : a;
    }, ZOOMS[0]);
    zoomActuel = proche;
    document.documentElement.style.setProperty('--zoom', String(proche / 100));
    var etiquette = element('zoomValue');
    if (etiquette) etiquette.textContent = proche + '%';
    if (persister && sdk) sdk.storage.set(KEY_ZOOM, proche).catch(function(){});
  }

  function decalerZoom(pas) {
    var i = ZOOMS.indexOf(zoomActuel);
    appliquerZoom(ZOOMS[Math.max(0, Math.min(ZOOMS.length - 1, i + pas))], true);
  }

  /* Le volet du bas, replie ou non.
   *
   * En colonne il prend jusqu'a 46% de la hauteur, ce qui est beaucoup pendant
   * qu'un agent ecrit dans le carnet. Replie il ne garde que ses onglets: on
   * sait qu'il est la, on le rouvre d'un clic, et le carnet recupere la place.
   * Le choix est retenu; sans choix enregistre, on part replie seulement quand
   * la mise en page est en colonne, c'est-a-dire quand la place manque. */
  var inspecteurReplie = false;

  function appliquerInspecteur(replie, persister) {
    inspecteurReplie = !!replie;
    var volet = element('inspector');
    if (volet) volet.classList.toggle('is-collapsed', inspecteurReplie);
    var bouton = element('inspectorToggle');
    if (bouton) bouton.setAttribute('aria-expanded', String(!inspecteurReplie));
    if (persister && sdk) sdk.storage.set(KEY_INSPECTEUR, inspecteurReplie).catch(function(){});
  }

  var MARGE_SUIVI = 12;

  function positionPour(pane, target) {
    var boite = target.getBoundingClientRect();
    var volet = pane.getBoundingClientRect();
    var haut = pane.scrollTop + (boite.top - volet.top);
    var visible = pane.clientHeight;
    var cible = boite.height <= visible - 2 * MARGE_SUIVI
      ? haut - (visible - boite.height) / 2
      : haut - MARGE_SUIVI;
    return Math.max(0, Math.min(cible, pane.scrollHeight - visible));
  }

  /* Le halo dit ou l'agent travaille, sans deplacer quoi que ce soit. */
  function marquerActive(cellId) {
    var precedent = document.querySelector('.cell.is-active');
    if (precedent && precedent.getAttribute('data-cell-id') !== cellId) {
      precedent.classList.remove('is-active');
    }
    if (!cellId) return;
    var actuel = document.querySelector('[data-cell-id="' + cellId + '"]');
    if (actuel) actuel.classList.add('is-active');
  }

  /* Deux trames d'attente: la cellule qui declenche ceci est mise en page a la
   * premiere, donc sa hauteur n'est connue qu'a la seconde. */
  function followAgent(cellId) {
    var pane = notebookPane();
    if (!pane) return;
    requestAnimationFrame(function(){
      requestAnimationFrame(function(){
        marquerActive(cellId);
        if (!stickToBottom) return;
        var target = cellId && document.querySelector('[data-cell-id="' + cellId + '"]');
        ownScroll = true;
        /* Toujours instantane. Un defilement anime emet des evenements pendant
         * toute sa duree, et le drapeau serait retombe au milieu: la position
         * relue aurait alors coupe le suivi que ce defilement etait en train
         * de servir. */
        pane.scrollTop = target ? positionPour(pane, target) : pane.scrollHeight;
        releaseOwnScroll();
      });
    });
  }

  function renderLibrary() {
    var select = element('notebookSelect');
    if (!select) return;
    select.textContent = '';
    library.notebooks.forEach(function(entry){
      var option = document.createElement('option');
      option.value = entry.id;
      option.textContent = (entry.title || t('untitledNotebook')) +
        ' · ' + t('cellCount', { count: entry.codeCells || 0 });
      option.selected = entry.id === notebook.id;
      select.appendChild(option);
    });
    element('deleteNotebookBtn').disabled = library.notebooks.length < 2;
  }

  function renderAll() {
    element('notebookTitle').value = notebook.title;
    renderLibrary();
    renderCells();
    renderDatasets();
    renderVariables();
    renderPackages();
    renderQuota();
    renderSummary();
    paintToolbar();
    paintKernelChip();
  }

  function renderCells() {
    var container = element('cells');
    var pane = notebookPane();
    var previousTop = pane ? pane.scrollTop : 0;
    /* Leve avant de vider: le vidage ramene le defilement en haut de lui-meme,
     * et cet evenement-la nous appartient aussi. */
    if (pane) ownScroll = true;
    container.textContent = '';
    notebook.cells.forEach(function(cell){
      container.appendChild(buildCell(cell));
    });
    element('emptyState').hidden = notebook.cells.length > 0;
    /* A textarea reports scrollHeight 0 while detached, so the editors are
     * sized once they are in the document, not while being built. */
    container.querySelectorAll('[data-role="editor"]').forEach(autosize);
    /* Vider la liste colle le volet en haut. On rend au lecteur la position
     * qu'il avait, sauf s'il suivait l'agent: dans ce cas la position d'avant
     * est deja perimee, c'est la fin du carnet qu'il veut voir. */
    if (pane) {
      pane.scrollTop = stickToBottom ? pane.scrollHeight : previousTop;
      releaseOwnScroll();
    }
  }

  function renderCell(cellId, options) {
    var existing = document.querySelector('[data-cell-id="' + cellId + '"]');
    if (!existing) { renderCells(); return; }
    var cell;
    try { cell = notebook.cell(cellId); } catch (error) { renderCells(); return; }
    var focused = document.activeElement;
    var keepValue = options && options.keepEditor && existing.contains(focused);
    var caret = keepValue ? { start: focused.selectionStart, end: focused.selectionEnd } : null;
    var replacement = buildCell(cell);
    existing.replaceWith(replacement);
    var editor = replacement.querySelector('[data-role="editor"]');
    if (editor) autosize(editor);
    if (caret && editor) {
      editor.focus();
      editor.setSelectionRange(caret.start, caret.end);
    }
  }

  function buildCell(cell) {
    var fragment = element('cellTemplate').content.cloneNode(true);
    var article = fragment.querySelector('.cell');
    article.dataset.cellId = cell.id;
    article.classList.add('is-' + cell.status);
    article.classList.add('kind-' + cell.type);

    i18n.apply(article);

    var editor = article.querySelector('[data-role="editor"]');
    editor.value = cell.source;
    editor.setAttribute('aria-label', t(cell.type === 'markdown' ? 'noteEditorLabel' : 'codeEditorLabel'));
    autosize(editor);

    article.querySelector('[data-role="kind"]').textContent =
      t(cell.type === 'markdown' ? 'kindNote' : 'kindCode');
    article.querySelector('[data-role="execCount"]').textContent =
      cell.execCount ? '[' + cell.execCount + ']' : '';
    article.querySelector('[data-role="agentBadge"]').hidden = cell.author !== 'agent';

    var timing = article.querySelector('[data-role="timing"]');
    if (cell.status === 'running') timing.textContent = t('statusRunning');
    else if (cell.status === 'queued') timing.textContent = t('statusQueued');
    else if (cell.status === 'stale') timing.textContent = t('statusStale');
    else if (cell.durationMs) timing.textContent = i18n.duration(cell.durationMs);
    else timing.textContent = '';

    var runButton = article.querySelector('[data-action="run"]');
    runButton.disabled = cell.type !== 'code';
    runButton.classList.toggle('is-busy', cell.status === 'running' || cell.status === 'queued');

    var preview = article.querySelector('[data-role="notePreview"]');
    if (cell.type === 'markdown') {
      preview.innerHTML = renderMarkdown(cell.source);
      preview.hidden = false;
      editor.hidden = true;
      preview.addEventListener('dblclick', function(){
        preview.hidden = true;
        editor.hidden = false;
        editor.focus();
      });
    }

    renderOutputs(article.querySelector('[data-role="outputs"]'), cell);
    return article;
  }

  function autosize(editor) {
    editor.style.height = 'auto';
    editor.style.height = Math.min(520, Math.max(48, editor.scrollHeight)) + 'px';
  }

  function renderOutputs(container, cell) {
    container.textContent = '';
    if (cell.error) container.appendChild(buildError(cell.error));
    cell.outputs.forEach(function(output){
      var node = buildOutput(output);
      if (node) container.appendChild(node);
    });
    container.hidden = !container.childNodes.length;
  }

  function buildError(error) {
    var node = document.createElement('div');
    node.className = 'output output-error';
    var title = document.createElement('strong');
    title.textContent = error.line
      ? t('errorAtLine', { line: error.line, message: error.message })
      : error.message;
    node.appendChild(title);
    if (error.hint) {
      var hint = document.createElement('p');
      hint.className = 'muted small';
      hint.textContent = error.hint;
      node.appendChild(hint);
    }
    return node;
  }

  function buildOutput(output) {
    if (output.kind === 'stream' || output.kind === 'value') {
      var pre = document.createElement('pre');
      pre.className = 'output output-stream' + (output.stream === 'err' ? ' is-stderr' : '');
      pre.textContent = output.text;
      return pre;
    }
    if (output.kind === 'markdown') {
      var note = document.createElement('div');
      note.className = 'output output-note';
      note.innerHTML = renderMarkdown(output.text);
      return note;
    }
    if (output.kind === 'image') {
      var figure = document.createElement('figure');
      figure.className = 'output output-image';
      var image = document.createElement('img');
      image.src = output.source;
      image.alt = output.title || t('figureAlt');
      image.loading = 'lazy';
      figure.appendChild(image);
      figure.appendChild(buildDownloadRow([
        { label: t('downloadPng'), filename: 'figure.png', href: output.source }
      ]));
      return figure;
    }
    if (output.kind === 'table') return buildTable(output);
    if (output.kind === 'chart') return buildChart(output);
    if (output.kind === 'plotly') return buildPlotly(output);
    return null;
  }

  /* Une figure Plotly arrive en JSON et se dessine cote navigateur.
   *
   * Le noyau Python n'emet que `fig.to_json()`: c'est plotly.js qui rend, et il
   * n'est charge que sur le chemin CDN. Sans lui la figure existe quand meme, et
   * il vaut mieux le dire que d'afficher un cadre vide dont personne ne saurait
   * s'il calcule encore. */
  function buildPlotly(output) {
    var figure = document.createElement('figure');
    figure.className = 'output output-plotly';
    if (typeof window.Plotly === 'undefined') {
      var absent = document.createElement('p');
      absent.className = 'output-note';
      absent.textContent = t('plotlyMissing');
      figure.appendChild(absent);
      return figure;
    }
    var cible = document.createElement('div');
    cible.className = 'plotly-target';
    figure.appendChild(cible);

    var mise = Object.assign({}, output.layout || {});
    // Le graphique suit le theme de l'hote plutot que le blanc par defaut de
    // Plotly, sinon une figure interactive tranche avec tout le reste du carnet.
    mise.paper_bgcolor = 'rgba(0,0,0,0)';
    mise.plot_bgcolor = 'rgba(0,0,0,0)';
    mise.font = Object.assign({ color: getComputedStyle(document.body).color }, mise.font || {});
    mise.margin = mise.margin || { l: 48, r: 16, t: output.title ? 40 : 16, b: 44 };
    mise.autosize = true;

    try {
      window.Plotly.newPlot(cible, output.data || [], mise, {
        responsive: true,
        displaylogo: false
      });
    } catch (erreur) {
      cible.textContent = String((erreur && erreur.message) || erreur);
      return figure;
    }

    var plein = document.createElement('button');
    plein.type = 'button';
    plein.className = 'apps-btn';
    plein.textContent = t('plotlyFullscreen');
    plein.addEventListener('click', function(){
      if (figure.requestFullscreen) {
        figure.requestFullscreen().then(function(){
          window.Plotly.Plots.resize(cible);
        }).catch(function(){});
      }
    });
    var rangee = document.createElement('div');
    rangee.className = 'output-actions';
    rangee.appendChild(plein);
    figure.appendChild(rangee);
    return figure;
  }

  var PAGE_SIZE = 25;

  /* Sorting, searching and paging happen in the page, on the rows the cell
   * already produced. Nothing is recomputed and the kernel is not involved, so
   * exploring a result never costs a re-run. */
  function buildTable(output, options) {
    var settings = options || {};
    var columns = output.columns || [];
    var allRows = output.rows || [];
    var formatters = columnFormatters(output);

    var state = { sort: -1, ascending: true, query: '', page: 0 };

    var wrapper = document.createElement('div');
    wrapper.className = 'output output-table';

    if (output.title) {
      var caption = document.createElement('p');
      caption.className = 'output-title';
      caption.textContent = output.title;
      wrapper.appendChild(caption);
    }

    var controls = document.createElement('div');
    controls.className = 'table-controls';
    var search = document.createElement('input');
    search.type = 'search';
    search.className = 'table-search';
    search.placeholder = t('tableSearch');
    search.setAttribute('aria-label', t('tableSearch'));
    controls.appendChild(search);
    var matchCount = document.createElement('span');
    matchCount.className = 'muted small';
    controls.appendChild(matchCount);
    wrapper.appendChild(controls);
    controls.hidden = allRows.length <= 8;

    var scroller = document.createElement('div');
    scroller.className = 'table-scroll';
    var table = document.createElement('table');
    var head = document.createElement('thead');
    var body = document.createElement('tbody');
    table.appendChild(head);
    table.appendChild(body);
    scroller.appendChild(table);
    wrapper.appendChild(scroller);

    var footer = document.createElement('div');
    footer.className = 'output-foot';
    var count = document.createElement('span');
    count.className = 'muted small';
    footer.appendChild(count);

    var pager = document.createElement('span');
    pager.className = 'pager';
    var previous = document.createElement('button');
    previous.type = 'button';
    previous.className = 'link-button';
    previous.textContent = '‹';
    previous.setAttribute('aria-label', t('previousPage'));
    var pageLabel = document.createElement('span');
    pageLabel.className = 'muted small';
    var next = document.createElement('button');
    next.type = 'button';
    next.className = 'link-button';
    next.textContent = '›';
    next.setAttribute('aria-label', t('nextPage'));
    pager.appendChild(previous);
    pager.appendChild(pageLabel);
    pager.appendChild(next);
    footer.appendChild(pager);

    if (!settings.hideExport) {
      footer.appendChild(buildDownloadRow([
        { label: t('downloadCsv'), filename: 'table.csv', text: tableToCsv(output), type: 'text/csv' }
      ]));
    }
    wrapper.appendChild(footer);

    function textOf(value, index) {
      if (value === null || value === undefined) return '';
      return (formatters[index] || formatCell)(value);
    }

    function matching() {
      if (!state.query) return allRows;
      var needle = state.query.toLowerCase();
      return allRows.filter(function(row){
        for (var i = 0; i < row.length; i += 1) {
          if (textOf(row[i], i).toLowerCase().indexOf(needle) !== -1) return true;
        }
        return false;
      });
    }

    function ordered(rows) {
      if (state.sort === -1) return rows;
      var column = state.sort;
      return rows.slice().sort(function(left, right){
        var result = Frames.compareValues(sortable(left[column]), sortable(right[column]));
        return state.ascending ? result : -result;
      });
    }

    function sortable(value) {
      if (value && typeof value === 'object' && typeof value.__date === 'string') {
        return new Date(value.__date);
      }
      return value;
    }

    function renderHead() {
      head.textContent = '';
      var row = document.createElement('tr');
      columns.forEach(function(name, index){
        var th = document.createElement('th');
        var schema = output.schema && output.schema[index];
        th.textContent = name;
        th.tabIndex = 0;
        th.className = 'is-sortable' + (schema && schema.dtype === 'number' ? ' is-number' : '') +
          (state.sort === index ? (state.ascending ? ' sorted-asc' : ' sorted-desc') : '');
        th.title = (schema ? t('columnType', { type: t('dtype_' + schema.dtype) }) + ' · ' : '') + t('sortByColumn');
        function toggle() {
          if (state.sort === index) state.ascending = !state.ascending;
          else { state.sort = index; state.ascending = true; }
          state.page = 0;
          draw();
        }
        th.addEventListener('click', toggle);
        th.addEventListener('keydown', function(event){
          if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); toggle(); }
        });
        row.appendChild(th);
      });
      head.appendChild(row);
    }

    function draw() {
      var rows = ordered(matching());
      var pages = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
      if (state.page >= pages) state.page = pages - 1;
      var slice = rows.slice(state.page * PAGE_SIZE, state.page * PAGE_SIZE + PAGE_SIZE);

      renderHead();
      body.textContent = '';
      slice.forEach(function(row){
        var tr = document.createElement('tr');
        columns.forEach(function(name, index){
          var td = document.createElement('td');
          var schema = output.schema && output.schema[index];
          if (schema && schema.dtype === 'number') td.classList.add('is-number');
          var value = row[index];
          if (value === null || value === undefined) {
            td.className += ' is-missing';
            td.textContent = t('missingValue');
          } else {
            td.textContent = textOf(value, index);
          }
          tr.appendChild(td);
        });
        body.appendChild(tr);
      });

      matchCount.textContent = state.query ? t('tableMatches', { count: rows.length }) : '';
      pager.hidden = pages < 2;
      pageLabel.textContent = t('pageOf', { page: state.page + 1, pages: pages });
      previous.disabled = state.page === 0;
      next.disabled = state.page >= pages - 1;

      count.textContent = output.truncated
        ? t('tableTruncated', { shown: allRows.length, total: output.total })
        : t('tableRows', { count: output.total || allRows.length });
    }

    search.addEventListener('input', function(){
      state.query = search.value.trim();
      state.page = 0;
      draw();
    });
    previous.addEventListener('click', function(){ state.page -= 1; draw(); });
    next.addEventListener('click', function(){ state.page += 1; draw(); });

    draw();
    return wrapper;
  }

  function tableToCsv(output) {
    var lines = [(output.columns || []).join(',')];
    (output.rows || []).forEach(function(row){
      lines.push(row.map(function(value){
        if (value === null || value === undefined) return '';
        var text = value && typeof value === 'object' && value.__date ? value.__date : String(value);
        return /[",\n]/.test(text) ? '"' + text.replace(/"/g, '""') + '"' : text;
      }).join(','));
    });
    return lines.join('\n');
  }

  /* A chart always ships its table view: it is the accessible fallback, and
   * the relief the light palette's contrast warning requires. */
  function buildChart(output) {
    var wrapper = document.createElement('figure');
    wrapper.className = 'output output-chart';

    var rendered = Charts.render(output, {
      theme: theme,
      width: 720,
      otherLabel: t('otherSeries'),
      formatNumber: formatChartNumber
    });

    if (!rendered) {
      var empty = document.createElement('p');
      empty.className = 'muted small';
      empty.textContent = t('chartEmpty');
      wrapper.appendChild(empty);
      return wrapper;
    }

    var holder = document.createElement('div');
    holder.className = 'chart-holder';
    holder.innerHTML = rendered.svg;
    wrapper.appendChild(holder);

    var tableSpec = Charts.toTable(output);
    var tableView = buildTable({
      columns: tableSpec.columns,
      rows: tableSpec.rows,
      total: tableSpec.rows.length,
      schema: tableSpec.columns.map(function(name, index){
        return { name: name, dtype: index === 0 ? 'string' : 'number', missing: 0 };
      })
    }, { hideExport: true });
    tableView.classList.add('chart-table');
    tableView.hidden = true;
    wrapper.appendChild(tableView);

    var footer = document.createElement('div');
    footer.className = 'output-foot';

    var toggle = document.createElement('button');
    toggle.type = 'button';
    toggle.className = 'link-button';
    toggle.textContent = t('showTable');
    toggle.setAttribute('aria-expanded', 'false');
    toggle.addEventListener('click', function(){
      tableView.hidden = !tableView.hidden;
      toggle.textContent = t(tableView.hidden ? 'showTable' : 'hideTable');
      toggle.setAttribute('aria-expanded', String(!tableView.hidden));
    });
    footer.appendChild(toggle);

    footer.appendChild(buildDownloadRow([
      { label: t('downloadSvg'), filename: 'chart.svg', text: svgAutonome(rendered), type: 'image/svg+xml' },
      { label: t('downloadPng'), filename: 'chart.png', png: rendered }
    ]));
    wrapper.appendChild(footer);

    attachChartTooltip(holder);
    if (rendered.rotatable) attachRotation(holder, output, footer);
    return wrapper;
  }

  /* Drag to turn the cloud. Depth on a flat screen is a weak cue, so being
   * able to move the viewpoint is what makes a third axis readable at all. */
  function attachRotation(holder, output, footer) {
    var angles = { yaw: -0.62, pitch: 0.42 };
    var dragging = null;

    var hint = document.createElement('span');
    hint.className = 'muted small';
    hint.textContent = t('dragToRotate');
    footer.insertBefore(hint, footer.firstChild);

    function redraw() {
      var next = Charts.render(output, {
        theme: theme,
        width: 720,
        yaw: angles.yaw,
        pitch: angles.pitch,
        otherLabel: t('otherSeries'),
        formatNumber: formatChartNumber
      });
      if (!next) return;
      var tip = holder.querySelector('.chart-tip');
      holder.innerHTML = next.svg;
      if (tip) holder.appendChild(tip);
    }

    holder.addEventListener('pointerdown', function(event){
      dragging = { x: event.clientX, y: event.clientY, yaw: angles.yaw, pitch: angles.pitch };
      holder.setPointerCapture(event.pointerId);
      holder.classList.add('is-rotating');
    });

    holder.addEventListener('pointermove', function(event){
      if (!dragging) return;
      event.preventDefault();
      angles.yaw = dragging.yaw + (event.clientX - dragging.x) * 0.008;
      /* Clamped short of the poles, where the box degenerates to a line. */
      angles.pitch = Math.max(-1.35, Math.min(1.35, dragging.pitch - (event.clientY - dragging.y) * 0.008));
      redraw();
    });

    function stop(event) {
      if (!dragging) return;
      dragging = null;
      holder.classList.remove('is-rotating');
      if (event && event.pointerId !== undefined && holder.hasPointerCapture(event.pointerId)) {
        holder.releasePointerCapture(event.pointerId);
      }
    }

    holder.addEventListener('pointerup', stop);
    holder.addEventListener('pointercancel', stop);
  }

  function attachChartTooltip(holder) {
    var tip = document.createElement('div');
    tip.className = 'chart-tip';
    tip.hidden = true;
    holder.appendChild(tip);

    function show(event) {
      var mark = event.target.closest ? event.target.closest('.mark') : null;
      if (!mark) { tip.hidden = true; return; }
      var lines;
      if (mark.classList.contains('crosshair')) {
        var points = (mark.getAttribute('data-points') || '').split(String.fromCharCode(30));
        lines = [mark.getAttribute('data-label')].concat(points.map(function(entry){
          var parts = entry.split(String.fromCharCode(31));
          return parts[0] + ' : ' + parts[1];
        }));
      } else {
        lines = [
          mark.getAttribute('data-label'),
          mark.getAttribute('data-series') + ' : ' + mark.getAttribute('data-value')
        ];
      }
      tip.textContent = '';
      lines.filter(Boolean).forEach(function(line, index){
        var row = document.createElement('span');
        row.className = index === 0 ? 'tip-label' : 'tip-value';
        row.textContent = line;
        tip.appendChild(row);
      });
      var bounds = holder.getBoundingClientRect();
      var x = event.clientX - bounds.left;
      var y = event.clientY - bounds.top;
      tip.hidden = false;
      tip.style.left = Math.min(Math.max(8, x + 12), bounds.width - tip.offsetWidth - 8) + 'px';
      tip.style.top = Math.max(4, y - tip.offsetHeight - 10) + 'px';
    }

    holder.addEventListener('pointermove', show);
    holder.addEventListener('pointerleave', function(){ tip.hidden = true; });
    holder.addEventListener('pointerdown', show);
  }

  function buildDownloadRow(entries) {
    var row = document.createElement('span');
    row.className = 'download-row';
    entries.forEach(function(entry){
      var button = document.createElement('button');
      button.type = 'button';
      button.className = 'link-button';
      button.textContent = entry.label;
      button.addEventListener('click', function(){
        if (entry.png) exportPng(entry.png, entry.filename);
        else if (entry.href) triggerDownload(entry.href, entry.filename);
        else triggerDownload(URL.createObjectURL(new Blob([entry.text], { type: entry.type })), entry.filename, true);
      });
      row.appendChild(button);
    });
    return row;
  }

  /* Une confirmation qui vit dans la page, et non dans le navigateur.
   *
   * L'App est servie dans une iframe en bac a sable dont les jetons sont
   * `allow-scripts allow-forms allow-downloads`. Sans `allow-modals`,
   * `window.confirm()` n'ouvre rien et rend `false`: le bouton de suppression
   * d'un carnet sortait donc immediatement, sans dialogue, sans erreur et sans
   * rien supprimer. Il paraissait mort.
   *
   * Un `<dialog>` est un element du DOM et non une fenetre du navigateur: le bac
   * a sable ne le concerne pas. Corriger ici plutot que d'ajouter `allow-modals`
   * a toutes les Apps: une App qui peut ouvrir un modal du navigateur peut aussi
   * bloquer l'interface entiere de LaRuche. */
  function confirmer(message) {
    return new Promise(function(resolve){
      var boite = document.createElement('dialog');
      boite.className = 'confirm-box';
      var texte = document.createElement('p');
      texte.textContent = message;
      boite.appendChild(texte);

      var rangee = document.createElement('div');
      rangee.className = 'confirm-actions';
      var non = document.createElement('button');
      non.type = 'button';
      non.className = 'button';
      non.textContent = t('confirmNo');
      var oui = document.createElement('button');
      oui.type = 'button';
      oui.className = 'button danger';
      oui.textContent = t('confirmYes');
      rangee.appendChild(non);
      rangee.appendChild(oui);
      boite.appendChild(rangee);
      document.body.appendChild(boite);

      var repondu = false;
      function fermer(reponse) {
        if (repondu) return;
        repondu = true;
        try { boite.close(); } catch (e) {}
        boite.remove();
        resolve(reponse);
      }
      non.addEventListener('click', function(){ fermer(false); });
      oui.addEventListener('click', function(){ fermer(true); });
      // Echap ferme un <dialog> sans passer par nos boutons.
      boite.addEventListener('cancel', function(event){ event.preventDefault(); fermer(false); });
      if (typeof boite.showModal === 'function') boite.showModal(); else boite.setAttribute('open', '');
      oui.focus();
    });
  }

  function triggerDownload(href, filename, revoke) {
    var link = document.createElement('a');
    link.href = href;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    link.remove();
    if (revoke) setTimeout(function(){ URL.revokeObjectURL(href); }, 4000);
  }

  /* Un SVG destine a un fichier porte ses dimensions reelles.
   *
   * A l'ecran il est en width="100%" height="auto" pour suivre son conteneur, et
   * c'est exactement ce qu'il ne faut pas dans un fichier: charge comme Image,
   * un pourcentage n'est pas une dimension intrinseque, et le navigateur retombe
   * sur 300x150, la taille par defaut d'un element remplace. Le dessin etait
   * alors mis en boite dans un coin du canvas puis rogne, ce qui se voyait
   * surtout sur un nuage 3D dont les proportions ne sont pas celles-la. */
  function svgAutonome(rendered) {
    return String(rendered.svg).replace(
      'width="100%" height="auto"',
      'width="' + rendered.width + '" height="' + rendered.height + '"'
    );
  }

  /* Rasterises the chart through a canvas. The sandbox allows downloads and
   * blob: images, so this needs no network and no library. */
  function exportPng(rendered, filename) {
    var scale = 2;
    var blob = new Blob([svgAutonome(rendered)], { type: 'image/svg+xml;charset=utf-8' });
    var url = URL.createObjectURL(blob);
    var image = new Image();
    image.onload = function(){
      var canvas = document.createElement('canvas');
      canvas.width = rendered.width * scale;
      canvas.height = rendered.height * scale;
      var context = canvas.getContext('2d');
      context.scale(scale, scale);
      context.drawImage(image, 0, 0, rendered.width, rendered.height);
      URL.revokeObjectURL(url);
      canvas.toBlob(function(output){
        if (!output) { toast(t('exportFailed'), 'error'); return; }
        triggerDownload(URL.createObjectURL(output), filename, true);
      }, 'image/png');
    };
    image.onerror = function(){
      URL.revokeObjectURL(url);
      toast(t('exportFailed'), 'error');
    };
    image.src = url;
  }

  /* Deliberately small: headings, emphasis, code, lists and rules. Anything
   * else is shown as text, and every fragment is escaped first. */
  function renderMarkdown(source) {
    var lines = String(source || '').split('\n');
    var html = '';
    var inList = false;   /* false, ou le nom de la balise ouverte */
    var inCode = false;

    lines.forEach(function(line){
      if (/^```/.test(line)) {
        if (inCode) { html += '</code></pre>'; inCode = false; }
        else { if (inList) { html += '</ul>'; inList = false; } html += '<pre class="note-code"><code>'; inCode = true; }
        return;
      }
      if (inCode) { html += escapeHtml(line) + '\n'; return; }

      var heading = /^(#{1,4})\s+(.*)$/.exec(line);
      if (heading) {
        if (inList) { html += '</ul>'; inList = false; }
        var level = Math.min(heading[1].length + 1, 5);
        html += '<h' + level + '>' + inline(heading[2]) + '</h' + level + '>';
        return;
      }
      var puce = /^\s*([-*])\s+/.test(line);
      var numero = /^\s*\d+[.)]\s+/.test(line);
      if (puce || numero) {
        var balise = puce ? 'ul' : 'ol';
        if (inList && inList !== balise) { html += '</' + inList + '>'; inList = false; }
        if (!inList) { html += '<' + balise + '>'; inList = balise; }
        html += '<li>' + inline(line.replace(/^\s*(?:[-*]|\d+[.)])\s+/, '')) + '</li>';
        return;
      }
      if (inList) { html += '</' + inList + '>'; inList = false; }
      if (/^\s*>\s?/.test(line)) {
        html += '<blockquote>' + inline(line.replace(/^\s*>\s?/, '')) + '</blockquote>';
        return;
      }
      if (/^\s*---+\s*$/.test(line)) { html += '<hr>'; return; }
      if (!line.trim()) return;
      html += '<p>' + inline(line) + '</p>';
    });

    if (inList) html += '</' + inList + '>';
    if (inCode) html += '</code></pre>';
    return html || '<p class="muted">' + escapeHtml(t('emptyNote')) + '</p>';
  }

  function inline(text) {
    return escapeHtml(text)
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
      .replace(/(^|[^*])\*([^*]+)\*/g, '$1<em>$2</em>')
      .replace(/~~([^~]+)~~/g, '<del>$1</del>')
      /* Le libelle seul: une App ne doit pas fabriquer de lien sortant depuis
         du texte que l'agent a ecrit. */
      .replace(/\[([^\]]*)\]\(([^)\s]+)\)/g, '$1');
  }

  /* --------------------------------------------------------- Side panels */

  function renderDatasets() {
    var list = element('datasetList');
    var entries = store.list();
    list.textContent = '';
    element('datasetEmpty').hidden = entries.length > 0;

    entries.forEach(function(entry){
      var item = document.createElement('li');
      item.className = 'dataset';

      var head = document.createElement('div');
      head.className = 'dataset-head';
      var name = document.createElement('code');
      name.textContent = entry.name;
      head.appendChild(name);

      var actions = document.createElement('span');
      actions.className = 'dataset-actions';

      var insert = document.createElement('button');
      insert.type = 'button';
      insert.className = 'link-button';
      insert.textContent = t('insertCell');
      insert.title = t('insertCellHint', { name: entry.name });
      insert.addEventListener('click', function(){
        var cell = notebook.addCell({ source: t('previewCellCode', { name: entry.name }) });
        runIfReady(cell.id);
      });
      actions.appendChild(insert);

      var remove = document.createElement('button');
      remove.type = 'button';
      remove.className = 'link-button danger';
      remove.textContent = t('remove');
      remove.addEventListener('click', function(){
        store.remove(entry.name);
        notebook.touch({ type: 'data.remove' });
        renderDatasets();
        renderQuota();
        schedulePersist();
      });
      actions.appendChild(remove);
      head.appendChild(actions);
      item.appendChild(head);

      var meta = document.createElement('p');
      meta.className = 'muted small';
      meta.textContent = t('datasetMeta', {
        rows: entry.rows,
        columns: entry.columns,
        size: i18n.bytes(entry.bytes)
      });
      item.appendChild(meta);

      var schema = document.createElement('p');
      schema.className = 'schema';
      entry.schema.slice(0, 12).forEach(function(column){
        var chip = document.createElement('span');
        chip.className = 'schema-chip is-' + column.dtype;
        chip.textContent = column.name;
        chip.title = t('columnType', { type: t('dtype_' + column.dtype) }) +
          (column.missing ? ' · ' + t('missingCount', { count: column.missing }) : '');
        schema.appendChild(chip);
      });
      if (entry.schema.length > 12) {
        var more = document.createElement('span');
        more.className = 'schema-chip';
        more.textContent = '+' + (entry.schema.length - 12);
        schema.appendChild(more);
      }
      item.appendChild(schema);

      if (!entry.persistable) {
        var warning = document.createElement('p');
        warning.className = 'muted small warn';
        warning.textContent = t('datasetSessionOnly');
        item.appendChild(warning);
      }

      list.appendChild(item);
    });
  }

  /* Installe un paquet PyPI dans le noyau en cours.
   *
   * micropip n'existe que sur le chemin reseau: un noyau vendorise embarque ce
   * qu'on lui a mis et rien d'autre, et un noyau interne ne parle pas Python du
   * tout. Le dire est plus utile qu'un bouton qui echoue sans expliquer. */
  function installerPaquet() {
    var champ = element('packageName');
    var statut = element('packageStatus');
    var nom = (champ.value || '').trim();
    if (!nom) return;
    if (!kernel || typeof kernel.installer !== 'function' || kernel.source !== 'cdn') {
      statut.textContent = t('packagesOffline');
      return;
    }
    var bouton = element('packageInstall');
    bouton.disabled = true;
    statut.textContent = t('packagesInstalling', { name: nom });
    kernel.installer([nom]).then(function(bilan){
      if (bilan.failed.length) {
        statut.textContent = t('packagesFailed', {
          name: bilan.failed[0].name,
          error: bilan.failed[0].error
        });
      } else {
        statut.textContent = t('packagesInstalled', { name: nom });
        champ.value = '';
      }
      renderPackages();
    }).catch(function(erreur){
      statut.textContent = t('packagesFailed', {
        name: nom,
        error: String((erreur && erreur.message) || erreur)
      });
    }).then(function(){
      bouton.disabled = false;
    });
  }

  /* Ce que le noyau porte vraiment, roues chargees puis ajouts par micropip.
   * La liste vient du noyau et non d'un registre tenu a part: deux comptes
   * separes finissent toujours par diverger, et c'est celui qui ment qu'on lit. */
  /* Remplace le noyau en cours d'execution.
   *
   * Tout ce qui vivait dans l'ancien espace de noms disparait, et c'est
   * inevitable: deux interpreteurs ne partagent pas leurs variables. On le dit
   * plutot que de laisser l'utilisateur decouvrir que ses variables se sont
   * evaporees. Si le nouveau noyau ne demarre pas, on garde l'ancien. */
  async function basculerNoyau(voulu) {
    var ancien = kernel;
    var candidat = Kernels.create(voulu, store);
    try {
      await candidat.init(function(progress, messageKey){
        setBoot(progress, messageKey);
        reportStatus('loading', messageKey, progress);
      });
    } catch (erreur) {
      toast(t('kernelPythonFailed', { message: String(erreur.message || erreur) }), 'error');
      kernel = ancien;
      return false;
    }
    kernel = candidat;
    if (ancien && typeof ancien.reset === 'function') {
      try { ancien.reset(); } catch (e) {}
    }
    setBoot(100, 'kernelReady');
    await reportStatus('ready', 'kernelReady', 100);
    renderPackages();
    renderVariables();
    return true;
  }

  function renderPackages() {
    var liste = element('packageList');
    if (!liste) return;
    liste.textContent = '';
    var noms = [];
    if (kernel) {
      noms = (kernel.packages || []).concat(kernel.installed || []);
    }
    noms = noms.filter(function(nom, index){ return noms.indexOf(nom) === index; }).sort();
    var bouton = element('packageInstall');
    if (bouton) bouton.disabled = !kernel || kernel.source !== 'cdn';
    if (!noms.length) {
      var vide = document.createElement('li');
      vide.className = 'muted small';
      vide.textContent = t('packagesNone');
      liste.appendChild(vide);
      return;
    }
    noms.forEach(function(nom){
      var item = document.createElement('li');
      var code = document.createElement('code');
      code.textContent = nom;
      item.appendChild(code);
      liste.appendChild(item);
    });
  }

  function renderVariables() {
    var list = element('variableList');
    list.textContent = '';
    var variables = kernel ? kernel.variables() : [];
    element('variableEmpty').hidden = variables.length > 0;

    variables.forEach(function(variable){
      var item = document.createElement('li');
      var name = document.createElement('code');
      name.textContent = variable.name;
      item.appendChild(name);
      var type = document.createElement('span');
      type.className = 'badge';
      type.textContent = variable.type;
      item.appendChild(type);
      var summary = document.createElement('span');
      summary.className = 'muted small';
      summary.textContent = variable.summary;
      item.appendChild(summary);
      list.appendChild(item);
    });
  }

  function renderQuota() {
    var used = store.bytes();
    var ratio = Math.min(100, Math.round((used / storageLimit) * 100));
    element('quotaFill').style.width = ratio + '%';
    element('quotaFill').className = 'quota-fill' + (ratio > 80 ? ' is-high' : '');
    element('quotaValue').textContent = i18n.bytes(used) + ' / ' + i18n.bytes(storageLimit);
    var note = element('quotaNote');
    if (droppedOutputs) {
      note.textContent = t('quotaOutputsDropped', { count: droppedOutputs });
    } else if (skippedDatasets.length) {
      note.textContent = t('quotaSkipped', {
        count: skippedDatasets.length,
        names: skippedDatasets.map(function(entry){ return entry.name; }).join(', ')
      });
    } else {
      note.textContent = t('quotaNote');
    }
  }

  function renderSummary() {
    var codeCells = notebook.cells.filter(function(cell){ return cell.type === 'code'; }).length;
    element('cellSummary').textContent = t('cellCount', { count: codeCells });
  }

  function paintToolbar() {
    var busy = runner && runner.busy();
    element('stopBtn').disabled = !busy;
    element('runAllBtn').disabled = !!busy || !kernel || !kernel.ready;
  }

  /* -------------------------------------------------------------- Events */

  function runIfReady(cellId) {
    if (!kernel || !kernel.ready) { toast(t('kernelNotReady'), 'error'); return null; }
    if (notebook.kernelId !== kernel.id) { toast(t('kernelLanguageMismatch'), 'error'); return null; }
    try {
      return runner.submit(cellId);
    } catch (error) {
      toast(error.message, 'error');
      return null;
    }
  }

  function bindEvents() {
    element('notebookTitle').addEventListener('input', function(event){
      notebook.title = event.target.value.slice(0, 120);
      notebook.touch({ type: 'title' });
      if (sdk) sdk.ui.setTitle(notebook.title || t('appName')).catch(function(){});
    });

    element('notebookSelect').addEventListener('change', function(event){
      var id = event.target.value;
      openNotebook(id).then(function(){
        toast(t('notebookOpened', { title: notebook.title || t('untitledNotebook') }));
      }).catch(function(error){
        toast(String(error.message || error), 'error');
        renderLibrary();
      });
    });

    element('newNotebookBtn').addEventListener('click', function(){
      newNotebook('').then(function(){
        element('notebookTitle').focus();
      }).catch(function(error){
        toast(String(error.message || error), 'error');
      });
    });

    element('deleteNotebookBtn').addEventListener('click', function(){
      var title = notebook.title || t('untitledNotebook');
      confirmer(t('deleteNotebookConfirm', { title: title }))
        .then(function(accord){
          if (!accord) return null;
          return deleteNotebook(notebook.id).then(function(){
            toast(t('notebookDeleted', { title: title }));
          });
        })
        .catch(function(error){
          toast(String(error.message || error), 'error');
        });
    });

    element('addCodeBtn').addEventListener('click', function(){
      focusNextAdded = true;
      notebook.addCell({ source: '' });
    });
    element('emptyAddBtn').addEventListener('click', function(){
      focusNextAdded = true;
      notebook.addCell({ source: '' });
    });
    element('addNoteBtn').addEventListener('click', function(){
      focusNextAdded = true;
      notebook.addCell({ type: 'markdown', source: '' });
    });
    element('sampleBtn').addEventListener('click', loadSample);

    element('runAllBtn').addEventListener('click', function(){
      if (!kernel || !kernel.ready) { toast(t('kernelNotReady'), 'error'); return; }
      if (notebook.kernelId !== kernel.id) { toast(t('kernelLanguageMismatch'), 'error'); return; }
      runner.runAll();
    });
    element('stopBtn').addEventListener('click', function(){ runner.cancelAll(); });
    element('clearBtn').addEventListener('click', function(){ notebook.clearOutputs(); });

    element('importBtn').addEventListener('click', function(){ element('fileInput').click(); });
    element('fileInput').addEventListener('change', function(event){
      importFiles(Array.prototype.slice.call(event.target.files));
      event.target.value = '';
    });
    element('exportBtn').addEventListener('click', exportNotebook);

    var pane = element('cells');
    pane.addEventListener('click', onCellClick);
    pane.addEventListener('input', onCellInput);
    pane.addEventListener('keydown', onCellKeydown);

    notebookPane().addEventListener('scroll', updateStickiness, { passive: true });

    element('zoomInBtn').addEventListener('click', function(){ decalerZoom(1); });
    element('zoomOutBtn').addEventListener('click', function(){ decalerZoom(-1); });
    element('zoomValue').addEventListener('click', function(){ appliquerZoom(100, true); });

    element('inspectorToggle').addEventListener('click', function(event){
      event.stopPropagation();
      appliquerInspecteur(!inspecteurReplie, true);
    });
    document.querySelectorAll('.tab').forEach(function(tab){
      tab.addEventListener('click', function(){
        /* Choisir un onglet veut dire vouloir le lire: on deplie. */
        if (inspecteurReplie) appliquerInspecteur(false, true);
        document.querySelectorAll('.tab').forEach(function(other){
          var active = other === tab;
          other.classList.toggle('is-active', active);
          other.setAttribute('aria-selected', String(active));
        });
        ['data', 'vars', 'packages', 'agent'].forEach(function(name){
          var panel = element('panel' + name.charAt(0).toUpperCase() + name.slice(1));
          panel.hidden = name !== tab.dataset.panel;
          panel.classList.toggle('is-active', name === tab.dataset.panel);
        });
      });
    });

    element('packageInstall').addEventListener('click', installerPaquet);
    element('packageName').addEventListener('keydown', function(event){
      if (event.key === 'Enter') installerPaquet();
    });

    element('refreshAgentsBtn').addEventListener('click', refreshAgents);
    element('agentRunBtn').addEventListener('click', runAgent);
    element('agentSelect').addEventListener('change', function(event){
      element('agentRunBtn').disabled = !event.target.value || agentBusy;
    });

    bindDropTarget();

    if (sdk && sdk.agents) refreshAgents();

    window.addEventListener('beforeunload', function(){
      if (saveTimer) { clearTimeout(saveTimer); persist(); }
    });
  }

  function onCellClick(event) {
    var button = event.target.closest('[data-action]');
    if (!button) return;
    var article = button.closest('.cell');
    if (!article) return;
    var cellId = article.dataset.cellId;
    var action = button.dataset.action;

    if (action === 'run') {
      var pending = runner.jobFor(cellId);
      if (pending) runner.cancel(pending.id);
      else runIfReady(cellId);
      return;
    }
    if (action === 'delete') {
      notebook.deleteCell(cellId);
      return;
    }
    if (action === 'toggleType') {
      var cell = notebook.cell(cellId);
      notebook.updateCell(cellId, { type: cell.type === 'code' ? 'markdown' : 'code' });
      return;
    }
    if (action === 'moveUp' || action === 'moveDown') {
      var index = notebook.index(cellId);
      notebook.moveCell(cellId, index + (action === 'moveUp' ? -1 : 1));
    }
  }

  function onCellInput(event) {
    var editor = event.target.closest('[data-role="editor"]');
    if (!editor) return;
    var article = editor.closest('.cell');
    autosize(editor);
    var cell = notebook.cell(article.dataset.cellId);
    cell.source = editor.value;
    cell.updatedAt = Date.now();
    if (cell.status === 'ok' || cell.status === 'error') {
      cell.status = 'stale';
      article.className = article.className.replace(/is-(ok|error)/, 'is-stale');
    }
    notebook.revision += 1;
    schedulePersist();
  }

  function onCellKeydown(event) {
    var editor = event.target.closest('[data-role="editor"]');
    if (!editor) return;
    var article = editor.closest('.cell');
    var cellId = article.dataset.cellId;

    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      runIfReady(cellId);
      return;
    }
    if (event.key === 'Enter' && event.shiftKey) {
      event.preventDefault();
      runIfReady(cellId);
      var index = notebook.index(cellId);
      if (index === notebook.cells.length - 1) notebook.addCell({ source: '' });
      return;
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      var start = editor.selectionStart;
      var end = editor.selectionEnd;
      editor.value = editor.value.slice(0, start) + '  ' + editor.value.slice(end);
      editor.setSelectionRange(start + 2, start + 2);
      editor.dispatchEvent(new Event('input', { bubbles: true }));
    }
  }

  function bindDropTarget() {
    var pane = element('shell');
    var hint = element('dropHint');
    var depth = 0;

    pane.addEventListener('dragenter', function(event){
      if (!event.dataTransfer || event.dataTransfer.types.indexOf('Files') === -1) return;
      event.preventDefault();
      depth += 1;
      hint.hidden = false;
    });
    pane.addEventListener('dragover', function(event){
      if (!event.dataTransfer || event.dataTransfer.types.indexOf('Files') === -1) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = 'copy';
    });
    pane.addEventListener('dragleave', function(){
      depth = Math.max(0, depth - 1);
      if (!depth) hint.hidden = true;
    });
    pane.addEventListener('drop', function(event){
      if (!event.dataTransfer || !event.dataTransfer.files.length) return;
      event.preventDefault();
      depth = 0;
      hint.hidden = true;
      importFiles(Array.prototype.slice.call(event.dataTransfer.files));
    });
  }

  /* ------------------------------------------------------------ Datasets */

  function readFile(file) {
    return new Promise(function(resolve, reject){
      var reader = new FileReader();
      reader.onload = function(){ resolve(String(reader.result)); };
      reader.onerror = function(){ reject(new Error(file.name)); };
      reader.readAsText(file);
    });
  }

  async function importFiles(files) {
    var imported = [];
    for (var i = 0; i < files.length; i += 1) {
      var file = files[i];
      try {
        var text = await readFile(file);
        var name;
        if (/\.json$/i.test(file.name)) {
          var parsed = JSON.parse(text);
          var rows = Array.isArray(parsed) ? parsed : (Array.isArray(parsed.data) ? parsed.data : null);
          if (!rows) throw new Error(t('jsonShape'));
          name = store.addRows(file.name, rows, 'file');
        } else {
          name = store.addCsv(file.name, text, { source: 'file' });
        }
        imported.push(name);
        notebook.touch({ type: 'data.add' });
      } catch (error) {
        toast(t('importFailed', { name: file.name, message: String(error.message || error) }), 'error');
      }
    }

    if (!imported.length) return;
    renderDatasets();
    renderQuota();
    schedulePersist();
    toast(t('importDone', { count: imported.length, names: imported.join(', ') }));

    var first = store.get(imported[0]);
    notebook.addCell({
      source: t('previewCellCode', { name: imported[0] }),
      type: 'code'
    });
    if (first) runIfReady(notebook.cells[notebook.cells.length - 1].id);
  }

  function loadSample() {
    var rows = [];
    var products = ['Ruche', 'Cadre', 'Enfumoir', 'Combinaison'];
    var regions = ['Nord', 'Sud', 'Est', 'Ouest'];
    var seed = 7;
    function random() {
      seed = (seed * 1103515245 + 12345) % 2147483648;
      return seed / 2147483648;
    }
    for (var year = 2021; year <= 2024; year += 1) {
      products.forEach(function(product){
        regions.forEach(function(region){
          rows.push({
            annee: year,
            produit: product,
            region: region,
            quantite: Math.round(20 + random() * 180),
            montant: Math.round((80 + random() * 420) * 100) / 100
          });
        });
      });
    }
    var name = store.addRows('ventes', rows, 'sample');
    renderDatasets();
    renderQuota();
    var cell = notebook.addCell({ source: t('sampleCellCode_' + kernel.language, { name: name }) });
    runIfReady(cell.id);
    schedulePersist();
  }

  function exportNotebook() {
    var lines = ['# ' + (notebook.title || t('untitledNotebook')), ''];
    notebook.cells.forEach(function(cell){
      if (cell.type === 'markdown') {
        lines.push(cell.source, '');
        return;
      }
      lines.push('```' + (notebook.kernelId === 'python' ? 'python' : 'studio'), cell.source, '```', '');
      cell.outputs.forEach(function(output){
        if (output.kind === 'stream' || output.kind === 'value') lines.push('    ' + output.text.split('\n').join('\n    '), '');
        if (output.kind === 'table') {
          lines.push('| ' + output.columns.join(' | ') + ' |');
          lines.push('| ' + output.columns.map(function(){ return '---'; }).join(' | ') + ' |');
          output.rows.slice(0, 30).forEach(function(row){
            lines.push('| ' + row.map(formatCell).join(' | ') + ' |');
          });
          lines.push('');
        }
        if (output.kind === 'chart') lines.push('_' + t('chartPlaceholder', { title: output.title || output.chart }) + '_', '');
        if (output.kind === 'plotly') lines.push('_' + t('chartPlaceholder', { title: output.title || 'plotly' }) + '_', '');
      });
      if (cell.error) lines.push('> ' + cell.error.message, '');
    });

    var filename = (notebook.title || 'notebook').replace(/[^A-Za-z0-9_-]+/g, '_').slice(0, 60) + '.md';
    triggerDownload(
      URL.createObjectURL(new Blob([lines.join('\n')], { type: 'text/markdown' })),
      filename,
      true
    );
  }

  /* --------------------------------------------------------------- Agent */

  function refreshAgents() {
    var select = element('agentSelect');
    var status = element('agentStatus');
    if (!sdk || !sdk.agents) {
      status.textContent = t('agentUnavailable');
      return;
    }
    sdk.agents.list().then(function(agents){
      select.textContent = '';
      if (!agents || !agents.length) {
        var option = document.createElement('option');
        option.value = '';
        option.textContent = t('permissionRequired');
        select.appendChild(option);
        element('agentRunBtn').disabled = true;
        status.textContent = t('agentPermission');
        return;
      }
      agents.forEach(function(agent){
        var option = document.createElement('option');
        option.value = agent.id;
        option.textContent = (agent.avatar ? agent.avatar + ' ' : '') + agent.name;
        select.appendChild(option);
      });
      element('agentRunBtn').disabled = agentBusy;
      status.textContent = t('agentIdle');
    }).catch(function(error){
      status.textContent = String(error.message || error);
    });
  }

  function runAgent() {
    if (agentBusy) return;
    var agentId = element('agentSelect').value;
    var task = element('agentPrompt').value.trim();
    var status = element('agentStatus');
    if (!agentId) { status.textContent = t('agentPermission'); return; }
    if (!task) { status.textContent = t('agentTaskRequired'); return; }

    agentBusy = true;
    element('agentRunBtn').disabled = true;
    status.textContent = t('agentWorking');

    var prompt = t('agentPromptTemplate', {
      task: task,
      language: kernel.language,
      datasets: store.list().map(function(entry){ return entry.name; }).join(', ') || t('noneShort')
    });

    sdk.agents.act(agentId, agentSeat, 'notebook.state', prompt).then(function(result){
      status.textContent = (result.model || 'Agent') + ' · ' + (result.text || t('agentDone'));
    }).catch(function(error){
      status.textContent = String(error.message || error);
    }).then(function(){
      agentBusy = false;
      element('agentRunBtn').disabled = !element('agentSelect').value;
    });
  }

  /* ------------------------------------------------------------ Actions */

  function requireRevision(args) {
    if (args.revision !== notebook.revision) {
      throw new Error('Stale revision ' + args.revision + ', current is ' + notebook.revision +
        '. Read notebook.state again and rebuild the call.');
    }
  }

  /* Callers reliably invent argument names: "content" for source, "id" for
   * cellId. The host would answer with a generic schema rejection, which costs
   * several turns to diagnose, so the aliases are accepted and anything else
   * fails with the exact names spelled out. */
  function pick(args, names, label) {
    for (var i = 0; i < names.length; i += 1) {
      var value = args[names[i]];
      if (value !== undefined && value !== null && value !== '') return value;
    }
    if (args[names[0]] !== undefined) return args[names[0]];
    throw new Error('Missing argument "' + names[0] + '" for ' + label +
      '. Expected: ' + names[0] + (names.length > 1 ? ' (also accepted: ' + names.slice(1).join(', ') + ')' : '') + '.');
  }

  function datasetSummaries() {
    return store.list().map(function(entry){
      return {
        name: entry.name,
        rows: entry.rows,
        columns: entry.columns,
        source: entry.source,
        schema: entry.schema.map(function(column){
          return { name: column.name, dtype: column.dtype, missing: column.missing };
        })
      };
    });
  }

  function kernelSnapshot() {
    var description = kernel.describe();
    return {
      id: description.id,
      language: description.language,
      notebookKernelId: notebook.kernelId,
      compatible: notebook.kernelId === kernel.id,
      version: description.version || '',
      ready: !!description.ready,
      state: kernelStatus.state,
      progress: kernelStatus.progress,
      packages: description.packages || [],
      /* The full syntax reference travels with the status, so a caller never
       * has to guess the language or go reading the package. */
      reference: description.reference || null,
      syntaxHint: description.syntaxHint || ''
    };
  }

  /* The short form repeated on every notebook.state: enough to write a correct
   * cell, small enough not to crowd the byte budget. */
  function languageBrief() {
    var description = kernel.describe();
    var reference = description.reference || {};
    return {
      language: description.language,
      notebookKernelId: notebook.kernelId,
      compatible: notebook.kernelId === kernel.id,
      ready: !!description.ready,
      hint: description.syntaxHint || '',
      example: reference.example || '',
      fullReference: 'kernel.status'
    };
  }

  function stateSnapshot(includeOutputs) {
    var snapshot = Notebooks.summarize(notebook, {
      kernel: languageBrief(),
      datasets: datasetSummaries(),
      budget: 40 * 1024
    });
    if (includeOutputs === false) {
      snapshot.cells.forEach(function(cell){ cell.outputs = []; });
    }
    return snapshot;
  }

  function jobSnapshot(job) {
    return {
      jobId: job.id,
      cellId: job.cellId,
      status: job.status,
      durationMs: job.durationMs,
      revision: notebook.revision,
      outputs: job.status === 'queued' || job.status === 'running'
        ? []
        : job.outputs.map(function(output){ return Notebooks.summarizeOutput(output, {}); }),
      error: job.error ? {
        message: String(job.error.message).slice(0, 400),
        line: job.error.line || 0,
        hint: String(job.error.hint || '').slice(0, 200)
      } : null
    };
  }

  function registerActions() {
    sdk.actions.register('storage.status', function(){
      return {state:saveState,error:saveError,savedAt:savedAt,savedRevision:savedRevision,currentRevision:notebook.revision,
        quotaBytes:storageLimit,datasetSaveLimitBytes:DATASET_PERSIST_BYTES,
        datasets:store.list().map(function(d){return {name:d.name,estimatedBytes:d.bytes,persistable:d.persistable};}),
        skippedDatasets:skippedDatasets,outputsDropped:droppedOutputs,
        note:'Only the open notebook retains saved outputs. A quota error leaves unsaved data in memory: export before closing. Multi-key saves are not power-loss atomic.'};
    });
    sdk.actions.register('storage.flush', async function(args){
      requireRevision(args);
      clearTimeout(saveTimer); saveTimer = null;
      await persist();
      if (saveError) throw new Error('Save failed: ' + saveError);
      return {state:saveState,savedAt:savedAt,savedRevision:savedRevision,revision:notebook.revision,outputsDropped:droppedOutputs,skippedDatasets:skippedDatasets};
    });
    sdk.actions.register('kernel.status', function(){
      return kernelSnapshot();
    });

    /* Le pont fichiers, expose au noyau Python par une seule fonction.
       Le noyau ignore le SDK et doit continuer de l'ignorer: lui passer l'objet
       entier le rendrait dependant d'une surface qui bouge, alors qu'il n'a
       besoin que d'un verbe et d'une charge utile. */
    window.__dsFiles = function(op, payload){
      var args = payload || {};
      if (op === 'read') return sdk.files.read(args.path);
      if (op === 'write') return sdk.files.write(args.path, args.content, args.append);
      if (op === 'delete') return sdk.files.delete(args.path);
      if (op === 'exists') return sdk.files.exists(args.path);
      if (op === 'list') return sdk.files.list(args.path);
      if (op === 'mkdir') return sdk.files.mkdir(args.path);
      return Promise.reject(new Error('unknown file operation: ' + op));
    };

    /* Le carnet, depuis Python. Aucune capacite ne le garde, et c'est voulu:
       le noyau tourne dans l'App, donc une cellule qui enregistre un dataset est
       l'App qui ecrit son propre etat. Une permission n'aurait rien a refuser,
       et une case qui n'accorde rien est pire que pas de case. */
    window.__dsNotebook = function(op, payload){
      var args = payload || {};
      if (op === 'datasets') {
        return Promise.resolve(Object.keys(store.all()).sort());
      }
      if (op === 'save') {
        return Promise.resolve()
          .then(function(){
            return store.addCsv(args.name, args.text, { delimiter: args.delimiter || ',', source: 'python' });
          })
          .then(function(entree){
            renderDatasets();
            if (kernel && kernel.pushDatasets) kernel.pushDatasets(true);
            return { name: entree.name, rows: entree.frame.length };
          });
      }
      if (op === 'remove') {
        store.remove(args.name);
        renderDatasets();
        if (kernel && kernel.pushDatasets) kernel.pushDatasets(true);
        return Promise.resolve({ removed: true });
      }
      if (op === 'cell') {
        var cellule = notebook.addCell({ source: String(args.source || ''), type: args.type || 'code' });
        render();
        return Promise.resolve({ cellId: cellule.id });
      }
      if (op === 'state') {
        return Promise.resolve({
          notebookId: notebook.id,
          title: notebook.title,
          cells: notebook.cells.length,
          datasets: Object.keys(store.all()).sort()
        });
      }
      return Promise.reject(new Error('unknown notebook operation: ' + op));
    };

    window.__dsMemory = function(op, payload){
      var args = payload || {};
      if (op === 'search') return sdk.memory.search(args.query, args.limit);
      if (op === 'read') return sdk.memory.read(args.nodeId);
      if (op === 'list') return sdk.memory.list();
      if (op === 'propose') return sdk.memory.propose(args.nodeId, args.content, args.tags);
      if (op === 'write') return sdk.memory.write(args.nodeId, args.content, args.tags);
      return Promise.reject(new Error('unknown memory operation: ' + op));
    };

    sdk.actions.register('packages.list', function(){
      return {
        source: kernel ? (kernel.source || 'builtin') : 'none',
        canInstall: !!(kernel && kernel.source === 'cdn'),
        packages: ((kernel && kernel.packages) || []).concat((kernel && kernel.installed) || [])
      };
    });

    /* Installer refuse plutot que d'essayer quand le noyau ne peut pas: un
       agent a qui l'on repond "ok" sur une installation impossible ecrira la
       cellule suivante avec un import qui echouera loin de la cause. */
    sdk.actions.register('packages.install', function(args){
      var noms = Array.isArray(args.packages) ? args.packages : [args.name];
      noms = noms.filter(function(n){ return typeof n === 'string' && n.trim(); });
      if (!noms.length) throw new Error('packages.install needs name or packages');
      if (!kernel || typeof kernel.installer !== 'function' || kernel.source !== 'cdn') {
        throw new Error(
          'This kernel cannot install packages. It is either the built-in language kernel ' +
          'or a vendored Python runtime: only the network-backed Python kernel has micropip.'
        );
      }
      return kernel.installer(noms).then(function(bilan){
        renderPackages();
        return bilan;
      });
    });

    sdk.actions.register('notebook.state', function(args){
      return stateSnapshot(args.includeOutputs);
    });

    sdk.actions.register('notebook.list', function(){
      return {
        activeId: notebook.id,
        notebooks: library.notebooks.map(function(entry){
          return {
            id: entry.id,
            title: entry.title,
            cells: entry.cellCount,
            codeCells: entry.codeCells,
            updatedAt: new Date(entry.updatedAt).toISOString(),
            active: entry.id === notebook.id
          };
        })
      };
    });

    sdk.actions.register('notebook.new', function(args){
      requireRevision(args);
      var title = typeof args.title === 'string' ? args.title.slice(0, 120) : '';
      return newNotebook(title).then(function(created){
        return { notebookId: created.id, title: created.title, revision: notebook.revision };
      });
    });

    sdk.actions.register('notebook.open', function(args){
      requireRevision(args);
      return openNotebook(pick(args, ['notebookId', 'id'], 'notebook.open')).then(function(){
        return {
          notebookId: notebook.id,
          title: notebook.title,
          cells: notebook.cells.length,
          revision: notebook.revision,
          note: 'The kernel was reset: variables from the previous notebook are gone. Re-run the cells that define what you need.'
        };
      });
    });

    sdk.actions.register('notebook.rename', function(args){
      requireRevision(args);
      notebook.title = String(args.title).slice(0, 120);
      notebook.touch({ type: 'title' });
      element('notebookTitle').value = notebook.title;
      renderLibrary();
      if (sdk.ui.setTitle) sdk.ui.setTitle(notebook.title || t('appName')).catch(function(){});
      return { notebookId: notebook.id, title: notebook.title, revision: notebook.revision };
    });

    sdk.actions.register('notebook.delete', function(args){
      requireRevision(args);
      return deleteNotebook(pick(args, ['notebookId', 'id'], 'notebook.delete')).then(function(removed){
        return { deleted: removed, activeId: notebook.id, revision: notebook.revision };
      });
    });

    sdk.actions.register('notebook.clear', function(args){
      requireRevision(args);
      if (args.outputsOnly) {
        notebook.clearOutputs();
        return { cleared: 'outputs', revision: notebook.revision };
      }
      notebook.cells = [];
      notebook.touch({ type: 'notebook.clear' });
      renderCells();
      return { cleared: 'cells', revision: notebook.revision };
    });

    sdk.actions.register('cell.add', function(args){
      requireRevision(args);
      var cell = notebook.addCell({
        type: args.type === 'markdown' ? 'markdown' : 'code',
        source: pick(args, ['source', 'content'], 'cell.add'),
        index: args.index,
        author: 'agent'
      });
      return { cellId: cell.id, index: notebook.index(cell.id), revision: notebook.revision };
    });

    sdk.actions.register('cell.update', function(args){
      requireRevision(args);
      notebook.updateCell(pick(args, ['cellId', 'id'], 'cell.update'), {
        source: pick(args, ['source', 'content'], 'cell.update'),
        mode: args.mode === 'append' ? 'append' : 'replace',
        type: args.type,
        author: 'agent'
      });
      return { cellId: pick(args, ['cellId', 'id'], 'cell.update'), revision: notebook.revision };
    });

    sdk.actions.register('cell.delete', function(args){
      requireRevision(args);
      notebook.deleteCell(pick(args, ['cellId', 'id'], 'cell.delete'));
      return { revision: notebook.revision };
    });

    sdk.actions.register('cell.run', function(args){
      requireRevision(args);
      if (!kernel.ready) throw new Error('The kernel is not ready. Call app_wait first.');
      if (notebook.kernelId !== kernel.id) throw new Error('Notebook/kernel language mismatch. Do not run Python as studio code. Restore the required runtime or create a new notebook for the active kernel.');
      var job = runner.submit(pick(args, ['cellId', 'id'], 'cell.run'));
      return {
        jobId: job.id,
        cellId: job.cellId,
        status: job.status,
        revision: notebook.revision,
        pollHint: 'Poll job.status with this jobId until status is ok, error or cancelled.'
      };
    });

    sdk.actions.register('job.status', function(args){
      var job = runner.job(args.jobId);
      if (!job) throw new Error('Unknown job "' + args.jobId + '"');
      return jobSnapshot(job);
    });

    sdk.actions.register('job.cancel', function(args){
      var job = runner.cancel(args.jobId);
      return jobSnapshot(job);
    });

    /* The extension's flush button, as an action: empties the namespace and
     * leaves the code and the datasets alone. */
    sdk.actions.register('kernel.reset', function(args){
      requireRevision(args);
      runner.cancelAll();
      return Promise.resolve(kernel.reset()).then(function(){
        notebook.cells.forEach(function(cell){
          if (cell.status === 'ok') cell.status = 'stale';
        });
        notebook.touch({ type: 'kernel.reset' });
        renderVariables();
      renderPackages();
        return {
          reset: true,
          revision: notebook.revision,
          note: 'The namespace is empty. Cell code and saved results are kept; re-run the cells that define the variables you need.'
        };
      });
    });

    sdk.actions.register('vars.list', function(){
      return { variables: kernel.variables(), language: kernel.language };
    });

    sdk.actions.register('vars.set', function(args){
      requireRevision(args);
      var name = String(args.name);
      if (!/^[A-Za-z_][A-Za-z0-9_]{0,63}$/.test(name)) {
        throw new Error('"' + name + '" is not a usable variable name');
      }
      var value = args.value;
      if (args.rows !== undefined && args.rows !== null) {
        value = Frames.fromRows(JSON.parse(args.rows));
      } else if (typeof value === 'string' && args.parse) {
        value = Csv.parse(value);
      }
      kernel.setVariable(name, value);
      renderVariables();
      renderPackages();
      return { name: name, variables: kernel.variables(), revision: notebook.revision };
    });

    sdk.actions.register('vars.delete', function(args){
      requireRevision(args);
      var removed = kernel.deleteVariable(String(args.name));
      if (!removed) throw new Error('No variable named "' + args.name + '"');
      renderVariables();
      renderPackages();
      return { deleted: args.name, variables: kernel.variables(), revision: notebook.revision };
    });

    /* Read a dataset back as delimited text, so a result computed here can be
     * carried out of the App. */
    sdk.actions.register('data.export', function(args){
      var entry = store.get(args.name);
      if (!entry) {
        throw new Error('No dataset named "' + args.name + '". Available: ' +
          (store.list().map(function(item){ return item.name; }).join(', ') || 'none'));
      }
      var page = Csv.exportPage(entry.frame,args.name,args.offset,args.limit,args.delimiter);
      page.revision = notebook.revision;
      return page;
    });

    sdk.actions.register('data.list', function(){
      return { datasets: datasetSummaries(), revision: notebook.revision };
    });

    sdk.actions.register('data.preview', function(args){
      var entry = store.get(args.name);
      if (!entry) {
        throw new Error('No dataset named "' + args.name + '". Available: ' +
          (store.list().map(function(item){ return item.name; }).join(', ') || 'none'));
      }
      var limit = Math.max(1, Math.min(args.limit || 10, 50));
      return {
        name: args.name,
        rows: entry.frame.length,
        columns: entry.frame.columns,
        schema: entry.frame.schema(),
        preview: entry.frame.rows(limit).map(function(row){
          return entry.frame.columns.map(function(column){
            var value = row[column];
            return value instanceof Date ? value.toISOString() : value;
          });
        })
      };
    });

    sdk.actions.register('data.add', function(args){
      requireRevision(args);
      var frame = Csv.parse(args.text, args.delimiter ? { delimiter: args.delimiter } : undefined);
      var name = store.add(store.uniqueName(args.name), frame, 'agent');
      notebook.touch({ type: 'data.add' });
      renderDatasets();
      renderQuota();
      schedulePersist();
      var entry = store.get(name);
      return {
        name: name,
        rows: entry.frame.length,
        columns: entry.frame.columns,
        schema: entry.frame.schema(),
        revision: notebook.revision
      };
    });

    sdk.actions.register('data.remove', function(args){
      requireRevision(args);
      if (!store.remove(args.name)) throw new Error('No dataset named "' + args.name + '"');
      notebook.touch({ type: 'data.remove' });
      renderDatasets();
      renderQuota();
      schedulePersist();
      return { removed: args.name, revision: notebook.revision };
    });
  }

  /* --------------------------------------------------------------- Boot */

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function(){ start(); });
  } else {
    start();
  }
})();
