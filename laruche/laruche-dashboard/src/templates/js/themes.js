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
        /* Pas d'opacite sur celui-la: derriere la racine il n'y a pas de page,
           il y a la toile de l'HOTE. Rendre ce fond transparent ne revelait donc
           rien qui appartienne au logiciel, cela laissait voir le blanc du
           navigateur ou la couleur de fenetre de l'application, et le meme theme
           s'affichait pale d'un cote et sombre de l'autre. */
        { cle: '--bg', fr: 'Fond général', en: 'Base background', sansAlpha: true },
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
      id: 'markdown', titre: { fr: 'Texte mis en forme', en: 'Formatted text' }, jetons: [
        { cle: '--md-titre', fr: 'Titres', en: 'Headings' },
        { cle: '--md-lien', fr: 'Liens', en: 'Links' },
        { cle: '--md-gras', fr: 'Gras', en: 'Bold' },
        { cle: '--md-code', fr: 'Code en ligne', en: 'Inline code' },
        { cle: '--md-code-fond', fr: 'Code en ligne, fond', en: 'Inline code, surface' },
        { cle: '--md-citation', fr: 'Citation, barre', en: 'Quote, bar' },
        { cle: '--md-citation-fond', fr: 'Citation, fond', en: 'Quote, surface' }
      ]
    },
    {
      id: 'typo', titre: { fr: 'Typographie', en: 'Typography' }, jetons: [
        { cle: '--font', fr: 'Police de l’interface', en: 'Interface typeface', type: 'police' },
        { cle: '--font-msg', fr: 'Police des messages', en: 'Message typeface', type: 'police' },
        { cle: '--font-contenu', fr: 'Police du contenu (mémoire, flux)', en: 'Content typeface (memory, feed)', type: 'police' },
        { cle: '--mono', fr: 'Police à chasse fixe', en: 'Monospace typeface', type: 'police', mono: true },
        { cle: '--taille-ui', fr: 'Taille du texte, interface', en: 'Interface text size', type: 'taille', min: 10, max: 20, pas: 0.5, unite: 'px' },
        { cle: '--taille-msg', fr: 'Taille du texte, messages', en: 'Message text size', type: 'taille', min: 11, max: 24, pas: 0.5, unite: 'px' },
        { cle: '--taille-contenu', fr: 'Taille du texte, contenu', en: 'Content text size', type: 'taille', min: 10, max: 22, pas: 0.5, unite: 'px' },
        { cle: '--taille-h1', fr: 'Taille des titres, niveau 1', en: 'Heading size, level 1', type: 'taille', min: 1, max: 3, pas: 0.05, unite: 'em' },
        { cle: '--taille-h2', fr: 'Taille des titres, niveau 2', en: 'Heading size, level 2', type: 'taille', min: 1, max: 2.6, pas: 0.05, unite: 'em' },
        { cle: '--taille-h3', fr: 'Taille des titres, niveau 3', en: 'Heading size, level 3', type: 'taille', min: 1, max: 2.2, pas: 0.05, unite: 'em' }
      ]
    },
    {
      /* Le VERRE. Deux flous, et c'est la distinction qui compte.

         Le premier porte sur l'IMAGE de fond: une photo nette derriere du texte
         se lit comme un desordre, la flouter la rend a son role de fond. Le
         second porte sur les PANNEAUX: ils deviennent translucides et floutent ce
         qui passe dessous, ce qui donne la matiere du verre. Les melanger en un
         seul reglage obligerait a choisir entre une image lisible et des panneaux
         qui ont de la matiere. */
      id: 'verre', titre: { fr: 'Verre et flou', en: 'Glass and blur' }, jetons: [
        { cle: '--fond-flou', fr: 'Flou de l’image de fond', en: 'Background image blur', type: 'taille', min: 0, max: 40, pas: 1, unite: 'px' },
        { cle: '--verre-flou', fr: 'Flou des panneaux', en: 'Panel blur', type: 'taille', min: 0, max: 30, pas: 1, unite: 'px' },
        { cle: '--verre-opacite', fr: 'Opacité des panneaux', en: 'Panel opacity', type: 'taille', min: 0.15, max: 1, pas: 0.01, unite: '' },
        { cle: '--verre-bord', fr: 'Reflet du bord', en: 'Edge highlight' },
        { cle: '--msg-flou', fr: 'Flou des messages', en: 'Message blur', type: 'taille', min: 0, max: 30, pas: 1, unite: 'px' },
        { cle: '--msg-opacite', fr: 'Opacité des messages', en: 'Message opacity', type: 'taille', min: 0.05, max: 1, pas: 0.01, unite: '' },
        { cle: '--anim-flou', fr: 'Flou des animations', en: 'Animation blur', type: 'taille', min: 0, max: 30, pas: 1, unite: 'px' },
        { cle: '--anim-opacite', fr: 'Opacité des animations', en: 'Animation opacity', type: 'taille', min: 0.05, max: 1, pas: 0.01, unite: '' }
      ]
    },
    {
      id: 'formes', titre: { fr: 'Formes et mouvement', en: 'Shapes and motion' }, jetons: [
        { cle: '--radius-xs', fr: 'Arrondi des badges et pastilles', en: 'Badge rounding', type: 'taille', min: 0, max: 14, pas: 1, unite: 'px' },
        { cle: '--radius-btn', fr: 'Arrondi des boutons et champs', en: 'Button and field rounding', type: 'taille', min: 0, max: 20, pas: 1, unite: 'px' },
        { cle: '--radius', fr: 'Arrondi général', en: 'General rounding', type: 'taille', min: 0, max: 24, pas: 1, unite: 'px' },
        { cle: '--radius-card', fr: 'Arrondi des cartes et fenêtres', en: 'Card and window rounding', type: 'taille', min: 0, max: 32, pas: 1, unite: 'px' },
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
    { cle: 'bas', fr: 'Barre du bas', en: 'Bottom bar' },
    { cle: 'partage', fr: 'Volet du partage d’écran', en: 'Split-screen pane' }
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
    { id: 'papier', nom: { fr: 'Papier', en: 'Paper' }, fond: '#faf7f2', point: '#b45309' },
    { id: 'verre', nom: { fr: 'Verre', en: 'Glass' }, fond: '#0b0d12', point: '#e9b872' }
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

  /* Les icones remplacables, par EMPLACEMENT nomme.

     La page compte une cinquantaine de SVG en ligne, et le reste de
     l'application autant: les offrir un par un donnerait une liste que personne
     ne parcourt, ou l'on chercherait le chevron d'un repli entre deux fleches de
     pagination. Ce qu'on reconnait, ce sont les marques: les six onglets, la
     ruche, l'essaim, la couronne. Douze emplacements nommes couvrent donc ce que
     l'oeil identifie, et chacun vise un element stable par un selecteur.

     Une icone posee est un SVG lave par le serveur, comme le logo. Elle herite de
     `currentColor`, donc elle suit l'accent du theme sans etre reecrite. */
  var ICONES = [
    { cle: 'nav-chat', fr: 'Onglet Chat', en: 'Chat tab', sel: '.header-nav a[data-page="chat"] .tab-icon' },
    { cle: 'nav-memoire', fr: 'Onglet Mémoire', en: 'Memory tab', sel: '.header-nav a[data-page="memory"] .tab-icon' },
    { cle: 'nav-missions', fr: 'Onglet Missions', en: 'Missions tab', sel: '.header-nav a[data-page="automations"] .tab-icon' },
    { cle: 'nav-capacites', fr: 'Onglet Capacités', en: 'Capabilities tab', sel: '.header-nav a[data-page="capabilities"] .tab-icon' },
    { cle: 'nav-tableau', fr: 'Onglet Tableau de bord', en: 'Dashboard tab', sel: '.header-nav a[data-page="dashboard"] .tab-icon' },
    { cle: 'nav-reglages', fr: 'Onglet Paramètres', en: 'Settings tab', sel: '.header-nav a[data-page="settings"] .tab-icon' },

    // `echelle`: ces deux-la sont dessines a leur taille reelle, cent pixels ou
    // plus, et debordaient de l'apercu de trente pixels en recouvrant la ligne
    // voisine. L'apercu les reduit, sans toucher a leur taille dans l'interface.
    { cle: 'ruche', fr: 'La ruche animée (en haut à gauche)', en: 'The animated hive (top left)', sel: '#statusHoneycomb', echelle: 0.7 },
    { cle: 'essaim', fr: "L'essaim animé (accueil, connexion)", en: 'The animated swarm (welcome, login)', sel: '.swarm-wrap', echelle: 0.11 },
    { cle: 'reine', fr: 'LaReine (barre du bas)', en: 'LaReine (bottom bar)', sel: '#sbReineLabel' },
    { cle: 'muet', fr: 'Le muet (barre du bas)', en: 'Mute (bottom bar)', sel: '#sbMuteLabel' },
    { cle: 'partage', fr: "Le partage d'écran (barre du bas)", en: 'Split screen (bottom bar)', sel: '#sbSplitLabel' },

    { cle: 'messages', fr: 'Messages entre ruches', en: 'Messages between hives', sel: '#meshBtn' },
    { cle: 'feed', fr: "Le flux d'activité", en: 'Activity feed', sel: '#feedToggleBtn' },
    { cle: 'feed-epingle', fr: 'Épingler le flux', en: 'Pin the feed', sel: '#feedAnchorBtn' },
    { cle: 'feed-vider', fr: 'Vider le flux', en: 'Clear the feed', sel: '#feedClearBtn' },
    { cle: 'feed-filtres', fr: 'Filtres du flux', en: 'Feed filters', sel: '#feedFiltersHead' },
    { cle: 'feed-envoyer', fr: 'Envoyer au flux', en: 'Send to the feed', sel: '#feedAskSend' },

    { cle: 'chat-descendre', fr: 'Revenir en bas du chat', en: 'Jump to the latest message', sel: '#chatJumpBtn' },
    { cle: 'chat-micro', fr: 'Le microphone', en: 'Microphone', sel: '#micBtn' },
    { cle: 'chat-appel', fr: "Le mode d'appel vocal", en: 'Voice call mode', sel: '#voiceModeBtn' },
    { cle: 'chat-eveil', fr: "Le mot d'éveil", en: 'Wake word', sel: '#wakeWordBtn' },
    { cle: 'chat-pieces', fr: 'Les pièces jointes', en: 'Attachments', sel: '#trayToggleBtn' },
    { cle: 'chat-dossier', fr: 'Le dossier de travail', en: 'Working folder', sel: '#cwdToggleBtn' },

    // Les personnages: ils sont dessines en CSS, pas en SVG, mais leur conteneur
    // se remplace comme les autres.
    { cle: 'abeille', fr: "L'abeille (personnage)", en: 'The bee (character)', sel: '.bee:not(.bee-reine)', echelle: 0.4 },
    { cle: 'reine-perso', fr: 'LaReine (personnage)', en: 'LaReine (character)', sel: '.bee.bee-reine', echelle: 0.4 }
    //
    // `#stats-reset-zoom` et `#mem2GraphWrap` ont ete retires: le premier est un
    // bouton de TEXTE, le second un conteneur de graphe vide tant qu'on n'a pas
    // ouvert la memoire. Ni l'un ni l'autre n'est une icone, et les offrir
    // donnait un apercu illisible pour le premier, vide pour le second.
  ];

  /* Ce que contenait chaque emplacement avant qu'on y touche.

     Sans cette memoire, retirer une icone personnalisee laissait un trou: on sait
     poser, on ne saurait pas revenir. La capture se fait au premier remplacement,
     donc sur le contenu d'origine, et jamais sur celui d'un theme precedent. */
  var _iconesOrigine = {};

  /* D'autres modules declarent leurs propres emplacements.

     Les onglets des reglages sont dessines par `settings.js`, qui tient deja leur
     liste et leurs libelles traduits. Les recopier ici en aurait fait une seconde
     source a garder en phase, et le jour ou un onglet s'ajoute il aurait manque
     sans que rien ne le signale. */
  function ajouterIcones(liste) {
    (liste || []).forEach(function (it) {
      if (!it || !it.cle || !it.sel) return;
      for (var i = 0; i < ICONES.length; i++) if (ICONES[i].cle === it.cle) return;
      ICONES.push(it);
    });
  }

  /* Le contenu d'ORIGINE d'un emplacement, capture a la demande.

     Le panneau montrait un tiret tant qu'aucune icone personnelle n'etait posee,
     donc on choisissait a l'aveugle: rien ne disait ce qu'on allait remplacer.
     La capture est paresseuse et ne se fait qu'une fois, sur le contenu livre,
     jamais sur celui d'un theme precedent. */
  function iconeOrigine(cle) {
    if (cle in _iconesOrigine) return _iconesOrigine[cle];
    var it = null;
    for (var i = 0; i < ICONES.length; i++) if (ICONES[i].cle === cle) { it = ICONES[i]; break; }
    if (!it) return '';
    var cible = document.querySelector(it.sel);
    if (!cible) return '';
    _iconesOrigine[cle] = cible.innerHTML;
    return _iconesOrigine[cle];
  }

  function peindreIcones(icones, tailles) {
    icones = icones || {};
    tailles = tailles || {};
    ICONES.forEach(function (it) {
      var cible = document.querySelector(it.sel);
      if (!cible) return;
      if (!(it.cle in _iconesOrigine)) _iconesOrigine[it.cle] = cible.innerHTML;
      var v = icones[it.cle];
      if (v && String(v).trim()) {
        cible.innerHTML = String(v);
        cible.classList.add('lr-icone-perso');
      } else {
        cible.innerHTML = _iconesOrigine[it.cle];
        cible.classList.remove('lr-icone-perso');
      }
      /* La taille se pose sur le CONTENEUR, jamais sur le SVG.
         
         La regle `.tab-icon svg { width: 100% !important }` existe deja, et le
         conteneur porte une taille fixe de dix-huit pixels: agrandir le SVG ne
         pouvait donc rien donner, il remplissait un cadre qui, lui, ne bougeait
         pas. On dimensionne le cadre, et le SVG suit. `font-size` accompagne,
         pour les icones qui sont dimensionnees en `em` plutot qu'en pourcentage.
         
         `important` parce que ces regles-la en portent aussi, et parce qu'un
         reglage explicite de l'utilisateur doit gagner: c'est l'usage legitime
         de ce mot, et il n'y en a pas d'autre ici. */
      var t = tailles[it.cle];
      var props = ['width', 'height', 'min-width', 'min-height'];
      if (t) {
        props.forEach(function (p) { cible.style.setProperty(p, t + 'px', 'important'); });
        cible.style.setProperty('font-size', t + 'px', 'important');
        cible.setAttribute('data-lr-ic', '1');
      } else {
        props.forEach(function (p) { cible.style.removeProperty(p); });
        cible.style.removeProperty('font-size');
        cible.removeAttribute('data-lr-ic');
      }
    });
  }

  /* ----------------------------------------------------------------------
     La marque: le nom et le logo en haut a gauche.

     Le logo accepte un SVG ou une image encodee. Il arrive LAVE du serveur, qui
     est la seule frontiere qui compte: un fichier de theme se partage, et un SVG
     est du code. Laver ici en plus ne rajouterait qu'une illusion de securite.
     ---------------------------------------------------------------------- */
  function peindreMarque(marque) {
    marque = marque || {};
    /* La taille du logo appartient a la MARQUE, pas a la palette: un logo carre
       et un logo long n'occupent pas la meme place, et c'est une propriete de
       l'image qu'on pose, pas du theme qui l'entoure. */
    var r0 = document.documentElement;
    /* Le logo et les animations suivent-ils le theme, ou gardent-ils leurs
       couleurs? Un theme repeint tout ce qui puise dans son accent, y compris la
       marque, et c'est presque toujours ce qu'on veut. Presque: une marque est
       justement ce qui ne doit pas changer de couleur avec le decor. */
    if (marque.couleursOrigine) r0.setAttribute('data-marque-brute', '1');
    else r0.removeAttribute('data-marque-brute');
    if (marque.taille) r0.style.setProperty('--lr-logo-taille', marque.taille + 'px');
    else r0.style.removeProperty('--lr-logo-taille');
    var nom = document.querySelector('.header-brand');
    if (nom) {
      if (!nom.dataset.origine) nom.dataset.origine = nom.textContent;
      nom.textContent = marque.nom || nom.dataset.origine;
      nom.title = marque.nom || nom.dataset.origine;
      // Un logo se suffit souvent a lui-meme: pouvoir retirer le mot laisse la
      // barre respirer, et c'est le choix de celui qui pose sa marque.
      nom.style.display = marque.masquerNom ? 'none' : '';
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

  /* Les polices IMPORTEES, posees en `@font-face` dans une feuille a nous.

     Elles voyagent DANS le theme, encodees, et non a cote: un theme se partage en
     un fichier, et une police citee par son nom seul ne suivrait pas. Celui qui
     recoit le theme verrait alors le repli de la pile, sans comprendre pourquoi
     ce n'est pas ce qu'on lui a montre.

     Le nom declare ici devient une famille CSS ordinaire: elle apparait dans les
     listes deroulantes a cote des piles systeme, et se tape aussi a la main. */
  function peindrePolices(polices) {
    var f = document.getElementById('lr-polices');
    if (!f) {
      f = document.createElement('style');
      f.id = 'lr-polices';
      document.head.appendChild(f);
    }
    f.textContent = (polices || []).map(function (p) {
      if (!p || !p.nom || !p.data) return '';
      // Le nom est entre guillemets et debarrasse des siens: il vient de
      // l'utilisateur et atterrit dans une feuille de style.
      var nom = String(p.nom).replace(/["\\]/g, '').trim();
      if (!nom) return '';
      return '@font-face{font-family:"' + nom + '";src:url(' + JSON.stringify(String(p.data)) +
             ');font-display:swap;}';
    }).join('\n');
  }

  function policesDe(id) {
    if (!estPerso(id)) return [];
    var t = persoParId(id) || {};
    return t.polices || [];
  }

  function habillageDe(id) {
    if (!estPerso(id)) return { marque: {}, fond: {}, icones: {}, polices: [] };
    var t = persoParId(id) || {};
    return {
      marque: t.marque || {}, fond: t.fond || {},
      icones: t.icones || {}, polices: t.polices || [],
      taillesIcones: t.taillesIcones || {}
    };
  }

  /* Peindre. Un integre pose l'attribut et retire toute valeur en ligne; un
     personnalise fait l'inverse. Les deux chemins nettoient ce que l'autre a
     laisse, sinon un theme perso survivrait au passage a un integre. */
  /* Les COMPOSANTES se deduisent de la couleur, elles ne se reglent pas.

     Tout ce qui est translucide dans la feuille de style s'ecrit
     `rgba(var(--amber-rgb), .12)`: il faut donc que `--amber-rgb` existe. Les
     themes livres le declarent, un theme fabrique par l'utilisateur ne declare
     que `--amber`. Sans cette derivation, ses bandeaux et ses survols garderaient
     l'ambre par defaut alors que son accent est ailleurs, ce qui est exactement
     le defaut qu'on vient de corriger. On les calcule donc apres chaque
     peinture, pour tous les themes, integres compris: la valeur derivee est
     toujours celle de la couleur effective. */
  var COMPOSANTES = [
    ['--amber', '--amber-rgb'], ['--red', '--red-rgb'], ['--green', '--green-rgb'],
    ['--blue', '--blue-rgb'], ['--purple', '--purple-rgb'], ['--cyan', '--cyan-rgb'],
    ['--border', '--border-rgb'], ['--bg', '--bg-rgb'],
    ['--bg-panel', '--bg-panel-rgb'], ['--bg-card', '--bg-card-rgb']
  ];

  function deriverComposantes() {
    var r = document.documentElement;
    var st = getComputedStyle(r);
    /* Le verre s'allume tout seul, a partir de son propre reglage. Une case a
       cocher de plus dirait la meme chose que « flou a zero », et permettrait de
       les contredire: verre coche, flou nul. */
    var flou = parseFloat(st.getPropertyValue('--verre-flou')) || 0;
    if (flou > 0) r.setAttribute('data-verre', '1');
    else r.removeAttribute('data-verre');
    COMPOSANTES.forEach(function (paire) {
      var v = (st.getPropertyValue(paire[0]) || '').trim();
      if (!v) return;
      var c = resoudreCouleur(v);
      if (c) r.style.setProperty(paire[1], c.r + ',' + c.v + ',' + c.b);
    });
  }

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
      deriverComposantes();
      peindrePolices(br.polices || []);
      peindreFond(br.fond || {});
      peindreMarque(br.marque || {});
      peindreIcones(br.icones || {}, br.taillesIcones || {});
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
    deriverComposantes();
    var h = habillageDe(id);
    // Les polices d'abord: une famille absente au moment ou le jeton se pose
    // ferait clignoter le texte dans son repli avant de se corriger.
    peindrePolices(h.polices);
    peindreFond(h.fond);
    peindreMarque(h.marque);
    peindreIcones(h.icones, h.taillesIcones);
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
    // Le brouillon appartenait au theme qu'on quitte. Le garder ferait peindre
    // ses valeurs par-dessus le nouveau, et pire, le ferait enregistrer dedans.
    if (etat.brouillon && etat.brouillon.id !== id) etat.brouillon = null;
    etat.actif = id;
    peindre(id);
    /* Le theme a change: que tout ce qui l'affiche se remette a jour.
       Le menu de la barre du haut et les vignettes de l'onglet Apparence
       montraient chacun leur propre idee du theme actif, celle du moment ou ils
       avaient ete dessines. Choisir dans l'un laissait donc l'autre en arriere. */
    try {
      document.dispatchEvent(new CustomEvent('laruche:theme', { detail: { actif: id } }));
    } catch (e) {}
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
    /* Le NOEUD fait foi, le cache local ne sert qu'a peindre sans clignoter.

       C'etait l'inverse: le cache gagnait, et le serveur n'etait consulte que
       s'il n'y avait pas de cache. Un theme choisi dans l'application de bureau
       n'atteignait donc jamais le meme navigateur ouvert a cote, qui gardait le
       sien et paraissait ignorer le changement. La ruche est une seule ruche;
       ses fenetres doivent montrer la meme chose.

       On garde la peinture immediate depuis le cache, faite avant meme ce
       module, puis on adopte la reponse du noeud des qu'elle arrive. */
    try { etat.actif = localStorage.getItem(CLE_ACTIF) || etat.actif; } catch (e) {}
    try {
      var a = await fetch('/api/themes/actif').then(function (r) { return r.json(); });
      if (a && a.actif) {
        etat.actif = a.actif;
        try { localStorage.setItem(CLE_ACTIF, a.actif); } catch (e) {}
      }
    } catch (e) {
      // Noeud injoignable: le cache local reste le meilleur choix disponible.
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
  async function dupliquer(nom, jetons, marque, fond, icones, polices, taillesIcones) {
    var parent = etat.actif;
    var base = jetons ? Object.assign({}, jetons) : baseCourante();
    var r = await envoyer({
      id: null, nom: nom, jetons: jetons || baseCourante(),
      marque: marque || {}, fond: fond || {}, icones: icones || {}, polices: polices || [],
      taillesIcones: taillesIcones || {},
      parent: estPerso(parent) ? clePerso(parent) : parent, base: base
    });
    if (r && r.status === 'ok') {
      await charger();
      etat.brouillon = null;
      appliquer('perso:' + r.theme.id);
    }
    return r;
  }

  /* Un echec RENVOIE un echec, il ne le jette pas.
     Un corps trop lourd revenait en 413 sans JSON: `x.json()` levait, la promesse
     etait rejetee, et l'appelant qui affichait "enregistrement..." n'etait jamais
     rappele. Le panneau restait donc bloque sur ce mot, en repetant que rien
     n'etait enregistre, ce qui etait vrai et invisible a la fois. */
  async function envoyer(corps) {
    try {
      var rep = await fetch('/api/themes', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(corps)
      });
      if (!rep.ok) {
        return { status: 'error', error: rep.status === 413 ? 'theme trop lourd' : ('HTTP ' + rep.status) };
      }
      return await rep.json();
    } catch (e) {
      return { status: 'error', error: String((e && e.message) || e) };
    }
  }

  async function enregistrer(nom, jetons, id, marque, fond, icones, polices, taillesIcones) {
    // `undefined` disparait a la serialisation: c'est exactement ce qu'on veut,
    // le serveur garde alors ce qui est deja sur le disque.
    var r = await envoyer({
      id: id || null, nom: nom, jetons: jetons,
      marque: marque || {}, fond: fond || {}, icones: icones, polices: polices,
      taillesIcones: taillesIcones
    });
    if (r && r.status === 'ok') {
      await charger();
      // Basculer dessus n'a de sens qu'a la CREATION. Le faire aussi quand on
      // met a jour un theme deja actif ramenait dessus une seconde apres qu'on
      // en ait choisi un autre: le temps de l'enregistrement automatique, et le
      // choix etait defait sans que rien ne le dise.
      if (!id) appliquer('perso:' + r.theme.id);
    }
    return r;
  }

  /* Un theme est deja UN objet: jetons, marque, fond, icones, polices. L'exporter
     n'est donc que le rendre, et l'importer que le reposer. Il se partageait deja
     en copiant son fichier dans `<foyer>/themes/`; il manquait les deux gestes qui
     evitent d'avoir a savoir ou est ce dossier. */
  function exporter(id) {
    var t = estPerso(id) ? persoParId(id) : null;
    if (!t) {
      // Un integre n'a pas de fichier: on exporte ce qu'il DONNE, c'est-a-dire ses
      // valeurs calculees, ce qui en fait une copie qu'on pourra rejouer ailleurs.
      t = { id: id, nom: id, jetons: jetonsCourants(), marque: {}, fond: {}, icones: {}, polices: [] };
    }
    return {
      laruche_theme: 1,
      nom: t.nom, jetons: t.jetons || {},
      marque: t.marque || {}, fond: t.fond || {},
      icones: t.icones || {}, polices: t.polices || [],
      taillesIcones: t.taillesIcones || {}
    };
  }

  async function importer(objet) {
    if (!objet || typeof objet !== 'object' || !objet.jetons) {
      return { status: 'error', error: 'ce fichier n est pas un theme' };
    }
    var r = await envoyer({
      id: null, nom: String(objet.nom || 'Theme importe').slice(0, 60),
      jetons: objet.jetons, marque: objet.marque || {}, fond: objet.fond || {},
      icones: objet.icones || {}, polices: objet.polices || [],
      taillesIcones: objet.taillesIcones || {}, base: objet.jetons
    });
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

  /* Les couleurs qu'une vignette doit montrer. Pour un integre elles vivent dans
     la feuille de style, donc on les LIT en peignant l'attribut sur un element
     detache: plus fiable qu'une seconde table a tenir en phase a la main. */
  function apercuIntegre(id) {
    var d = document.createElement('div');
    d.setAttribute('data-theme', id);
    d.style.cssText = 'position:absolute;visibility:hidden;pointer-events:none';
    document.body.appendChild(d);
    var st = getComputedStyle(d);
    var lu = {
      panneau: st.getPropertyValue('--bg-panel').trim(),
      texte: st.getPropertyValue('--text').trim(),
      bordure: st.getPropertyValue('--border').trim()
    };
    d.parentNode.removeChild(d);
    return lu;
  }

  function catalogue() {
    return INTEGRES.map(function (t) {
      var a = document.body ? apercuIntegre(t.id) : {};
      return {
        id: t.id, nom: nomDe(t), fond: t.fond, point: t.point, integre: true,
        panneau: a.panneau, texte: a.texte, bordure: a.bordure
      };
    }).concat(etat.perso.map(function (t) {
      var j = t.jetons || {};
      return {
        id: 'perso:' + t.id, nom: t.nom, integre: false,
        fond: j['--bg'] || '#09090b', point: j['--amber'] || '#f59e0b',
        panneau: j['--bg-panel'] || '', texte: j['--text'] || '', bordure: j['--border'] || '',
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
    ICONES: ICONES, peindreIcones: peindreIcones, iconeOrigine: iconeOrigine,
    peindrePolices: peindrePolices, policesDe: policesDe,
    ajouterIcones: ajouterIcones,
    peindreFond: peindreFond, peindreMarque: peindreMarque,
    habillageDe: habillageDe, dupliquer: dupliquer, baseCourante: baseCourante,
    exporter: exporter, importer: importer,
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
