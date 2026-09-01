/* L'abeille de LaRuche, sur les pages du site.
 *
 * Une mascotte qui bouge sans arret devient insupportable au bout d'une heure.
 * Celle-ci passe donc l'essentiel de son temps POSEE, et ne vole que par
 * moments, avec des vitesses et des trajets differents a chaque fois. La
 * diversite ne vient pas d'un mouvement complique, elle vient de la variete des
 * etats et de la duree aleatoire de chacun.
 *
 * Les etats:
 *
 *   posee     immobile sur un element de la page (un titre, une carte, un
 *             bouton). Elle suit l'element quand on defile, donc elle a l'air
 *             reellement assise dessus. C'est l'etat le plus frequent.
 *   derive    vol lent vers un point au hasard, sans but apparent.
 *   file      pointe rapide vers un point, puis elle se calme.
 *   curieuse  elle s'approche du curseur, de loin.
 *
 * Et une reaction qui passe avant tout le reste: si le curseur arrive trop
 * pres, elle s'enfuit, quel que soit l'etat en cours.
 *
 * S'il y a une autre abeille dans la fenetre, celle de l'extension par exemple,
 * elles se voient et se repondent. Tout ce qui concerne ces rencontres est dans
 * `rencontre.js`, qui ne fait que designer un point a viser: le vol reste ici.
 *
 * Le clignement des yeux est pilote ici plutot que par l'animation CSS, qui
 * bat toutes les deux secondes comme un metronome. Un vrai clignement arrive a
 * intervalle irregulier, et parfois deux fois de suite.
 */
(function () {
  'use strict';

  var LENT = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  /* Les styles des ETATS vivent ici, pas dans les pages: il y en a deux a
     servir, et un comportement dont la moitie serait dans une feuille de style
     et l'autre dans un script se desynchronise a la premiere modification. Le
     DESSIN de l'abeille, lui, reste dans chaque page: c'est l'identite, pas le
     comportement. */
  var STYLE = [
    '.abeille-titre.posee .bee--wings, .abeille-titre.posee .bee--wings::before{animation-duration:1.1s;opacity:.5}',
    '.abeille-titre .bee--head-eyes{animation:none;transition:height .12s ease}',
    '.abeille-titre.cligne .bee--head-eyes,.abeille-titre.cligne .bee--head-eyes::before{height:0}',

    /* Endormie: les ailes se replient, la respiration prend le relais du vol. */
    '.abeille-titre.dort .bee--wings,.abeille-titre.dort .bee--wings::before{animation:none;opacity:.32;transform:scale(.72)}',
    '.abeille-titre.dort .bee--head-eyes,.abeille-titre.dort .bee--head-eyes::before{height:1px;background:#fff9}',
    '.abeille-titre.dort .bee{animation:lr-respire 3.4s ease-in-out infinite}',
    '@keyframes lr-respire{0%,100%{transform:translateY(0) scale(1)}50%{transform:translateY(1.5px) scale(1.045)}}',

    /* Le reveil: elle s etire une fois, les ailes remontent en regime. Sans
       cette demi-seconde, elle passe du sommeil au vol comme un interrupteur. */
    '.abeille-titre.reveil .bee{animation:lr-etire .62s cubic-bezier(.22,.61,.36,1)}',
    '.abeille-titre.reveil .bee--wings,.abeille-titre.reveil .bee--wings::before{animation-duration:.42s;transition:opacity .5s ease,transform .5s ease}',
    '@keyframes lr-etire{0%{transform:scale(1)}35%{transform:scale(1.16) rotate(-4deg)}100%{transform:scale(1)}}',

    /* Ce qui sort de sa tete quand elle dort. */
    '.lr-songe{position:absolute;left:34px;top:-4px;font-size:13px;line-height:1;pointer-events:none;',
    '  color:rgba(255,255,255,.62);animation:lr-monte 2.6s ease-out forwards;will-change:transform,opacity}',
    '.lr-songe.doux{font-size:12px}',
    '.lr-songe.tendre{color:rgba(244,114,182,.75)}',
    '@keyframes lr-monte{',
    '  0%{opacity:0;transform:translate(0,0) scale(.6) rotate(-8deg)}',
    '  18%{opacity:.95}',
    '  100%{opacity:0;transform:translate(16px,-38px) scale(1.18) rotate(10deg)}}',

    /* Ce qui sort de sa tete quand elle rencontre une autre abeille. */
    '.lr-songe.coeur{font-size:15px;color:rgba(244,114,182,.95);animation-duration:1.9s}',
    '.lr-songe.eclair{font-size:15px;animation-duration:.95s}',
    '.lr-songe.souffle{font-size:13px;animation-duration:.95s}',
    /* Quand elle regarde a gauche, tout l'element est retourne, et un `z` ou un
       souffle sortait a l'envers. La bulle se retourne alors une seconde fois. */
    '.lr-songe.mire{animation-name:lr-monte-mire}',
    '@keyframes lr-monte-mire{',
    '  0%{opacity:0;transform:translate(0,0) scale(-.6,.6) rotate(8deg)}',
    '  18%{opacity:.95}',
    '  100%{opacity:0;transform:translate(16px,-38px) scale(-1.18,1.18) rotate(-10deg)}}',
    '.abeille-titre.eteinte{display:none}',
    '@media (prefers-reduced-motion:reduce){.abeille-titre.dort .bee,.lr-songe{animation:none}.lr-songe{display:none}}'
  ].join('');
  (function(){
    var f = document.createElement('style');
    f.textContent = STYLE;
    document.head.appendChild(f);
  })();

  function hasard(a, b) { return a + Math.random() * (b - a); }
  function choix(t) { return t[Math.floor(Math.random() * t.length)]; }

  /* Le dernier endroit ou une abeille volait. La navigation douce echange le
     contenu de la page sans la recharger: l'abeille de la vue qui arrive part
     donc de la ou celle de la vue qui s'en va en etait, plutot que de reapparaitre
     dans un coin. */
  var dernierPoint = null;

  /* L'interrupteur.
   *
   * Une abeille qui traverse le champ de vision est charmante la premiere fois
   * et genante la dixieme, surtout en train de lire. Le choix se prend dans la
   * barre du haut et tient d'une visite a l'autre.
   *
   * Eteinte, elle est retiree de la mise en page: sa boite fait alors zero de
   * large, ce que le protocole des rencontres lit deja comme une absence. Rien
   * d'autre a debrancher de ce cote. */
  var CLE = 'laruche.abeille';
  var allumee = true;
  try { allumee = localStorage.getItem(CLE) !== '0'; } catch (e) {}
  var reprises = [];

  function installer(el) {
    if (!el || el.dataset.vivante) return;
    el.dataset.vivante = '1';

    // Ou elle peut se poser. On prend ce qui existe sur la page, du plus
    // interessant au plus banal: se poser sur un titre a du sens, se poser au
    // milieu d'un paragraphe non.
    var PERCHOIRS = 'h2, h3, .card, .app, .btn, .pill, .sw-item, .mem-item, .wt-item, .tr-seat, blockquote, pre';

    var PRES = 165;      // en deca, elle fuit
    var LOIN = 620;      // au dela, elle ignore

    var libre = el.classList.contains('vole') || el.dataset.libre === '1';
    var x = 0, y = 0, vx = 0, vy = 0;
    var bx = 0, by = 0;                    // origine de l'element une fois en vol
    var mx = -9999, my = -9999;
    var sens = 1;
    var etat = libre ? 'derive' : 'repos'; // repos = encore dans le titre
    var jusqua = 0;                        // horodatage de fin de l'etat
    var cible = null;                      // point vise
    var vitesse = 1;                       // vitesse max de l'etat en cours
    var perchoir = null;                   // element sur lequel elle est posee
    var perchoirDx = 0;
    var t = Math.random() * 1000;

    if (libre) {
      x = dernierPoint ? dernierPoint.x : Math.max(80, window.innerWidth * 0.7);
      y = dernierPoint ? dernierPoint.y : 140;
      x = Math.max(30, Math.min(window.innerWidth - 60, x));
      y = Math.max(70, Math.min(window.innerHeight - 50, y));
      el.classList.add('vole');
      el.style.transform = 'translate(' + (x - 26) + 'px,' + (y - 16) + 'px)';
      changerEtat(performance.now());
    }

    /* Le sommeil.
     *
     * Une page laissee ouverte n'a pas besoin d'une abeille qui tourne en rond
     * dans le vide. Apres un moment sans souris, sans defilement et sans
     * clavier, elle se pose et s'endort. C'est aussi ce qui rend le reveil
     * agreable: on revient, elle s'etire, et la page a l'air de vous avoir
     * attendu. */
    var INACTIVITE = 26000;
    // Le rabiot laisse au rendez-vous du coucher le temps de se conclure: sans
    // lui, elle s'endort seule pendant que l'autre repond a son invitation.
    var GRACE = 3400;
    var derniere = performance.now();
    var dort = false, songeur = null;

    function bouge() {
      derniere = performance.now();
      if (dort) reveiller();
    }
    ['mousemove', 'wheel', 'keydown', 'touchstart', 'scroll', 'pointerdown'].forEach(function (n) {
      window.addEventListener(n, bouge, { passive: true });
    });
    document.addEventListener('visibilitychange', function () { if (!document.hidden) bouge(); });

    /* Ce qui lui sort de la tete. Des `z` presque toujours, et de loin en loin
     * un coeur ou une goutte de miel: une surprise qui arrive une fois sur dix
     * reste une surprise, une qui arrive une fois sur deux devient un motif. */
    function bulle(txt, classe, duree) {
      var e = document.createElement('span');
      e.textContent = txt;
      e.className = 'lr-songe' + (classe ? ' ' + classe : '') + (sens === -1 ? ' mire' : '');
      e.style.left = (30 + hasard(-3, 7)) + 'px';
      el.appendChild(e);
      setTimeout(function () { if (e.parentNode) e.parentNode.removeChild(e); }, duree || 2700);
    }

    function songe() {
      if (!dort) return;
      var d = Math.random();
      if (d < 0.06) bulle('\u2665', 'doux tendre');
      else if (d < 0.12) bulle('\ud83c\udf6f', 'doux');
      else bulle(choix(['z', 'Z', 'z']), '');
      songeur = setTimeout(songe, hasard(1700, 3400));
    }

    function endormir(sensVoulu) {
      if (dort) return;
      dort = true;
      el.classList.add('dort');
      el.classList.remove('cligne');
      // Elle dort face a droite, sauf couchee a cote d'une autre abeille: dans
      // ce cas elle se tourne vers elle. La transformation est REAPPLIQUEE tout
      // de suite: la boucle sort desormais avant de la reecrire, donc le miroir
      // de son dernier vol restait en place.
      sens = sensVoulu === -1 ? -1 : 1;
      el.style.transform = 'translate(' + (x - bx - 26) + 'px,' + (y - by - 16) + 'px) scaleX(' + sens + ')';
      vx = vy = 0;
      songeur = setTimeout(songe, 700);
    }

    function reveiller() {
      if (!dort) return;
      dort = false;
      clearTimeout(songeur);
      if (lien) lien.finir();          // elles se separent en se reveillant
      el.classList.remove('dort');
      el.classList.add('reveil');
      setTimeout(function () { el.classList.remove('reveil'); }, 640);
      // Elle repart doucement: un etat calme d'abord, pas une pointe de vitesse.
      etat = 'derive'; cible = pointLibre(); vitesse = 0.9;
      jusqua = performance.now() + hasard(2500, 5000);
    }

    document.addEventListener('mousemove', function (e) { mx = e.clientX; my = e.clientY; }, { passive: true });
    document.addEventListener('mouseleave', function () { mx = -9999; my = -9999; });

    /* ── Le clignement ──────────────────────────────────────────────────── */
    function cligner() {
      if (LENT) return;
      if (dort) { setTimeout(cligner, hasard(2600, 9000)); return; }
      el.classList.add('cligne');
      setTimeout(function () { el.classList.remove('cligne'); }, 110);
      // Une fois sur cinq, elle recligne aussitot.
      if (Math.random() < 0.2) {
        setTimeout(function () {
          el.classList.add('cligne');
          setTimeout(function () { el.classList.remove('cligne'); }, 100);
        }, 240);
      }
      setTimeout(cligner, hasard(2600, 9000));
    }
    setTimeout(cligner, hasard(1200, 4000));

    /* ── Choix du prochain etat ─────────────────────────────────────────── */
    function perchoirsVisibles() {
      var out = [];
      var els = document.querySelectorAll(PERCHOIRS);
      for (var i = 0; i < els.length; i++) {
        var r = els[i].getBoundingClientRect();
        // Assez large pour l'accueillir, et vraiment a l'ecran.
        if (r.width > 90 && r.top > 90 && r.bottom < window.innerHeight - 40) out.push(els[i]);
      }
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
      // Ponderation: elle se pose beaucoup plus souvent qu'elle ne vole.
      var d = Math.random();
      var perchoirs = perchoirsVisibles();

      if (d < 0.46 && perchoirs.length) {
        etat = 'posee';
        perchoir = choix(perchoirs);
        var r = perchoir.getBoundingClientRect();
        // Un point au hasard le long du bord superieur, garde en relatif pour
        // qu'elle reste au meme endroit de l'element pendant qu'on defile.
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
        cible = null;                    // la cible est le curseur, qui bouge
        vitesse = hasard(1.2, 2.4);
        jusqua = maintenant + hasard(2000, 5000);
      }
      el.classList.toggle('posee', etat === 'posee');
    }

    /* ── Les autres abeilles ────────────────────────────────────────────── */

    /* Ce que les autres ont besoin de savoir d'elle. Rangee dans son titre ou
       masquee sur un petit ecran, elle n'existe pas pour elles. */
    function etatSocial() {
      if (etat === 'repos' || el.getBoundingClientRect().width === 0) return 'absente';
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

    var lien = (!LENT && window.LaRucheRencontre) ? window.LaRucheRencontre.rejoindre({
      position: function () { return { x: x, y: y }; },
      etat: etatSocial,
      perchoir: litProche,
      emoji: bulle,
      debut: function () {
        el.classList.remove('posee');
        perchoir = null;
        cible = null;
      },
      fin: function () { jusqua = 0; }     // elle reprend sa vie a l'image suivante
    }) : null;

    /* ── La boucle ──────────────────────────────────────────────────────── */
    var enVol = false;

    function reprendre() {
      if (allumee && !LENT && !enVol) { enVol = true; requestAnimationFrame(boucle); }
    }
    reprises.push(reprendre);

    function boucle() {
      if (!allumee) { enVol = false; return; }

      // Vue detachee: la navigation douce garde son DOM de cote pendant qu'on
      // lit l'autre page. La boucle reste en vie, mais ne calcule rien, et
      // l'abeille reprend exactement ou elle en etait si la vue revient.
      if (!el.isConnected) { requestAnimationFrame(boucle); return; }

      if (etat === 'repos' && el.getBoundingClientRect().width === 0) {
        return requestAnimationFrame(boucle);      // masquee (petit ecran)
      }

      var enHaut = window.scrollY < 140;
      var maintenant = performance.now();

      if (!libre) {
        if (!enHaut && etat === 'repos') {
          var r0 = el.getBoundingClientRect();
          x = r0.left + r0.width / 2; y = r0.top + r0.height / 2;
          vx = 0; vy = 0;
          el.classList.add('vole');
          // En position fixe, l'origine n'est pas forcement le coin de la fenetre:
          // un ancetre portant un filtre devient son repere. On la releve une fois.
          el.style.transform = 'none';
          var o = el.getBoundingClientRect();
          bx = o.left; by = o.top;
          changerEtat(maintenant);
        } else if (enHaut && etat !== 'repos') {
          etat = 'repos';
          el.classList.remove('vole', 'posee');
          el.style.transform = '';
        }
      }

      if (etat !== 'repos') {
        // Une rencontre en cours donne le point a viser, et remplace tout le
        // reste tant qu'elle dure.
        var consigne = lien ? lien.consigne() : null;

        // Assez longtemps sans rien: elle se pose et s'endort. On attend
        // qu'elle soit posee pour la laisser dormir, sinon elle s'endormirait
        // en plein vol. Et si elle est en route vers une autre abeille pour se
        // coucher a cote d'elle, elle ne s'endort surtout pas en chemin.
        if (!dort && !consigne && maintenant - derniere > INACTIVITE + GRACE) {
          if (etat === 'posee' && perchoir) endormir();
          else if (maintenant > jusqua || etat !== 'posee') {
            var pp = perchoirsVisibles();
            if (pp.length) {
              etat = 'posee'; perchoir = choix(pp);
              var rr = perchoir.getBoundingClientRect();
              perchoirDx = hasard(0.12, 0.88);
              cible = { x: rr.left + rr.width * perchoirDx, y: rr.top - 12 };
              vitesse = 3.2; jusqua = maintenant + 60000;
              el.classList.add('posee');
            } else { endormir(); }
          }
        }
        if (dort) { requestAnimationFrame(boucle); return; }

        if (!consigne) {
          if (maintenant > jusqua) changerEtat(maintenant);

          // Le perchoir bouge avec le defilement: elle reste dessus.
          if (perchoir) {
            var rp = perchoir.getBoundingClientRect();
            if (rp.width === 0 || rp.bottom < 40 || rp.top > window.innerHeight - 40) {
              changerEtat(maintenant);               // il est sorti de l'ecran
            } else {
              cible = { x: rp.left + rp.width * perchoirDx, y: rp.top - 12 };
            }
          }
        }

        var dx = x - mx, dy = y - my;
        var d = Math.sqrt(dx * dx + dy * dy);
        var fuit = !consigne && d > 0.01 && d < PRES;

        if (consigne) {
          // Occupee ailleurs: le curseur ne l'interesse plus le temps de la
          // rencontre. Deux abeilles qui se courent apres en fuyant la souris
          // ne se rattrapent jamais.
          t += 0.02;
          var ax = consigne.x - x, ay = consigne.y - y;
          var da = Math.sqrt(ax * ax + ay * ay) || 1;
          if (da > consigne.marge) {
            var ka = Math.min(da, 40) / 40 * 0.62;
            vx += ax / da * ka + Math.cos(t * 2.6) * 0.05;
            vy += ay / da * ka + Math.sin(t * 2.2) * 0.05;
          } else {
            vx *= 0.72; vy *= 0.72;
            // Arrivee au lit, et posee: elle ferme les yeux.
            if (consigne.dodo && Math.abs(vx) + Math.abs(vy) < 0.5) {
              endormir(consigne.sens);
              requestAnimationFrame(boucle);
              return;
            }
          }
        } else if (fuit) {
          var f = (PRES - d) / PRES * 2.3;
          vx += dx / d * f; vy += dy / d * f;
          if (etat === 'posee') { el.classList.remove('posee'); perchoir = null; jusqua = maintenant + 900; }
        } else if (etat === 'curieuse' && d < LOIN && d > 0.01) {
          var g = (LOIN - d) / (LOIN - PRES) * 0.22;
          vx -= dx / d * g; vy -= dy / d * g;
        } else if (cible) {
          // Vol vers la cible, avec un peu de flottement pour ne jamais aller
          // droit. Une abeille corrige sans arret, elle ne plane pas.
          t += 0.01;
          var cx = cible.x - x, cy = cible.y - y;
          var dc = Math.sqrt(cx * cx + cy * cy) || 1;
          var arrivee = etat === 'posee' && dc < 6;
          if (!arrivee) {
            var k = Math.min(dc, 40) / 40 * 0.5;
            vx += cx / dc * k + Math.cos(t * 2.1) * 0.05;
            vy += cy / dc * k + Math.sin(t * 1.6) * 0.05;
          } else {
            vx *= 0.6; vy *= 0.6;                    // elle se pose et se tait
          }
        }

        vx *= 0.9; vy *= 0.9;
        var v = Math.sqrt(vx * vx + vy * vy);
        var vmax = consigne ? consigne.vitesse : (fuit ? 9 : vitesse);
        if (v > vmax) { vx = vx / v * vmax; vy = vy / v * vmax; }

        x += vx; y += vy;
        x = Math.max(30, Math.min(window.innerWidth - 60, x));
        y = Math.max(70, Math.min(window.innerHeight - 50, y));

        // Elle se retourne pour aller ou elle va, avec un seuil qui l'empeche de
        // clignoter d'un sens a l'autre quand elle est presque immobile.
        if (vx < -0.4) sens = -1; else if (vx > 0.4) sens = 1;
        el.style.transform = 'translate(' + (x - bx - 26) + 'px,' + (y - by - 16) + 'px) scaleX(' + sens + ')';
        dernierPoint = { x: x, y: y };
      }
      requestAnimationFrame(boucle);
    }

    el.classList.toggle('eteinte', !allumee);
    reprendre();
  }

  function demarrer() {
    cablerBascule();
    var els = document.querySelectorAll('.abeille-titre');
    for (var i = 0; i < els.length; i++) installer(els[i]);
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', demarrer);
  else demarrer();

  // La navigation douce rappelle `demarrer` apres chaque echange de vue: les
  // abeilles deja installees sont reconnues et laissees tranquilles.
  function reglage(actif) {
    allumee = !!actif;
    try { localStorage.setItem(CLE, allumee ? '1' : '0'); } catch (e) {}
    var els = document.querySelectorAll('.abeille-titre');
    for (var i = 0; i < els.length; i++) els[i].classList.toggle('eteinte', !allumee);
    for (var j = 0; j < reprises.length; j++) reprises[j]();
    peindreBascule();
    return allumee;
  }

  /* Le bouton de la barre du haut. Il est cable ici et non dans chaque page:
     les deux barres sont identiques a l'attribut pres, et la navigation douce
     rappelle `demarrer` apres chaque echange de vue. */
  function peindreBascule() {
    var b = document.getElementById('bascule-abeille');
    if (!b) return;
    b.setAttribute('aria-pressed', allumee ? 'true' : 'false');
    b.classList.toggle('eteinte', !allumee);
    var texte = allumee ? 'Hide the bee' : 'Show the bee';
    b.title = texte;
    b.setAttribute('aria-label', texte);
  }

  function cablerBascule() {
    var b = document.getElementById('bascule-abeille');
    if (!b || b.dataset.cable) return;
    b.dataset.cable = '1';
    b.addEventListener('click', function () { reglage(!allumee); });
    peindreBascule();
  }

  window.LaRucheAbeille = {
    demarrer: demarrer,
    reglage: reglage,
    active: function () { return allumee; }
  };
})();
