/** Popup: connection state, the on/off switch and the node port. */

const $ = (id) => document.getElementById(id);
let dernierEtat = null;
let actionCaptureEnCours = false;
let messagesSecours = null;

async function chargerMessagesSecours() {
  if (messagesSecours) return messagesSecours;
  try {
    const lang = (chrome.i18n && chrome.i18n.getUILanguage && chrome.i18n.getUILanguage().slice(0, 2)) || 'fr';
    let url = chrome.runtime.getURL(`_locales/${lang}/messages.json`);
    let rep = await fetch(url).catch(() => null);
    if (!rep || !rep.ok) {
      url = chrome.runtime.getURL('_locales/fr/messages.json');
      rep = await fetch(url).catch(() => null);
    }
    if (rep && rep.ok) {
      messagesSecours = await rep.json();
    }
  } catch {}
  return messagesSecours;
}

function msg(cle, substitutions) {
  try {
    const direct = chrome.i18n && chrome.i18n.getMessage(cle, substitutions);
    if (direct) return direct;
  } catch {}

  if (messagesSecours && messagesSecours[cle] && messagesSecours[cle].message) {
    let brut = messagesSecours[cle].message;
    const liste = Array.isArray(substitutions)
      ? substitutions
      : substitutions !== undefined && substitutions !== null
      ? [substitutions]
      : [];

    liste.forEach((val, idx) => {
      brut = brut.replace(new RegExp(`\\$${idx + 1}`, 'g'), String(val));
    });

    if (messagesSecours[cle].placeholders) {
      for (const [nom, ph] of Object.entries(messagesSecours[cle].placeholders)) {
        const phIndex = parseInt((ph.content || '').replace('$', ''), 10) - 1;
        const remplacement = phIndex >= 0 && liste[phIndex] !== undefined ? String(liste[phIndex]) : '';
        brut = brut.replace(new RegExp(`\\$${nom.toUpperCase()}\\$`, 'g'), remplacement);
      }
    }
    return brut;
  }

  return '';
}

// Every visible string comes from _locales, none is written here.
function traduire() {
  $('libelleActif').textContent = msg('popup_enable');
  $('libellePort').textContent = msg('popup_port');
  $('titreCapture').textContent = msg('capture_title');
  $('libelleCaptureActivee').textContent = msg('capture_enable');
  $('libelleCaptureSource').textContent = msg('capture_source');
  $('sourceOnglet').textContent = msg('capture_source_tab');
  $('sourceFenetre').textContent = msg('capture_source_window');
  $('sourceEcran').textContent = msg('capture_source_screen');
  $('libelleCaptureAudio').textContent = msg('capture_audio');
  $('libelleCaptureDossier').textContent = msg('capture_folder');
  $('libelleCaptureDemanderEmplacement').textContent = msg('capture_save_as');
  $('demarrerCapture').textContent = msg('capture_start');
  $('arreterCapture').textContent = msg('capture_stop');
  $('aideCapture').textContent = msg('capture_help');
  $('note').textContent = msg('popup_note');
  document.title = msg('ext_name');
}

function peindre(etat) {
  dernierEtat = etat;
  const pastille = $('pastille');
  pastille.classList.toggle('on', etat.connecte);
  pastille.classList.toggle('erreur', etat.actif && !etat.connecte);

  let cle = 'state_off';
  if (etat.actif) cle = etat.connecte ? 'state_connected' : 'state_connecting';
  $('etat').textContent = msg(cle);

  $('actif').checked = !!etat.actif;
  if (document.activeElement !== $('port')) $('port').value = etat.port;

  const pilote = $('pilote');
  if (etat.piloteUrl) {
    pilote.hidden = false;
    pilote.textContent = msg('popup_driving', [etat.piloteUrl]);
  } else {
    pilote.hidden = true;
  }

  $('captureActivee').checked = !!etat.captureActivee;
  if (document.activeElement !== $('captureSource')) $('captureSource').value = etat.captureSource;
  $('captureAudio').checked = !!etat.captureAudio;
  if (document.activeElement !== $('captureDossier')) {
    $('captureDossier').value = etat.captureDossier || 'LaRuche/showcases';
  }
  $('captureDemanderEmplacement').checked = !!etat.captureDemanderEmplacement;

  const enregistrement = etat.enregistrement || { etat: 'inactif' };
  // 'arme' n'est PAS occupe: la source est prise mais rien ne tourne, et le
  // bouton d'arret doit rester accessible pour la relacher.
  const occupe = ['demarrage', 'enregistrement', 'finalisation', 'sauvegarde', 'arme'].includes(enregistrement.etat);
  const options = $('captureOptions');
  options.classList.toggle('desactive', !etat.captureActivee);
  for (const champ of options.querySelectorAll('input, select')) {
    champ.disabled = !etat.captureActivee || occupe;
  }
  $('demarrerCapture').hidden = occupe;
  $('demarrerCapture').disabled = !etat.captureActivee || actionCaptureEnCours;
  $('arreterCapture').hidden = !occupe;
  $('arreterCapture').disabled =
    actionCaptureEnCours || !['enregistrement', 'arme'].includes(enregistrement.etat);

  const statut = $('etatCapture');
  statut.classList.toggle('en-cours', occupe);
  statut.classList.toggle('erreur', enregistrement.etat === 'erreur');
  if (!etat.captureActivee) {
    statut.textContent = msg('capture_state_disabled');
  } else if (enregistrement.etat === 'arme') {
    statut.textContent = msg('capture_state_armed');
  } else if (enregistrement.etat === 'demarrage') {
    statut.textContent = msg('capture_state_starting');
  } else if (enregistrement.etat === 'enregistrement') {
    statut.textContent = msg('capture_state_recording', [
      String(enregistrement.extension || 'mp4').toUpperCase(),
    ]);
  } else if (enregistrement.etat === 'finalisation' || enregistrement.etat === 'sauvegarde') {
    statut.textContent = msg('capture_state_saving');
  } else if (enregistrement.etat === 'erreur') {
    statut.textContent = msg('capture_state_error', [enregistrement.erreur || '']);
  } else if (enregistrement.dernierFichier) {
    statut.textContent = msg('capture_state_saved', [enregistrement.dernierFichier]);
  } else {
    statut.textContent = msg('capture_state_ready');
  }
  // L'agent a change d'onglet pendant une capture liee a un onglet: la video ne
  // le suivra pas, et c'est le genre de chose qu'on decouvre en regardant le
  // fichier, trop tard.
  if (enregistrement.avertissement) {
    statut.classList.add('erreur');
    statut.textContent += ' ' + msg('capture_state_warning', [enregistrement.avertissement]);
  }
}

async function rafraichir() {
  const etat = await chrome.runtime.sendMessage({ type: 'get-etat' });
  if (etat) peindre(etat);
}

$('actif').addEventListener('change', async (e) => {
  await chrome.runtime.sendMessage({ type: 'set-actif', actif: e.target.checked });
  setTimeout(rafraichir, 250);
});

$('port').addEventListener('change', async (e) => {
  const port = parseInt(e.target.value, 10);
  if (!Number.isInteger(port) || port < 1 || port > 65535) return rafraichir();
  await chrome.runtime.sendMessage({ type: 'set-port', port });
  setTimeout(rafraichir, 250);
});

function valeursCapture() {
  return {
    captureActivee: $('captureActivee').checked,
    captureSource: $('captureSource').value,
    captureAudio: $('captureAudio').checked,
    captureDossier: $('captureDossier').value,
    captureDemanderEmplacement: $('captureDemanderEmplacement').checked,
  };
}

async function sauverCapture() {
  await chrome.runtime.sendMessage({
    type: 'set-capture-settings',
    patch: valeursCapture(),
  });
  await rafraichir();
}

for (const id of [
  'captureActivee',
  'captureSource',
  'captureAudio',
  'captureDossier',
  'captureDemanderEmplacement',
]) {
  $(id).addEventListener('change', sauverCapture);
}

$('demarrerCapture').addEventListener('click', async () => {
  actionCaptureEnCours = true;
  peindre(dernierEtat || {});
  try {
    const resultat = await chrome.runtime.sendMessage({
      type: 'enregistrement-preparer',
      source: $('captureSource').value,
      audio: $('captureAudio').checked,
    });

    if (!resultat || !resultat.ok) {
      throw new Error((resultat && resultat.error) || 'recording could not start');
    }
  } catch (err) {
    $('etatCapture').classList.add('erreur');
    $('etatCapture').textContent = msg('capture_state_error', [
      String((err && err.message) || err),
    ]);
  } finally {
    actionCaptureEnCours = false;
    setTimeout(rafraichir, 150);
  }
});

$('arreterCapture').addEventListener('click', async () => {
  actionCaptureEnCours = true;
  peindre(dernierEtat || {});
  await chrome.runtime.sendMessage({ type: 'enregistrement-arreter' }).catch(() => {});
  actionCaptureEnCours = false;
  setTimeout(rafraichir, 150);
});

chrome.runtime.onMessage.addListener((msgEvent) => {
  if (msgEvent.type === 'etat') rafraichir();
});

chargerMessagesSecours().then(() => {
  traduire();
  if (dernierEtat) peindre(dernierEtat);
});

traduire();
rafraichir();
// The socket may settle a moment after the popup opens.
setInterval(rafraichir, 1500);
