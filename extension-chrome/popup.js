/** Popup: connection state, the on/off switch and the node port. */

const $ = (id) => document.getElementById(id);

// Every visible string comes from _locales, none is written here.
function traduire() {
  $('libelleActif').textContent = chrome.i18n.getMessage('popup_enable');
  $('libellePort').textContent = chrome.i18n.getMessage('popup_port');
  $('note').textContent = chrome.i18n.getMessage('popup_note');
  document.title = chrome.i18n.getMessage('ext_name');
}

function peindre(etat) {
  const pastille = $('pastille');
  pastille.classList.toggle('on', etat.connecte);
  pastille.classList.toggle('erreur', etat.actif && !etat.connecte);

  let cle = 'state_off';
  if (etat.actif) cle = etat.connecte ? 'state_connected' : 'state_connecting';
  $('etat').textContent = chrome.i18n.getMessage(cle);

  $('actif').checked = !!etat.actif;
  if (document.activeElement !== $('port')) $('port').value = etat.port;

  const pilote = $('pilote');
  if (etat.piloteUrl) {
    pilote.hidden = false;
    pilote.textContent = chrome.i18n.getMessage('popup_driving', [etat.piloteUrl]);
  } else {
    pilote.hidden = true;
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

chrome.runtime.onMessage.addListener((msg) => {
  if (msg.type === 'etat') rafraichir();
});

traduire();
rafraichir();
// The socket may settle a moment after the popup opens.
setInterval(rafraichir, 1500);
