/**
 * LaRuche browser bridge, service worker side.
 *
 * Connects to the node at ws://127.0.0.1:<port>/ws/navigateur and executes the
 * commands it receives. The vocabulary is deliberately small: navigate, eval,
 * screenshot, glow, tap, cdp, tabs, select, close. Everything clever (mapping a
 * page, clicking a ref, drawing the indicator, recording the console) is
 * JavaScript built by LaRuche and passed in, so there is exactly one
 * implementation of it and this file rarely changes. `cdp` is the one raw
 * passthrough, and it exists for Input events alone: a key press synthesised
 * from page script is untrusted and half the web ignores it.
 *
 * Why chrome.debugger rather than chrome.scripting: running script text that
 * arrives at runtime means `eval` or `new Function`, and both are governed by
 * the CSP of the visited page. On any site with a strict policy, and that is
 * most of the interesting ones, injection simply fails. The debugger protocol
 * is not subject to page CSP. It also puts a visible banner on the window,
 * which is the honest behaviour for a page under agent control.
 */

const REGLAGES_DEFAUT = { port: 8419, actif: false };
const PROTOCOLE_DEBUG = '1.3';
const PING_MS = 20000;
const RECONNEXION_MIN_MS = 1000;
const RECONNEXION_MAX_MS = 30000;

let socket = null;
let ongletId = null;
let groupeId = null;
let attacheA = null;
let scriptGlow = null;
let scriptTap = null;
// Registered once per attached tab: stacking a copy per command would run the
// recorder N times on every new document.
let tapRegistre = false;
let reconnexionMs = RECONNEXION_MIN_MS;
let minuteurReconnexion = null;
let minuteurPing = null;
// A tab the agent borrowed from the user (via select). While it drives it, the
// tab sits in the LaRuche group; when control ends it goes back where it was.
let adopte = null; // { id, groupeOrigine, groupeLaRuche }
let minuteurControle = null;
const CONTROLE_IDLE_MS = 20000;

/* -------------------------------------------------------------- réglages */

async function reglages() {
  const stored = await chrome.storage.local.get(REGLAGES_DEFAUT);
  return { ...REGLAGES_DEFAUT, ...stored };
}

async function majEtat(patch) {
  await chrome.storage.local.set(patch);
  chrome.runtime.sendMessage({ type: 'etat' }).catch(() => {});
}

async function etatCourant() {
  const r = await reglages();
  return {
    ...r,
    connecte: socket !== null && socket.readyState === WebSocket.OPEN,
    ongletId,
    piloteUrl: attacheA ? await urlOnglet(attacheA) : null,
  };
}

async function urlOnglet(id) {
  try {
    const t = await chrome.tabs.get(id);
    return t.url || null;
  } catch {
    return null;
  }
}

/* ------------------------------------------------------------ connexion */

async function connecter() {
  clearTimeout(minuteurReconnexion);
  const { port, actif } = await reglages();
  if (!actif) return;
  if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) return;

  const agent = encodeURIComponent(navigator.userAgent.slice(0, 120));
  socket = new WebSocket(`ws://127.0.0.1:${port}/ws/navigateur?agent=${agent}`);

  socket.onopen = () => {
    reconnexionMs = RECONNEXION_MIN_MS;
    majBadge(true);
    majEtat({});
    // A quiet websocket lets the service worker be shut down. A periodic frame
    // keeps both the socket and this worker alive.
    clearInterval(minuteurPing);
    minuteurPing = setInterval(() => {
      if (socket && socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ type: 'ping' }));
      }
    }, PING_MS);
  };

  socket.onmessage = async (ev) => {
    let commande;
    try {
      commande = JSON.parse(ev.data);
    } catch {
      return;
    }
    if (typeof commande.id !== 'number') return;
    try {
      const result = await executer(commande.action, commande.params || {});
      repondre({ id: commande.id, ok: true, result });
    } catch (e) {
      repondre({ id: commande.id, ok: false, error: String((e && e.message) || e) });
    }
  };

  socket.onclose = () => {
    socket = null;
    clearInterval(minuteurPing);
    majBadge(false);
    majEtat({});
    planifierReconnexion();
  };

  socket.onerror = () => {
    // onclose always follows, so reconnection is handled there.
  };
}

function repondre(objet) {
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(objet));
  }
}

async function planifierReconnexion() {
  const { actif } = await reglages();
  if (!actif) return;
  clearTimeout(minuteurReconnexion);
  minuteurReconnexion = setTimeout(connecter, reconnexionMs);
  reconnexionMs = Math.min(reconnexionMs * 2, RECONNEXION_MAX_MS);
}

function deconnecter() {
  clearInterval(minuteurPing);
  clearTimeout(minuteurReconnexion);
  clearTimeout(minuteurControle);
  if (socket) {
    const s = socket;
    socket = null;
    try {
      s.close();
    } catch {}
  }
  // Losing the node ends control: give the borrowed tab back, then detach.
  rendreAdopte().finally(() => detacher());
  majBadge(false);
}

function majBadge(connecte) {
  chrome.action.setBadgeText({ text: connecte ? '●' : '' });
  chrome.action.setBadgeBackgroundColor({ color: '#F5A623' });
}

/* --------------------------------------------------------------- onglet */

/**
 * The tab LaRuche drives, created on first use and kept in its own tab group so
 * the user can see at a glance which tabs are the agent's.
 */
async function ongletPilote() {
  if (ongletId !== null) {
    try {
      await chrome.tabs.get(ongletId);
      return ongletId;
    } catch {
      ongletId = null;
      groupeId = null;
    }
  }
  const onglet = await chrome.tabs.create({ url: 'about:blank', active: false });
  ongletId = onglet.id;
  try {
    groupeId = await chrome.tabs.group({ tabIds: [ongletId] });
    await chrome.tabGroups.update(groupeId, {
      title: chrome.i18n.getMessage('group_title'),
      color: 'yellow',
      collapsed: false,
    });
  } catch {
    // Tab groups are unavailable in some windows; not worth failing over.
  }
  return ongletId;
}

async function attacher() {
  const id = await ongletPilote();
  if (attacheA === id) return id;
  if (attacheA !== null) await detacher();
  await chrome.debugger.attach({ tabId: id }, PROTOCOLE_DEBUG);
  attacheA = id;
  // A fresh target has none of our registrations.
  tapRegistre = false;
  await cdp('Page.enable', {});
  await cdp('Runtime.enable', {});
  return id;
}

async function detacher() {
  if (attacheA === null) return;
  const id = attacheA;
  attacheA = null;
  try {
    await chrome.debugger.detach({ tabId: id });
  } catch {}
}

// Send a borrowed user tab back to the group it came from (or ungroup it), which
// also collapses the empty LaRuche group. Safe to call when nothing is adopted.
async function rendreAdopte() {
  if (!adopte) return;
  const { id, groupeOrigine } = adopte;
  adopte = null;
  try {
    if (groupeOrigine !== undefined && groupeOrigine >= 0) {
      await chrome.tabs.group({ tabIds: [id], groupId: groupeOrigine });
    } else {
      await chrome.tabs.ungroup([id]);
    }
  } catch {}
}

// Control is over once no command has arrived for a while. We then detach the
// debugger (dropping Chrome's banner) and hand any borrowed tab back. The
// in-page glow fades on its own, slightly sooner. Rearmed on every command.
function armerControle() {
  clearTimeout(minuteurControle);
  minuteurControle = setTimeout(() => {
    finDeControle().catch(() => {});
  }, CONTROLE_IDLE_MS);
}
async function finDeControle() {
  clearTimeout(minuteurControle);
  await rendreAdopte();
  await detacher();
}

function cdp(methode, params) {
  return new Promise((resolve, reject) => {
    chrome.debugger.sendCommand({ tabId: attacheA }, methode, params, (res) => {
      const err = chrome.runtime.lastError;
      if (err) reject(new Error(err.message));
      else resolve(res || {});
    });
  });
}

chrome.debugger.onDetach.addListener((source) => {
  if (source.tabId === attacheA) attacheA = null;
  // If the user closed Chrome's debug banner themselves, that means "stop":
  // release the borrowed tab too.
  if (adopte && source.tabId === adopte.id) rendreAdopte().catch(() => {});
});

chrome.tabs.onRemoved.addListener((id) => {
  if (adopte && id === adopte.id) adopte = null;
  if (id === ongletId) {
    ongletId = null;
    groupeId = null;
    if (attacheA === id) attacheA = null;
  }
});

/* ------------------------------------------------------------- commandes */

async function executer(action, params) {
  // Any real command means control is ongoing; a keepalive ping does not.
  if (action !== 'ping') armerControle();
  switch (action) {
    case 'ping':
      return { pong: true };

    case 'navigate': {
      await attacher();
      await cdp('Page.navigate', { url: params.url });
      await attendreChargement();
      if (scriptGlow) await evaluer(scriptGlow);
      return { url: params.url };
    }

    case 'eval': {
      await attacher();
      const value = await evaluer(params.script);
      return { value };
    }

    case 'screenshot': {
      await attacher();
      const res = await cdp('Page.captureScreenshot', { format: 'png' });
      return { data: res.data };
    }

    case 'glow': {
      if (params.on) {
        scriptGlow = params.script || scriptGlow;
        await attacher();
        // Registered so it survives the page's own navigations, then run once
        // for the document already loaded.
        try {
          await cdp('Page.addScriptToEvaluateOnNewDocument', { source: scriptGlow });
        } catch {}
        if (scriptGlow) await evaluer(scriptGlow);
      } else {
        if (attacheA !== null) {
          await evaluer('window.__larucheGlowOff && window.__larucheGlowOff()').catch(() => {});
        }
        scriptGlow = null;
      }
      return { on: !!params.on };
    }

    case 'tap': {
      // The console/network recorder. Registered for future documents so a
      // navigation does not come back mute, then run once for the current one.
      await attacher();
      scriptTap = params.script || scriptTap;
      if (scriptTap && !tapRegistre) {
        try {
          await cdp('Page.addScriptToEvaluateOnNewDocument', { source: scriptTap });
          tapRegistre = true;
        } catch {}
      }
      if (scriptTap) await evaluer(scriptTap).catch(() => {});
      return { tap: true };
    }

    case 'cdp': {
      // Raw DevTools passthrough, used for Input events: a key press or a mouse
      // move synthesised from page script is untrusted and widely ignored, so
      // those two have to come from the protocol itself.
      await attacher();
      return await cdp(params.methode, params.params || {});
    }

    case 'tab': {
      const id = await ongletPilote();
      if (params.focus) {
        await chrome.tabs.update(id, { active: true });
        const t = await chrome.tabs.get(id);
        await chrome.windows.update(t.windowId, { focused: true });
      }
      return { tabId: id, url: await urlOnglet(id) };
    }

    case 'tabs': {
      // Every open tab across every Chrome window, so the agent can see and adopt
      // what is already there. Read-only: this lists, it does not drive anything.
      const tabs = await chrome.tabs.query({});
      let focusedWin = -1;
      try {
        const w = await chrome.windows.getLastFocused();
        focusedWin = w.id;
      } catch {}
      const windows = [...new Set(tabs.map((t) => t.windowId))];
      return {
        tabs: tabs.map((t) => ({
          tabId: t.id,
          title: t.title || '',
          url: t.url || t.pendingUrl || '',
          active: !!t.active,
          windowId: t.windowId,
          windowFocused: t.windowId === focusedWin,
          ours: t.id === ongletId,
        })),
        driving: ongletId,
        windowCount: windows.length,
      };
    }

    case 'select': {
      // Adopt an existing tab as the driven one. Attaching the debugger to it
      // raises Chrome's own banner on that tab, so the takeover is never silent.
      const target = Number(params.tabId);
      if (!Number.isInteger(target)) throw new Error('select needs a numeric tabId');
      let tab;
      try {
        tab = await chrome.tabs.get(target);
      } catch {
        throw new Error(`no tab ${target} (list them with the tabs action)`);
      }
      // A previously adopted tab goes home before we borrow another.
      await rendreAdopte();
      await detacher();
      ongletId = target;
      // Move it into a LaRuche group IN ITS OWN WINDOW (tab groups are per-window,
      // so never reuse a group id from another window: that would yank the tab
      // across windows). Remember its original group to restore it afterwards.
      adopte = { id: target, groupeOrigine: tab.groupId, groupeLaRuche: null };
      try {
        const g = await chrome.tabs.group({ tabIds: [target] });
        await chrome.tabGroups.update(g, {
          title: chrome.i18n.getMessage('group_title'),
          color: 'yellow',
          collapsed: false,
        });
        adopte.groupeLaRuche = g;
        groupeId = g;
      } catch {}
      await attacher();
      if (scriptGlow) await evaluer(scriptGlow).catch(() => {});
      return { tabId: target, url: tab.url || '', title: tab.title || '' };
    }

    case 'close': {
      clearTimeout(minuteurControle);
      // Hand a borrowed user tab back BEFORE detaching, unless the caller asked
      // to close our own tab outright.
      const borrowed = adopte && adopte.id;
      await rendreAdopte();
      await detacher();
      if (params.closeTab && ongletId !== null && ongletId !== borrowed) {
        try {
          await chrome.tabs.remove(ongletId);
        } catch {}
        ongletId = null;
        groupeId = null;
      }
      scriptGlow = null;
      return { closed: true };
    }

    default:
      throw new Error(`unknown action: ${action}`);
  }
}

async function evaluer(expression) {
  const res = await cdp('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
    userGesture: true,
  });
  if (res.exceptionDetails) {
    const d = res.exceptionDetails;
    throw new Error((d.exception && d.exception.description) || d.text || 'script error');
  }
  return res.result ? res.result.value : null;
}

async function attendreChargement(msMax = 15000) {
  const limite = Date.now() + msMax;
  while (Date.now() < limite) {
    try {
      const etat = await evaluer('document.readyState');
      if (etat === 'complete') return;
    } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
}

/* ------------------------------------------------------------- cycle de vie */

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg.type === 'get-etat') {
    etatCourant().then(sendResponse);
    return true;
  }
  if (msg.type === 'set-actif') {
    majEtat({ actif: msg.actif }).then(() => {
      if (msg.actif) {
        reconnexionMs = RECONNEXION_MIN_MS;
        connecter();
      } else {
        deconnecter();
      }
      sendResponse({ ok: true });
    });
    return true;
  }
  if (msg.type === 'set-port') {
    majEtat({ port: msg.port }).then(() => {
      deconnecter();
      connecter();
      sendResponse({ ok: true });
    });
    return true;
  }
  return false;
});

chrome.runtime.onStartup.addListener(connecter);
chrome.runtime.onInstalled.addListener(connecter);
connecter();
