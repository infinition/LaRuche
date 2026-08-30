/** Hidden media recorder. All privileged extension work stays in background.js. */

let media = null;
let recorder = null;
let morceaux = [];
let audioSortie = null;
let typeMime = null;
let extensionFichier = null;
const urls = new Set();

function formatEnregistrement(avecAudio) {
  const candidats = avecAudio
    ? [
        ['video/mp4;codecs=avc1.42E01E,mp4a.40.2', 'mp4'],
        ['video/webm;codecs=vp9,opus', 'webm'],
        ['video/webm;codecs=vp8,opus', 'webm'],
        ['video/webm', 'webm'],
      ]
    : [
        ['video/mp4;codecs=avc1.42E01E', 'mp4'],
        ['video/mp4', 'mp4'],
        ['video/webm;codecs=vp9', 'webm'],
        ['video/webm;codecs=vp8', 'webm'],
        ['video/webm', 'webm'],
      ];
  for (const [mimeType, extension] of candidats) {
    if (MediaRecorder.isTypeSupported(mimeType)) return { mimeType, extension };
  }
  throw new Error('Chrome ne propose aucun encodeur video compatible');
}

function contraintes(streamId, source, audio) {
  const chromeMediaSource = source === 'tab' ? 'tab' : 'desktop';
  const mandatory = {
    chromeMediaSource,
    chromeMediaSourceId: streamId,
    maxWidth: 3840,
    maxHeight: 2160,
    maxFrameRate: 30,
  };
  return {
    video: { mandatory },
    audio: audio
      ? { mandatory: { chromeMediaSource, chromeMediaSourceId: streamId } }
      : false,
  };
}

/* ---------------------------------------------------- capture par screencast */

/// Enregistrer un onglet SANS geste de l'utilisateur.
///
/// `chrome.tabCapture` exige que l'utilisateur ait invoque l'extension sur
/// l'onglet vise. Pour un agent qui travaille dans un onglet qu'il vient de
/// creer, ce geste n'existe pas et ne peut pas exister: personne n'etait la.
///
/// Le protocole de debogage, lui, est deja attache a cet onglet et sait le
/// filmer: `Page.startScreencast` envoie des images JPEG. On les dessine dans un
/// canevas et on enregistre le flux du canevas. Deux consequences, et la seconde
/// est un bonus:
///
///   - aucun geste requis, donc l'enregistrement demarre vraiment tout seul;
///   - le canevas est la source, pas l'onglet. Quand l'agent change d'onglet, on
///     rebranche le screencast sur le nouveau et les images continuent d'arriver
///     dans le MEME canevas: la video est continue, ce que la capture d'onglet
///     ne pouvait pas faire.
///
/// Le prix est honnete: du JPEG a la place d'un flux video, donc un peu moins
/// fin, et pas de son. Pour montrer un agent qui pilote, c'est le bon compromis.
let toile = null;
let contexte = null;
let dernierAck = null;
let frameEnCours = false;

// La toile a la taille des images recues, donnee par la qualite choisie:
// agrandir n'ajoute aucun detail et fait encoder la video sur plus de pixels
// pour rien. Valeurs de repli si le message n'en porte pas.
let TOILE_L = 1280;
let TOILE_H = 720;

function preparerToile(taille) {
  if (taille && taille.l > 0 && taille.h > 0) {
    TOILE_L = taille.l;
    TOILE_H = taille.h;
  }
  // Un <canvas> du DOM, pas un OffscreenCanvas: `captureStream` n'existe que
  // sur le premier. Le document offscreen EST un document, il peut donc en
  // porter un; il n'est simplement jamais affiche.
  toile = document.createElement('canvas');
  toile.width = TOILE_L;
  toile.height = TOILE_H;
  document.body.appendChild(toile);
  contexte = toile.getContext('2d', { alpha: false });
  contexte.fillStyle = '#101014';
  contexte.fillRect(0, 0, TOILE_L, TOILE_H);
  return toile;
}

/// Dessine une image en la contenant dans le cadre, sans la deformer.
///
/// Les onglets n'ont pas tous la meme taille et l'utilisateur redimensionne sa
/// fenetre pendant l'enregistrement. Un canevas de taille fixe evite de
/// redemarrer l'enregistreur a chaque changement, ce qu'on ne peut pas faire
/// sans couper le fichier en morceaux.
async function dessinerFrame(base64) {
  if (!contexte) return;
  const bin = atob(base64);
  const octets = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) octets[i] = bin.charCodeAt(i);
  const image = await createImageBitmap(new Blob([octets], { type: 'image/jpeg' }));
  const ratio = Math.min(TOILE_L / image.width, TOILE_H / image.height);
  const l = image.width * ratio;
  const h = image.height * ratio;
  contexte.fillStyle = '#101014';
  contexte.fillRect(0, 0, TOILE_L, TOILE_H);
  contexte.drawImage(image, (TOILE_L - l) / 2, (TOILE_H - h) / 2, l, h);
  image.close();
}

async function demarrer(message) {
  if (recorder && recorder.state !== 'inactive') {
    throw new Error('Un enregistrement est deja actif');
  }
  const format = formatEnregistrement();
  try {
    if (message.source === 'screencast') {
      media = preparerToile(message.taille).captureStream(30);
    } else if (message.source === 'tab' && message.streamId) {
      media = await navigator.mediaDevices.getUserMedia(
        contraintes(message.streamId, message.source, message.audio),
      );
    } else {
      const displayConstraints = {
        video: {
          width: { ideal: 3840 },
          height: { ideal: 2160 },
          frameRate: { ideal: 30 },
        },
        audio: !!message.audio,
        selfBrowserSurface: 'exclude',
        systemAudio: message.audio ? 'include' : 'exclude',
      };
      media = await navigator.mediaDevices.getDisplayMedia(displayConstraints);
    }
    typeMime = format.mimeType;
    extensionFichier = format.extension;
    morceaux = [];

    if (message.source === 'tab' && media.getAudioTracks().length) {
      audioSortie = new AudioContext();
      audioSortie.createMediaStreamSource(media).connect(audioSortie.destination);
    }

    recorder = new MediaRecorder(media, {
      mimeType: typeMime,
      videoBitsPerSecond: 10_000_000,
      audioBitsPerSecond: 192_000,
    });
  } catch (e) {
    if (media) media.getTracks().forEach((piste) => piste.stop());
    media = null;
    if (audioSortie) await audioSortie.close().catch(() => {});
    audioSortie = null;
    recorder = null;
    throw e;
  }
  recorder.addEventListener('dataavailable', (event) => {
    if (event.data && event.data.size) morceaux.push(event.data);
  });
  recorder.addEventListener('error', (event) => {
    chrome.runtime.sendMessage({
      target: 'background',
      type: 'enregistrement-erreur',
      error: String((event.error && event.error.message) || event.error || 'MediaRecorder error'),
    }).catch(() => {});
  });
  recorder.addEventListener('stop', finaliser, { once: true });
  const pisteVideo = media.getVideoTracks()[0];
  // Une piste de canevas ne se termine jamais seule; seule une piste de partage
  // s'arrete quand l'utilisateur clique "Arreter le partage".
  if (pisteVideo && message.source !== 'screencast') {
    pisteVideo.addEventListener('ended', () => {
      if (recorder && recorder.state !== 'inactive') recorder.stop();
    }, { once: true });
  }
  if (message.differe) {
    // Source acquise, enregistreur pret, mais rien ne tourne encore.
    //
    // C'est ce que demande un showcase: le selecteur d'ecran de Chrome exige un
    // geste de l'utilisateur, donc il ne peut apparaitre qu'au moment ou il
    // clique. Mais demarrer l'enregistrement a cet instant filme tout le temps
    // mort avant que l'agent ne commence. On separe donc les deux: on demande
    // l'ecran maintenant, on enregistre quand l'agent prend la main.
    return { ok: true, mimeType: typeMime, extension: extensionFichier, arme: true };
  }
  recorder.start(1000);
  return { ok: true, mimeType: typeMime, extension: extensionFichier };
}

/// Lance un enregistreur deja arme par `demarrer({ differe: true })`.
function lancer() {
  if (!recorder) throw new Error("Aucune source armee: appeler d'abord la preparation");
  if (recorder.state !== 'inactive') return { ok: true, deja: true };
  recorder.start(1000);
  return { ok: true, mimeType: typeMime, extension: extensionFichier };
}

/// Abandonne une source armee sans produire de fichier.
function desarmer() {
  if (recorder && recorder.state !== 'inactive') return { ok: false, actif: true };
  if (media) media.getTracks().forEach((piste) => piste.stop());
  media = null;
  recorder = null;
  morceaux = [];
  if (audioSortie) audioSortie.close().catch(() => {});
  audioSortie = null;
  return { ok: true };
}

async function finaliser() {
  const blob = new Blob(morceaux, { type: typeMime });
  morceaux = [];
  if (media) media.getTracks().forEach((piste) => piste.stop());
  media = null;
  if (audioSortie) await audioSortie.close().catch(() => {});
  audioSortie = null;
  recorder = null;
  if (toile && toile.parentNode) toile.parentNode.removeChild(toile);
  toile = null;
  contexte = null;

  if (!blob.size) {
    await chrome.runtime.sendMessage({
      target: 'background',
      type: 'enregistrement-erreur',
      error: 'Le fichier video est vide',
    }).catch(() => {});
    return;
  }
  const url = URL.createObjectURL(blob);
  urls.add(url);
  await chrome.runtime.sendMessage({
    target: 'background',
    type: 'enregistrement-pret',
    url,
    mimeType: typeMime,
    extension: extensionFichier,
    taille: blob.size,
  }).catch(async (e) => {
    URL.revokeObjectURL(url);
    urls.delete(url);
    await chrome.runtime.sendMessage({
      target: 'background',
      type: 'enregistrement-erreur',
      error: String((e && e.message) || e),
    }).catch(() => {});
  });
}

function arreter() {
  if (!recorder || recorder.state === 'inactive') return { ok: true, inactive: true };
  recorder.stop();
  return { ok: true };
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.target !== 'offscreen') return false;
  if (message.type === 'enregistrement-demarrer') {
    demarrer(message).then(sendResponse).catch((e) => {
      sendResponse({ ok: false, error: String((e && e.message) || e) });
    });
    return true;
  }
  if (message.type === 'enregistrement-lancer') {
    try {
      sendResponse(lancer());
    } catch (e) {
      sendResponse({ ok: false, error: String((e && e.message) || e) });
    }
    return false;
  }
  if (message.type === 'enregistrement-desarmer') {
    sendResponse(desarmer());
    return false;
  }
  if (message.type === 'screencast-frame') {
    // Si le decodage precedent n'est pas fini, on saute celle-ci. Empiler des
    // decodages sur un canevas qui n'affichera de toute facon que le dernier
    // etat ne rend pas la video meilleure, ca ne fait qu'allonger la file.
    if (!frameEnCours) {
      frameEnCours = true;
      dessinerFrame(message.data)
        .catch(() => {})
        .finally(() => { frameEnCours = false; });
    }
    sendResponse({ ok: true });
    return false;
  }
  if (message.type === 'enregistrement-arreter') {
    sendResponse(arreter());
    return false;
  }
  if (message.type === 'enregistrement-liberer-url') {
    if (urls.has(message.url)) {
      URL.revokeObjectURL(message.url);
      urls.delete(message.url);
    }
    sendResponse({ ok: true });
    return false;
  }
  return false;
});
