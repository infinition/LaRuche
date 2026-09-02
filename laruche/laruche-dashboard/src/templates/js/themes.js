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

  /* Les jetons modifiables, groupes comme on les lit, pas comme ils sont
     declares. L'ordre ici EST l'ordre de l'editeur. */
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
        { cle: '--red', fr: 'Erreur', en: 'Error' },
        { cle: '--blue', fr: 'Information', en: 'Info' },
        { cle: '--purple', fr: 'Violet', en: 'Purple' },
        { cle: '--cyan', fr: 'Cyan', en: 'Cyan' }
      ]
    }
  ];

  var TOUS = GROUPES.reduce(function (acc, g) {
    return acc.concat(g.jetons.map(function (j) { return j.cle; }));
  }, []);

  /* Les integres. `defaut` n'a pas de bloc CSS: c'est `:root`, donc y revenir
     consiste a retirer l'attribut et les valeurs en ligne. */
  var INTEGRES = [
    { id: 'defaut', nom: { fr: 'LaRuche', en: 'LaRuche' }, fond: '#09090b', point: '#f59e0b' },
    { id: 'ardoise', nom: { fr: 'Ardoise', en: 'Slate' }, fond: '#0b0d10', point: '#7dd3fc' },
    { id: 'foret', nom: { fr: 'Forêt', en: 'Forest' }, fond: '#0a0f0d', point: '#6ee7b7' },
    { id: 'nuit', nom: { fr: 'Nuit', en: 'Night' }, fond: '#000000', point: '#fbbf24' },
    { id: 'papier', nom: { fr: 'Papier', en: 'Paper' }, fond: '#faf7f2', point: '#b45309' }
  ];

  var etat = { actif: 'defaut', perso: [] };

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

  /* Peindre. Un integre pose l'attribut et retire toute valeur en ligne; un
     personnalise fait l'inverse. Les deux chemins nettoient ce que l'autre a
     laisse, sinon un theme perso survivrait au passage a un integre. */
  function peindre(id) {
    var r = document.documentElement;
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

  async function enregistrer(nom, jetons, id) {
    var r = await fetch('/api/themes', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id: id || null, nom: nom, jetons: jetons })
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
        fond: j['--bg'] || '#09090b', point: j['--amber'] || '#f59e0b'
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
    } catch (e) {}
  })();

  /* Le libelle du bouton d'en-tete, ajoute au dictionnaire global. */
  window.LaRuche = window.LaRuche || {};
  if (LaRuche.i18n && LaRuche.i18n.add) {
    LaRuche.i18n.add({ 'theme.titre': { fr: 'Thème', en: 'Theme' } });
  }
  LaRuche.Themes = {
    GROUPES: GROUPES, TOUS: TOUS,
    charger: charger, appliquer: appliquer, peindre: peindre,
    apercuSur: apercuSur, apercuFin: apercuFin,
    jetonsCourants: jetonsCourants, catalogue: catalogue,
    enregistrer: enregistrer, supprimer: supprimer,
    actif: function () { return etat.actif; },
    estPerso: estPerso
  };
})();
