/** Popup: connection state, the on/off switch and the node port. */

const $ = (id) => document.getElementById(id);

/// Pose un ecouteur, ou ne fait rien si l'element n'est pas la.
///
/// Neuf `$('x').addEventListener(...)` vivaient au niveau racine de ce fichier.
/// Un seul id absent, et l'exception remontait a la racine: tout ce qui suit
/// cessait d'exister. Le panneau s'ouvrait a moitie cable, les onglets ne
/// repondaient plus, et la seule trace etait une ligne dans la console. Un
/// element manquant doit couter un ecouteur, pas le fichier entier.
function sur(id, evenement, action) {
  const el = $(id);
  if (el) el.addEventListener(evenement, action);
  else console.warn(`LaRuche: element "${id}" absent du popup`);
}
let dernierEtat = null;
let actionCaptureEnCours = false;
let messagesSecours = null;
let interfacePrete = false;

/// La langue choisie par l'utilisateur, ou l'anglais.
///
/// `chrome.i18n` suit la langue du NAVIGATEUR et ne se detourne pas: il n'existe
/// aucune API pour lui demander une autre langue a l'execution. On charge donc
/// le fichier de locale nous-memes, ce que ce popup faisait deja en secours, et
/// on le fait passer AVANT `chrome.i18n` pour que le choix de l'utilisateur
/// l'emporte sur celui de Chrome.
///
/// Anglais par defaut: le depot, le code et les messages d'outil le sont, et
/// une extension qui bascule en francais parce que le navigateur est francais
/// surprend plus qu'elle n'aide.
const LANGUES = ['en', 'fr'];
let langue = 'en';

async function chargerLangue() {
  const { langue: choisie } = await chrome.storage.local.get({ langue: 'en' });
  langue = LANGUES.includes(choisie) ? choisie : 'en';
  return langue;
}

async function chargerMessagesSecours() {
  if (messagesSecours) return messagesSecours;
  const essais = [...new Set([langue, 'en', 'fr'])];
  for (const code of essais) {
    try {
      const url = chrome.runtime.getURL(`_locales/${code}/messages.json`);
      const rep = await fetch(url).catch(() => null);
      if (rep && rep.ok) {
        messagesSecours = await rep.json();
        break;
      }
    } catch {}
  }
  return messagesSecours;
}

function msg(cle, substitutions) {
  // Le fichier charge d'abord: c'est lui qui porte le choix de l'utilisateur.
  // `chrome.i18n` ne sert plus que de filet si le fetch a echoue.
  if (!(messagesSecours && messagesSecours[cle] && messagesSecours[cle].message)) {
    try {
      const direct = chrome.i18n && chrome.i18n.getMessage(cle, substitutions);
      if (direct) return direct;
    } catch {}
  }

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
  // Une seule table, et un element absent est saute plutot que fatal.
  //
  // C'etait ecrit en une suite de `$('x').textContent = ...`, et le premier id
  // manquant levait, ce qui interrompait TOUT le reste: le panneau se retrouvait
  // a moitie traduit, le bouton de langue vide, et la seule trace etait une
  // ligne dans la page des erreurs de Chrome. Un libelle manquant doit couter
  // un libelle, pas la traduction entiere.
  const TABLE = [
    ['libelleDestination', 'garder_destination'],
    ['destArticles', 'garder_dest_articles'],
    ['destVideos', 'garder_dest_videos'],
    ['destNotes', 'garder_dest_notes'],
    ['libelleCommentaire', 'garder_commentaire'],
    ['libelleGarderLien', 'toolbar_keep'],
    ['libelleGarderNote', 'toolbar_note'],
    ['aideGarder', 'garder_aide'],
    ['libelleActif', 'toolbar_control'],
    ['libelleCompagnon', 'toolbar_companion'],
    ['libelleCurseurAgent', 'popup_agent_cursor'],
    ['libellePort', 'popup_port'],
    ['titreConfidentialite', 'popup_privacy_summary'],
    ['noteConfidentialite', 'popup_note'],
    ['lienConfidentialite', 'popup_privacy_link'],
    ['titreCapture', 'capture_title'],
    ['libelleCaptureActivee', 'capture_enable'],
    ['libelleCaptureSource', 'capture_source'],
    ['sourceOnglet', 'capture_source_tab'],
    ['sourceFenetre', 'capture_source_window'],
    ['sourceEcran', 'capture_source_screen'],
    ['libelleCaptureQualite', 'capture_quality'],
    ['qualiteBasse', 'capture_quality_low'],
    ['qualiteStandard', 'capture_quality_standard'],
    ['qualiteHaute', 'capture_quality_high'],
    ['libelleCaptureAudio', 'capture_audio'],
    ['libelleCaptureDossier', 'capture_folder'],
    ['libelleCaptureDemanderEmplacement', 'capture_save_as'],
    ['libelleDemarrerCapture', 'toolbar_record'],
    ['libelleArreterCapture', 'toolbar_stop'],
    ['aideCapture', 'capture_help'],
  ];
  const manquants = [];
  for (const [id, cle] of TABLE) {
    const el = $(id);
    if (el) el.textContent = msg(cle);
    else manquants.push(id);
  }
  if (manquants.length) {
    console.warn('LaRuche: elements absents du popup:', manquants.join(', '));
  }
  document.title = msg('ext_name');
  document.documentElement.lang = langue;
  // Le bouton montre la langue vers laquelle il bascule, pas la langue courante:
  // "FR" quand on est en anglais. C'est ce que fait tout selecteur de langue, et
  // l'inverse fait hesiter.
  const bouton = $('langue');
  if (bouton) {
    const autre = langue === 'en' ? 'fr' : 'en';
    bouton.textContent = autre.toUpperCase();
    bouton.title = msg('popup_language');
    bouton.setAttribute('aria-label', msg('popup_language'));
  }
}

async function basculerLangue() {
  langue = langue === 'en' ? 'fr' : 'en';
  await chrome.storage.local.set({ langue });
  messagesSecours = null;
  await chargerMessagesSecours();
  traduire();
  peindre(dernierEtat || {});
}

function peindre(etat) {
  dernierEtat = etat;
  if (!interfacePrete) return;
  const pastille = $('pastille');
  pastille.classList.toggle('on', etat.connecte);
  pastille.classList.toggle('erreur', etat.actif && !etat.connecte);

  let cle = 'state_off';
  if (etat.actif) cle = etat.connecte ? 'state_connected' : 'state_connecting';
  $('etat').textContent = msg(cle);

  $('actif').checked = !!etat.actif;
  $('compagnon').checked = !!etat.compagnon;
  $('curseurAgent').checked = !!etat.curseurAgent;
  if (document.activeElement !== $('port')) $('port').value = etat.port;

  const pilote = $('pilote');
  if (etat.piloteUrl) {
    pilote.hidden = false;
    pilote.textContent = msg('popup_driving', [etat.piloteUrl]);
  } else {
    pilote.hidden = true;
  }

  peindreCapture(etat);

  const enregistrement = etat.enregistrement || { etat: 'inactif' };
  const occupe = ['demarrage', 'enregistrement', 'finalisation', 'sauvegarde', 'arme'].includes(enregistrement.etat);
  const captureEnCours = ['demarrage', 'enregistrement', 'finalisation', 'sauvegarde'].includes(enregistrement.etat);
  const options = $('captureOptions');
  options.classList.toggle('desactive', !etat.captureActivee);
  for (const champ of options.querySelectorAll('input, select')) {
    champ.disabled = !etat.captureActivee || occupe;
  }
  // Un seul bouton occupe la case Enregistrer de la barre d'outils. L'etat
  // arme garde le bouton de demarrage visible, car aucune capture ne tourne.
  $('demarrerCapture').hidden = captureEnCours;
  $('demarrerCapture').disabled = !etat.captureActivee || actionCaptureEnCours;
  $('arreterCapture').hidden = !captureEnCours;
  $('arreterCapture').disabled = actionCaptureEnCours || enregistrement.etat !== 'enregistrement';

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

sur('actif', 'change', async (e) => {
  await chrome.runtime.sendMessage({ type: 'set-actif', actif: e.target.checked });
  setTimeout(rafraichir, 250);
});

sur('compagnon', 'change', async (e) => {
  if (e.target.checked && !(await demanderPermissionsVisuelles())) {
    e.target.checked = false;
    $('etat').textContent = msg('visual_permission_denied');
    return;
  }
  await chrome.storage.local.set({ compagnon: e.target.checked });
  setTimeout(rafraichir, 150);
});

sur('curseurAgent', 'change', async (e) => {
  if (e.target.checked && !(await demanderPermissionsVisuelles())) {
    e.target.checked = false;
    $('etat').textContent = msg('visual_permission_denied');
    return;
  }
  await chrome.storage.local.set({ curseurAgent: e.target.checked });
  setTimeout(rafraichir, 150);
});

async function demanderPermissionsVisuelles() {
  const demande = {
    permissions: ['scripting'],
    origins: ['http://*/*', 'https://*/*'],
  };
  // `request` doit etre le premier appel asynchrone issu du clic. Un
  // `contains` attendu juste avant suffit a faire perdre le geste utilisateur
  // dans certaines versions de Chrome.
  return await chrome.permissions.request(demande);
}

sur('port', 'change', async (e) => {
  const port = parseInt(e.target.value, 10);
  if (!Number.isInteger(port) || port < 1 || port > 65535) return lirePageCourante().catch(() => {});
  await chrome.runtime.sendMessage({ type: 'set-port', port });
  setTimeout(rafraichir, 250);
});

/// Les reglages de capture, et comment chacun se lit et s'ecrit.
///
/// UNE table, parcourue dans les deux sens. Il y en avait deux, une pour
/// l'ecriture et une pour la relecture, tenues en phase a la main: la case
/// "masquer le decor" a ete ajoutee a la premiere et oubliee dans la seconde,
/// donc elle s'enregistrait correctement et revenait decochee a chaque
/// ouverture. Un reglage qui ne se relit pas est un reglage qui n'existe pas,
/// et le symptome ne ressemble pas a un bug de lecture: on croit que
/// l'enregistrement n'a pas ete fait.
const REGLAGES_CAPTURE = [
  ['captureActivee', 'case'],
  ['captureSource', 'liste'],
  ['captureQualite', 'liste'],
  ['captureAudio', 'case'],
  ['captureDossier', 'texte'],
  ['captureDemanderEmplacement', 'case'],
];

function valeursCapture() {
  const out = {};
  for (const [id, genre] of REGLAGES_CAPTURE) {
    const el = $(id);
    if (!el) continue;
    out[id] = genre === 'case' ? el.checked : el.value;
  }
  return out;
}

/// Repose les reglages dans les controles.
///
/// On ne touche pas au controle que l'utilisateur est en train d'editer: le
/// rafraichissement tourne toutes les 1,5s et lui reprendrait son curseur au
/// milieu d'une frappe.
function peindreCapture(etat) {
  for (const [id, genre] of REGLAGES_CAPTURE) {
    const el = $(id);
    if (!el || document.activeElement === el) continue;
    if (genre === 'case') el.checked = !!etat[id];
    else if (etat[id] !== undefined && etat[id] !== null) el.value = etat[id];
  }
  if (!$('captureDossier').value) $('captureDossier').value = 'LaRuche/showcases';
}

async function sauverCapture() {
  const patch = valeursCapture();
  try {
    await demanderPermissionsCapture(patch);
  } catch (err) {
    $('captureActivee').checked = false;
    await chrome.runtime.sendMessage({
      type: 'set-capture-settings',
      patch: { ...patch, captureActivee: false },
    }).catch(() => {});
    $('etatCapture').classList.add('erreur');
    $('etatCapture').textContent = msg('capture_state_error', [
      String((err && err.message) || err),
    ]);
    return;
  }
  await chrome.runtime.sendMessage({ type: 'set-capture-settings', patch });
  await armerSiNecessaire(patch);
  await rafraichir();
}

/// L'enregistrement est annexe au pilotage. Ses droits ne sont donc demandes
/// qu'au moment ou l'utilisateur l'active, avec le clic qui permet a Chrome
/// d'afficher sa boite de permission. Le pilotage de base n'en a pas besoin.
async function demanderPermissionsCapture(patch) {
  if (!patch.captureActivee) return;
  const permissions = ['downloads', 'offscreen'];
  if (patch.captureSource === 'tab') permissions.push('tabCapture');
  const accord = await chrome.permissions.request({ permissions });
  if (!accord) throw new Error(msg('capture_permission_denied'));
}

/// Demande la source MAINTENANT, si elle a besoin d'etre choisie.
///
/// Le selecteur d'ecran de Chrome exige un geste de l'utilisateur. Le seul geste
/// naturel, c'est celui qu'il vient de faire: choisir "Ecran" ou "Fenetre" dans
/// la liste. Demander a ce moment-la est ce que tout logiciel d'enregistrement
/// fait, et ca laisse ensuite l'enregistrement demarrer tout seul quand l'agent
/// prend la main.
///
/// L'onglet est le cas facile et n'a rien a armer: l'agent travaille dans un
/// onglet qu'il cree lui-meme, qui n'existe pas encore, et c'est l'extension qui
/// le capturera au moment ou il prendra la main.
async function armerSiNecessaire(patch) {
  const etat = (dernierEtat && dernierEtat.enregistrement) || {};
  const besoinDeChoisir = patch.captureActivee && patch.captureSource !== 'tab';

  // Plus besoin de la source armee: on la relache plutot que de laisser un
  // partage d'ecran actif que l'utilisateur voit dans sa barre sans comprendre.
  if (!besoinDeChoisir && etat.etat === 'arme') {
    await chrome.runtime.sendMessage({ type: 'enregistrement-arreter' }).catch(() => {});
    return;
  }
  if (!besoinDeChoisir) return;
  // Deja armee sur la meme source: ne pas redemander, ce serait un selecteur
  // qui resurgit a chaque case cochee.
  if (etat.etat === 'arme' && etat.source === patch.captureSource) return;
  if (['enregistrement', 'demarrage', 'finalisation', 'sauvegarde'].includes(etat.etat)) return;

  actionCaptureEnCours = true;
  peindre(dernierEtat || {});
  try {
    const r = await chrome.runtime.sendMessage({
      type: 'enregistrement-preparer',
      source: patch.captureSource,
      audio: patch.captureAudio,
    });
    if (!r || !r.ok) throw new Error((r && r.error) || 'source refusee');
  } catch (err) {
    // Refuser le partage est un choix legitime, pas une panne: on remet la
    // liste sur l'onglet plutot que de laisser un reglage qui ne marchera pas.
    $('captureSource').value = 'tab';
    await chrome.runtime.sendMessage({
      type: 'set-capture-settings',
      patch: { ...patch, captureSource: 'tab' },
    }).catch(() => {});
  } finally {
    actionCaptureEnCours = false;
  }
}

/// Les sites ou une page EST une video. Pre-selectionner la bonne destination
/// evite le geste le plus penible: corriger un menu avant chaque sauvegarde.
const SITES_VIDEO = /(youtube\.com|youtu\.be|vimeo\.com|dailymotion\.com|twitch\.tv|peertube)/i;

let pageCourante = { titre: '', url: '' };

async function lirePageCourante() {
  const [onglet] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!onglet) return;
  pageCourante = { titre: onglet.title || '', url: onglet.url || '' };
  $('pageTitre').textContent = pageCourante.titre || pageCourante.url;
  $('pageUrl').textContent = pageCourante.url;
  $('pageTitre').title = $('pageTitre').textContent;
  $('pageUrl').title = pageCourante.url;
  // On ne devine QUE la video, qui est sans ambiguite. Pour le reste, articles
  // est le choix le plus courant et deviner mal ferait perdre plus de temps
  // qu'un menu deja bien place.
  if (SITES_VIDEO.test(pageCourante.url)) $('destination').value = 'veille.videos';
}

/// Le contenu ecrit en memoire.
///
/// Une seule ligne lisible telle quelle: le titre, l'URL, puis le commentaire.
/// La memoire se relit a la recherche, et un item qui ne dit pas de quoi il
/// parle ne ressort jamais.
function composer(avecLien) {
  const commentaire = $('commentaire').value.trim();
  if (!avecLien) return commentaire;
  const titre = pageCourante.titre || pageCourante.url;
  const bouts = [titre, pageCourante.url];
  if (commentaire) bouts.push('', commentaire);
  return bouts.join('\n');
}

function nomDestination() {
  const opt = $('destination').selectedOptions[0];
  return (opt && opt.textContent) || $('destination').value;
}

async function garder(avecLien) {
  const contenu = composer(avecLien);
  if (!contenu) {
    direGarder(msg('garder_vide'), 'erreur');
    return;
  }
  const boutons = [$('garderLien'), $('garderNote')];
  boutons.forEach((b) => { b.disabled = true; });
  try {
    const r = await chrome.runtime.sendMessage({
      type: 'garder',
      entree: { noeud: $('destination').value, contenu },
    });
    if (!r || !r.ok) throw new Error((r && r.error) || 'echec');
    if (r.envoye) {
      direGarder(msg('garder_ok', [nomDestination()]), 'ok');
      $('commentaire').value = '';
    } else {
      // Le point qui compte: l'utilisateur doit savoir que c'est garde, pas
      // perdu, sinon il refait le geste ou renonce. Et savoir CE QU'IL doit
      // faire: attendre ne debloque pas un probleme de connexion.
      const cle = r.genre === 'connexion' ? 'garder_connexion' : 'garder_en_attente';
      direGarder(msg(cle), 'attente');
      $('commentaire').value = '';
    }
  } catch (e) {
    direGarder(msg('garder_echec', [String((e && e.message) || e)]), 'erreur');
  } finally {
    boutons.forEach((b) => { b.disabled = false; });
    majFileGarder();
  }
}

function direGarder(texte, genre) {
  const el = $('etatGarder');
  el.textContent = texte;
  el.classList.toggle('erreur', genre === 'erreur');
  el.classList.toggle('en-cours', genre === 'attente' || genre === 'ok');
}

async function majFileGarder() {
  const r = await chrome.runtime.sendMessage({ type: 'garder-file' }).catch(() => null);
  if (r && r.ok && r.restants > 0) {
    const el = $('etatGarder');
    const suffixe = msg('garder_file', [String(r.restants)]);
    if (!el.textContent.includes(suffixe)) {
      el.textContent = (el.textContent ? el.textContent + ' ' : '') + suffixe;
    }
  }
}

sur('garderLien', 'click', () => garder(true));
sur('garderNote', 'click', () => garder(false));

for (const [id] of REGLAGES_CAPTURE) {
  const el = $(id);
  if (el) el.addEventListener('change', sauverCapture);
}

sur('demarrerCapture', 'click', async () => {
  actionCaptureEnCours = true;
  peindre(dernierEtat || {});
  try {
    await demanderPermissionsCapture(valeursCapture());
    const resultat = await chrome.runtime.sendMessage({
      type: 'enregistrement-demarrer-manuel',
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

sur('arreterCapture', 'click', async () => {
  actionCaptureEnCours = true;
  peindre(dernierEtat || {});
  await chrome.runtime.sendMessage({ type: 'enregistrement-arreter' }).catch(() => {});
  actionCaptureEnCours = false;
  setTimeout(rafraichir, 150);
});

chrome.runtime.onMessage.addListener((msgEvent) => {
  if (msgEvent.type === 'etat') rafraichir();
});

sur('langue', 'click', () => {
  basculerLangue().catch(() => {});
});

// Rien ne se peint avant que le catalogue choisi soit charge. Sinon le premier
// rafraichissement passe par chrome.i18n, qui suit la langue du navigateur et
// affiche brièvement du francais sur un Chrome francais, meme si l'anglais est
// bien le choix par defaut de l'extension.
async function initialiser() {
  await chargerLangue();
  await chargerMessagesSecours();
  interfacePrete = true;
  traduire();
  await lirePageCourante().catch(() => {});
  await majFileGarder().catch(() => {});
  await rafraichir();
  // The socket may settle a moment after the popup opens.
  setInterval(rafraichir, 1500);
}

initialiser().catch(() => {
  langue = 'en';
  interfacePrete = true;
  traduire();
  rafraichir().catch(() => {});
});
