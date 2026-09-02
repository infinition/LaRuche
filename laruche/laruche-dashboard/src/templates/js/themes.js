/* ==========================================================================
   Les themes de l'interface.

   Un theme n'est qu'un jeu de valeurs pour les jetons CSS de `:root`. Les
   integres vivent dans `app.css`, en blocs `html[data-theme="..."]`; ceux que
   l'utilisateur fabrique vivent en JSON dans `<foyer>/themes/`, servis par
   `/api/themes`, et s'appliquent en valeurs en ligne sur `documentElement`.

   Deux consequences utiles. Le catalogue est declaratif, donc l'editeur des
   reglages se genere tout seul et un jeton ajoute demain apparait sans qu'on y
   touche. Et appliquer un theme se resume a poser un attribut ou une poignee de
   proprietes, ce qui rend l'apercu au survol instantane et reversible.
   ========================================================================== */
(function () {
  'use strict';

  var CLE_ACTIF = 'laruche_theme';
  var CLE_CACHE = 'laruche_theme_jetons';
  var CLE_HABILLAGE = 'laruche_theme_habillage';

  /* Les jetons modifiables, groupes comme on les lit, pas comme ils sont
     declares. L'ordre ici EST l'ordre de l'editeur.

     `type` dit comment se REGLE un jeton, pas ce qu'il vaut. Une couleur veut une
     pipette et un curseur d'opacite, une taille veut un curseur borne, une police
     veut une liste de piles sures plus la saisie libre. Sans ce champ l'editeur
     offrait une pipette pour tout, ce qui laissait hors d'atteinte les dix-huit
     jetons qui ne sont pas des couleurs: les polices, les arrondis, la largeur du
     volet, les durees d'animation.

     Ne sont PAS exposes les quatre `--safe-*`: ils viennent de `env(safe-area-*)`,
     c'est-a-dire de l'encoche de l'appareil. Les regler a la main casserait
     l'affichage sur telephone sans rien apporter ailleurs. */
  var GROUPES = [
    {
      id: 'fond', titre: { fr: 'Fonds', en: 'Backgrounds' }, jetons: [
        { cle: '--bg', fr: 'Fond général', en: 'Base background' },
        { cle: '--bg-panel', fr: 'Panneaux', en: 'Panels' },
        { cle: '--bg-card', fr: 'Cartes', en: 'Cards' },
        { cle: '--bg-input', fr: 'Champs de saisie', en: 'Inputs' },
        { cle: '--bg-hover', fr: 'Survol', en: 'Hover' }
      ]
    },
    {
      id: 'texte', titre: { fr: 'Textes', en: 'Text' }, jetons: [
        { cle: '--text', fr: 'Texte principal', en: 'Primary text' },
        { cle: '--text-dim', fr: 'Texte atténué', en: 'Dimmed text' },
        { cle: '--text-mid', fr: 'Texte intermédiaire', en: 'Mid text' },
        { cle: '--text-muted', fr: 'Texte discret', en: 'Muted text' }
      ]
    },
    {
      id: 'accent', titre: { fr: 'Accent', en: 'Accent' }, jetons: [
        { cle: '--amber', fr: 'Accent', en: 'Accent' },
        { cle: '--amber-light', fr: 'Accent clair', en: 'Light accent' },
        { cle: '--amber-dim', fr: 'Accent sombre', en: 'Dark accent' },
        { cle: '--amber-glow', fr: 'Halo de l’accent', en: 'Accent glow' }
      ]
    },
    {
      id: 'bordure', titre: { fr: 'Bordures', en: 'Borders' }, jetons: [
        { cle: '--border', fr: 'Bordure', en: 'Border' },
        { cle: '--border-light', fr: 'Bordure marquée', en: 'Strong border' }
      ]
    },
    {
      id: 'etats', titre: { fr: 'États', en: 'States' }, jetons: [
        { cle: '--green', fr: 'Succès', en: 'Success' },
        { cle: '--green-dim', fr: 'Succès, fond', en: 'Success, surface' },
        { cle: '--red', fr: 'Erreur', en: 'Error' },
        { cle: '--red-dim', fr: 'Erreur, fond', en: 'Error, surface' },
        { cle: '--blue', fr: 'Information', en: 'Info' },
        { cle: '--blue-dim', fr: 'Information, fond', en: 'Info, surface' },
        { cle: '--purple', fr: 'Violet', en: 'Purple' },
        { cle: '--purple-dim', fr: 'Violet, fond', en: 'Purple, surface' },
        { cle: '--cyan', fr: 'Cyan', en: 'Cyan' },
        { cle: '--cyan-dim', fr: 'Cyan, fond', en: 'Cyan, surface' }
      ]
    },
    {
      id: 'typo', titre: { fr: 'Typographie', en: 'Typography' }, jetons: [
        { cle: '--font', fr: 'Police de l’interface', en: 'Interface typeface', type: 'police' },
        { cle: '--mono', fr: 'Police à chasse fixe', en: 'Monospace typeface', type: 'police', mono: true }
      ]
    },
    {
      id: 'formes', titre: { fr: 'Formes et mouvement', en: 'Shapes and motion' }, jetons: [
        { cle: '--radius', fr: 'Arrondi général', en: 'General rounding', type: 'taille', min: 0, max: 24, pas: 1, unite: 'px' },
        { cle: '--radius-msg', fr: 'Arrondi des messages', en: 'Message rounding', type: 'taille', min: 0, max: 28, pas: 1, unite: 'px' },
        { cle: '--sidebar-width', fr: 'Largeur du volet', en: 'Sidebar width', type: 'taille', min: 180, max: 460, pas: 5, unite: 'px' },
        { cle: '--hex-size', fr: 'Taille de la jauge', en: 'Gauge size', type: 'taille', min: 28, max: 120, pas: 2, unite: 'px' },
        { cle: '--transition-fast', fr: 'Animation courte', en: 'Fast transition', type: 'taille', min: 0, max: 0.6, pas: 0.05, unite: 's' },
        { cle: '--transition-med', fr: 'Animation longue', en: 'Slow transition', type: 'taille', min: 0, max: 1.2, pas: 0.05, unite: 's' }
      ]
    }
  ];

  /* Des piles SYSTEME, pas des polices telechargees. LaRuche fonctionne hors
     ligne et sa CSP interdit les feuilles distantes: pointer Google Fonts
     donnerait une interface qui change d'allure selon la connexion. Chaque pile
     se termine par une famille generique, donc elle rend toujours quelque chose.
     La saisie reste libre a cote: qui a une police installee la nomme. */
  var PILES = [
    { nom: 'Système', v: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" },
    { nom: 'Segoe UI', v: "'Segoe UI', system-ui, sans-serif" },
    { nom: 'Inter', v: "Inter, 'Segoe UI', system-ui, sans-serif" },
    { nom: 'Humaniste', v: "Optima, Candara, 'Gill Sans', sans-serif" },
    { nom: 'Grotesque', v: "'Helvetica Neue', Arial, sans-serif" },
    { nom: 'Serif', v: "Georgia, 'Times New Roman', serif" },
    { nom: 'Serif moderne', v: "Cambria, Constantia, Georgia, serif" },
    { nom: 'Ronde', v: "'Comic Sans MS', 'Segoe Print', cursive" }
  ];
  var PILES_MONO = [
    { nom: 'Système', v: "ui-monospace, 'Cascadia Mono', Consolas, monospace" },
    { nom: 'Consolas', v: "Consolas, 'Courier New', monospace" },
    { nom: 'Cascadia', v: "'Cascadia Code', 'Cascadia Mono', monospace" },
    { nom: 'JetBrains', v: "'JetBrains Mono', Consolas, monospace" },
    { nom: 'Courier', v: "'Courier New', Courier, monospace" }
  ];

  /* Les zones ou l'image de fond peut apparaitre. Chacune se coupe seule: une
     photo derriere la conversation est agreable, la meme derriere la barre
     d'outils rend les icones illisibles. */
  var ZONES = [
    { cle: 'app', fr: 'Zone centrale', en: 'Main area' },
    { cle: 'gauche', fr: 'Volet gauche', en: 'Left panel' },
    { cle: 'droite', fr: 'Volet droit', en: 'Right panel' },
    { cle: 'haut', fr: 'Barre du haut', en: 'Top bar' },
    { cle: 'bas', fr: 'Barre du bas', en: 'Bottom bar' }
  ];

  var TOUS = GROUPES.reduce(function (acc, g) {
    return acc.concat(g.jetons.map(function (j) { return j.cle; }));
  }, []);

  function descripteur(cle) {
    for (var i = 0; i < GROUPES.length; i++) {
      var js = GROUPES[i].jetons;
      for (var k = 0; k < js.length; k++) if (js[k].cle === cle) return js[k];
    }
    return null;
  }

  /* Les integres. `defaut` n'a pas de bloc CSS: c'est `:root`, donc y revenir
     consiste a retirer l'attribut et les valeurs en ligne. */
  var INTEGRES = [
    { id: 'defaut', nom: { fr: 'LaRuche', en: 'LaRuche' }, fond: '#09090b', point: '#f59e0b' },
    { id: 'ardoise', nom: { fr: 'Ardoise', en: 'Slate' }, fond: '#0b0d10', point: '#7dd3fc' },
    { id: 'foret', nom: { fr: 'Forêt', en: 'Forest' }, fond: '#0a0f0d', point: '#6ee7b7' },
    { id: 'nuit', nom: { fr: 'Nuit', en: 'Night' }, fond: '#000000', point: '#fbbf24' },
    { id: 'papier', nom: { fr: 'Papier', en: 'Paper' }, fond: '#faf7f2', point: '#b45309' }
  ];

  /* Le BROUILLON: ce qui est en cours d'edition pour le theme actif.

     Il manquait, et c'est ce qui faisait disparaitre une image de fond a peine
     posee. `apercuFin()` repeint le theme actif quand la souris quitte une
     vignette, et il le repeignait depuis le fichier ENREGISTRE: tout ce qui
     n'avait pas encore ete sauve etait efface par un simple survol. Le meme
     effacement se produisait a chaque repeinture, d'ou l'impression que le fond
     tenait tant qu'on ne touchait a rien.

     Desormais peindre le theme actif peint le brouillon quand il y en a un. Un
     apercu de theme voisin reste un apercu: il peint l'autre theme, et le retour
     rend le brouillon intact. */
  var etat = { actif: 'defaut', perso: [], brouillon: null };

  function definirBrouillon(b) { etat.brouillon = b; }
  function brouillonCourant() { return etat.brouillon; }

  function lang() {
    return (window.LaRuche && LaRuche.i18n && LaRuche.i18n.get) ? LaRuche.i18n.get() : 'fr';
  }
  function nomDe(t) {
    if (typeof t.nom === 'string') return t.nom;
    return t.nom[lang()] || t.nom.fr;
  }
  function estPerso(id) { return typeof id === 'string' && id.indexOf('perso:') === 0; }
  function clePerso(id) { return estPerso(id) ? id.slice('perso:'.length) : id; }
  function persoParId(id) {
    var k = clePerso(id);
    for (var i = 0; i < etat.perso.length; i++) if (etat.perso[i].id === k) return etat.perso[i];
    return null;
  }

  /* ----------------------------------------------------------------------
     Couleurs: lire une valeur CSS quelconque, la rendre en hex + opacite.

     `<input type="color">` ne connait que `#rrggbb`. Une palette qui contient
     des `rgba(24,24,27,.85)` lui etait donc illisible, et l'editeur desactivait
     la pipette sur ces jetons: les fonds de cartes, les champs de saisie, le
     halo de l'accent, c'est-a-dire precisement ceux dont la transparence fait
     tout l'effet. Le couple pipette + curseur les rend enfin reglables.
     ---------------------------------------------------------------------- */

  /* La valeur telle que le navigateur la comprend, quelle que soit sa syntaxe
     d'origine (nom, hex court, hsl, rgba). On delegue au moteur plutot que
     d'ecrire un analyseur: il connait deja toutes les formes. */
  function resoudreCouleur(css) {
    var d = document.createElement('span');
    d.style.color = '';
    d.style.color = css;
    if (!d.style.color) return null;
    d.style.display = 'none';
    document.body.appendChild(d);
    var calc = getComputedStyle(d).color;
    d.parentNode.removeChild(d);
    var m = calc.match(/rgba?\(([^)]+)\)/);
    if (!m) return null;
    var p = m[1].split(',').map(function (x) { return parseFloat(x.trim()); });
    if (p.length < 3 || p.some(isNaN)) return null;
    return { r: p[0] | 0, v: p[1] | 0, b: p[2] | 0, a: p.length > 3 ? p[3] : 1 };
  }

  function versHex(c) {
    if (!c) return null;
    function d(n) { return ('0' + Math.max(0, Math.min(255, n)).toString(16)).slice(-2); }
    return '#' + d(c.r) + d(c.v) + d(c.b);
  }

  /* Recomposer. Opaque -> hex, plus court et plus lisible dans un fichier de
     theme relu a la main. Transparent -> rgba, seule forme que tout navigateur
     accepte partout (le hex a huit chiffres reste mal supporte en valeur de
     variable heritee sur d'anciens moteurs). */
  function composerCouleur(hex, alpha) {
    var c = resoudreCouleur(hex);
    if (!c) return hex;
    var a = Math.max(0, Math.min(1, alpha === undefined ? 1 : alpha));
    if (a >= 0.999) return versHex(c);
    return 'rgba(' + c.r + ',' + c.v + ',' + c.b + ',' + Math.round(a * 100) / 100 + ')';
  }

  /* Le triplet `r, g, b` sans enveloppe, pour un canvas qui compose ses propres
     alphas. C'est ce qui manquait a l'essaim de la page de connexion: il peignait
     un jaune ecrit en dur et restait dore sur un theme ardoise ou papier. */
  function tripletRgb(css, repli) {
    var c = resoudreCouleur(css);
    if (!c) return repli || '255, 205, 40';
    return c.r + ', ' + c.v + ', ' + c.b;
  }

  /* ----------------------------------------------------------------------
     L'image de fond.

     Une seule image, posee une fois dans une couche fixe derriere tout, et des
     zones qui decident de la laisser voir ou non. L'alternative, peindre l'image
     dans chaque zone, obligeait a repeter l'ajustement du cadrage cinq fois et
     donnait cinq morceaux d'image sans continuite entre eux.

     Une zone allumee devient transparente; le fond continue donc a travers elle,
     et le curseur d'opacite regle tout d'un seul geste.
     ---------------------------------------------------------------------- */
  function couche() {
    var d = document.getElementById('lr-fond');
    if (!d) {
      d = document.createElement('div');
      d.id = 'lr-fond';
      d.setAttribute('aria-hidden', 'true');
      // Premier enfant du body: derriere tout, sans toucher a l'ordre du reste.
      if (document.body) document.body.insertBefore(d, document.body.firstChild);
    }
    return d;
  }

  function peindreFond(fond) {
    var r = document.documentElement;
    fond = fond || {};
    var zones = fond.zones || {};
    ZONES.forEach(function (z) {
      if (fond.image && zones[z.cle]) r.setAttribute('data-fond-' + z.cle, '1');
      else r.removeAttribute('data-fond-' + z.cle);
    });
    if (!document.body) return;          // pre-peinture: la couche viendra apres
    var d = couche();
    if (!fond.image) { d.style.display = 'none'; d.style.backgroundImage = ''; return; }
    d.style.display = '';
    d.style.backgroundImage = 'url("' + String(fond.image).replace(/"/g, '%22') + '")';
    d.style.opacity = String(fond.opacite === undefined ? 0.35 : fond.opacite);
    d.style.backgroundSize = fond.cadrage || 'cover';
  }

  /* ----------------------------------------------------------------------
     La marque: le nom et le logo en haut a gauche.

     Le logo accepte un SVG ou une image encodee. Il arrive LAVE du serveur, qui
     est la seule frontiere qui compte: un fichier de theme se partage, et un SVG
     est du code. Laver ici en plus ne rajouterait qu'une illusion de securite.
     ---------------------------------------------------------------------- */
  function peindreMarque(marque) {
    marque = marque || {};
    var nom = document.querySelector('.header-brand');
    if (nom) {
      if (!nom.dataset.origine) nom.dataset.origine = nom.textContent;
      nom.textContent = marque.nom || nom.dataset.origine;
      nom.title = marque.nom || nom.dataset.origine;
    }
    var ruche = document.getElementById('statusHoneycomb');
    if (!ruche) return;
    var pose = document.getElementById('lr-logo');
    if (!marque.logo) {
      if (pose) pose.parentNode.removeChild(pose);
      ruche.style.display = '';
      return;
    }
    ruche.style.display = 'none';
    if (!pose) {
      pose = document.createElement('span');
      pose.id = 'lr-logo';
      pose.className = 'lr-logo';
      pose.onclick = function () {
        if (window.LaRuche && LaRuche.Chat && LaRuche.Chat.toggleSidebar) LaRuche.Chat.toggleSidebar();
      };
      ruche.parentNode.insertBefore(pose, ruche);
    }
    var l = String(marque.logo).trim();
    if (l.slice(0, 4) === 'data' || l.slice(0, 1) === '/') {
      pose.innerHTML = '';
      var img = document.createElement('img');
      img.src = l;
      img.alt = marque.nom || 'logo';
      pose.appendChild(img);
    } else {
      pose.innerHTML = l;               // SVG deja lave par le serveur
    }
  }

  function habillageDe(id) {
    if (!estPerso(id)) return { marque: {}, fond: {} };
    var t = persoParId(id) || {};
    return { marque: t.marque || {}, fond: t.fond || {} };
  }

  /* Peindre. Un integre pose l'attribut et retire toute valeur en ligne; un
     personnalise fait l'inverse. Les deux chemins nettoient ce que l'autre a
     laisse, sinon un theme perso survivrait au passage a un integre. */
  function peindre(id) {
    var r = document.documentElement;
    var br = (etat.brouillon && etat.brouillon.id === id) ? etat.brouillon : null;
    if (br) {
      // Un brouillon vaut pour tout: ses jetons se posent en ligne, meme sur un
      // integre, sinon editer une couleur d'un theme livre ne se verrait pas.
      if (!estPerso(id) && id !== 'defaut') r.setAttribute('data-theme', id);
      else r.removeAttribute('data-theme');
      TOUS.forEach(function (c) {
        if (br.jetons && br.jetons[c]) r.style.setProperty(c, br.jetons[c]);
        else r.style.removeProperty(c);
      });
      peindreFond(br.fond || {});
      peindreMarque(br.marque || {});
      return;
    }
    if (estPerso(id)) {
      var t = persoParId(id);
      var jetons = (t && t.jetons) || {};
      r.removeAttribute('data-theme');
      TOUS.forEach(function (c) {
        if (jetons[c]) r.style.setProperty(c, jetons[c]);
        else r.style.removeProperty(c);
      });
      try { localStorage.setItem(CLE_CACHE, JSON.stringify(jetons)); } catch (e) {}
    } else {
      TOUS.forEach(function (c) { r.style.removeProperty(c); });
      try { localStorage.removeItem(CLE_CACHE); } catch (e) {}
      if (id && id !== 'defaut') r.setAttribute('data-theme', id);
      else r.removeAttribute('data-theme');
    }
    // L'habillage suit le theme, et se REMET A ZERO quand le theme n'en porte
    // pas: sans cela le logo d'un theme perso survivait au retour sur un integre.
    var h = habillageDe(id);
    peindreFond(h.fond);
    peindreMarque(h.marque);
    try {
      if (estPerso(id)) localStorage.setItem(CLE_HABILLAGE, JSON.stringify(h));
      else localStorage.removeItem(CLE_HABILLAGE);
    } catch (e) {}
  }

  /* Appliquer pour de bon: on peint, on retient, et on annule tout apercu en
     cours. Sans cette annulation, quitter le survol repeindrait l'ancien
     par-dessus le choix qui vient d'etre fait. */
  function appliquer(id, opts) {
    opts = opts || {};
    apercu.attente = null; apercu.actif = null;
    clearTimeout(apercu.minuteur);
    etat.actif = id;
    peindre(id);
    if (opts.sansEnregistrer) return;
    try { localStorage.setItem(CLE_ACTIF, id); } catch (e) {}
    fetch('/api/themes/actif', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ actif: id })
    }).catch(function () {});
  }

  /* L'apercu au survol.
     Temporise: traverser la liste ne doit pas repeindre vingt fois. Et la sortie
     restaure le theme retenu, jamais celui d'avant l'apercu precedent. */
  var apercu = { minuteur: null, attente: null, actif: null, delai: 120 };

  function apercuSur(id) {
    if (apercu.actif === id) return;
    clearTimeout(apercu.minuteur);
    apercu.attente = id;
    apercu.minuteur = setTimeout(function () {
      if (apercu.attente !== id) return;
      apercu.actif = id;
      peindre(id);
    }, apercu.delai);
  }
  function apercuFin() {
    clearTimeout(apercu.minuteur);
    apercu.attente = null;
    if (!apercu.actif) return;
    apercu.actif = null;
    peindre(etat.actif);
  }

  /* La valeur effective d'un jeton, telle que le navigateur la calcule. C'est
     elle qu'on montre dans l'editeur: partir des valeurs du theme courant plutot
     que d'une liste ecrite en dur evite qu'elles divergent le jour ou la feuille
     de style change. */
  function jetonsCourants() {
    var st = getComputedStyle(document.documentElement);
    var out = {};
    TOUS.forEach(function (c) { out[c] = (st.getPropertyValue(c) || '').trim(); });
    return out;
  }

  async function charger() {
    try {
      var d = await fetch('/api/themes').then(function (r) { return r.json(); });
      etat.perso = (d && d.themes) || [];
    } catch (e) { etat.perso = []; }
    var local = null;
    try { local = localStorage.getItem(CLE_ACTIF); } catch (e) {}
    if (local) { etat.actif = local; }
    else {
      // Aucun cache: cet appareil decouvre la ruche, on prend son choix a elle.
      try {
        var a = await fetch('/api/themes/actif').then(function (r) { return r.json(); });
        if (a && a.actif) etat.actif = a.actif;
      } catch (e) {}
    }
    peindre(etat.actif);
  }

  /* La BASE d'un theme: les valeurs dont il est parti.

     C'est elle qui rend possible le petit bouton de retour a la valeur d'origine,
     jeton par jeton. La capturer a la creation est exact et ne coute rien; la
     recalculer plus tard obligerait a repeindre le theme parent pour le lire,
     donc a faire clignoter l'interface a chaque fois. */
  function baseCourante() {
    var st = getComputedStyle(document.documentElement);
    var out = {};
    TOUS.forEach(function (c) { out[c] = (st.getPropertyValue(c) || '').trim(); });
    return out;
  }

  /* Copier le theme actif sous un nouveau nom, et basculer dessus.

     C'est le seul chemin d'edition d'un theme livre, et c'est voulu: un integre
     vit dans la feuille de style de l'application, le reecrire demanderait de
     reinstaller pour revenir en arriere. Le garder intact rend au contraire la
     remise a zero gratuite, il suffit de le reselectionner. */
  async function dupliquer(nom, jetons, marque, fond) {
    var parent = etat.actif;
    var base = jetons ? Object.assign({}, jetons) : baseCourante();
    var r = await fetch('/api/themes', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: null, nom: nom, jetons: jetons || baseCourante(),
        marque: marque || {}, fond: fond || {},
        parent: estPerso(parent) ? clePerso(parent) : parent, base: base
      })
    }).then(function (x) { return x.json(); });
    if (r && r.status === 'ok') {
      await charger();
      etat.brouillon = null;
      appliquer('perso:' + r.theme.id);
    }
    return r;
  }

  async function enregistrer(nom, jetons, id, marque, fond) {
    var r = await fetch('/api/themes', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: id || null, nom: nom, jetons: jetons,
        marque: marque || {}, fond: fond || {}
      })
    }).then(function (x) { return x.json(); });
    if (r && r.status === 'ok') {
      await charger();
      appliquer('perso:' + r.theme.id);
    }
    return r;
  }

  async function supprimer(id) {
    await fetch('/api/themes/' + encodeURIComponent(clePerso(id)), { method: 'DELETE' })
      .catch(function () {});
    if (etat.actif === id) appliquer('defaut');
    await charger();
  }

  function catalogue() {
    return INTEGRES.map(function (t) {
      return { id: t.id, nom: nomDe(t), fond: t.fond, point: t.point, integre: true };
    }).concat(etat.perso.map(function (t) {
      var j = t.jetons || {};
      return {
        id: 'perso:' + t.id, nom: t.nom, integre: false,
        fond: j['--bg'] || '#09090b', point: j['--amber'] || '#f59e0b',
        image: (t.fond && t.fond.image) || '', marque: t.marque || {}
      };
    }));
  }

  /* Peinture AVANT le premier rendu, depuis le cache local seul: attendre une
     reponse du serveur ferait clignoter l'interface dans l'ancien theme. */
  (function prePeindre() {
    try {
      var id = localStorage.getItem(CLE_ACTIF);
      if (!id) return;
      if (id.indexOf('perso:') === 0) {
        var j = JSON.parse(localStorage.getItem(CLE_CACHE) || '{}');
        var r = document.documentElement;
        Object.keys(j).forEach(function (c) { r.style.setProperty(c, j[c]); });
      } else if (id !== 'defaut') {
        document.documentElement.setAttribute('data-theme', id);
      }
      // L'habillage vient du cache lui aussi. Les attributs de zone se posent
      // tout de suite; la couche d'image attend le body, qui n'existe pas encore
      // quand ce module s'execute.
      var h = JSON.parse(localStorage.getItem(CLE_HABILLAGE) || 'null');
      if (h) {
        peindreFond(h.fond);
        document.addEventListener('DOMContentLoaded', function () {
          peindreFond(h.fond);
          peindreMarque(h.marque);
        });
      }
    } catch (e) {}
  })();

  /* Le libelle vit dans boot.js, pas ici: ce module tourne AVANT core.js, pour
     peindre le theme sans clignotement, et LaRuche.i18n n'existe pas encore. */
  window.LaRuche = window.LaRuche || {};
  LaRuche.Themes = {
    GROUPES: GROUPES, TOUS: TOUS, ZONES: ZONES,
    PILES: PILES, PILES_MONO: PILES_MONO, descripteur: descripteur,
    resoudreCouleur: resoudreCouleur, versHex: versHex,
    composerCouleur: composerCouleur, tripletRgb: tripletRgb,
    peindreFond: peindreFond, peindreMarque: peindreMarque,
    habillageDe: habillageDe, dupliquer: dupliquer, baseCourante: baseCourante,
    definirBrouillon: definirBrouillon, brouillonCourant: brouillonCourant,
    baseDe: function (id) {
      var t = estPerso(id) ? persoParId(id) : null;
      return (t && t.base) || null;
    },
    charger: charger, appliquer: appliquer, peindre: peindre,
    apercuSur: apercuSur, apercuFin: apercuFin,
    jetonsCourants: jetonsCourants, catalogue: catalogue,
    enregistrer: enregistrer, supprimer: supprimer,
    actif: function () { return etat.actif; },
    estPerso: estPerso
  };
})();
