/* Le compagnon: l'abeille de LaRuche, sur les pages que vous visitez.
 *
 * Desactive par defaut. Reproduction fidele du comportement de l'abeille du site
 * (docs/abeille.js et docs/index.html) avec isolation Shadow DOM et synchronisation
 * inter-onglets.
 *
 * Sur les pages de LaRuche, l'abeille du site est la aussi. Les deux se voient
 * et se repondent: coeurs quand elles se rapprochent, course poursuite quand
 * elles sont cote a cote, et coucher a deux quand la page est laissee de cote.
 * Tout cela est dans `rencontre.js`, charge juste avant celui-ci, en copie
 * conforme de celui du site.
 */
(function () {
  'use strict';
  if (window.__larucheCompagnonInstalle) return;
  window.__larucheCompagnonInstalle = true;

  var LENT = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var hote = null;
  var racine = null;
  var el = null;
  var boucleActive = false;
  var frameId = null;

  function hasard(a, b) { return a + Math.random() * (b - a); }
  function choix(t) { return t[Math.floor(Math.random() * t.length)]; }

  var CSS = `
    :host { all: initial; }
    .abeille-compagnon {
      position: fixed; left: 0; top: 0; width: 51px; height: 33px; margin: 0;
      z-index: 2147483600; pointer-events: none; will-change: transform;
    }
    .abeille-compagnon .bee {
      width: 51px; height: 33px; position: absolute; left: 0; top: 0;
      filter: drop-shadow(0 0 4px rgba(234,179,8,.45)); pointer-events: none;
      animation: bee-float .8s infinite ease-in-out;
    }
    .abeille-compagnon .bee > *, .abeille-compagnon .bee--body > span { display: block; }
    @keyframes bee-float { 0%, 100% { transform: translateY(0); } 50% { transform: translateY(-3px); } }

    .bee--wings {
      width: 15px; height: 15px; background: #fff; opacity: .8; position: absolute;
      top: -6px; left: 9px; z-index: 99; border-radius: 50%; animation: bee-fly-lw .15s infinite;
    }
    .bee--wings::before {
      content: ''; position: absolute; width: 15px; height: 15px; background: #fff;
      opacity: .8; border-radius: 50%; top: 1px; left: 17px; animation: bee-fly-rw .15s infinite;
    }
    @keyframes bee-fly-lw { 0%, 100% { top: -6px; left: 9px; } 25% { top: -9px; } 50% { top: -12px; left: 11px; } 75% { top: -9px; } }
    @keyframes bee-fly-rw { 0%, 100% { top: 1px; left: 17px; } 25% { top: -1px; } 50% { left: 14px; top: -2px; } 75% { top: -1px; } }

    .bee--body { position: absolute; top: 0; }
    .bee--body span { position: absolute; border-radius: 50%; width: 15px; height: 15px; background: #000; }
    .bee--body span:first-child::after {
      content: ''; position: absolute; left: 5px; border-radius: 50%;
      width: 15px; height: 15px; background: #eab308;
    }
    .bee--body span:last-child { left: 9px; }
    .bee--body span:last-child::after {
      content: ''; position: absolute; left: 6px; border-radius: 50%;
      width: 15px; height: 15px; background: #eab308;
    }

    .bee--head {
      width: 32px; height: 32px; background: #000; border-radius: 50%; position: relative;
      margin-left: 20px; top: -1px; z-index: 99;
    }
    .bee--head-eyes {
      background: #fff; width: 5px; height: 3px; position: absolute; top: 8px; left: 12px;
      border-bottom-left-radius: 100%; border-bottom-right-radius: 100%;
      transition: height .12s ease;
    }
    .bee--head-eyes::before {
      content: ''; position: absolute; top: 0; left: 8px; width: 5px; height: 3px;
      background: #fff; border-bottom-left-radius: 100%; border-bottom-right-radius: 100%;
    }

    .bee--head-antennas {
      width: 3px; height: 8px; background: #000; position: absolute; left: 3px; top: -4px;
      border-radius: 30px; transform: rotate(-50deg); animation: bee-la .3s infinite;
    }
    .bee--head-antennas::before {
      content: ''; position: absolute; left: 5px; top: 1px; width: 3px; height: 10px;
      background: #000; border-radius: 30px; transform: rotate(-180deg); animation: bee-ra .5s infinite;
    }
    @keyframes bee-la {
      0%, 100% { transform: rotate(-50deg); left: 3px; }
      25% { transform: rotate(-35deg); left: 5px; }
      50% { transform: rotate(-15deg); left: 5px; }
      75% { transform: rotate(-35deg); left: 4px; }
    }
    @keyframes bee-ra {
      0%, 100% { transform: rotate(-170deg); left: 5px; }
      25% { transform: rotate(-168deg); left: 5px; }
      50% { transform: rotate(-165deg); left: 4px; }
      75% { transform: rotate(-168deg); left: 5px; }
    }

    /* Etats */
    .abeille-compagnon.cligne .bee--head-eyes,
    .abeille-compagnon.cligne .bee--head-eyes::before { height: 0; }

    .abeille-compagnon.posee .bee { animation: none; }
    .abeille-compagnon.posee .bee--wings,
    .abeille-compagnon.posee .bee--wings::before { animation-duration: 1.1s; opacity: .5; }

    .abeille-compagnon.dort .bee--wings,
    .abeille-compagnon.dort .bee--wings::before { animation: none; opacity: .32; transform: scale(.72); }
    .abeille-compagnon.dort .bee--head-eyes,
    .abeille-compagnon.dort .bee--head-eyes::before { height: 1px; background: #fff9; }
    .abeille-compagnon.dort .bee { animation: lr-respire 3.4s ease-in-out infinite; }
    @keyframes lr-respire {
      0%, 100% { transform: translateY(0) scale(1); }
      50% { transform: translateY(1.5px) scale(1.045); }
    }

    .abeille-compagnon.reveil .bee { animation: lr-etire .62s cubic-bezier(.22,.61,.36,1); }
    .abeille-compagnon.reveil .bee--wings,
    .abeille-compagnon.reveil .bee--wings::before {
      animation-duration: .42s; transition: opacity .5s ease, transform .5s ease;
    }
    @keyframes lr-etire {
      0% { transform: scale(1); }
      35% { transform: scale(1.16) rotate(-4deg); }
      100% { transform: scale(1); }
    }

    .lr-songe {
      position: absolute; left: 34px; top: -4px; font-size: 13px; line-height: 1; pointer-events: none;
      color: rgba(255,255,255,.62); animation: lr-monte 2.6s ease-out forwards; will-change: transform, opacity;
      font-family: system-ui, sans-serif;
    }
    .lr-songe.doux { font-size: 12px; }
    .lr-songe.tendre { color: rgba(244,114,182,.75); }
    @keyframes lr-monte {
      0% { opacity: 0; transform: translate(0,0) scale(.6) rotate(-8deg); }
      18% { opacity: .95; }
      100% { opacity: 0; transform: translate(16px,-38px) scale(1.18) rotate(10deg); }
    }

    /* Ce qui sort de sa tete quand elle rencontre une autre abeille. */
    .lr-songe.coeur { font-size: 15px; color: rgba(244,114,182,.95); animation-duration: 1.9s; }
    .lr-songe.eclair { font-size: 15px; animation-duration: .95s; }
    .lr-songe.souffle { font-size: 13px; animation-duration: .95s; }
    /* Quand elle regarde a gauche, tout l'element est retourne, et un z ou un
       souffle sortait a l'envers. La bulle se retourne alors une seconde fois. */
    .lr-songe.mire { animation-name: lr-monte-mire; }
    @keyframes lr-monte-mire {
      0% { opacity: 0; transform: translate(0,0) scale(-.6,.6) rotate(8deg); }
      18% { opacity: .95; }
      100% { opacity: 0; transform: translate(16px,-38px) scale(-1.18,1.18) rotate(-10deg); }
    }

    @media (prefers-reduced-motion: reduce) {
      .abeille-compagnon, .abeille-compagnon .bee, .abeille-compagnon .bee * { animation: none !important; }
      .lr-songe { display: none; }
    }
  `;

  function creer() {
    if (hote) return;
    hote = document.createElement('div');
    hote.id = '__laruche_compagnon__';
    hote.style.cssText = 'position:fixed;left:0;top:0;width:0;height:0;z-index:2147483600;pointer-events:none;';
    racine = hote.attachShadow({ mode: 'open' });
    var style = document.createElement('style');
    style.textContent = CSS;
    racine.appendChild(style);

    el = document.createElement('div');
    el.className = 'abeille-compagnon';
    el.innerHTML = '<span class="bee">' +
      '<span class="bee--wings"></span>' +
      '<span class="bee--body"><span></span><span></span></span>' +
      '<span class="bee--head">' +
        '<span class="bee--head-eyes"></span>' +
        '<span class="bee--head-antennas"></span>' +
      '</span>' +
    '</span>';

    racine.appendChild(el);
    (document.body || document.documentElement).appendChild(hote);
  }

  function detruire() {
    boucleActive = false;
    if (lien) {
      lien.quitter();
      lien = null;
    }
    if (frameId) {
      cancelAnimationFrame(frameId);
      frameId = null;
    }
    if (hote && hote.parentNode) {
      hote.parentNode.removeChild(hote);
    }
    hote = null;
    racine = null;
    el = null;
  }

  /* Dynamique de vol, de perchoir et de sommeil identique a docs/abeille.js */
  var PERCHOIRS = 'h1, h2, h3, article, .card, img, figure, button, blockquote, pre, table, a.btn, .pill';
  var PRES = 165;
  var LOIN = 620;
  var INACTIVITE = 26000;
  // Le rabiot laisse au rendez-vous du coucher le temps de se conclure: sans
  // lui, elle s'endort seule pendant que l'autre repond a son invitation.
  var GRACE = 3400;

  var x = window.innerWidth * 0.7;
  var y = 140;
  var vx = 0;
  var vy = 0;
  var mx = -9999;
  var my = -9999;
  var sens = 1;
  var etat = 'derive';
  var jusqua = 0;
  var cible = null;
  var vitesse = 1;
  var perchoir = null;
  var perchoirDx = 0;
  var t = Math.random() * 1000;
  var derniere = performance.now();
  var dort = false;
  var songeur = null;
  var dernierSauvegarde = 0;
  var lien = null;              // le canal des rencontres, quand elle est posee sur la page

  function poser() {
    if (el) {
      el.style.transform = 'translate(' + Math.round(x - 26) + 'px,' + Math.round(y - 16) + 'px) scaleX(' + sens + ')';
    }
  }

  function sauverPosition() {
    var now = performance.now();
    if (now - dernierSauvegarde < 500) return;
    dernierSauvegarde = now;
    try {
      chrome.storage.local.set({
        compagnonPosition: {
          x: Math.round(x),
          y: Math.round(y),
          sens: sens,
          dort: dort,
          etat: etat,
          maj: Date.now()
        }
      });
    } catch (e) {}
  }

  function chargerPosition(cb) {
    try {
      chrome.storage.local.get({ compagnonPosition: null }, function (res) {
        if (res && res.compagnonPosition) {
          var p = res.compagnonPosition;
          if (typeof p.x === 'number' && typeof p.y === 'number') {
            x = Math.max(30, Math.min(window.innerWidth - 60, p.x));
            y = Math.max(60, Math.min(window.innerHeight - 50, p.y));
            if (p.sens === 1 || p.sens === -1) sens = p.sens;
            poser();
          }
        }
        if (cb) cb();
      });
    } catch (e) {
      if (cb) cb();
    }
  }

  function bouge() {
    derniere = performance.now();
    if (dort) reveiller();
  }

  function cligner() {
    if (LENT || !el) return;
    if (dort) {
      setTimeout(cligner, hasard(2600, 9000));
      return;
    }
    if (el) {
      el.classList.add('cligne');
      setTimeout(function () {
        if (el) el.classList.remove('cligne');
      }, 110);
      if (Math.random() < 0.2) {
        setTimeout(function () {
          if (el) el.classList.add('cligne');
          setTimeout(function () {
            if (el) el.classList.remove('cligne');
          }, 100);
        }, 240);
      }
    }
    setTimeout(cligner, hasard(2600, 9000));
  }

  function bulle(txt, classe, duree) {
    if (!el) return;
    var e = document.createElement('span');
    e.textContent = txt;
    e.className = 'lr-songe' + (classe ? ' ' + classe : '') + (sens === -1 ? ' mire' : '');
    e.style.left = (30 + hasard(-3, 7)) + 'px';
    el.appendChild(e);
    setTimeout(function () {
      if (e.parentNode) e.parentNode.removeChild(e);
    }, duree || 2700);
  }

  function songe() {
    if (!dort || !el) return;
    var d = Math.random();
    if (d < 0.06) bulle('\u2665', 'doux tendre');
    else if (d < 0.12) bulle('\ud83c\udf6f', 'doux');
    else bulle(choix(['z', 'Z', 'z']), '');
    songeur = setTimeout(songe, hasard(1700, 3400));
  }

  /* Elle dort face a droite, sauf couchee a cote d'une autre abeille: dans ce
     cas elle se tourne vers elle. */
  function endormir(sensVoulu) {
    if (dort || !el) return;
    dort = true;
    el.classList.add('dort');
    el.classList.remove('cligne');
    sens = sensVoulu === -1 ? -1 : 1;
    poser();
    vx = vy = 0;
    songeur = setTimeout(songe, 700);
    sauverPosition();
  }

  function reveiller() {
    if (!dort || !el) return;
    dort = false;
    clearTimeout(songeur);
    if (lien) lien.finir();          // elles se separent en se reveillant
    el.classList.remove('dort');
    el.classList.add('reveil');
    setTimeout(function () {
      if (el) el.classList.remove('reveil');
    }, 640);
    etat = 'derive';
    cible = pointLibre();
    vitesse = 0.9;
    jusqua = performance.now() + hasard(2500, 5000);
  }

  function perchoirsVisibles() {
    var out = [];
    try {
      var els = document.querySelectorAll(PERCHOIRS);
      for (var i = 0; i < els.length; i++) {
        var r = els[i].getBoundingClientRect();
        if (r.width > 90 && r.top > 90 && r.bottom < window.innerHeight - 40) {
          out.push(els[i]);
        }
      }
    } catch (e) {}
    return out;
  }

  function pointLibre() {
    return {
      x: hasard(70, Math.max(80, window.innerWidth - 90)),
      y: hasard(90, Math.max(100, window.innerHeight - 80))
    };
  }

  function changerEtat(maintenant) {
    perchoir = null;
    var d = Math.random();
    var perchoirs = perchoirsVisibles();

    if (d < 0.46 && perchoirs.length) {
      etat = 'posee';
      perchoir = choix(perchoirs);
      var r = perchoir.getBoundingClientRect();
      perchoirDx = hasard(0.12, 0.88);
      cible = { x: r.left + r.width * perchoirDx, y: r.top - 12 };
      vitesse = 3.2;
      jusqua = maintenant + hasard(3500, 11000);
    } else if (d < 0.72) {
      etat = 'derive';
      cible = pointLibre();
      vitesse = hasard(0.7, 1.5);
      jusqua = maintenant + hasard(2500, 6000);
    } else if (d < 0.88) {
      etat = 'file';
      cible = pointLibre();
      vitesse = hasard(4, 7);
      jusqua = maintenant + hasard(700, 1600);
    } else {
      etat = 'curieuse';
      cible = null;
      vitesse = hasard(1.2, 2.4);
      jusqua = maintenant + hasard(2000, 5000);
    }
    if (el) el.classList.toggle('posee', etat === 'posee');
  }

  /* ── Les autres abeilles ───────────────────────────────────────────────
     Sur les pages du site, l'abeille de la page est la aussi. Ce qui suit lui
     dit ou en est celle-ci, et `rencontre.js` s'occupe du reste. */

  function etatSocial() {
    if (!el || !boucleActive || document.hidden) return 'absente';
    if (dort) return 'endormie';
    if (performance.now() - derniere > INACTIVITE) return 'somnolente';
    return 'eveillee';
  }

  /* Le perchoir le plus proche d'un point, pour que le rendez-vous du coucher
     tombe sur un titre plutot qu'au milieu du vide. */
  function litProche(px, py) {
    var pp = perchoirsVisibles();
    var mieux = null, ecart = Infinity;
    for (var i = 0; i < pp.length; i++) {
      var r = pp[i].getBoundingClientRect();
      var cx = r.left + r.width / 2, cy = r.top - 12;
      var d = (cx - px) * (cx - px) + (cy - py) * (cy - py);
      if (d < ecart) { ecart = d; mieux = { x: cx, y: cy }; }
    }
    return mieux;
  }

  function rejoindreLesAutres() {
    if (lien || LENT || !window.LaRucheRencontre) return;
    lien = window.LaRucheRencontre.rejoindre({
      position: function () { return { x: x, y: y }; },
      etat: etatSocial,
      perchoir: litProche,
      emoji: bulle,
      debut: function () {
        if (el) el.classList.remove('posee');
        perchoir = null;
        cible = null;
      },
      fin: function () { jusqua = 0; }   // elle reprend sa vie a l'image suivante
    });
  }

  function boucle() {
    if (!boucleActive || !el || document.hidden) return;
    var maintenant = performance.now();

    // Une rencontre en cours donne le point a viser, et remplace tout le reste
    // tant qu'elle dure.
    var consigne = lien ? lien.consigne() : null;

    // Si elle est en route vers une autre abeille pour se coucher a cote
    // d'elle, elle ne s'endort surtout pas en chemin.
    if (!dort && !consigne && maintenant - derniere > INACTIVITE + GRACE) {
      if (etat === 'posee' && perchoir) {
        endormir();
      } else if (maintenant > jusqua || etat !== 'posee') {
        var pp = perchoirsVisibles();
        if (pp.length) {
          etat = 'posee';
          perchoir = choix(pp);
          var rr = perchoir.getBoundingClientRect();
          perchoirDx = hasard(0.12, 0.88);
          cible = { x: rr.left + rr.width * perchoirDx, y: rr.top - 12 };
          vitesse = 3.2;
          jusqua = maintenant + 60000;
          el.classList.add('posee');
        } else {
          endormir();
        }
      }
    }

    if (dort) {
      frameId = requestAnimationFrame(boucle);
      return;
    }

    if (!consigne) {
      if (maintenant > jusqua) changerEtat(maintenant);

      if (perchoir) {
        var rp = perchoir.getBoundingClientRect();
        if (rp.width === 0 || rp.bottom < 40 || rp.top > window.innerHeight - 40) {
          changerEtat(maintenant);
        } else {
          cible = { x: rp.left + rp.width * perchoirDx, y: rp.top - 12 };
        }
      }
    }

    var dx = x - mx;
    var dy = y - my;
    var dist = Math.sqrt(dx * dx + dy * dy);
    var fuit = !consigne && dist > 0.01 && dist < PRES;

    if (consigne) {
      // Occupee ailleurs: le curseur ne l'interesse plus le temps de la
      // rencontre. Deux abeilles qui se courent apres en fuyant la souris ne
      // se rattrapent jamais.
      t += 0.02;
      var ax = consigne.x - x;
      var ay = consigne.y - y;
      var da = Math.sqrt(ax * ax + ay * ay) || 1;
      if (da > consigne.marge) {
        var ka = Math.min(da, 40) / 40 * 0.62;
        vx += ax / da * ka + Math.cos(t * 2.6) * 0.05;
        vy += ay / da * ka + Math.sin(t * 2.2) * 0.05;
      } else {
        vx *= 0.72;
        vy *= 0.72;
        // Arrivee au lit, et posee: elle ferme les yeux.
        if (consigne.dodo && Math.abs(vx) + Math.abs(vy) < 0.5) {
          endormir(consigne.sens);
          frameId = requestAnimationFrame(boucle);
          return;
        }
      }
    } else if (fuit) {
      var f = (PRES - dist) / PRES * 2.3;
      vx += dx / dist * f;
      vy += dy / dist * f;
      if (etat === 'posee') {
        el.classList.remove('posee');
        perchoir = null;
        jusqua = maintenant + 900;
      }
    } else if (etat === 'curieuse' && dist < LOIN && dist > 0.01) {
      var g = (LOIN - dist) / (LOIN - PRES) * 0.22;
      vx -= dx / dist * g;
      vy -= dy / dist * g;
    } else if (cible) {
      t += 0.01;
      var cx = cible.x - x;
      var cy = cible.y - y;
      var dc = Math.sqrt(cx * cx + cy * cy) || 1;
      var arrivee = etat === 'posee' && dc < 6;
      if (!arrivee) {
        var k = Math.min(dc, 40) / 40 * 0.5;
        vx += cx / dc * k + Math.cos(t * 2.1) * 0.05;
        vy += cy / dc * k + Math.sin(t * 1.6) * 0.05;
      } else {
        vx *= 0.6;
        vy *= 0.6;
      }
    }

    vx *= 0.9;
    vy *= 0.9;
    var v = Math.sqrt(vx * vx + vy * vy);
    var vmax = consigne ? consigne.vitesse : (fuit ? 9 : vitesse);
    if (v > vmax) {
      vx = vx / v * vmax;
      vy = vy / v * vmax;
    }

    x += vx;
    y += vy;
    x = Math.max(30, Math.min(window.innerWidth - 60, x));
    y = Math.max(70, Math.min(window.innerHeight - 50, y));

    if (vx < -0.4) sens = -1;
    else if (vx > 0.4) sens = 1;

    poser();
    sauverPosition();
    if (!LENT) {
      frameId = requestAnimationFrame(boucle);
    }
  }

  function demarrerBoucle() {
    if (boucleActive) return;
    boucleActive = true;
    changerEtat(performance.now());
    if (!LENT) {
      frameId = requestAnimationFrame(boucle);
    }
  }

  var ecouteursPoses = false;

  function ecouteurs() {
    if (ecouteursPoses) return;
    ecouteursPoses = true;
    ['mousemove', 'wheel', 'keydown', 'touchstart', 'scroll', 'pointerdown'].forEach(function (n) {
      window.addEventListener(n, bouge, { passive: true });
    });

    document.addEventListener('mousemove', function (e) {
      mx = e.clientX;
      my = e.clientY;
    }, { passive: true });

    document.addEventListener('mouseleave', function () {
      mx = -9999;
      my = -9999;
    });

    document.addEventListener('visibilitychange', function () {
      if (document.hidden) {
        sauverPosition();
        if (frameId) {
          cancelAnimationFrame(frameId);
          frameId = null;
        }
      } else {
        bouge();
        chargerPosition(function () {
          if (boucleActive && !frameId && !LENT) {
            frameId = requestAnimationFrame(boucle);
          }
        });
      }
    });

    window.addEventListener('focus', function () {
      bouge();
      chargerPosition(function () {
        if (boucleActive && !frameId && !LENT) {
          frameId = requestAnimationFrame(boucle);
        }
      });
    });

    window.addEventListener('pagehide', sauverPosition);
    window.addEventListener('beforeunload', sauverPosition);
  }

  function vivre() {
    ecouteurs();
    rejoindreLesAutres();
    setTimeout(cligner, hasard(1200, 4000));
    chargerPosition(function () {
      if (!document.hidden) {
        demarrerBoucle();
      }
    });
  }

  function appliquer(actif) {
    if (actif && !hote) {
      creer();
      vivre();
    } else if (!actif && hote) {
      detruire();
    }
  }

  try {
    chrome.storage.local.get({ compagnon: false }, function (o) {
      if (o && o.compagnon) appliquer(true);
    });

    chrome.storage.onChanged.addListener(function (ch, zone) {
      if (zone === 'local' && ch.compagnon !== undefined) {
        appliquer(!!ch.compagnon.newValue);
      }
    });
  } catch (e) {}
})();
