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
    '@keyframes lr-monte{',
    '  0%{opacity:0;transform:translate(0,0) scale(.6) rotate(-8deg)}',
    '  18%{opacity:.95}',
    '  100%{opacity:0;transform:translate(16px,-38px) scale(1.18) rotate(10deg)}}',
    '@media (prefers-reduced-motion:reduce){.abeille-titre.dort .bee,.lr-songe{animation:none}.lr-songe{display:none}}'
  ].join('');
  (function(){
    var f = document.createElement('style');
    f.textContent = STYLE;
    document.head.appendChild(f);
  })();

  function hasard(a, b) { return a + Math.random() * (b - a); }
  function choix(t) { return t[Math.floor(Math.random() * t.length)]; }

  function installer(el) {
    if (!el || el.dataset.vivante) return;
    el.dataset.vivante = '1';

    // Ou elle peut se poser. On prend ce qui existe sur la page, du plus
    // interessant au plus banal: se poser sur un titre a du sens, se poser au
    // milieu d'un paragraphe non.
    var PERCHOIRS = 'h2, h3, .card, .app, .btn, .pill, .sw-item, .mem-item, .wt-item, .tr-seat, blockquote, pre';

    var PRES = 165;      // en deca, elle fuit
    var LOIN = 620;      // au dela, elle ignore

    var x = 0, y = 0, vx = 0, vy = 0;
    var bx = 0, by = 0;                    // origine de l'element une fois en vol
    var mx = -9999, my = -9999;
    var sens = 1;
    var etat = 'repos';                    // repos = encore dans le titre
    var jusqua = 0;                        // horodatage de fin de l'etat
    var cible = null;                      // point vise
    var vitesse = 1;                       // vitesse max de l'etat en cours
    var perchoir = null;                   // element sur lequel elle est posee
    var perchoirDx = 0;
    var t = Math.random() * 1000;

    /* Le sommeil.
     *
     * Une page laissee ouverte n'a pas besoin d'une abeille qui tourne en rond
     * dans le vide. Apres un moment sans souris, sans defilement et sans
     * clavier, elle se pose et s'endort. C'est aussi ce qui rend le reveil
     * agreable: on revient, elle s'etire, et la page a l'air de vous avoir
     * attendu. */
    var INACTIVITE = 26000;
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
    function songe() {
      if (!dort) return;
      var e = document.createElement('span');
      var d = Math.random();
      if (d < 0.06) { e.textContent = '\u2665'; e.className = 'lr-songe doux'; e.style.color = 'rgba(244,114,182,.75)'; }
      else if (d < 0.12) { e.textContent = '\ud83c\udf6f'; e.className = 'lr-songe doux'; }
      else { e.textContent = choix(['z', 'Z', 'z']); e.className = 'lr-songe'; }
      e.style.left = (30 + hasard(-3, 7)) + 'px';
      el.appendChild(e);
      setTimeout(function () { if (e.parentNode) e.parentNode.removeChild(e); }, 2700);
      songeur = setTimeout(songe, hasard(1700, 3400));
    }

    function endormir() {
      if (dort) return;
      dort = true;
      el.classList.add('dort');
      el.classList.remove('cligne');
      sens = 1;                       // elle dort face a droite: les `z` ne sont pas mis a l'envers
      vx = vy = 0;
      songeur = setTimeout(songe, 700);
    }

    function reveiller() {
      if (!dort) return;
      dort = false;
      clearTimeout(songeur);
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

    /* ── La boucle ──────────────────────────────────────────────────────── */
    function boucle() {
      if (etat === 'repos' && el.getBoundingClientRect().width === 0) {
        return requestAnimationFrame(boucle);      // masquee (petit ecran)
      }

      var enHaut = window.scrollY < 140;
      var maintenant = performance.now();

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

      if (etat !== 'repos') {
        // Assez longtemps sans rien: elle se pose et s'endort. On attend
        // qu'elle soit posee pour la laisser dormir, sinon elle s'endormirait
        // en plein vol.
        if (!dort && maintenant - derniere > INACTIVITE) {
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

        if (maintenant > jusqua) changerEtat(maintenant);

        // Le perchoir bouge avec le defilement: elle reste dessus.
        if (perchoir) {
          var rp = perchoir.getBoundingClientRect();
          if (rp.width === 0 || rp.bottom < 40 || rp.top > window.innerHeight - 40) {
            changerEtat(maintenant);                 // il est sorti de l'ecran
          } else {
            cible = { x: rp.left + rp.width * perchoirDx, y: rp.top - 12 };
          }
        }

        var dx = x - mx, dy = y - my;
        var d = Math.sqrt(dx * dx + dy * dy);
        var fuit = d > 0.01 && d < PRES;

        if (fuit) {
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
        var vmax = fuit ? 9 : vitesse;
        if (v > vmax) { vx = vx / v * vmax; vy = vy / v * vmax; }

        x += vx; y += vy;
        x = Math.max(30, Math.min(window.innerWidth - 60, x));
        y = Math.max(70, Math.min(window.innerHeight - 50, y));

        // Elle se retourne pour aller ou elle va, avec un seuil qui l'empeche de
        // clignoter d'un sens a l'autre quand elle est presque immobile.
        if (vx < -0.4) sens = -1; else if (vx > 0.4) sens = 1;
        el.style.transform = 'translate(' + (x - bx - 26) + 'px,' + (y - by - 16) + 'px) scaleX(' + sens + ')';
      }
      requestAnimationFrame(boucle);
    }

    if (!LENT) requestAnimationFrame(boucle);
  }

  function demarrer() {
    var els = document.querySelectorAll('.abeille-titre');
    for (var i = 0; i < els.length; i++) installer(els[i]);
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', demarrer);
  else demarrer();
})();
