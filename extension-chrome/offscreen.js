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

async function demarrer(message) {
  if (recorder && recorder.state !== 'inactive') {
    throw new Error('Un enregistrement est deja actif');
  }
  const format = formatEnregistrement();
  try {
    if (message.source === 'tab' && message.streamId) {
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
  if (pisteVideo) {
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
