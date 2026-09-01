/* Curseur abeille de l'agent.
 *
 * Le halo de pilotage installe deja un curseur virtuel dans un Shadow DOM et
 * le moteur deplace cet element avant chaque action. Cette option ne cree pas
 * une seconde trajectoire. Elle remplace seulement le dessin de la fleche par
 * l'abeille, ce qui conserve exactement les animations et les clics du moteur.
 *
 * Desactive par defaut et independant du compagnon libre.
 */
(function () {
  'use strict';
  if (window.__larucheCurseurAgentInstalle) return;
  window.__larucheCurseurAgentInstalle = true;

  var actif = false;
  var observes = new WeakMap();
  var sauvegardes = new WeakMap();

  var FLECHE = '<span class="ring"></span>' +
    '<svg width="36" height="36" viewBox="0 0 22 22"><path d="M2 2 L2 17 L6.5 12.7 L9.4 19 L12 17.8 L9.1 11.7 L15 11.5 Z" ' +
    'fill="#F5A623" stroke="#3a2a08" stroke-width="1"></path></svg>';

  var ABEILLE = '<span class="ring"></span>' +
    '<span class="abeille-position"><span class="bee">' +
      '<span class="bee--wings"></span>' +
      '<span class="bee--body"><span></span><span></span></span>' +
      '<span class="bee--head">' +
        '<span class="bee--head-eyes"></span>' +
        '<span class="bee--head-antennas"></span>' +
      '</span>' +
    '</span></span>';

  var CSS = `
    .cursor.abeille-agent {
      width: 1px; height: 1px; filter: none; overflow: visible;
    }
    .cursor.abeille-agent .abeille-position {
      display: block; position: absolute; left: -25px; top: -16px;
      width: 51px; height: 33px; transform-origin: 25px 16px;
      transition: transform .12s ease;
    }
    .cursor.abeille-agent.vers-gauche .abeille-position { transform: scaleX(-1); }
    .cursor.abeille-agent .bee {
      display: block; width: 51px; height: 33px; position: absolute; left: 0; top: 0;
      filter: drop-shadow(0 2px 4px rgba(0,0,0,.65)) drop-shadow(0 0 4px rgba(234,179,8,.45));
      animation: lr-bee-float .8s infinite ease-in-out;
    }
    .cursor.abeille-agent .bee > *,
    .cursor.abeille-agent .bee--body > span { display: block; }
    .cursor.abeille-agent .bee--wings {
      width: 15px; height: 15px; background: #fff; opacity: .8; position: absolute;
      top: -6px; left: 9px; z-index: 99; border-radius: 50%;
      animation: lr-bee-fly-lw .15s infinite;
    }
    .cursor.abeille-agent .bee--wings::before {
      content: ''; position: absolute; width: 15px; height: 15px; background: #fff;
      opacity: .8; border-radius: 50%; top: 1px; left: 17px;
      animation: lr-bee-fly-rw .15s infinite;
    }
    .cursor.abeille-agent .bee--body { position: absolute; top: 0; }
    .cursor.abeille-agent .bee--body span {
      position: absolute; border-radius: 50%; width: 15px; height: 15px; background: #000;
    }
    .cursor.abeille-agent .bee--body span:first-child::after {
      content: ''; position: absolute; left: 5px; border-radius: 50%;
      width: 15px; height: 15px; background: #eab308;
    }
    .cursor.abeille-agent .bee--body span:last-child { left: 9px; }
    .cursor.abeille-agent .bee--body span:last-child::after {
      content: ''; position: absolute; left: 6px; border-radius: 50%;
      width: 15px; height: 15px; background: #eab308;
    }
    .cursor.abeille-agent .bee--head {
      width: 32px; height: 32px; background: #000; border-radius: 50%;
      position: relative; margin-left: 20px; top: -1px; z-index: 99;
    }
    .cursor.abeille-agent .bee--head-eyes {
      background: #fff; width: 5px; height: 3px; position: absolute; top: 8px; left: 12px;
      border-bottom-left-radius: 100%; border-bottom-right-radius: 100%;
    }
    .cursor.abeille-agent .bee--head-eyes::before {
      content: ''; position: absolute; top: 0; left: 8px; width: 5px; height: 3px;
      background: #fff; border-bottom-left-radius: 100%; border-bottom-right-radius: 100%;
    }
    .cursor.abeille-agent .bee--head-antennas {
      width: 3px; height: 8px; background: #000; position: absolute; left: 3px; top: -4px;
      border-radius: 30px; transform: rotate(-50deg); animation: lr-bee-la .3s infinite;
    }
    .cursor.abeille-agent .bee--head-antennas::before {
      content: ''; position: absolute; left: 5px; top: 1px; width: 3px; height: 10px;
      background: #000; border-radius: 30px; transform: rotate(-180deg);
      animation: lr-bee-ra .5s infinite;
    }
    .cursor.abeille-agent .ring {
      left: -30px; top: -30px; width: 60px; height: 60px;
      border-color: rgba(245,166,35,.95);
    }
    .cursor.abeille-agent.down .bee { animation: lr-bee-clic .38s ease-out, lr-bee-float .8s infinite ease-in-out .38s; }
    @keyframes lr-bee-float { 0%,100%{transform:translateY(0)} 50%{transform:translateY(-3px)} }
    @keyframes lr-bee-clic { 0%{transform:scale(1)} 35%{transform:scale(.78)} 100%{transform:scale(1)} }
    @keyframes lr-bee-fly-lw { 0%,100%{top:-6px;left:9px} 25%{top:-9px} 50%{top:-12px;left:11px} 75%{top:-9px} }
    @keyframes lr-bee-fly-rw { 0%,100%{top:1px;left:17px} 25%{top:-1px} 50%{left:14px;top:-2px} 75%{top:-1px} }
    @keyframes lr-bee-la { 0%,100%{transform:rotate(-50deg);left:3px} 50%{transform:rotate(-15deg);left:5px} }
    @keyframes lr-bee-ra { 0%,100%{transform:rotate(-170deg);left:5px} 50%{transform:rotate(-165deg);left:4px} }
    @media (prefers-reduced-motion: reduce) {
      .cursor.abeille-agent .bee,
      .cursor.abeille-agent .bee * { animation: none !important; }
    }
  `;

  function coordonneeX(transform) {
    var m = String(transform || '').match(/translate\(\s*(-?[\d.]+)px/i);
    return m ? parseFloat(m[1]) : null;
  }

  function suivreDirection(cur) {
    if (observes.has(cur)) return;
    var precedent = coordonneeX(cur.style.transform);
    var observateur = new MutationObserver(function () {
      var suivant = coordonneeX(cur.style.transform);
      if (suivant === null) return;
      if (precedent !== null && Math.abs(suivant - precedent) > 1) {
        cur.classList.toggle('vers-gauche', suivant < precedent);
      }
      precedent = suivant;
    });
    observateur.observe(cur, { attributes: true, attributeFilter: ['style'] });
    observes.set(cur, observateur);
  }

  function transformer() {
    var hote = document.getElementById('__laruche_glow__');
    var racine = hote && hote.shadowRoot;
    var cur = racine && racine.getElementById('lr-cur');
    if (!cur) return false;

    if (actif) {
      if (!sauvegardes.has(cur)) sauvegardes.set(cur, cur.innerHTML || FLECHE);
      var style = racine.getElementById('lr-cur-abeille-style');
      if (!style) {
        style = document.createElement('style');
        style.id = 'lr-cur-abeille-style';
        style.textContent = CSS;
        racine.appendChild(style);
      }
      if (!cur.classList.contains('abeille-agent')) {
        cur.innerHTML = ABEILLE;
        cur.classList.add('abeille-agent');
      }
      suivreDirection(cur);
    } else if (cur.classList.contains('abeille-agent')) {
      var observateur = observes.get(cur);
      if (observateur) observateur.disconnect();
      observes.delete(cur);
      cur.innerHTML = sauvegardes.get(cur) || FLECHE;
      cur.classList.remove('abeille-agent', 'vers-gauche');
      var ancienStyle = racine.getElementById('lr-cur-abeille-style');
      if (ancienStyle) ancienStyle.remove();
    }
    return true;
  }

  function appliquer(valeur) {
    actif = !!valeur;
    transformer();
  }

  var mutations = new MutationObserver(function (liste) {
    if (!actif) return;
    for (var i = 0; i < liste.length; i += 1) {
      for (var j = 0; j < liste[i].addedNodes.length; j += 1) {
        if (liste[i].addedNodes[j].id === '__laruche_glow__') {
          transformer();
          return;
        }
      }
    }
  });

  if (document.body) mutations.observe(document.body, { childList: true });
  else mutations.observe(document.documentElement, { childList: true, subtree: true });

  try {
    chrome.storage.local.get({ curseurAgent: false }, function (r) {
      appliquer(r && r.curseurAgent);
    });
    chrome.storage.onChanged.addListener(function (changements, zone) {
      if (zone === 'local' && changements.curseurAgent !== undefined) {
        appliquer(!!changements.curseurAgent.newValue);
      }
    });
  } catch (e) {}
})();
