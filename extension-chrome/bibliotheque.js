const $ = (id) => document.getElementById(id);

const apiExtension =
  typeof chrome !== 'undefined' && chrome.runtime && chrome.runtime.id ? chrome : null;

const LANGUES = ['en', 'fr'];
const VIDEO_EXTENSIONS = new Set(['mp4', 'webm', 'mov', 'm4v', 'ogv']);
const DB_NAME = 'laruche-video-library';
const DB_VERSION = 1;
const STORE_HANDLES = 'handles';
const HANDLE_LIBRARY = 'showcases-directory';

let langue = 'en';
let messages = {};
let notificationTimer = null;

const etat = {
  dossier: null,
  permissionDossier: 'prompt',
  videos: [],
  selection: null,
  urlSelection: null,
  duree: 0,
  debut: 0,
  fin: 0,
  recadrage: { x: 0, y: 0, w: 1, h: 1 },
  recadrageActif: false,
  generationTimeline: 0,
  export: null,
};

/* ------------------------------------------------------------- traduction */

async function lireLangue() {
  if (apiExtension) {
    const resultat = await apiExtension.storage.local.get({ langue: 'en' });
    return LANGUES.includes(resultat.langue) ? resultat.langue : 'en';
  }
  const locale = localStorage.getItem('laruche-language') || 'en';
  return LANGUES.includes(locale) ? locale : 'en';
}

async function ecrireLangue(code) {
  if (apiExtension) await apiExtension.storage.local.set({ langue: code });
  else localStorage.setItem('laruche-language', code);
}

async function chargerMessages() {
  const base = apiExtension
    ? apiExtension.runtime.getURL(`_locales/${langue}/messages.json`)
    : `_locales/${langue}/messages.json`;
  const reponse = await fetch(base);
  if (!reponse.ok) throw new Error(`Locale ${langue} indisponible`);
  messages = await reponse.json();
}

function t(cle, substitutions = []) {
  const entree = messages[cle];
  if (!entree || !entree.message) return cle;
  const valeurs = Array.isArray(substitutions) ? substitutions : [substitutions];
  let texte = entree.message;
  valeurs.forEach((valeur, index) => {
    texte = texte.replace(new RegExp(`\\$${index + 1}`, 'g'), String(valeur));
  });
  if (entree.placeholders) {
    for (const [nom, placeholder] of Object.entries(entree.placeholders)) {
      const index = Number.parseInt(String(placeholder.content || '').replace('$', ''), 10) - 1;
      const valeur = index >= 0 && valeurs[index] !== undefined ? String(valeurs[index]) : '';
      texte = texte.replace(new RegExp(`\\$${nom.toUpperCase()}\\$`, 'g'), valeur);
    }
  }
  return texte;
}

function traduirePage() {
  document.documentElement.lang = langue;
  document.title = t('library_document_title');
  for (const element of document.querySelectorAll('[data-i18n]')) {
    element.textContent = t(element.dataset.i18n);
  }
  for (const element of document.querySelectorAll('[data-i18n-placeholder]')) {
    element.placeholder = t(element.dataset.i18nPlaceholder);
  }
  for (const element of document.querySelectorAll('[data-i18n-aria-label]')) {
    element.setAttribute('aria-label', t(element.dataset.i18nAriaLabel));
  }
  $('langue').textContent = langue === 'en' ? 'FR' : 'EN';
  $('langue').title = t('popup_language');
  actualiserEtatDossier();
  afficherVideos();
  actualiserEdition();
}

/* --------------------------------------------------------------- utilitaires */

function notifier(message, erreur = false) {
  const element = $('notification');
  element.textContent = message;
  element.classList.toggle('erreur', erreur);
  element.hidden = false;
  clearTimeout(notificationTimer);
  notificationTimer = setTimeout(() => {
    element.hidden = true;
  }, erreur ? 7000 : 3800);
}

function erreurLisible(erreur) {
  if (!erreur) return t('library_unknown_error');
  if (erreur.name === 'AbortError') return t('library_cancelled');
  if (erreur.name === 'NotAllowedError') return t('library_permission_denied');
  return erreur.message || String(erreur);
}

function extensionDe(nom) {
  const index = nom.lastIndexOf('.');
  return index >= 0 ? nom.slice(index + 1).toLowerCase() : '';
}

function baseDe(nom) {
  const index = nom.lastIndexOf('.');
  return index > 0 ? nom.slice(0, index) : nom;
}

function nettoyerNom(nom) {
  return String(nom || '')
    .replace(/[<>:"/\\|?*\u0000-\u001f]/g, '-')
    .replace(/[. ]+$/g, '')
    .trim();
}

function formaterTaille(octets) {
  if (!Number.isFinite(octets) || octets < 1) return '0 B';
  const unites = ['B', 'KB', 'MB', 'GB', 'TB'];
  const index = Math.min(Math.floor(Math.log(octets) / Math.log(1024)), unites.length - 1);
  const valeur = octets / 1024 ** index;
  return `${valeur >= 10 || index === 0 ? valeur.toFixed(0) : valeur.toFixed(1)} ${unites[index]}`;
}

function formaterTemps(secondes, precis = false) {
  if (!Number.isFinite(secondes) || secondes < 0) secondes = 0;
  const heures = Math.floor(secondes / 3600);
  const minutes = Math.floor((secondes % 3600) / 60);
  const reste = secondes % 60;
  const sec = precis ? reste.toFixed(2).padStart(5, '0') : Math.floor(reste).toString().padStart(2, '0');
  return heures > 0
    ? `${heures}:${minutes.toString().padStart(2, '0')}:${sec}`
    : `${minutes}:${sec}`;
}

function formaterDate(timestamp) {
  return new Intl.DateTimeFormat(langue === 'fr' ? 'fr-FR' : 'en-US', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(timestamp));
}

function attendre(element, evenement, delai = 15000) {
  return new Promise((resolve, reject) => {
    const minuteur = setTimeout(() => {
      nettoyer();
      reject(new Error(t('library_media_timeout')));
    }, delai);
    const succes = () => {
      nettoyer();
      resolve();
    };
    const echec = () => {
      nettoyer();
      reject(new Error(t('library_media_error')));
    };
    const nettoyer = () => {
      clearTimeout(minuteur);
      element.removeEventListener(evenement, succes);
      element.removeEventListener('error', echec);
    };
    element.addEventListener(evenement, succes, { once: true });
    element.addEventListener('error', echec, { once: true });
  });
}

async function positionner(video, temps) {
  if (Math.abs(video.currentTime - temps) < 0.015) return;
  const attente = attendre(video, 'seeked');
  video.currentTime = temps;
  await attente;
}

/* ---------------------------------------------------------- dossier persistant */

function ouvrirBase() {
  return new Promise((resolve, reject) => {
    const requete = indexedDB.open(DB_NAME, DB_VERSION);
    requete.addEventListener('upgradeneeded', () => {
      if (!requete.result.objectStoreNames.contains(STORE_HANDLES)) {
        requete.result.createObjectStore(STORE_HANDLES);
      }
    });
    requete.addEventListener('success', () => resolve(requete.result));
    requete.addEventListener('error', () => reject(requete.error));
  });
}

async function lireHandleDossier() {
  const base = await ouvrirBase();
  try {
    return await new Promise((resolve, reject) => {
      const transaction = base.transaction(STORE_HANDLES, 'readonly');
      const requete = transaction.objectStore(STORE_HANDLES).get(HANDLE_LIBRARY);
      requete.addEventListener('success', () => resolve(requete.result || null));
      requete.addEventListener('error', () => reject(requete.error));
    });
  } finally {
    base.close();
  }
}

async function memoriserHandleDossier(handle) {
  const base = await ouvrirBase();
  try {
    await new Promise((resolve, reject) => {
      const transaction = base.transaction(STORE_HANDLES, 'readwrite');
      transaction.objectStore(STORE_HANDLES).put(handle, HANDLE_LIBRARY);
      transaction.addEventListener('complete', resolve);
      transaction.addEventListener('error', () => reject(transaction.error));
    });
  } finally {
    base.close();
  }
}

async function permissionDossier(handle, demander = false) {
  const options = { mode: 'readwrite' };
  if (!handle) return 'denied';
  if (typeof handle.queryPermission !== 'function') return 'granted';
  let permission = await handle.queryPermission(options);
  if (permission !== 'granted' && demander && typeof handle.requestPermission === 'function') {
    permission = await handle.requestPermission(options);
  }
  return permission;
}

function actualiserEtatDossier() {
  const bouton = $('choisirDossier');
  if (!etat.dossier) {
    $('etatDossier').textContent = t('library_no_folder');
    bouton.textContent = t('library_choose_folder');
    return;
  }
  if (etat.permissionDossier !== 'granted') {
    $('etatDossier').textContent = t('library_reconnect_folder', [etat.dossier.name]);
    bouton.textContent = t('library_reconnect');
    return;
  }
  $('etatDossier').textContent = t('library_folder_ready', [etat.dossier.name]);
  bouton.textContent = t('library_change_folder');
}

async function choisirDossier() {
  try {
    if (etat.dossier && etat.permissionDossier !== 'granted') {
      etat.permissionDossier = await permissionDossier(etat.dossier, true);
      if (etat.permissionDossier === 'granted') {
        actualiserEtatDossier();
        await chargerBibliotheque();
        return;
      }
    }

    if (typeof window.showDirectoryPicker !== 'function') {
      throw new Error(t('library_picker_unavailable'));
    }
    const handle = await window.showDirectoryPicker({
      id: 'laruche-showcases',
      mode: 'readwrite',
      startIn: 'downloads',
    });
    etat.dossier = handle;
    etat.permissionDossier = await permissionDossier(handle, false);
    await memoriserHandleDossier(handle);
    actualiserEtatDossier();
    await chargerBibliotheque();
  } catch (erreur) {
    if (erreur && erreur.name === 'AbortError') return;
    notifier(erreurLisible(erreur), true);
  }
}

async function parcourirVideos(dossier, chemin = '', resultat = []) {
  for await (const [nom, handle] of dossier.entries()) {
    if (handle.kind === 'directory') {
      await parcourirVideos(handle, chemin ? `${chemin}/${nom}` : nom, resultat);
      continue;
    }
    if (!VIDEO_EXTENSIONS.has(extensionDe(nom))) continue;
    const fichier = await handle.getFile();
    resultat.push({
      cle: chemin ? `${chemin}/${nom}` : nom,
      chemin,
      nom,
      handle,
      dossier,
      fichier,
      taille: fichier.size,
      modifie: fichier.lastModified,
      duree: 0,
      largeur: 0,
      hauteur: 0,
      vignette: '',
      erreur: '',
    });
  }
  return resultat;
}

async function chargerBibliotheque(selectionAReprendre = '') {
  if (!etat.dossier || etat.permissionDossier !== 'granted') {
    etat.videos = [];
    afficherVideos();
    return;
  }
  $('actualiser').disabled = true;
  $('etatDossier').textContent = t('library_scanning', [etat.dossier.name]);
  try {
    etat.videos = await parcourirVideos(etat.dossier);
    afficherVideos();

    for (let index = 0; index < etat.videos.length; index += 3) {
      const lot = etat.videos.slice(index, index + 3);
      await Promise.all(lot.map((video) => lireMetadonnees(video).catch((erreur) => {
        video.erreur = erreurLisible(erreur);
      })));
      afficherVideos();
    }

    if (selectionAReprendre) {
      const video = etat.videos.find((entree) => entree.cle === selectionAReprendre);
      if (video) await selectionnerVideo(video);
    }
    actualiserEtatDossier();
  } catch (erreur) {
    notifier(erreurLisible(erreur), true);
    $('etatDossier').textContent = t('library_scan_error');
  } finally {
    $('actualiser').disabled = false;
  }
}

async function lireMetadonnees(entree) {
  const url = URL.createObjectURL(entree.fichier);
  const video = document.createElement('video');
  video.preload = 'auto';
  video.muted = true;
  video.src = url;
  try {
    await attendre(video, 'loadedmetadata');
    entree.duree = Number.isFinite(video.duration) ? video.duration : 0;
    entree.largeur = video.videoWidth;
    entree.hauteur = video.videoHeight;
    if (entree.duree > 0) {
      await positionner(video, Math.min(Math.max(entree.duree * 0.08, 0.05), Math.max(0.05, entree.duree - 0.05)));
    }
    const canvas = document.createElement('canvas');
    canvas.width = 320;
    canvas.height = 180;
    const contexte = canvas.getContext('2d');
    contexte.fillStyle = '#070604';
    contexte.fillRect(0, 0, canvas.width, canvas.height);
    const ratio = Math.min(canvas.width / Math.max(1, video.videoWidth), canvas.height / Math.max(1, video.videoHeight));
    const largeur = video.videoWidth * ratio;
    const hauteur = video.videoHeight * ratio;
    contexte.drawImage(video, (canvas.width - largeur) / 2, (canvas.height - hauteur) / 2, largeur, hauteur);
    entree.vignette = canvas.toDataURL('image/jpeg', 0.72);
  } finally {
    video.removeAttribute('src');
    video.load();
    URL.revokeObjectURL(url);
  }
}

/* --------------------------------------------------------------- liste videos */

function videosAffichees() {
  const recherche = $('recherche').value.trim().toLocaleLowerCase(langue);
  const resultat = etat.videos.filter((video) =>
    !recherche || video.nom.toLocaleLowerCase(langue).includes(recherche) || video.chemin.toLocaleLowerCase(langue).includes(recherche)
  );
  const tri = $('tri').value;
  resultat.sort((a, b) => {
    if (tri === 'old') return a.modifie - b.modifie;
    if (tri === 'name') return a.nom.localeCompare(b.nom, langue);
    if (tri === 'size') return b.taille - a.taille;
    return b.modifie - a.modifie;
  });
  return resultat;
}

function afficherVideos() {
  if (!$('listeVideos')) return;
  const videos = videosAffichees();
  const taille = etat.videos.reduce((total, video) => total + video.taille, 0);
  $('compteurVideos').textContent = t('library_video_count', [videos.length]);
  $('tailleVideos').textContent = formaterTaille(taille);
  $('listeVideos').replaceChildren();
  $('etatVide').hidden = videos.length > 0;

  for (const video of videos) {
    const fragment = $('modeleVideo').content.cloneNode(true);
    const carte = fragment.querySelector('.carte-video');
    carte.classList.toggle('selectionnee', etat.selection && etat.selection.cle === video.cle);
    carte.title = video.erreur || video.nom;
    const image = carte.querySelector('img');
    if (video.vignette) image.src = video.vignette;
    else image.removeAttribute('src');
    carte.querySelector('.duree').textContent = video.duree ? formaterTemps(video.duree) : '';
    carte.querySelector('.infos-carte strong').textContent = video.nom;
    carte.querySelector('.chemin').textContent = video.chemin || t('library_root_folder');
    carte.querySelector('.details').textContent = `${formaterTaille(video.taille)} · ${formaterDate(video.modifie)}`;
    carte.addEventListener('click', () => selectionnerVideo(video));
    $('listeVideos').appendChild(fragment);
  }
}

/* ------------------------------------------------------------ lecteur et coupe */

async function selectionnerVideo(video) {
  if (etat.export) return;
  if (etat.urlSelection) URL.revokeObjectURL(etat.urlSelection);
  etat.selection = video;
  etat.urlSelection = URL.createObjectURL(video.fichier);
  etat.recadrage = { x: 0, y: 0, w: 1, h: 1 };
  etat.recadrageActif = false;
  etat.generationTimeline += 1;

  const lecteur = $('video');
  lecteur.src = etat.urlSelection;
  lecteur.load();
  try {
    await attendre(lecteur, 'loadedmetadata');
  } catch (erreur) {
    notifier(erreurLisible(erreur), true);
    return;
  }

  etat.duree = Number.isFinite(lecteur.duration) ? lecteur.duration : video.duree;
  etat.debut = 0;
  etat.fin = etat.duree;
  video.duree = etat.duree;
  video.largeur = lecteur.videoWidth;
  video.hauteur = lecteur.videoHeight;
  $('sceneVideo').style.aspectRatio = `${Math.max(1, lecteur.videoWidth)} / ${Math.max(1, lecteur.videoHeight)}`;
  $('nomVideo').textContent = video.nom;
  $('metaVideo').textContent = `${lecteur.videoWidth} × ${lecteur.videoHeight} · ${formaterTemps(etat.duree, true)} · ${formaterTaille(video.taille)}`;
  $('nouveauNom').value = video.nom;
  $('nomExport').value = `${baseDe(video.nom)}-edited.${extensionDe(video.nom) || 'mp4'}`;
  $('tempsDebut').max = String(etat.duree);
  $('tempsFin').max = String(etat.duree);
  $('accueilEditeur').hidden = true;
  $('editeur').hidden = false;
  actualiserEdition();
  afficherVideos();
  genererTimeline(video, etat.generationTimeline).catch(() => {});
}

function bornerCoupe() {
  const minimum = Math.min(0.05, etat.duree);
  etat.debut = Math.max(0, Math.min(etat.debut, Math.max(0, etat.fin - minimum)));
  etat.fin = Math.min(etat.duree, Math.max(etat.fin, etat.debut + minimum));
}

function dimensionsSource() {
  const lecteur = $('video');
  return {
    largeur: Math.max(1, lecteur.videoWidth || etat.selection?.largeur || 1),
    hauteur: Math.max(1, lecteur.videoHeight || etat.selection?.hauteur || 1),
  };
}

function bornerRecadrage(recadrage = etat.recadrage) {
  const dimensions = dimensionsSource();
  const minimumLargeur = Math.min(1, 2 / dimensions.largeur);
  const minimumHauteur = Math.min(1, 2 / dimensions.hauteur);
  const w = Math.max(minimumLargeur, Math.min(1, Number(recadrage.w) || minimumLargeur));
  const h = Math.max(minimumHauteur, Math.min(1, Number(recadrage.h) || minimumHauteur));
  const x = Math.max(0, Math.min(1 - w, Number(recadrage.x) || 0));
  const y = Math.max(0, Math.min(1 - h, Number(recadrage.y) || 0));
  return { x, y, w, h };
}

function afficherInstantCoupe(temps) {
  if (!Number.isFinite(temps) || !etat.duree) return;
  const cible = Math.max(0, Math.min(etat.duree, temps));
  const lecteur = $('video');
  lecteur.pause();
  lecteur.currentTime = cible;
  $('teteLecture').style.left = `${(cible / etat.duree) * 100}%`;
}

function actualiserEdition() {
  if (!etat.selection || !$('editeur')) return;
  bornerCoupe();
  const duree = Math.max(0.001, etat.duree);
  const debutPct = (etat.debut / duree) * 100;
  const finPct = (etat.fin / duree) * 100;
  $('poigneeDebut').style.left = `${debutPct}%`;
  $('poigneeFin').style.left = `calc(${finPct}% - 12px)`;
  $('selectionTimeline').style.left = `${debutPct}%`;
  $('selectionTimeline').style.width = `${Math.max(0, finPct - debutPct)}%`;
  $('masqueAvant').style.width = `${debutPct}%`;
  $('masqueApres').style.width = `${Math.max(0, 100 - finPct)}%`;
  $('tempsDebut').value = etat.debut.toFixed(2);
  $('tempsFin').value = etat.fin.toFixed(2);
  $('dureeSelection').textContent = t('timeline_selection', [formaterTemps(etat.fin - etat.debut, true)]);

  const lecture = Number.isFinite($('video').currentTime) ? $('video').currentTime : 0;
  $('teteLecture').style.left = `${Math.max(0, Math.min(100, (lecture / duree) * 100))}%`;

  etat.recadrage = bornerRecadrage();
  const crop = etat.recadrage;
  const dimensions = dimensionsSource();
  $('calqueRecadrage').classList.toggle('actif', etat.recadrageActif);
  $('basculerRecadrage').textContent = t(etat.recadrageActif ? 'crop_done' : 'crop_edit');
  const cadre = $('cadreRecadrage');
  cadre.style.left = `${crop.x * 100}%`;
  cadre.style.top = `${crop.y * 100}%`;
  cadre.style.width = `${crop.w * 100}%`;
  cadre.style.height = `${crop.h * 100}%`;

  const xPixels = Math.round(crop.x * dimensions.largeur);
  const yPixels = Math.round(crop.y * dimensions.hauteur);
  const largeurPixels = Math.round(crop.w * dimensions.largeur);
  const hauteurPixels = Math.round(crop.h * dimensions.hauteur);
  $('recadrageX').max = String(Math.max(0, dimensions.largeur - largeurPixels));
  $('recadrageY').max = String(Math.max(0, dimensions.hauteur - hauteurPixels));
  $('recadrageLargeur').max = String(Math.max(2, dimensions.largeur - xPixels));
  $('recadrageHauteur').max = String(Math.max(2, dimensions.hauteur - yPixels));
  $('recadrageX').value = String(xPixels);
  $('recadrageY').value = String(yPixels);
  $('recadrageLargeur').value = String(largeurPixels);
  $('recadrageHauteur').value = String(hauteurPixels);
  $('valeursRecadrage').textContent = t('crop_values', [
    Math.round(crop.x * 100),
    Math.round(crop.y * 100),
    Math.round(crop.w * 100),
    Math.round(crop.h * 100),
  ]);
}

async function genererTimeline(videoEntree, generation) {
  const conteneur = $('vignettesTimeline');
  conteneur.replaceChildren();
  const url = URL.createObjectURL(videoEntree.fichier);
  const source = document.createElement('video');
  source.preload = 'auto';
  source.muted = true;
  source.src = url;
  try {
    await attendre(source, 'loadedmetadata');
    const nombre = Math.max(6, Math.min(14, Math.round(conteneurLargeurTimeline() / 110)));
    const canvas = document.createElement('canvas');
    canvas.width = 220;
    canvas.height = 124;
    const contexte = canvas.getContext('2d');
    for (let index = 0; index < nombre; index += 1) {
      if (generation !== etat.generationTimeline) return;
      const temps = source.duration > 0 ? ((index + 0.5) / nombre) * source.duration : 0;
      await positionner(source, Math.min(temps, Math.max(0, source.duration - 0.03)));
      contexte.fillStyle = '#050403';
      contexte.fillRect(0, 0, canvas.width, canvas.height);
      contexte.drawImage(source, 0, 0, canvas.width, canvas.height);
      const image = document.createElement('img');
      image.alt = '';
      image.src = canvas.toDataURL('image/jpeg', 0.62);
      conteneur.appendChild(image);
    }
  } finally {
    source.removeAttribute('src');
    source.load();
    URL.revokeObjectURL(url);
  }
}

function conteneurLargeurTimeline() {
  return Math.max(660, $('timeline').clientWidth || 660);
}

function ratioTimeline(clientX) {
  const rectangle = $('timeline').getBoundingClientRect();
  return Math.max(0, Math.min(1, (clientX - rectangle.left) / Math.max(1, rectangle.width)));
}

function commencerGlissementTimeline(type, evenement) {
  evenement.preventDefault();
  evenement.stopPropagation();
  const element = evenement.currentTarget;
  if (typeof element.setPointerCapture === 'function') {
    try { element.setPointerCapture(evenement.pointerId); } catch {}
  }
  let dernierTemps = type === 'debut' ? etat.debut : etat.fin;
  const mouvement = (event) => {
    const temps = ratioTimeline(event.clientX) * etat.duree;
    if (type === 'debut') etat.debut = Math.min(temps, etat.fin - Math.min(0.05, etat.duree));
    else etat.fin = Math.max(temps, etat.debut + Math.min(0.05, etat.duree));
    dernierTemps = type === 'debut' ? etat.debut : etat.fin;
    actualiserEdition();
    afficherInstantCoupe(dernierTemps);
  };
  const fin = () => {
    window.removeEventListener('pointermove', mouvement);
    window.removeEventListener('pointerup', fin);
    window.removeEventListener('pointercancel', fin);
    afficherInstantCoupe(dernierTemps);
  };
  window.addEventListener('pointermove', mouvement);
  window.addEventListener('pointerup', fin, { once: true });
  window.addEventListener('pointercancel', fin, { once: true });
  mouvement(evenement);
}

/* -------------------------------------------------------------- recadrage */

function commencerRecadrage(evenement) {
  if (!etat.selection || etat.export || !etat.recadrageActif) return;
  evenement.preventDefault();
  evenement.stopPropagation();
  const poignee = evenement.target.dataset.poignee || '';
  const mode = poignee || 'move';
  const origine = { ...etat.recadrage };
  const departX = evenement.clientX;
  const departY = evenement.clientY;
  const calque = $('calqueRecadrage');
  const dimensions = dimensionsSource();
  const minimumLargeur = Math.min(1, 2 / dimensions.largeur);
  const minimumHauteur = Math.min(1, 2 / dimensions.hauteur);

  const mouvement = (event) => {
    const dx = (event.clientX - departX) / Math.max(1, calque.clientWidth);
    const dy = (event.clientY - departY) / Math.max(1, calque.clientHeight);
    let { x, y, w, h } = origine;
    if (mode === 'move') {
      x = Math.max(0, Math.min(1 - w, origine.x + dx));
      y = Math.max(0, Math.min(1 - h, origine.y + dy));
    } else {
      if (mode.includes('e')) w = Math.max(minimumLargeur, Math.min(1 - x, origine.w + dx));
      if (mode.includes('s')) h = Math.max(minimumHauteur, Math.min(1 - y, origine.h + dy));
      if (mode.includes('w')) {
        const droite = origine.x + origine.w;
        x = Math.max(0, Math.min(droite - minimumLargeur, origine.x + dx));
        w = droite - x;
      }
      if (mode.includes('n')) {
        const bas = origine.y + origine.h;
        y = Math.max(0, Math.min(bas - minimumHauteur, origine.y + dy));
        h = bas - y;
      }
    }
    etat.recadrage = { x, y, w, h };
    actualiserEdition();
  };
  const fin = () => {
    window.removeEventListener('pointermove', mouvement);
    window.removeEventListener('mousemove', mouvement);
    window.removeEventListener('pointerup', fin);
    window.removeEventListener('mouseup', fin);
    window.removeEventListener('pointercancel', fin);
  };
  window.addEventListener('pointermove', mouvement);
  window.addEventListener('mousemove', mouvement);
  window.addEventListener('pointerup', fin, { once: true });
  window.addEventListener('mouseup', fin, { once: true });
  window.addEventListener('pointercancel', fin, { once: true });
}

function appliquerChampRecadrage(type) {
  const ids = {
    x: 'recadrageX',
    y: 'recadrageY',
    w: 'recadrageLargeur',
    h: 'recadrageHauteur',
  };
  const valeur = Number.parseFloat($(ids[type]).value);
  if (!Number.isFinite(valeur)) return;
  const dimensions = dimensionsSource();
  const diviseur = type === 'x' || type === 'w' ? dimensions.largeur : dimensions.hauteur;
  const crop = { ...etat.recadrage };
  const normalisee = Math.max(0, valeur / diviseur);
  if (type === 'x') crop.x = Math.min(1 - crop.w, normalisee);
  if (type === 'y') crop.y = Math.min(1 - crop.h, normalisee);
  if (type === 'w') crop.w = Math.min(1 - crop.x, normalisee);
  if (type === 'h') crop.h = Math.min(1 - crop.y, normalisee);
  etat.recadrage = bornerRecadrage(crop);
  actualiserEdition();
}

function deplacerRecadrageClavier(evenement) {
  if (!etat.recadrageActif) return;
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(evenement.key)) return;
  evenement.preventDefault();
  const pas = evenement.shiftKey ? 0.01 : 0.0025;
  const crop = { ...etat.recadrage };
  if (evenement.key === 'ArrowLeft') crop.x = Math.max(0, crop.x - pas);
  if (evenement.key === 'ArrowRight') crop.x = Math.min(1 - crop.w, crop.x + pas);
  if (evenement.key === 'ArrowUp') crop.y = Math.max(0, crop.y - pas);
  if (evenement.key === 'ArrowDown') crop.y = Math.min(1 - crop.h, crop.y + pas);
  etat.recadrage = crop;
  actualiserEdition();
}

/* --------------------------------------------------------------- renommage */

async function nomExiste(dossier, nom) {
  try {
    await dossier.getFileHandle(nom, { create: false });
    return true;
  } catch (erreur) {
    if (erreur && erreur.name === 'NotFoundError') return false;
    throw erreur;
  }
}

async function nomDisponible(dossier, souhaite) {
  if (!(await nomExiste(dossier, souhaite))) return souhaite;
  const extension = extensionDe(souhaite);
  const base = baseDe(souhaite);
  for (let index = 2; index < 10000; index += 1) {
    const candidat = `${base}-${index}.${extension}`;
    if (!(await nomExiste(dossier, candidat))) return candidat;
  }
  throw new Error(t('library_name_exhausted'));
}

async function renommerSelection() {
  if (!etat.selection || etat.export) return;
  const source = etat.selection;
  const extension = extensionDe(source.nom);
  const brut = nettoyerNom($('nouveauNom').value);
  const base = baseDe(brut);
  const cible = `${base}.${extension}`;
  if (!base) return notifier(t('rename_invalid'), true);
  if (cible === source.nom) return notifier(t('rename_unchanged'));
  if (await nomExiste(source.dossier, cible)) return notifier(t('rename_exists'), true);

  $('renommer').disabled = true;
  notifier(t('rename_working'));
  try {
    const fichier = await source.handle.getFile();
    const nouveauHandle = await source.dossier.getFileHandle(cible, { create: true });
    const destination = await nouveauHandle.createWritable();
    await fichier.stream().pipeTo(destination);
    await source.dossier.removeEntry(source.nom);
    const nouvelleCle = source.chemin ? `${source.chemin}/${cible}` : cible;
    etat.selection = null;
    notifier(t('rename_done', [cible]));
    await chargerBibliotheque(nouvelleCle);
  } catch (erreur) {
    notifier(erreurLisible(erreur), true);
  } finally {
    $('renommer').disabled = false;
  }
}

/* ----------------------------------------------------------- export video */

function creerEnregistreur(stream, largeur, hauteur) {
  const candidats = [
    ['video/mp4;codecs=avc1.42E01E,mp4a.40.2', 'mp4'],
    ['video/mp4;codecs=avc1.42E01E', 'mp4'],
    ['video/mp4', 'mp4'],
    ['video/webm;codecs=vp9,opus', 'webm'],
    ['video/webm;codecs=vp8,opus', 'webm'],
    ['video/webm', 'webm'],
  ];
  const debit = Math.max(2_000_000, Math.min(16_000_000, largeur * hauteur * 4));
  for (const [mimeType, extension] of candidats) {
    if (!MediaRecorder.isTypeSupported(mimeType)) continue;
    try {
      return {
        recorder: new MediaRecorder(stream, {
          mimeType,
          videoBitsPerSecond: debit,
          audioBitsPerSecond: 160_000,
        }),
        extension,
      };
    } catch {}
  }
  throw new Error(t('export_no_encoder'));
}

function afficherProgression(valeur, message) {
  const pourcentage = Math.max(0, Math.min(1, valeur));
  $('blocProgression').hidden = false;
  $('progression').style.width = `${pourcentage * 100}%`;
  $('etatExport').textContent = message || t('export_progress', [Math.round(pourcentage * 100)]);
}

async function exporterSelection() {
  if (!etat.selection || etat.export) return;
  if (etat.fin - etat.debut < 0.05) return notifier(t('export_selection_too_short'), true);

  const sourceEntree = etat.selection;
  const sourceFichier = await sourceEntree.handle.getFile();
  const sourceUrl = URL.createObjectURL(sourceFichier);
  const processeur = document.createElement('video');
  processeur.preload = 'auto';
  processeur.playsInline = true;
  processeur.src = sourceUrl;
  processeur.style.display = 'none';
  document.body.appendChild(processeur);

  let audioContexte = null;
  let writable = null;
  let nomSortie = '';
  let recorder = null;
  let ecritures = Promise.resolve();
  let sortieCreee = false;
  const travail = { annule: false, arreter: null };
  etat.export = travail;
  $('exporter').disabled = true;
  $('annulerExport').hidden = false;
  afficherProgression(0, t('export_preparing'));

  try {
    await attendre(processeur, 'loadedmetadata');
    if (travail.annule) throw new DOMException('Cancelled', 'AbortError');
    const crop = etat.recadrage;
    const sx = Math.round(processeur.videoWidth * crop.x);
    const sy = Math.round(processeur.videoHeight * crop.y);
    const sw = Math.max(2, Math.round(processeur.videoWidth * crop.w));
    const sh = Math.max(2, Math.round(processeur.videoHeight * crop.h));
    const largeur = Math.max(2, Math.floor(sw / 2) * 2);
    const hauteur = Math.max(2, Math.floor(sh / 2) * 2);
    const canvas = document.createElement('canvas');
    canvas.width = largeur;
    canvas.height = hauteur;
    const contexte = canvas.getContext('2d', { alpha: false });
    const flux = canvas.captureStream(30);

    audioContexte = new AudioContext();
    const sourceAudio = audioContexte.createMediaElementSource(processeur);
    const destinationAudio = audioContexte.createMediaStreamDestination();
    sourceAudio.connect(destinationAudio);
    for (const piste of destinationAudio.stream.getAudioTracks()) flux.addTrack(piste);
    await audioContexte.resume();
    if (travail.annule) throw new DOMException('Cancelled', 'AbortError');

    const cree = creerEnregistreur(flux, largeur, hauteur);
    recorder = cree.recorder;
    const demande = nettoyerNom($('nomExport').value) || `${baseDe(sourceEntree.nom)}-edited.${cree.extension}`;
    const avecExtension = `${baseDe(demande)}.${cree.extension}`;
    nomSortie = await nomDisponible(sourceEntree.dossier, avecExtension);
    const handleSortie = await sourceEntree.dossier.getFileHandle(nomSortie, { create: true });
    sortieCreee = true;
    writable = await handleSortie.createWritable();

    let morceaux = 0;
    recorder.addEventListener('dataavailable', (evenement) => {
      if (!evenement.data || !evenement.data.size) return;
      morceaux += 1;
      ecritures = ecritures.then(() => writable.write(evenement.data));
    });

    let resoudreArret;
    let rejeterArret;
    const arret = new Promise((resolve, reject) => {
      resoudreArret = resolve;
      rejeterArret = reject;
    });
    recorder.addEventListener('error', (evenement) => {
      rejeterArret(new Error(evenement.error?.message || t('export_recording_error')));
    }, { once: true });
    recorder.addEventListener('stop', async () => {
      try {
        await ecritures;
        await writable.close();
        writable = null;
        if (!morceaux) throw new Error(t('export_empty'));
        resoudreArret();
      } catch (erreur) {
        rejeterArret(erreur);
      }
    }, { once: true });

    let arretDemande = false;
    const demanderArret = () => {
      if (arretDemande) return;
      arretDemande = true;
      processeur.pause();
      if (recorder.state !== 'inactive') recorder.stop();
    };
    travail.arreter = demanderArret;

    await positionner(processeur, etat.debut);
    if (travail.annule) throw new DOMException('Cancelled', 'AbortError');
    contexte.drawImage(processeur, sx, sy, sw, sh, 0, 0, largeur, hauteur);
    recorder.start(1000);

    const dessiner = () => {
      if (travail.annule || processeur.currentTime >= etat.fin || processeur.ended) {
        demanderArret();
        return;
      }
      contexte.drawImage(processeur, sx, sy, sw, sh, 0, 0, largeur, hauteur);
      const avancement = (processeur.currentTime - etat.debut) / Math.max(0.001, etat.fin - etat.debut);
      afficherProgression(avancement, t('export_progress', [Math.round(Math.max(0, Math.min(1, avancement)) * 100)]));
      if (typeof processeur.requestVideoFrameCallback === 'function') {
        processeur.requestVideoFrameCallback(dessiner);
      } else {
        requestAnimationFrame(dessiner);
      }
    };

    processeur.addEventListener('ended', demanderArret, { once: true });
    await processeur.play();
    dessiner();
    await arret;

    if (travail.annule) {
      await sourceEntree.dossier.removeEntry(nomSortie).catch(() => {});
      notifier(t('export_cancelled'));
    } else {
      afficherProgression(1, t('export_done', [nomSortie]));
      notifier(t('export_done', [nomSortie]));
      await chargerBibliotheque(sourceEntree.cle);
    }
  } catch (erreur) {
    if (recorder && recorder.state !== 'inactive') recorder.stop();
    if (writable) await writable.abort().catch(() => {});
    if (sortieCreee && nomSortie) await sourceEntree.dossier.removeEntry(nomSortie).catch(() => {});
    if (travail.annule) notifier(t('export_cancelled'));
    else notifier(erreurLisible(erreur), true);
  } finally {
    processeur.pause();
    processeur.removeAttribute('src');
    processeur.load();
    processeur.remove();
    URL.revokeObjectURL(sourceUrl);
    if (audioContexte) await audioContexte.close().catch(() => {});
    etat.export = null;
    $('exporter').disabled = false;
    $('annulerExport').hidden = true;
    setTimeout(() => {
      if (!etat.export) $('blocProgression').hidden = true;
    }, 5000);
  }
}

function annulerExport() {
  if (!etat.export) return;
  etat.export.annule = true;
  if (etat.export.arreter) etat.export.arreter();
  $('etatExport').textContent = t('export_cancelling');
}

/* --------------------------------------------------------------- evenements */

$('choisirDossier').addEventListener('click', choisirDossier);
$('actualiser').addEventListener('click', () => chargerBibliotheque(etat.selection?.cle || ''));
$('recherche').addEventListener('input', afficherVideos);
$('tri').addEventListener('change', afficherVideos);
$('langue').addEventListener('click', async () => {
  langue = langue === 'en' ? 'fr' : 'en';
  await ecrireLangue(langue);
  await chargerMessages();
  traduirePage();
});

$('video').addEventListener('timeupdate', () => {
  if ($('video').currentTime >= etat.fin && !$('video').paused) {
    $('video').pause();
    $('video').currentTime = etat.debut;
  }
  actualiserEdition();
});
$('video').addEventListener('loadedmetadata', actualiserEdition);
$('video').addEventListener('play', () => {
  if ($('video').currentTime < etat.debut || $('video').currentTime >= etat.fin) {
    $('video').currentTime = etat.debut;
  }
});

$('timeline').addEventListener('pointerdown', (evenement) => {
  if (evenement.target.closest('.poignee-timeline')) return;
  $('video').currentTime = ratioTimeline(evenement.clientX) * etat.duree;
  actualiserEdition();
});
$('poigneeDebut').addEventListener('pointerdown', (evenement) => commencerGlissementTimeline('debut', evenement));
$('poigneeFin').addEventListener('pointerdown', (evenement) => commencerGlissementTimeline('fin', evenement));
$('tempsDebut').addEventListener('input', () => {
  const valeur = Number.parseFloat($('tempsDebut').value);
  if (!Number.isFinite(valeur)) return;
  etat.debut = valeur;
  actualiserEdition();
  afficherInstantCoupe(etat.debut);
});
$('tempsFin').addEventListener('input', () => {
  const valeur = Number.parseFloat($('tempsFin').value);
  if (!Number.isFinite(valeur)) return;
  etat.fin = valeur;
  actualiserEdition();
  afficherInstantCoupe(etat.fin);
});
$('marquerDebut').addEventListener('click', () => {
  etat.debut = Math.min($('video').currentTime, etat.fin);
  actualiserEdition();
});
$('marquerFin').addEventListener('click', () => {
  etat.fin = Math.max($('video').currentTime, etat.debut);
  actualiserEdition();
});

$('cadreRecadrage').addEventListener('pointerdown', commencerRecadrage);
$('cadreRecadrage').addEventListener('keydown', deplacerRecadrageClavier);
$('basculerRecadrage').addEventListener('click', () => {
  etat.recadrageActif = !etat.recadrageActif;
  actualiserEdition();
});
$('reinitialiserRecadrage').addEventListener('click', () => {
  etat.recadrage = { x: 0, y: 0, w: 1, h: 1 };
  actualiserEdition();
});
for (const [id, type] of [
  ['recadrageX', 'x'],
  ['recadrageY', 'y'],
  ['recadrageLargeur', 'w'],
  ['recadrageHauteur', 'h'],
]) {
  $(id).addEventListener('input', () => appliquerChampRecadrage(type));
}

$('renommer').addEventListener('click', renommerSelection);
$('nouveauNom').addEventListener('keydown', (evenement) => {
  if (evenement.key === 'Enter') renommerSelection();
});
$('exporter').addEventListener('click', exporterSelection);
$('annulerExport').addEventListener('click', annulerExport);

window.addEventListener('beforeunload', (evenement) => {
  if (etat.export) {
    evenement.preventDefault();
    evenement.returnValue = '';
  }
});
window.addEventListener('pagehide', () => {
  if (etat.urlSelection) URL.revokeObjectURL(etat.urlSelection);
});

/* ------------------------------------------------------------ initialisation */

async function initialiser() {
  langue = await lireLangue();
  await chargerMessages();
  traduirePage();
  try {
    etat.dossier = await lireHandleDossier();
    if (etat.dossier) {
      etat.permissionDossier = await permissionDossier(etat.dossier, false);
      actualiserEtatDossier();
      if (etat.permissionDossier === 'granted') await chargerBibliotheque();
    }
  } catch (erreur) {
    notifier(erreurLisible(erreur), true);
  }
}

initialiser().catch((erreur) => notifier(erreurLisible(erreur), true));
