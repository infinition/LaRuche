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
  compagnon: false,
  curseurAgent: false,
  captureActivee: false,
  captureSource: 'tab',
  captureAudio: false,
  captureQualite: 'standard',
  captureDossier: 'LaRuche/showcases',
  captureDemanderEmplacement: false
};
/// Les trois qualites d'enregistrement, et ce qu'elles coutent.
///
/// `saut` est `everyNthFrame`: une image sur N. A soixante images par seconde
/// dans l'onglet, un saut de 3 donne une vingtaine d'images par seconde, ce qui
/// suffit largement pour montrer une suite de gestes.
///
/// La haute qualite est proposee mais pas par defaut: c'est exactement le
/// reglage qui rendait l'onglet enregistre inutilisable, et personne ne fait le
/// lien entre "j'ai mis la qualite au maximum" et "le popup ne s'ouvre plus".
const QUALITES = {
  basse: { l: 960, h: 540, jpeg: 50, saut: 4 },
  standard: { l: 1280, h: 720, jpeg: 60, saut: 3 },
  haute: { l: 1920, h: 1080, jpeg: 80, saut: 2 },
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

// Le pont n'est pas un proxy DevTools general. Cette liste correspond aux
// commandes structurees que l'outil browser utilise vraiment. Toute nouvelle
// methode doit arriver avec son cas d'usage et sa revue, pas par effet de bord.
const METHODES_CDP_AUTORISEES = new Set([
  'Input.dispatchMouseEvent',
  'Input.dispatchKeyEvent',
  'Runtime.evaluate',
  'DOM.setFileInputFiles',
  'Emulation.setDeviceMetricsOverride',
  'Network.getCookies',
]);

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
    // Le noeud repond de nouveau: on rattrape ce qui attendait.
    viderFileGarder().catch(() => {});
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
    // Un accuse porte `req`, une commande porte `id`. Les confondre ferait
    // executer une action inexistante et perdre l'accuse.
    if (typeof commande.req === 'number') {
      accuseRecu(commande);
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
  try { alarmes()?.clear(ALARME_CONTROLE); } catch {}
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

/// L'icone dit l'etat, en un coup d'oeil et sans ouvrir le panneau.
///
/// Trois choses valent la peine d'etre vues depuis la barre d'outils, et elles
/// se hierarchisent: on enregistre, l'agent a la main, la ruche est connectee.
/// L'enregistrement clignote en rouge parce que c'est le seul etat qui produit
/// un fichier et qui touche a la vie privee de quelqu'un: il doit etre
/// impossible de l'oublier. Un point fixe ne suffit pas, on cesse de le voir au
/// bout de dix minutes.
let clignotant = null;
let badgeConnecte = false;

function arreterClignotement() {
  if (clignotant !== null) {
    clearInterval(clignotant);
    clignotant = null;
  }
}

function rafraichirBadge() {
  const enregistre = etatEnregistrement.etat === 'enregistrement';
  const pilote = attacheA !== null;

  if (enregistre) {
    if (clignotant === null) {
      let allume = true;
      const battre = () => {
        chrome.action.setBadgeText({ text: allume ? '●' : ' ' });
        chrome.action.setBadgeBackgroundColor({ color: allume ? '#E5484D' : '#5A1E20' });
        allume = !allume;
      };
      battre();
      clignotant = setInterval(battre, 700);
    }
    return;
  }

  arreterClignotement();
  if (pilote) {
    // L'agent a la main sans enregistrer: fixe, et d'une autre couleur que le
    // simple "connecte", sinon les deux etats sont indiscernables.
    chrome.action.setBadgeText({ text: '▶' });
    chrome.action.setBadgeBackgroundColor({ color: '#3E8FF5' });
    return;
  }
  chrome.action.setBadgeText({ text: badgeConnecte ? '●' : '' });
  chrome.action.setBadgeBackgroundColor({ color: '#F5A623' });
}

function majBadge(connecte) {
  badgeConnecte = connecte;
  rafraichirBadge();
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
  rafraichirBadge();
  lancerSiArme().catch(() => {});
  brancherScreencast().catch(() => {});
  verifierDemarrageAuto(id).catch(() => {});
  return id;
}

async function detacher() {
  if (attacheA === null) return;
  const id = attacheA;
  attacheA = null;
  rafraichirBadge();
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
//
// DEUX minuteurs, et le second est celui qui compte vraiment.
//
// Un `setTimeout` vit dans le service worker, et Chrome suspend un service
// worker inactif. Tant que l'agent travaille, il arrive des commandes et des
// images de screencast qui le tiennent eveille. Mais des que l'agent a fini,
// tout s'arrete: `Page.screencastFrame` ne se declenche qu'au repaint, et une
// page immobile n'en produit aucun. Le worker s'endort, le `setTimeout` meurt
// avec lui, et l'enregistrement ne s'arrete jamais. C'est exactement au moment
// ou l'on a besoin de lui que ce minuteur disparait.
//
// `chrome.alarms` survit a la suspension et REVEILLE le worker. On garde le
// `setTimeout` pour la reactivite quand le worker est vivant, l'alarme est le
// filet. Chrome plancher les alarmes a 30 secondes, d'ou l'ecart.
const ALARME_CONTROLE = 'laruche-fin-de-controle';
/// La file d'attente se retente toute seule, sans dependre du pilotage.
///
/// Elle etait videe a l'ouverture de la SOCKET, ce qui liait deux choses sans
/// rapport: garder un lien n'a rien a voir avec autoriser LaRuche a piloter le
/// navigateur. Sans "Autoriser le pilotage", la socket ne s'ouvre jamais, donc
/// la file ne partait jamais, meme avec LaRuche allumee juste a cote. C'est
/// exactement le symptome: "j'ai lance LaRuche et elle ne la detecte pas".
const ALARME_FILE = 'laruche-file-garder';

/// `chrome.alarms`, ou rien.
///
/// L'API n'existe que si la permission est accordee, et une permission ajoutee
/// au manifeste n'est prise en compte qu'apres un rechargement complet de
/// l'extension. Entre les deux, `chrome.alarms` vaut `undefined`.
///
/// Ce detail vaut la peine d'etre traite proprement: un appel qui leve au
/// NIVEAU RACINE d'un service worker ne casse pas une fonction, il tue le
/// script entier. Plus de socket, plus de pilotage, plus d'enregistrement, et
/// une seule ligne dans la page des erreurs pour l'expliquer. Le filet ne doit
/// jamais pouvoir couter plus cher que ce qu'il rattrape.
const alarmes = () => (typeof chrome !== 'undefined' && chrome.alarms) || null;

function armerControle() {
  clearTimeout(minuteurControle);
  minuteurControle = setTimeout(() => {
    finDeControle().catch(() => {});
  }, CONTROLE_IDLE_MS);
  // Chrome plancher les alarmes a 30 secondes, d'ou l'ecart avec le timer.
  try {
    alarmes()?.create(ALARME_CONTROLE, { delayInMinutes: 0.6 });
  } catch {}
}

if (alarmes()) {
  chrome.alarms.onAlarm.addListener((alarme) => {
    if (alarme.name === ALARME_CONTROLE) finDeControle().catch(() => {});
  if (alarme.name === ALARME_FILE) viderFileGarder().catch(() => {});
  });
} else {
  // Sans alarme on retombe sur le seul `setTimeout`, qui meurt avec le worker
  // suspendu: l'enregistrement peut alors ne pas s'arreter tout seul. Le dire
  // plutot que de laisser croire que le filet est en place.
  console.warn(
    "LaRuche: permission `alarms` absente, rechargez l'extension. Sans elle, " +
      "l'arret automatique de l'enregistrement n'est pas garanti.",
  );
}
async function finDeControle() {
  clearTimeout(minuteurControle);
  try { alarmes()?.clear(ALARME_CONTROLE); } catch {}
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

/// Les images du screencast, transmises au canevas de l'offscreen.
///
/// L'ack est obligatoire: sans lui Chrome cesse d'envoyer apres quelques images
/// et l'enregistrement se fige sans erreur, ce qui est exactement le genre de
/// panne qu'on ne diagnostique jamais depuis la video.
chrome.debugger.onEvent.addListener((source, methode, params) => {
  if (methode !== 'Page.screencastFrame') return;
  if (!params || !params.data) return;
  // On accuse reception dans tous les cas, sinon Chrome cesse d'envoyer, mais
  // on ne transmet que si un enregistrement tourne vraiment: pendant la
  // finalisation et la sauvegarde, transmettre revient a faire travailler le
  // bus de messages pour un canevas que plus personne ne filme.
  if (etatEnregistrement.etat !== 'enregistrement') {
    if (params.sessionId !== undefined) {
      chrome.debugger.sendCommand(
        { tabId: source.tabId },
        'Page.screencastFrameAck',
        { sessionId: params.sessionId },
        () => void chrome.runtime.lastError,
      );
    }
    return;
  }
  chrome.runtime.sendMessage({
    target: 'offscreen',
    type: 'screencast-frame',
    data: params.data,
  }).catch(() => {});
  if (params.sessionId !== undefined) {
    chrome.debugger.sendCommand(
      { tabId: source.tabId },
      'Page.screencastFrameAck',
      { sessionId: params.sessionId },
      () => void chrome.runtime.lastError,
    );
  }
});

/// Branche le screencast sur l'onglet pilote, s'il y a un enregistrement.
///
/// Appele a chaque prise de main: quand l'agent change d'onglet, le nouveau
/// commence a envoyer ses images dans le meme canevas, donc la video continue
/// sans coupure. C'est ce que la capture d'onglet ne savait pas faire.
async function brancherScreencast() {
  if (etatEnregistrement.source !== 'screencast') return;
  if (!['enregistrement', 'arme'].includes(etatEnregistrement.etat)) return;
  if (attacheA === null) return;
  // Le debit est le point sensible, et il se paie sur l'onglet FILME.
  //
  // Chaque image traverse quatre etages: le renderer de l'onglet l'encode en
  // JPEG, Chrome l'envoie en base64 dans un evenement CDP, on la repousse dans
  // le bus de messages de l'extension, et l'offscreen la decode pour la
  // peindre. Le premier etage se fait dans le processus de l'onglet enregistre,
  // et c'est pour ca qu'une qualite trop haute rend CET onglet poussif alors
  // que les autres vont bien: ils ont chacun leur renderer.
  //
  // D'ou un reglage plutot qu'une valeur en dur, et un defaut prudent.
  const q = QUALITES[(await reglages()).captureQualite] || QUALITES.standard;
  await cdp('Page.startScreencast', {
    format: 'jpeg',
    quality: q.jpeg,
    maxWidth: q.l,
    maxHeight: q.h,
    everyNthFrame: q.saut,
  }).catch(() => {});
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
      if (!METHODES_CDP_AUTORISEES.has(params.methode)) {
        throw new Error(`CDP method not allowed: ${params.methode || '(missing)'}`);
      }
      await attacher();
      const resultat = await cdp(params.methode, params.params || {});
      // Une valeur de cookie est un jeton de session. L'outil n'en montre que
      // le nom, la taille et les attributs, donc elle ne doit jamais franchir
      // le pont local pour etre jetee ensuite cote noeud.
      if (params.methode === 'Network.getCookies' && Array.isArray(resultat.cookies)) {
        return {
          cookies: resultat.cookies.map((cookie) => ({
            name: cookie.name || '',
            domain: cookie.domain || '',
            path: cookie.path || '',
            secure: !!cookie.secure,
            httpOnly: !!cookie.httpOnly,
            sameSite: cookie.sameSite || '',
            expires: cookie.expires,
            valueLength: String(cookie.value || '').length,
          })),
        };
      }
      return resultat;
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
      try { alarmes()?.clear(ALARME_CONTROLE); } catch {}
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
  
  if (msg.type === 'garder') {
    garder(msg.entree).then(sendResponse).catch((e) => {
      sendResponse({ ok: false, error: String((e && e.message) || e) });
    });
    return true;
  }
  if (msg.type === 'garder-file') {
    // On RETENTE avant de repondre: le popup s'ouvre souvent juste apres que
    // l'utilisateur a lance LaRuche, et lui montrer "3 en attente" alors qu'on
    // pourrait les envoyer tout de suite est exactement ce qui donne
    // l'impression que rien ne detecte rien.
    viderFileGarder()
      .catch(() => ({}))
      .then(() => fileGarder())
      .then((f) => sendResponse({ ok: true, restants: f.length }));
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
  if (msg.type === 'enregistrement-demarrer-manuel') {
    demarrerEnregistrementManuel(msg).then(sendResponse).catch(e => {
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
    rafraichirBadge();
    return false;
  }

  return false;
});

/* ------------------------------------------------------------------ garder */

/// Ce que l'utilisateur garde depuis sa navigation, envoye a la memoire.
///
/// La premiere version passait par `/api/memory/write` en HTTP, sur la foi
/// d'une lecture du gestionnaire qui ne verifiait rien. C'etait faux: la
/// garde est posee au niveau du routeur, et l'endpoint repond 401. Le canal
/// est donc la websocket, qui a du etre ouverte dans les deux sens.
///
/// Les demandes en vol, par identifiant, en attente de l'accuse du noeud.
let prochainReq = 0;
const attentesReq = new Map();

/// Envoie une entree au noeud par la SOCKET, et attend son accuse.
///
/// Le chemin HTTP ne pouvait pas marcher, et pour deux raisons independantes.
/// Les ecritures du noeud sont derriere un cookie de session `SameSite=Lax`,
/// qu'une requete partie d'un service worker d'extension n'emporte jamais
/// puisqu'elle est cross-site. Et l'utilisateur de l'application de bureau n'a
/// de session dans aucun navigateur, donc il n'y a meme pas de cookie a
/// envoyer.
///
/// La socket, elle, est deja etablie, deja restreinte a l'identifiant de cette
/// extension cote noeud, et vit sur la boucle locale. Elle porte donc la meme
/// confiance que le pilotage qu'elle sert deja, sans cookie ni jeton.
async function envoyerEnMemoire(entree) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    // Deux causes tres differentes: la connexion n'a jamais ete autorisee,
    // ou elle l'est mais le noeud ne repond pas. Dire "LaRuche est
    // injoignable" a quelqu'un qui n'a pas coche la case l'envoie chercher
    // une panne qui n'existe pas.
    const { actif } = await reglages();
    const e = new Error(actif ? 'socket fermee' : 'connexion non autorisee');
    e.genre = actif ? 'injoignable' : 'connexion';
    throw e;
  }
  prochainReq += 1;
  const req = prochainReq;
  socket.send(JSON.stringify({ type: 'garder', req, entree }));
  return new Promise((resolve, reject) => {
    // Sans delai, une entree resterait en vol pour toujours si le noeud
    // redemarrait entre l'envoi et l'accuse, et la file ne repartirait jamais.
    const minuteur = setTimeout(() => {
      attentesReq.delete(req);
      const e = new Error('pas de reponse du noeud');
      e.genre = 'injoignable';
      reject(e);
    }, 10000);
    attentesReq.set(req, { resolve, reject, minuteur });
  });
}

/// L'accuse du noeud, arrive sur la meme socket.
function accuseRecu(v) {
  const attente = attentesReq.get(v.req);
  if (!attente) return;
  attentesReq.delete(v.req);
  clearTimeout(attente.minuteur);
  if (v.ok) attente.resolve({});
  else {
    const e = new Error(v.error || 'refus du noeud');
    e.genre = 'echec';
    attente.reject(e);
  }
}

/// La file d'attente: ce qui n'a pas pu partir.
///
/// Garder un lien est un geste d'une seconde, souvent fait en passant. Le
/// perdre parce que le noeud etait eteint serait la pire facon de traiter ce
/// geste: l'utilisateur ne saurait meme pas qu'il a perdu quelque chose. On
/// stocke, et on renvoie a la prochaine connexion.
async function fileGarder() {
  const { garderFile } = await chrome.storage.local.get({ garderFile: [] });
  return Array.isArray(garderFile) ? garderFile : [];
}

async function armerFile() {
  try {
    alarmes()?.create(ALARME_FILE, { delayInMinutes: 1, periodInMinutes: 1 });
  } catch {}
}

async function desarmerFile() {
  try {
    alarmes()?.clear(ALARME_FILE);
  } catch {}
}

async function empiler(entree) {
  const file = await fileGarder();
  // Plafonne: une file sans fin sur un noeud eteint depuis des semaines finirait
  // par remplir le stockage de l'extension.
  file.push(entree);
  await chrome.storage.local.set({ garderFile: file.slice(-200) });
  await armerFile();
}

/// Vide la file, en s'arretant au premier echec.
///
/// S'arreter plutot que continuer: si le noeud vient de retomber, insister sur
/// les cent suivantes ne fait que cent echecs de plus, et l'ordre d'arrivee en
/// memoire cesse d'etre celui dans lequel on a garde les choses.
async function viderFileGarder() {
  const file = await fileGarder();
  if (!file.length) return { envoyes: 0, restants: 0 };
  let envoyes = 0;
  while (file.length) {
    try {
      await envoyerEnMemoire(file[0]);
    } catch {
      break;
    }
    file.shift();
    envoyes += 1;
  }
  await chrome.storage.local.set({ garderFile: file });
  if (envoyes) tracerFile(envoyes, file.length);
  if (file.length) await armerFile();
  else await desarmerFile();
  return { envoyes, restants: file.length };
}

function tracerFile(envoyes, restants) {
  console.info(`LaRuche: ${envoyes} entree(s) envoyee(s), ${restants} en attente`);
}

/// Garde une entree: tout de suite si on peut, de cote sinon.
async function garder(entree) {
  try {
    await envoyerEnMemoire(entree);
    // La file part avec: si le noeud repond maintenant, autant tout rattraper.
    const { restants } = await viderFileGarder();
    return { ok: true, envoye: true, restants };
  } catch (e) {
    await empiler(entree);
    const file = await fileGarder();
    // Le genre compte: "LaRuche est eteinte" et "tu n'es pas connecte" appellent
    // deux gestes differents, et un message unique enverrait l'utilisateur
    // attendre une reconnexion qui ne reglera rien.
    return {
      ok: true,
      envoye: false,
      restants: file.length,
      genre: e.genre || 'injoignable',
      raison: String(e.message || e),
    };
  }
}

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
  rafraichirBadge();
}

/// Le bouton du popup ne depend jamais d'une prise de controle par l'agent.
/// Une source ecran ou fenetre est deja armee par le selecteur Chrome: le clic
/// lance ce flux. Pour un onglet, il cree et demarre directement le flux de
/// l'onglet actif.
async function demarrerEnregistrementManuel(msg) {
  if (etatEnregistrement.etat === 'arme') {
    if (etatEnregistrement.source !== msg.source) {
      await arreterEnregistrement('changement de source');
    } else {
      await lancerSiArme();
      if (etatEnregistrement.etat !== 'enregistrement') {
        throw new Error(etatEnregistrement.erreur || "La source armee n'a pas demarre");
      }
      return { ok: true, manuel: true };
    }
  }

  return await preparerEnregistrement({
    ...msg,
    differe: false,
  });
}

async function verifierDemarrageAuto(targetId) {
  const r = await reglages();
  if (!r.captureActivee) return;
  const dispo = ['inactif', 'erreur', 'sauvegarde'].includes(etatEnregistrement.etat);
  if (!dispo) {
    // Deja en cours: c'est le cas normal quand l'agent change d'onglet en
    // cours de route. On ne redemarre rien. Avec le screencast la video suit
    // toute seule; avec une capture d'onglet lancee a la main, elle ne suit pas,
    // et c'est ce que `signalerChangementDOnglet` va dire.
    signalerChangementDOnglet(targetId);
    return;
  }
  if (r.captureSource === 'tab') {
    if (!targetId) return;
    ongletEnregistre = targetId;
    // Screencast et non `tabCapture`: le premier passe par le debogueur, deja
    // attache, et ne demande aucun geste; le second exige que l'utilisateur ait
    // invoque l'extension sur CET onglet, ce qui est impossible pour un onglet
    // que l'agent vient de creer. C'est la raison pour laquelle la case cochee
    // ne demarrait rien.
    try {
      await preparerEnregistrement({ source: 'screencast', audio: false, differe: false });
      await brancherScreencast();
    } catch (e) {
      etatEnregistrement = { etat: 'erreur', erreur: String((e && e.message) || e) };
      majEtat({ enregistrement: etatEnregistrement });
      rafraichirBadge();
    }
  }
  // Ecran et fenetre passent par getDisplayMedia dans l'offscreen, qui ouvre le
  // selecteur de Chrome. Il exige un geste, donc il est demande au moment ou
  // l'utilisateur choisit la source dans la liste, pas ici.
}

/// La capture d'onglet lancee A LA MAIN est liee a UN onglet, pour toujours.
///
/// Ne concerne plus le demarrage automatique, qui passe par le screencast et
/// suit l'agent. Reste vrai pour le bouton manuel avec la source "Onglet".
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
  // Le screencast, lui, SUIT: les images du nouvel onglet arrivent dans le meme
  // canevas et la video est continue. Il n'y a rien a signaler.
  if (etatEnregistrement.source === 'screencast') return;
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
  rafraichirBadge();
}

async function preparerEnregistrement(msg) {
  if (!['inactif', 'erreur', 'sauvegarde', 'arme'].includes(etatEnregistrement.etat)) {
    throw new Error("Un enregistrement est deja en cours");
  }
  etatEnregistrement = { etat: 'demarrage', erreur: null, dernierFichier: null };
  majEtat({ enregistrement: etatEnregistrement });
  rafraichirBadge();
  
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
      // La toile prend la taille des images recues: l'agrandissement n'ajoute
      // aucun detail et fait encoder la video sur plus de pixels pour rien.
      taille: QUALITES[(await reglages()).captureQualite] || QUALITES.standard,
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
    rafraichirBadge();
    return { ok: true, arme: differe };
  } catch (e) {
    etatEnregistrement = { etat: 'erreur', erreur: e.message };
    majEtat({ enregistrement: etatEnregistrement });
    rafraichirBadge();
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
    rafraichirBadge();
    return;
  }
  if (etatEnregistrement.etat !== 'enregistrement') return;
  ongletEnregistre = null;
  // Couper le flux d'images: sans ca Chrome continue d'en envoyer dans le vide,
  // et le prochain enregistrement recevrait celles de l'ancien.
  if (etatEnregistrement.source === 'screencast' && attacheA !== null) {
    await cdp('Page.stopScreencast', {}).catch(() => {});
  }
  etatEnregistrement = { etat: 'finalisation' };
  majEtat({ enregistrement: etatEnregistrement });
  rafraichirBadge();
  await chrome.runtime.sendMessage({ target: 'offscreen', type: 'enregistrement-arreter' }).catch(() => {});
}

async function sauvegarderEnregistrement(msg) {
  etatEnregistrement = { etat: 'sauvegarde' };
  majEtat({ enregistrement: etatEnregistrement });
  rafraichirBadge();
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
    rafraichirBadge();
  } catch(e) {
    etatEnregistrement = { etat: 'erreur', erreur: "Erreur de sauvegarde: " + e.message };
    majEtat({ enregistrement: etatEnregistrement });
    rafraichirBadge();
  } finally {
    setTimeout(() => {
      chrome.runtime.sendMessage({ target: 'offscreen', type: 'enregistrement-liberer-url', url: msg.url }).catch(() => {});
    }, 10000);
  }
}

chrome.runtime.onStartup.addListener(connecter);
chrome.runtime.onInstalled.addListener(connecter);
connecter();
// Au reveil du worker: si quelque chose attend, on retente sans attendre
// l'alarme, et sans dependre de la socket de pilotage.
viderFileGarder().catch(() => {});

async function injecterCompagnonDansOnglets() {
  try {
    const onglets = await chrome.tabs.query({});
    for (const o of onglets) {
      if (!o.id || !o.url) continue;
      if (!o.url.startsWith('http://') && !o.url.startsWith('https://') && !o.url.startsWith('file://')) continue;
      // Les deux fichiers, et dans cet ordre: `compagnon.js` cherche
      // `LaRucheRencontre` au moment de rejoindre le canal des rencontres.
      // Chacun se garde d'une double installation, donc reinjecter un onglet
      // qui a deja recu le script de la declaration ne fait rien.
      chrome.scripting.executeScript({
        target: { tabId: o.id },
        files: ['rencontre.js', 'compagnon.js'],
      }).catch(() => {});
    }
  } catch {}
}

async function injecterCurseurAgentDansOnglets() {
  try {
    const onglets = await chrome.tabs.query({});
    for (const o of onglets) {
      if (!o.id || !o.url) continue;
      if (!o.url.startsWith('http://') && !o.url.startsWith('https://') && !o.url.startsWith('file://')) continue;
      chrome.scripting.executeScript({
        target: { tabId: o.id },
        files: ['curseur-agent.js'],
      }).catch(() => {});
    }
  } catch {}
}

const ACCES_SITES_VISUELS = {
  permissions: ['scripting'],
  origins: ['http://*/*', 'https://*/*'],
};

const SCRIPTS_VISUELS = {
  compagnon: {
    id: 'laruche-compagnon',
    js: ['rencontre.js', 'compagnon.js'],
  },
  curseurAgent: {
    id: 'laruche-curseur-agent',
    js: ['curseur-agent.js'],
  },
};

async function reglerScriptVisuel(definition, actif) {
  const trouves = await chrome.scripting.getRegisteredContentScripts({ ids: [definition.id] });
  const existe = trouves.length > 0;
  if (actif && !existe) {
    await chrome.scripting.registerContentScripts([{
      id: definition.id,
      matches: ['http://*/*', 'https://*/*'],
      js: definition.js,
      runAt: 'document_idle',
      persistAcrossSessions: true,
    }]);
  } else if (!actif && existe) {
    await chrome.scripting.unregisterContentScripts({ ids: [definition.id] });
  }
}

/// Le compagnon et le curseur sont facultatifs. Ils ne justifient pas un acces
/// permanent a tous les sites au moment de l'installation: Chrome le demande
/// au premier usage, puis seuls les scripts effectivement actives sont
/// enregistres pour les navigations suivantes.
async function synchroniserScriptsVisuels(injecter = false) {
  const etat = await chrome.storage.local.get({ compagnon: false, curseurAgent: false });
  const autorise = await chrome.permissions.contains(ACCES_SITES_VISUELS);

  if (!autorise) {
    if (etat.compagnon || etat.curseurAgent) {
      await chrome.storage.local.set({ compagnon: false, curseurAgent: false });
    }
    return;
  }

  await reglerScriptVisuel(SCRIPTS_VISUELS.compagnon, !!etat.compagnon);
  await reglerScriptVisuel(SCRIPTS_VISUELS.curseurAgent, !!etat.curseurAgent);

  if (injecter && etat.compagnon) await injecterCompagnonDansOnglets();
  if (injecter && etat.curseurAgent) await injecterCurseurAgentDansOnglets();

  if (!etat.compagnon && !etat.curseurAgent) {
    await chrome.permissions.remove(ACCES_SITES_VISUELS).catch(() => false);
  }
}

chrome.storage.onChanged.addListener((changes, zone) => {
  if (zone === 'local' && (changes.compagnon || changes.curseurAgent)) {
    synchroniserScriptsVisuels(true).catch(() => {});
  }
});

synchroniserScriptsVisuels(true).catch(() => {});
