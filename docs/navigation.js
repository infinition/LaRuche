/* La navigation douce entre l'accueil et le wiki.
 *
 * Les deux pages restent deux fichiers et deux vraies URL: le wiki est genere
 * par scripts/build_wiki.py, une action GitHub le regenere a chaque commit dans
 * wiki/, et le tableau de bord pointe sur wiki.html. Rien de tout cela ne doit
 * bouger. Ce qui change, c'est le trajet entre les deux: au lieu de recharger un
 * document entier, on va chercher l'autre page, on echange le bloc #vue et sa
 * feuille de style, et on pousse l'URL dans l'historique.
 *
 * Ce qui reste en place d'une page a l'autre, parce que ces morceaux vivent
 * dans la coquille et non dans #vue: l'abeille et son canal de rencontres, la
 * fleche de retour en haut, et le defilement de la fenetre.
 *
 * Le DOM d'une vue quittee n'est pas jete, il est garde de cote. Y revenir ne
 * coute donc ni requete ni reconstruction: le wiki retrouve sa page ouverte,
 * ses surlignages et sa position exacte. C'est aussi pour cela que les scripts
 * de chaque page ne sont executes qu'une fois, a leur premiere venue: leurs
 * variables pointent sur des elements qui restent vivants, simplement detaches.
 *
 * En echange, une vue detachee continue d'entendre les evenements poses sur
 * `window` et `document`. Les deux pages verifient donc `isConnected` avant
 * d'agir: sans cela le routeur du wiki tenterait de lire #engine comme une de
 * ses pages, et le carrousel de l'accueil tournerait dans le vide.
 *
 * Si quoi que ce soit manque a l'appel, l'ancre reprend son travail normal: un
 * lien reste un lien, et la page se recharge comme avant.
 */
(function () {
  'use strict';

  if (!window.fetch || !window.DOMParser || !window.history || !history.pushState) return;

  var LENT = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var FONDU = 150;                 // ms, la duree du fondu de sortie
  var PAGES = { 'index.html': 1, 'wiki.html': 1 };

  var vues = {};                   // cle -> {vue, feuille, titre, desc, hash, y, prete}
  var courant = null;
  var enCours = false;

  var STYLE =
    '#vue{transition:opacity ' + (FONDU / 1000) + 's ease}' +
    '#vue.lr-sort{opacity:0}' +
    '#vue:focus{outline:none}' +
    '@media (prefers-reduced-motion:reduce){#vue{transition:none}}';

  (function () {
    var f = document.createElement('style');
    f.textContent = STYLE;
    document.head.appendChild(f);
  })();

  /* ── Les URL que ce routeur prend en charge ─────────────────────────────
     Le nom de fichier sert de cle, mais seulement dans le dossier courant:
     un /autre/wiki.html n'est pas notre wiki. */
  function dossier(chemin) {
    return chemin.slice(0, chemin.lastIndexOf('/') + 1);
  }

  function cle(u) {
    if (u.origin !== location.origin) return null;
    if (dossier(u.pathname) !== dossier(location.pathname)) return null;
    var nom = u.pathname.slice(u.pathname.lastIndexOf('/') + 1) || 'index.html';
    return PAGES[nom] ? nom : null;
  }

  function cleCourante() {
    var nom = location.pathname.slice(location.pathname.lastIndexOf('/') + 1) || 'index.html';
    return PAGES[nom] ? nom : null;
  }

  /* ── La vue affichee au chargement ──────────────────────────────────── */
  function amorcer() {
    var vue = document.getElementById('vue');
    var feuille = document.getElementById('feuille');
    if (!vue || !feuille) return false;
    courant = cleCourante();
    if (!courant) return false;
    vues[courant] = {
      vue: vue, feuille: feuille, titre: document.title,
      desc: meta(document), hash: location.hash, y: 0, prete: true
    };
    vue.setAttribute('tabindex', '-1');
    return true;
  }

  function meta(d) {
    var m = d.querySelector('meta[name="description"]');
    return m ? m.getAttribute('content') : null;
  }

  /* ── Aller chercher une vue ─────────────────────────────────────────── */
  var textes = {};                 // cle -> promesse du HTML brut

  function telecharger(c, url) {
    if (!textes[c]) {
      textes[c] = fetch(url, { credentials: 'same-origin' }).then(function (r) {
        if (!r.ok) throw new Error(r.status);
        return r.text();
      });
      textes[c].catch(function () { delete textes[c]; });
    }
    return textes[c];
  }

  /* Le document recu est decoupe en trois: la feuille de style de la page, son
     bloc #vue, et ses scripts. Les scripts sont retires du bloc et rendus a
     part: un script arrive par DOMParser ne s'execute jamais, il faut le
     recreer pour qu'il tourne. */
  function construire(c, url) {
    if (vues[c]) return Promise.resolve(vues[c]);
    return telecharger(c, url).then(function (txt) {
      var d = new DOMParser().parseFromString(txt, 'text/html');
      var vue = d.getElementById('vue');
      var feuille = d.getElementById('feuille');
      if (!vue || !feuille) throw new Error('page sans #vue');

      var scripts = [];
      Array.prototype.forEach.call(vue.querySelectorAll('script'), function (s) {
        scripts.push({ texte: s.textContent, src: s.getAttribute('src') });
        s.parentNode.removeChild(s);
      });

      vues[c] = {
        vue: document.importNode(vue, true),
        feuille: document.importNode(feuille, true),
        titre: d.title,
        desc: meta(d),
        scripts: scripts,
        hash: null,
        y: 0,
        prete: false
      };
      vues[c].vue.setAttribute('tabindex', '-1');
      return vues[c];
    });
  }

  /* ── Le fondu ───────────────────────────────────────────────────────── */
  function sortir(v) {
    if (LENT) return Promise.resolve();
    return new Promise(function (fini) {
      v.classList.add('lr-sort');
      var fait = false;
      function fin() { if (!fait) { fait = true; fini(); } }
      v.addEventListener('transitionend', fin, { once: true });
      setTimeout(fin, FONDU + 60);          // au cas ou la transition ne part pas
    });
  }

  /* ── Le passage d'une vue a l'autre ─────────────────────────────────── */
  function afficher(c, url, restaurer) {
    var ancienne = vues[courant];
    var suivante = vues[c];

    var ancienneVue = document.getElementById('vue');
    ancienneVue.parentNode.replaceChild(suivante.vue, ancienneVue);
    ancienne.feuille.parentNode.replaceChild(suivante.feuille, ancienne.feuille);

    document.title = suivante.titre || document.title;
    var m = document.querySelector('meta[name="description"]');
    if (m && suivante.desc) m.setAttribute('content', suivante.desc);

    courant = c;

    /* Le wiki se place lui-meme: ses fragments sont ses routes, et son routeur
       decide de la page ouverte comme de l'endroit ou l'ouvrir. Il l'annonce
       par un attribut plutot que d'etre reconnu au nom de son fichier. */
    var fragmentaire = suivante.vue.getAttribute('data-routeur') === 'fragment';
    var dejaPlace = false;

    // Premiere venue: ses scripts n'ont jamais tourne. L'URL est deja poussee,
    // donc son routeur lit la bonne adresse du premier coup.
    if (!suivante.prete) {
      suivante.prete = true;
      suivante.scripts.forEach(function (s) {
        var n = document.createElement('script');
        if (s.src) n.src = s.src; else n.textContent = s.texte;
        suivante.vue.appendChild(n);
      });
      dejaPlace = fragmentaire;
    } else if (fragmentaire && suivante.hash !== location.hash) {
      // Deja construite, mais on ne lui demande pas la page qu'elle affichait
      // en partant: son routeur doit reprendre la main.
      window.dispatchEvent(new Event('hashchange'));
      dejaPlace = true;
    }
    suivante.hash = location.hash;

    placer(suivante, restaurer, dejaPlace);

    // L'abeille de la nouvelle vue prend le relais, en repartant d'ou l'ancienne
    // en etait. Celle qu'on vient de detacher se met en pause toute seule.
    if (window.LaRucheAbeille) window.LaRucheAbeille.demarrer();

    if (!LENT) {
      suivante.vue.classList.add('lr-sort');
      void suivante.vue.offsetWidth;               // on force le calcul, sinon rien ne s'anime
      suivante.vue.classList.remove('lr-sort');
    }
    suivante.vue.focus({ preventScroll: true });
    window.dispatchEvent(new CustomEvent('laruche:vue', { detail: { page: c } }));
  }

  /* Ou se poser dans la nouvelle vue: sur l'ancre demandee, a l'endroit qu'on
     avait quitte, ou en haut. Le defilement est explicitement brutal: les deux
     pages demandent un `scroll-behavior:smooth` qui, ici, ferait descendre
     l'ecran lentement a travers une page qui vient d'apparaitre. */
  function placer(v, restaurer, dejaPlace) {
    if (dejaPlace) return;                 // son propre routeur s'en est charge
    var h = location.hash;
    if (h.length > 1 && h.indexOf('#/') !== 0) {
      var cible = document.getElementById(decodeURIComponent(h.slice(1)));
      if (cible) { cible.scrollIntoView({ behavior: 'auto' }); return; }
    }
    window.scrollTo({ top: restaurer ? v.y : 0, behavior: 'auto' });
  }

  /* Ce qu'on veut retrouver en revenant est note au fil de l'eau, et non au
     moment du depart. Sur un retour arriere, `popstate` se declenche quand
     l'adresse a deja change: la vue qu'on quitte se verrait alors attribuer la
     position et la page de celle qui arrive, et le wiki rouvrirait sa premiere
     page au lieu de celle qu'on lisait. */
  function noter() {
    var v = vues[courant];
    if (!v || enCours) return;
    v.y = window.scrollY || document.documentElement.scrollTop || 0;
    v.hash = location.hash;
  }
  window.addEventListener('scroll', noter, { passive: true });
  window.addEventListener('hashchange', noter);

  /* ── L'aiguillage ───────────────────────────────────────────────────── */
  function aller(c, url, restaurer, pousser) {
    if (enCours) return;
    enCours = true;

    var ancienne = vues[courant];
    Promise.all([construire(c, url.href), ancienne ? sortir(ancienne.vue) : null])
      .then(function () {
        if (pousser) history.pushState({ lr: c }, '', url.href);
        afficher(c, url, restaurer);
        enCours = false;
      })
      .catch(function () {
        // Reseau coupe, page inattendue: on rend la main au navigateur.
        enCours = false;
        if (ancienne) ancienne.vue.classList.remove('lr-sort');
        location.href = url.href;
      });
  }

  document.addEventListener('click', function (e) {
    if (e.defaultPrevented || e.button !== 0) return;
    if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    var a = e.target && e.target.closest ? e.target.closest('a[href]') : null;
    if (!a || a.hasAttribute('download')) return;
    var cible = a.getAttribute('target');
    if (cible && cible !== '_self') return;

    var u;
    try { u = new URL(a.href, location.href); } catch (err) { return; }
    var c = cle(u);
    if (!c) return;

    e.preventDefault();

    // Meme page: rien a echanger. On garde l'URL propre et on se contente de se
    // deplacer dedans, ce qui evite au passage le rechargement que provoquait
    // un lien "index.html" depuis l'adresse du dossier.
    if (c === courant) {
      var change = u.href !== location.href;
      if (change) history.pushState({ lr: c }, '', u.href);
      if (location.hash.indexOf('#/') === 0) {
        if (change) window.dispatchEvent(new Event('hashchange'));
      } else {
        placer(vues[c], false, false);
      }
      vues[c].hash = location.hash;
      return;
    }
    aller(c, u, false, true);
  });

  window.addEventListener('popstate', function () {
    var c = cleCourante();
    // Une entree posee avant l'arrivee du routeur, ou une page qu'il ne connait
    // pas: le navigateur sait faire, nous non.
    if (!c || !vues[c]) { location.reload(); return; }
    if (c === courant) {
      var v = vues[c];
      if (location.hash.indexOf('#/') === 0) {
        if (v.hash !== location.hash) window.dispatchEvent(new Event('hashchange'));
      } else {
        placer(v, true, false);
      }
      v.hash = location.hash;
      return;
    }
    aller(c, new URL(location.href), true, false);
  });

  /* Survoler un lien, c'est presque toujours annoncer qu'on va cliquer. La page
     est alors deja en memoire quand le clic arrive. */
  function flairer(e) {
    var a = e.target && e.target.closest ? e.target.closest('a[href]') : null;
    if (!a) return;
    var u;
    try { u = new URL(a.href, location.href); } catch (err) { return; }
    var c = cle(u);
    if (c && c !== courant && !vues[c]) telecharger(c, u.href).catch(function () {});
  }

  if (amorcer()) {
    if ('scrollRestoration' in history) history.scrollRestoration = 'manual';
    history.replaceState({ lr: courant }, '', location.href);
    document.addEventListener('mouseover', flairer, { passive: true });
    document.addEventListener('touchstart', flairer, { passive: true });
    document.addEventListener('focusin', flairer, { passive: true });
  }
})();
