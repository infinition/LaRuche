/* Le retour en haut de page.
 *
 * Une fleche discrete en bas a droite, qui n'apparait qu'une fois la page
 * descendue. Elle vit dans la coquille, pas dans le contenu: les deux pages la
 * partagent, et la navigation douce ne la remplace pas d'une vue a l'autre.
 *
 * Le style est injecte ici plutot que dans chaque feuille, pour la meme raison
 * que celui de l'abeille: un bouton dont la moitie du comportement serait dans
 * une feuille et l'autre dans un script se desynchronise a la premiere retouche.
 *
 * Cachee, elle est aussi retiree du parcours au clavier: un bouton invisible qui
 * reste tabulable envoie l'utilisateur nulle part.
 */
(function () {
  'use strict';

  var SEUIL = 420;      // px descendus avant qu'elle se montre
  var LENT = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  var STYLE = [
    '#lr-haut{position:fixed;z-index:65;',
    '  right:calc(22px + env(safe-area-inset-right));',
    '  bottom:calc(22px + env(safe-area-inset-bottom));',
    '  width:38px;height:38px;padding:0;display:grid;place-items:center;',
    '  border:1px solid var(--border-light,#3a3a3e);border-radius:50%;',
    '  background:var(--bg-panel,#111113);color:var(--text-dim,#a1a1aa);',
    '  cursor:pointer;box-shadow:0 10px 30px rgba(0,0,0,.5);',
    '  transition:opacity .3s ease,transform .3s ease,border-color .2s ease,color .2s ease}',
    '#lr-haut:hover{border-color:var(--amber-dim,#b45309);color:var(--amber,#f59e0b)}',
    '#lr-haut svg{display:block}',
    /* Retiree du parcours au clavier tant qu'elle est invisible. */
    '#lr-haut[hidden]{display:grid;opacity:0;transform:translateY(10px);',
    '  pointer-events:none;visibility:hidden}',
    '@media (max-width:720px){#lr-haut{width:34px;height:34px;',
    '  right:calc(14px + env(safe-area-inset-right));',
    '  bottom:calc(14px + env(safe-area-inset-bottom))}}',
    '@media (prefers-reduced-motion:reduce){#lr-haut{transition:none}}'
  ].join('');

  function remonter() {
    window.scrollTo({ top: 0, behavior: LENT ? 'auto' : 'smooth' });
    // Le clavier suit le regard: sans ca, la tabulation suivante repartait du
    // bas de page alors que l'ecran est revenu en haut.
    var premier = document.querySelector('.topbar a, header a, h1');
    if (!premier) return;
    if (!premier.hasAttribute('tabindex') && !/^(A|BUTTON)$/.test(premier.tagName)) {
      premier.setAttribute('tabindex', '-1');
    }
    premier.focus({ preventScroll: true });
  }

  function installer() {
    if (document.getElementById('lr-haut')) return;

    var f = document.createElement('style');
    f.textContent = STYLE;
    document.head.appendChild(f);

    var b = document.createElement('button');
    b.id = 'lr-haut';
    b.type = 'button';
    b.hidden = true;
    b.setAttribute('aria-label', 'Back to top');
    b.innerHTML = '<svg width="15" height="15" viewBox="0 0 14 14" aria-hidden="true">' +
      '<path d="M7 11.5V2.8M3.2 6.4 7 2.5l3.8 3.9" fill="none" stroke="currentColor" ' +
      'stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>';

    b.addEventListener('click', remonter);
    document.body.appendChild(b);

    /* La barre du haut remonte elle aussi, comme la barre d'etat d'un telephone.
       L'ecoute est posee sur le document et non sur la barre: la navigation
       douce remplace le contenu, et un ecouteur pose sur la barre d'une vue
       serait perdu en changeant de page. Les liens et les boutons qu'elle
       contient gardent evidemment leur role. */
    document.addEventListener('click', function (e) {
      if (!e.target || !e.target.closest) return;
      if (!e.target.closest('.topbar')) return;
      if (e.target.closest('a, button, input, select, textarea, label, [role="button"]')) return;
      remonter();
    });

    // Un seul calcul par image affichee, quel que soit le nombre d'evenements
    // de defilement: sur un pave tactile ils arrivent par paquets.
    var prevu = false;
    function regarder() {
      if (prevu) return;
      prevu = true;
      requestAnimationFrame(function () {
        prevu = false;
        var visible = (window.scrollY || document.documentElement.scrollTop || 0) > SEUIL;
        if (visible === !b.hidden) return;
        b.hidden = !visible;
        // La page du wiki a deja un bouton dans ce coin. Il monte d'un cran
        // quand la fleche est la, plutot que de se superposer a elle.
        document.documentElement.classList.toggle('lr-haut-visible', visible);
      });
    }

    window.addEventListener('scroll', regarder, { passive: true });
    window.addEventListener('resize', regarder, { passive: true });
    regarder();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', installer);
  else installer();
})();
