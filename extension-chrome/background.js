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

const REGLAGES_DEFAUT = { 
  port: 8419, 
  actif: false,
  captureActivee: false,
  captureSource: 'tab',
  captureAudio: false,
  captureDossier: 'LaRuche/showcases',
  captureDemanderEmplacement: false
};
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
// L'onglet auquel la capture est liee, pour savoir quand l'agent s'en eloigne.
let ongletEnregistre = null;
const CONTROLE_IDLE_MS = 20000;

let etatEnregistrement = {
  etat: 'inactif',
  erreur: null,
  dernierFichier: null,
  extension: null
};

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
    enregistrement: etatEnregistrement,
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

  socket.onopen = async () => {
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

    // Rien a demarrer ici: la capture s'arme au moment ou l'agent prend la main
    // sur un onglet, dans `attacher`, pas a la connexion au noeud.
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

    if (etatEnregistrement.etat === 'enregistrement' || etatEnregistrement.etat === 'demarrage') {
      arreterEnregistrement('deconnexion').catch(() => {});
    }
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
  // Le pilotage commence ICI, et c'est le seul endroit ou on le sait avec
  // certitude: attacher le debogueur est le geste qui prend la main sur
  // l'onglet. `verifierDemarrageAuto` existait depuis le debut mais n'etait
  // appelee de nulle part, donc la case "Activer l'enregistrement" ne faisait
  // rien du tout: elle ecrivait un reglage que personne ne relisait au moment
  // d'agir, et le seul demarrage possible restait le bouton du popup.
  lancerSiArme().catch(() => {});
  verifierDemarrageAuto(id).catch(() => {});
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
  // Le filet, pour l'agent qui ne dit jamais `close`: sans lui l'enregistrement
  // tournait jusqu'a l'arret du noeud, et la video d'une demonstration de deux
  // minutes contenait deux minutes de demonstration suivies d'une heure de rien.
  await arreterEnregistrement('inactivite').catch(() => {});
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
      const tid = await ongletPilote();
      await chrome.tabs.update(tid, { active: true });
      try {
        const t = await chrome.tabs.get(tid);
        await chrome.windows.update(t.windowId, { focused: true });
      } catch {}
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
      await chrome.tabs.update(target, { active: true });
      try {
        await chrome.windows.update(tab.windowId, { focused: true });
      } catch {}
      return { tabId: target, url: tab.url || '', title: tab.title || '' };
    }

    case 'close': {
      clearTimeout(minuteurControle);
      // La fin du pilotage, c'est ici. Le panneau promet "sauvegardee
      // automatiquement a la fin du pilotage" et rien ne tenait cette promesse:
      // `arreterEnregistrement` n'etait appele que sur fermeture de la socket,
      // c'est-a-dire quand le noeud s'arrete, et par le bouton manuel. Entre
      // deux missions la socket reste ouverte, donc l'enregistrement continuait
      // indefiniment et le fichier n'etait jamais ecrit.
      await arreterEnregistrement('fin de pilotage').catch(() => {});
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
  
  if (msg.type === 'set-capture-settings') {
    majEtat(msg.patch).then(() => { sendResponse({ ok: true }); });
    return true;
  }
  if (msg.type === 'enregistrement-preparer') {
    preparerEnregistrement(msg).then(sendResponse).catch(e => {
      sendResponse({ ok: false, error: String(e.message || e) });
    });
    return true;
  }
  if (msg.type === 'enregistrement-arreter') {
    arreterEnregistrement('manuel').then(sendResponse).catch(e => {
      sendResponse({ ok: false, error: String(e.message || e) });
    });
    return true;
  }
  if (msg.type === 'enregistrement-pret') {
    sauvegarderEnregistrement(msg).catch(() => {});
    return false;
  }
  if (msg.type === 'enregistrement-erreur') {
    etatEnregistrement = { etat: 'erreur', erreur: msg.error };
    majEtat({ enregistrement: etatEnregistrement });
    return false;
  }

  return false;
});

/* ------------------------------------------------------------- enregistrement */

/// L'agent prend la main: si une source a ete armee, on enregistre maintenant.
async function lancerSiArme() {
  if (etatEnregistrement.etat !== 'arme') return;
  const r = await chrome.runtime.sendMessage({
    target: 'offscreen',
    type: 'enregistrement-lancer'
  }).catch(() => null);
  if (!r || !r.ok) {
    etatEnregistrement = { etat: 'erreur', erreur: (r && r.error) || 'La source armee a ete perdue' };
  } else {
    etatEnregistrement = { ...etatEnregistrement, etat: 'enregistrement' };
  }
  majEtat({ enregistrement: etatEnregistrement });
}

async function verifierDemarrageAuto(targetId) {
  const r = await reglages();
  if (!r.captureActivee) return;
  const dispo = ['inactif', 'erreur', 'sauvegarde'].includes(etatEnregistrement.etat);
  if (!dispo) {
    // Deja en cours: c'est le cas normal quand l'agent change d'onglet en
    // cours de route. On ne redemarre rien, mais on note que la video ne
    // montrera pas ce nouvel onglet.
    signalerChangementDOnglet(targetId);
    return;
  }
  if (r.captureSource === 'tab') {
    if (!targetId) return;
    ongletEnregistre = targetId;
    preparerEnregistrement({ source: 'tab', audio: r.captureAudio, targetId }).catch((e) => {
      // Chrome exige un geste de l'utilisateur pour capturer un onglet: sans
      // clic prealable sur l'extension, `getMediaStreamId` refuse. La tentative
      // vaut quand meme d'etre faite, parce qu'elle transforme un silence total
      // en une phrase que l'utilisateur peut suivre. C'est le vrai defaut du
      // reglage tel qu'il etait: la case cochee ne produisait AUCUN signe, ni
      // video, ni erreur, ni etat.
      const gesteManquant = /invoke|gesture|activeTab|not been invoked/i.test(e.message || '');
      etatEnregistrement = {
        etat: 'erreur',
        erreur: gesteManquant
          ? "Chrome refuse de capturer un onglet sans geste de l'utilisateur. " +
            "Ouvrir ce panneau et cliquer \"Preparer le showcase\" AVANT de lancer " +
            "l'agent: la source est choisie a ce moment-la, et l'enregistrement " +
            "demarre tout seul quand l'agent prend la main."
          : e.message,
      };
      majEtat({ enregistrement: etatEnregistrement });
    });
  }
  // Ecran et fenetre passent par getDisplayMedia dans l'offscreen, qui ouvre le
  // selecteur de Chrome. Le declencher tout seul ferait surgir une boite de
  // dialogue au milieu du travail de l'utilisateur, sans qu'il l'ait demandee:
  // ces deux sources restent au bouton du popup, volontairement.
}

/// La capture d'onglet est liee a UN onglet, pour toujours.
///
/// `chrome.tabCapture.getMediaStreamId` prend un `targetTabId` et le flux ne
/// suit pas: quand l'agent bascule, la video continue de filmer l'onglet de
/// depart. Une demonstration qui passe de Wikipedia a un autre site donne donc
/// une video ou il ne se passe plus rien apres la premiere bascule, sans que
/// rien ne le signale.
///
/// On ne peut pas repointer un `MediaRecorder` en cours de route. On le DIT,
/// une fois, et on nomme la source qui convient a ce genre de demonstration.
function signalerChangementDOnglet(nouveau) {
  if (etatEnregistrement.etat !== 'enregistrement') return;
  if (!ongletEnregistre || !nouveau || nouveau === ongletEnregistre) return;
  if (etatEnregistrement.avertiOnglet) return;
  etatEnregistrement = {
    ...etatEnregistrement,
    avertiOnglet: true,
    avertissement:
      "L'agent a change d'onglet. La capture est liee a l'onglet de depart et " +
      "ne le suivra pas: la suite de la video montrera l'onglet initial. Pour " +
      'une demonstration qui passe d\'un onglet a l\'autre, choisir la source ' +
      '"Ecran" avant de demarrer.',
  };
  majEtat({ enregistrement: etatEnregistrement });
}

async function preparerEnregistrement(msg) {
  if (!['inactif', 'erreur', 'sauvegarde', 'arme'].includes(etatEnregistrement.etat)) {
    throw new Error("Un enregistrement est deja en cours");
  }
  etatEnregistrement = { etat: 'demarrage', erreur: null, dernierFichier: null };
  majEtat({ enregistrement: etatEnregistrement });
  
  try {
    let streamId = null;
    if (msg.source === 'tab') {
      let targetId = msg.targetId || ongletId;
      if (!targetId) {
        const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
        if (tabs.length) targetId = tabs[0].id;
      }
      if (!targetId) throw new Error("Aucun onglet cible pour l'enregistrement");
      
      streamId = await new Promise((resolve, reject) => {
        chrome.tabCapture.getMediaStreamId({ targetTabId: targetId }, (id) => {
          if (chrome.runtime.lastError) reject(new Error(chrome.runtime.lastError.message));
          else resolve(id);
        });
      });
    }
    // Pour ecran/fenetre, pas de streamId : offscreen.js utilisera getDisplayMedia()

    const hasOffscreen = await chrome.offscreen.hasDocument();
    if (!hasOffscreen) {
      await chrome.offscreen.createDocument({
        url: 'offscreen.html',
        reasons: ['DISPLAY_MEDIA'],
        justification: 'Enregistrement video du showcase'
      });
    }

    // Ecran et fenetre s'ARMENT: on demande la source maintenant, pendant que
    // l'utilisateur est devant et que son clic autorise le selecteur de Chrome,
    // et on n'enregistre qu'au moment ou l'agent prend la main. Sans cette
    // separation il fallait choisir entre demander l'ecran trop tot, et filmer
    // tout le temps mort avant que l'agent ne commence.
    //
    // La capture d'onglet, elle, ne peut pas etre armee en avance: l'agent
    // travaille dans un onglet qu'il cree lui-meme, qui n'existe pas encore au
    // moment du clic. Armer maintenant filmerait l'onglet que l'utilisateur a
    // sous les yeux, pas celui de l'agent.
    const differe = msg.source !== 'tab' && msg.differe !== false;

    const result = await chrome.runtime.sendMessage({
      target: 'offscreen',
      type: 'enregistrement-demarrer',
      streamId: streamId,
      source: msg.source,
      audio: msg.audio,
      differe
    });

    if (!result || !result.ok) throw new Error((result && result.error) || 'Erreur offscreen');

    etatEnregistrement = {
      etat: differe ? 'arme' : 'enregistrement',
      extension: result.extension,
      dernierFichier: null,
      source: msg.source
    };
    majEtat({ enregistrement: etatEnregistrement });
    return { ok: true, arme: differe };
  } catch (e) {
    etatEnregistrement = { etat: 'erreur', erreur: e.message };
    majEtat({ enregistrement: etatEnregistrement });
    throw e;
  }
}

async function arreterEnregistrement(raison) {
  // Une source armee que personne n'a utilisee: on rend la main a Chrome plutot
  // que de laisser un partage d'ecran actif pour rien, ce que l'utilisateur voit
  // dans sa barre et ne comprend pas.
  if (etatEnregistrement.etat === 'arme') {
    await chrome.runtime.sendMessage({ target: 'offscreen', type: 'enregistrement-desarmer' }).catch(() => {});
    etatEnregistrement = { etat: 'inactif', dernierFichier: null };
    ongletEnregistre = null;
    majEtat({ enregistrement: etatEnregistrement });
    return;
  }
  if (etatEnregistrement.etat !== 'enregistrement') return;
  ongletEnregistre = null;
  etatEnregistrement = { etat: 'finalisation' };
  majEtat({ enregistrement: etatEnregistrement });
  await chrome.runtime.sendMessage({ target: 'offscreen', type: 'enregistrement-arreter' }).catch(() => {});
}

async function sauvegarderEnregistrement(msg) {
  etatEnregistrement = { etat: 'sauvegarde' };
  majEtat({ enregistrement: etatEnregistrement });
  try {
    const r = await reglages();
    const prefixe = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    const filename = `${r.captureDossier || 'LaRuche/showcases'}/${prefixe}-showcase.${msg.extension || 'mp4'}`;
    
    await chrome.downloads.download({
      url: msg.url,
      filename: filename,
      saveAs: !!r.captureDemanderEmplacement,
      conflictAction: 'uniquify'
    });
    
    etatEnregistrement = { etat: 'inactif', dernierFichier: filename };
    majEtat({ enregistrement: etatEnregistrement });
  } catch(e) {
    etatEnregistrement = { etat: 'erreur', erreur: "Erreur de sauvegarde: " + e.message };
    majEtat({ enregistrement: etatEnregistrement });
  } finally {
    setTimeout(() => {
      chrome.runtime.sendMessage({ target: 'offscreen', type: 'enregistrement-liberer-url', url: msg.url }).catch(() => {});
    }, 10000);
  }
}

chrome.runtime.onStartup.addListener(connecter);
chrome.runtime.onInstalled.addListener(connecter);
connecter();
