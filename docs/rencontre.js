/* Quand deux abeilles se voient.
 *
 * Il y a l'abeille du site, qui vit dans la page, et celle de l'extension, qui
 * vit dans son monde isole. Les deux mondes ne partagent rien sauf le DOM, donc
 * elles se parlent par evenement: chacune annonce sa position toutes les 180 ms
 * sur un evenement `laruche:abeilles`, et ecoute celles des autres. Le meme
 * fichier est charge des deux cotes, chaque monde en a sa propre copie, et les
 * deux copies suivent le meme protocole. `extension-chrome/rencontre.js` est
 * une copie a l'identique de ce fichier: les deux doivent le rester, sinon les
 * abeilles ne se comprennent plus.
 *
 * Ce qui peut arriver quand elles se remarquent:
 *
 *   coeur      elles se rapprochent, se tiennent a une cinquantaine de pixels
 *              et laissent monter des coeurs. Pas a chaque fois qu'elles se
 *              croisent: une rencontre systematique n'est plus une rencontre.
 *   poursuite  elles sont deja cote a cote, ou le rapprochement a bien tourne:
 *              l'une file, l'autre suit, avec des eclairs. Quand la chasseuse
 *              touche la fuyarde, les roles s'echangent.
 *   dodo       la page est laissee de cote depuis un moment. Plutot que de
 *              s'endormir chacune dans son coin, elles se rejoignent sur le
 *              meme perchoir et dorment cote a cote, tournees l'une vers
 *              l'autre. Celui-la n'a pas de tirage au sort: il arrive a chaque
 *              fois, parce que c'est ce qu'on a envie de retrouver en revenant.
 *
 * Le protocole tient en cinq messages: `la` (je suis ici), `invite`, `oui`,
 * `non`, `fin`, plus `touche` pendant une poursuite. Seule l'abeille au plus
 * petit identifiant invite, sinon les deux s'invitent en meme temps et aucune
 * ne sait laquelle mene. Les messages arrivent d'un monde qu'on ne controle
 * pas, donc tout ce qu'ils contiennent est relu et borne avant usage: au pire,
 * une page hostile fait voler une abeille de travers.
 *
 * Ce fichier ne deplace rien lui-meme. Il repond a `consigne()`: un point a
 * viser, une vitesse, et c'est l'abeille qui vole comme elle sait le faire.
 */
(function () {
  'use strict';

  if (window.LaRucheRencontre) return;

  var CANAL = 'laruche:abeilles';

  var VUE = 560;        // au dela, elles ne se remarquent pas
  var PROCHE = 190;     // deja cote a cote: on passe directement a la poursuite
  var TENDRESSE = 58;   // la distance ou elles se tiennent pendant les coeurs
  var COEURS = 250;     // les coeurs ne sortent qu'une fois l'autre approchee
  var LARGE = 300;      // ce que la fuyarde cherche a mettre entre elles
  var TOUCHE = 36;      // en deca, la chasseuse a touche
  var LIT = 30;         // l'ecart entre deux abeilles endormies

  var BATTEMENT = 180;  // ms entre deux annonces de position
  var GUETTE = 700;     // ms entre deux examens du voisinage
  var OUBLI = 1400;     // sans nouvelles depuis ce delai, la voisine est partie
  var ATTENTE = 700;    // delai laisse a une invitation pour trouver reponse
  var ENVIE = 0.32;     // une occasion sur trois environ donne une rencontre
  var REPOS_MIN = 20000, REPOS_MAX = 50000;   // temps mort apres une rencontre
  var NUIT = 900000;    // un rendez-vous pour dormir ne s'arrete pas tout seul

  function hasard(a, b) { return a + Math.random() * (b - a); }
  function nombre(v) { return typeof v === 'number' && isFinite(v); }
  function borne(v, min, max, defaut) {
    v = parseFloat(v);
    if (!isFinite(v)) return defaut;
    return Math.max(min, Math.min(max, v));
  }

  /* Rejoindre le canal.
   *
   * `opts` donne les quelques choses que le protocole ne peut pas savoir seul:
   *
   *   position()      ou elle est, en coordonnees de fenetre
   *   etat()          'absente' quand elle n'est pas de ce monde (rangee dans
   *                   son titre, ecran trop etroit), 'eveillee', 'somnolente'
   *                   quand l'inactivite dure, 'endormie'
   *   perchoir(x, y)  un endroit ou se poser pres de ce point, ou rien
   *   debut(mode, role) / fin()   pour qu'elle lache ce qu'elle faisait
   *   emoji(txt, classe, duree)   ce qui lui sort de la tete
   */
  function rejoindre(opts) {
    var moi = Math.random().toString(36).slice(2, 10) + Date.now().toString(36).slice(-4);
    var voisines = {};      // id -> {x, y, e, libre, vu}
    var rencontre = null;   // {avec, mode, role, jusqua, puis, blocage, rx, ry}
    var invitation = null;  // proposition envoyee, en attente de reponse
    var bulle = 0;          // horodatage de la prochaine bulle
    var oscille = Math.random() * 100;
    var vivant = true;
    // Pas de rencontre dans les premieres secondes: le temps que la page se
    // pose et que l'abeille sorte du titre.
    var repos = performance.now() + hasard(4000, 12000);

    function etat() {
      try {
        var e = opts.etat();
        return e === 'eveillee' || e === 'somnolente' || e === 'endormie' ? e : 'absente';
      } catch (err) { return 'absente'; }
    }

    function ou() {
      var p = null;
      try { p = opts.position(); } catch (e) {}
      return p && nombre(p.x) && nombre(p.y) ? p : null;
    }

    function emettre(m) {
      m.de = moi;
      try {
        window.dispatchEvent(new CustomEvent(CANAL, { detail: JSON.stringify(m) }));
      } catch (e) {}
    }

    function commencer(avec, mode, role, duree, puis, rx, ry) {
      rencontre = {
        avec: avec,
        mode: mode,
        role: role,
        puis: puis,
        rx: rx,
        ry: ry,
        jusqua: performance.now() + duree,
        blocage: 0
      };
      invitation = null;
      bulle = 0;
      if (opts.debut) opts.debut(mode, role);
    }

    function terminer(annoncer) {
      if (!rencontre) return;
      var avec = rencontre.avec;
      var mode = rencontre.mode;
      rencontre = null;
      // Un reveil ne doit pas etre suivi d'un long temps mort: elles viennent
      // de passer la nuit ensemble, pas de se courir apres.
      repos = performance.now() + (mode === 'dodo' ? hasard(6000, 15000) : hasard(REPOS_MIN, REPOS_MAX));
      if (annoncer) emettre({ t: 'fin', a: avec });
      if (opts.fin) opts.fin();
    }

    /* Ce qui arrive des autres. */
    function ecouter(ev) {
      if (!vivant) return;
      var m;
      try { m = JSON.parse(ev.detail); } catch (e) { return; }
      if (!m || typeof m.de !== 'string' || m.de === moi || m.de.length > 40) return;

      if (m.t === 'la') {
        if (!nombre(m.x) || !nombre(m.y)) return;
        voisines[m.de] = { x: m.x, y: m.y, e: m.e, libre: !!m.libre, vu: performance.now() };
        return;
      }
      if (m.a !== moi) return;

      if (m.t === 'invite') {
        var mien = etat();
        var dodo = m.mode === 'dodo';
        var possible = rencontre || invitation ? false
          : dodo ? mien === 'somnolente'
          : mien === 'eveillee' && performance.now() >= repos;
        if (!possible) { emettre({ t: 'non', a: m.de }); return; }
        emettre({ t: 'oui', a: m.de });
        if (dodo) {
          commencer(m.de, 'dodo', m.role === 'gauche' ? 'gauche' : 'droite', NUIT, 0,
            borne(m.rx, 0, 20000, window.innerWidth / 2),
            borne(m.ry, 0, 20000, window.innerHeight / 2));
        } else {
          commencer(m.de,
            m.mode === 'poursuite' ? 'poursuite' : 'coeur',
            m.role === 'chasseur' ? 'chasseur' : 'fuyard',
            borne(m.duree, 1500, 15000, 5000),
            borne(m.puis, 0, 15000, 0), 0, 0);
        }
        return;
      }
      if (m.t === 'oui' && invitation && invitation.a === m.de) {
        commencer(invitation.a, invitation.mode, invitation.role,
          invitation.duree, invitation.puis, invitation.rx, invitation.ry);
        return;
      }
      if (m.t === 'non' && invitation && invitation.a === m.de) {
        invitation = null;
        repos = performance.now() + hasard(6000, 14000);
        return;
      }
      if (m.t === 'fin' && rencontre && rencontre.avec === m.de) {
        terminer(false);
        return;
      }
      // Elle m'a touchee: a moi de courir.
      if (m.t === 'touche' && rencontre && rencontre.avec === m.de) {
        rencontre.role = 'chasseur';
        rencontre.blocage = performance.now() + 1200;
      }
    }

    /* L'annonce de position, et le menage qui va avec. */
    function battre() {
      if (!vivant) return;
      var maintenant = performance.now();
      var mien = etat();

      for (var k in voisines) {
        if (maintenant - voisines[k].vu > OUBLI) delete voisines[k];
      }
      if (invitation && maintenant > invitation.expire) {
        invitation = null;
        repos = maintenant + hasard(3000, 8000);
      }
      // La partenaire s'est tue, ou l'abeille n'est plus en etat de jouer: on
      // arrete plutot que de la laisser poursuivre un fantome. Le dodo fait
      // exception, il est justement fait pour durer pendant qu'elles dorment.
      if (rencontre && (!voisines[rencontre.avec] || mien === 'absente' ||
          (rencontre.mode !== 'dodo' && mien !== 'eveillee'))) {
        terminer(true);
      }

      var p = ou();
      if (!p) return;
      emettre({
        t: 'la',
        x: Math.round(p.x),
        y: Math.round(p.y),
        e: mien,
        libre: !rencontre && !invitation && mien === 'eveillee' && maintenant >= repos
      });
    }

    /* La plus proche voisine dans un certain etat, et dont l'identifiant passe
       apres le mien: c'est ce qui designe celle des deux qui invite. */
    function voisine(p, portee, filtre) {
      var elue = null, ecart = portee;
      for (var k in voisines) {
        if (k < moi || !filtre(voisines[k])) continue;
        var dx = voisines[k].x - p.x, dy = voisines[k].y - p.y;
        var d = Math.sqrt(dx * dx + dy * dy);
        if (d < ecart) { ecart = d; elue = k; }
      }
      return elue ? { id: elue, d: ecart } : null;
    }

    function proposer(inv) {
      invitation = inv;
      inv.expire = performance.now() + ATTENTE;
      emettre({
        t: 'invite', a: inv.a, mode: inv.mode, role: inv.sien,
        duree: Math.round(inv.duree), puis: Math.round(inv.puis),
        rx: Math.round(inv.rx), ry: Math.round(inv.ry)
      });
    }

    /* Le guet. */
    function guetter() {
      if (!vivant || rencontre || invitation) return;
      var maintenant = performance.now();
      var mien = etat();
      var p = ou();
      if (!p) return;

      /* Le coucher. Elles ne s'endorment pas chacune de son cote: celle qui
         mene choisit le lit, a mi-chemin, sur un perchoir si la page en offre
         un pres de la, et l'autre vient s'installer a cote. */
      if (mien === 'somnolente') {
        var dormeuse = voisine(p, 1e6, function (v) { return v.e === 'somnolente'; });
        if (!dormeuse) return;
        var v = voisines[dormeuse.id];
        var rx = (p.x + v.x) / 2, ry = (p.y + v.y) / 2;
        var lit = null;
        try { lit = opts.perchoir ? opts.perchoir(rx, ry) : null; } catch (e) {}
        if (lit && nombre(lit.x) && nombre(lit.y)) { rx = lit.x; ry = lit.y; }
        proposer({
          a: dormeuse.id, mode: 'dodo', sien: 'droite', role: 'gauche',
          duree: NUIT, puis: 0,
          rx: Math.max(90, Math.min(window.innerWidth - 110, rx)),
          ry: Math.max(100, Math.min(window.innerHeight - 70, ry))
        });
        return;
      }

      if (mien !== 'eveillee' || maintenant < repos) return;

      var elue = voisine(p, VUE, function (v) { return v.libre; });
      if (!elue) return;

      // Se voir ne suffit pas. Le plus souvent elles se croisent sans rien.
      if (Math.random() > ENVIE) {
        repos = maintenant + hasard(4000, 9000);
        return;
      }

      var mode = elue.d < PROCHE ? 'poursuite' : 'coeur';
      // Un rapprochement finit souvent en course: elles se frolent, l'une
      // s'echappe, l'autre suit.
      var puis = mode === 'coeur' && Math.random() < 0.55 ? hasard(4500, 8000) : 0;
      var sien = Math.random() < 0.5 ? 'chasseur' : 'fuyard';
      proposer({
        a: elue.id, mode: mode, sien: sien,
        role: sien === 'chasseur' ? 'fuyard' : 'chasseur',
        duree: mode === 'poursuite' ? hasard(5000, 9000) : hasard(4000, 7000),
        puis: puis, rx: 0, ry: 0
      });
    }

    /* Ce que l'abeille demande a chaque image: un point, une vitesse. */
    function consigne() {
      if (!rencontre) return null;
      var maintenant = performance.now();

      if (maintenant > rencontre.jusqua) {
        if (rencontre.mode === 'coeur' && rencontre.puis) {
          rencontre.mode = 'poursuite';       // les roles etaient deja repartis
          rencontre.jusqua = maintenant + rencontre.puis;
          rencontre.puis = 0;
        } else {
          terminer(true);
          return null;
        }
      }

      var v = voisines[rencontre.avec];
      var p = ou();
      if (!v || !p) { terminer(true); return null; }

      // Le lit est un point fixe, decide au moment de l'invitation: si chacune
      // visait l'autre, elles se pousseraient l'une l'autre a travers l'ecran
      // sans jamais s'arreter.
      if (rencontre.mode === 'dodo') {
        var cote = rencontre.role === 'gauche' ? -LIT : LIT;
        return {
          x: rencontre.rx + cote,
          y: rencontre.ry,
          vitesse: 2.8,
          marge: 3,
          dodo: true,
          sens: rencontre.role === 'gauche' ? 1 : -1   // tournees l'une vers l'autre
        };
      }

      var dx = v.x - p.x, dy = v.y - p.y;
      var d = Math.sqrt(dx * dx + dy * dy) || 1;

      if (rencontre.mode === 'coeur') {
        if (opts.emoji && maintenant > bulle && d < COEURS) {
          opts.emoji(Math.random() < 0.25 ? '💛' : '♥', 'coeur', 2200);
          bulle = maintenant + hasard(520, 1100);
        }
        // Elle vise un point a bonne distance de l'autre, pas l'autre: deux
        // abeilles qui visent le meme point se rentrent dedans.
        return {
          x: v.x - dx / d * TENDRESSE,
          y: v.y - dy / d * TENDRESSE,
          vitesse: d > TENDRESSE * 1.6 ? 2.6 : 1.1,
          marge: 8
        };
      }

      if (rencontre.role === 'chasseur') {
        if (d < TOUCHE && maintenant > rencontre.blocage) {
          emettre({ t: 'touche', a: rencontre.avec });
          rencontre.role = 'fuyard';
          rencontre.blocage = maintenant + 1200;
          if (opts.emoji) opts.emoji('💥', 'eclair', 900);
          bulle = maintenant + 500;
        } else if (opts.emoji && maintenant > bulle) {
          opts.emoji('⚡', 'eclair', 1000);
          bulle = maintenant + hasard(260, 520);
        }
        return { x: v.x, y: v.y, vitesse: 6.4, marge: 0 };
      }

      if (opts.emoji && maintenant > bulle) {
        opts.emoji('💨', 'souffle', 1000);
        bulle = maintenant + hasard(420, 900);
      }
      // La fuyarde part a l'oppose, sur un cap qui tourne lentement: en ligne
      // droite elle se colle a un bord au bout d'une seconde et la course
      // s'arrete la.
      oscille += 0.02;
      var cap = Math.atan2(-dy, -dx) + Math.sin(oscille) * 0.9;
      return {
        x: Math.max(60, Math.min(window.innerWidth - 80, p.x + Math.cos(cap) * LARGE)),
        y: Math.max(90, Math.min(window.innerHeight - 70, p.y + Math.sin(cap) * LARGE)),
        vitesse: 5.6,
        marge: 0
      };
    }

    function quitter() {
      if (!vivant) return;
      vivant = false;
      if (rencontre) emettre({ t: 'fin', a: rencontre.avec });
      rencontre = null;
      invitation = null;
      window.removeEventListener(CANAL, ecouter);
      window.removeEventListener('pagehide', quitter);
      clearInterval(minuteurBattement);
      clearInterval(minuteurGuette);
    }

    window.addEventListener(CANAL, ecouter);
    window.addEventListener('pagehide', quitter);
    var minuteurBattement = setInterval(battre, BATTEMENT);
    var minuteurGuette = setInterval(guetter, GUETTE);

    return {
      consigne: consigne,
      finir: function () { terminer(true); },
      quitter: quitter,
      mode: function () { return rencontre ? rencontre.mode : null; }
    };
  }

  window.LaRucheRencontre = { rejoindre: rejoindre };
})();
