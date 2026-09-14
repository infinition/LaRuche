/* Zoom d'interface d'une App, retenu entre deux ouvertures.
 *
 * Une App vit dans un panneau dont elle ne choisit pas la largeur. Reduire la
 * seule police rendrait le texte petit a cote d'un plateau reste grand; `zoom`
 * met toute la mise en page a l'echelle, donc le contenu recoit plus de pixels
 * logiques et garde ses proportions.
 *
 * Deux consequences obligatoires, et c'est pour elles que ce fichier existe
 * plutot qu'une ligne de CSS par App:
 *
 * - la hauteur doit etre divisee par le facteur, sinon `100dvh` est mis a
 *   l'echelle lui aussi et la coque deborde de son cadre;
 * - une requete media voit toujours la fenetre NON zoomee. A 60% dans un
 *   panneau etroit, la mise en page croit rester etroite et n'utilise jamais la
 *   largeur que le dezoom vient de lui donner. Il faut des requetes de
 *   conteneur, posees par l'App sur sa propre coque.
 *
 * Le reglage passe par le stockage prive de l'App, donc par utilisateur.
 */
(function(global){
  'use strict';

  var PALIERS = [60, 70, 80, 90, 100, 110, 125, 150];

  function create(options) {
    var lire = options.load || function(){ return Promise.resolve(null); };
    var ecrire = options.save || function(){ return Promise.resolve(); };
    var afficher = options.label || function(){};
    var racine = options.root || document.documentElement;
    var actuel = 100;

    function plusProche(valeur) {
      return PALIERS.reduce(function(a, b){
        return Math.abs(b - valeur) < Math.abs(a - valeur) ? b : a;
      }, PALIERS[0]);
    }

    function appliquer(valeur, persister) {
      actuel = plusProche(Number(valeur) || 100);
      racine.style.setProperty('--zoom', String(actuel / 100));
      afficher(actuel + '%');
      if (persister) {
        try { Promise.resolve(ecrire(actuel)).catch(function(){}); } catch (e) {}
      }
      return actuel;
    }

    return {
      paliers: PALIERS.slice(),
      valeur: function(){ return actuel; },
      appliquer: appliquer,
      /* Un cran vers le haut ou vers le bas, sans sortir de la liste. */
      decaler: function(pas) {
        var i = PALIERS.indexOf(actuel);
        return appliquer(PALIERS[Math.max(0, Math.min(PALIERS.length - 1, i + pas))], true);
      },
      reinitialiser: function(){ return appliquer(100, true); },
      /* Relit le reglage garde. Un stockage vide ou en panne laisse 100%:
         l'App doit s'afficher meme quand rien ne repond. */
      restaurer: function() {
        appliquer(100, false);
        return Promise.resolve()
          .then(lire)
          .then(function(garde){ if (garde) appliquer(garde, false); })
          .catch(function(){});
      }
    };
  }

  global.AppZoom = { create: create, paliers: PALIERS.slice() };
})(typeof window !== 'undefined' ? window : globalThis);
