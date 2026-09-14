/* Coloration du texte en cours d'edition.
 *
 * Un textarea ne sait pas colorer son contenu. La methode employee ici est la
 * seule qui garde une zone de saisie native, avec son curseur, sa selection,
 * son annulation et sa dictee: le meme texte est peint dans un <pre> place
 * dessous, et le textarea passe au-dessus en texte transparent.
 *
 * Cela impose une contrainte dont tout le reste decoule: les deux boites
 * doivent avoir exactement la meme metrique. Meme police, meme corps, meme
 * interligne, meme padding, meme retour a la ligne. On peut donc changer la
 * COULEUR et la GRAISSE d'un mot, jamais sa taille: un titre Markdown est mis
 * en gras et non en grand, sinon les caracteres ne tombent plus sous le
 * curseur. Les fontes a chasse fixe gardent la meme avance en gras, ce qui
 * rend la graisse utilisable.
 */
(function(global){
  'use strict';

  /* Au-dela, colorer a chaque frappe se sent sous les doigts. Le texte reste
   * lisible, simplement sans couleur. */
  var TAILLE_MAX = 40000;

  function esc(texte) {
    return texte.replace(/[&<>]/g, function(c){
      return c === '&' ? '&amp;' : c === '<' ? '&lt;' : '&gt;';
    });
  }

  function marquer(classe, texte) {
    return classe ? '<span class="t-' + classe + '">' + esc(texte) + '</span>' : esc(texte);
  }

  function table(mots) {
    var index = {};
    mots.split(' ').forEach(function(mot){ if (mot) index[mot] = true; });
    return index;
  }

  var PYTHON = {
    cles: table('False None True and as assert async await break class continue def del ' +
      'elif else except finally for from global if import in is lambda nonlocal not or ' +
      'pass raise return try while with yield match case'),
    soi: table('self cls super'),
    natifs: table('abs all any bool bytes callable chr dict dir divmod enumerate eval filter ' +
      'float format frozenset getattr hasattr hash hex id input int isinstance issubclass iter ' +
      'len list map max min next object oct open ord pow print range repr reversed round set ' +
      'setattr slice sorted str sum tuple type vars zip ' +
      'load save show datasets md bar line area scatter pie hist scatter3d ' +
      'pd np plt laruche')
  };

  /* Le langage studio n'est ni Python ni JavaScript: ses verbes sont sa
   * syntaxe, et les reconnaitre est ce qui rend une cellule lisible. */
  var STUDIO = {
    cles: table('and or not true false'),
    soi: {},
    natifs: table('load save datasets show print md bar line area scatter pie hist scatter3d ' +
      'filter select drop rename sort head tail slice assign withColumn distinct dropna fillna ' +
      'concat groupby agg pivot join describe valueCounts unique histogram ' +
      'sum mean min max count countDistinct median quantile std variance first last mode ' +
      'abs round floor ceil sqrt log exp len upper lower trim contains replace split ' +
      'ifElse coalesce isNull year month day weekday dateOf today range')
  };

  var R_COMMENTAIRE = /#[^\n]*/y;
  var R_TRIPLE = /[rbfuRBFU]{0,2}("""|''')[\s\S]*?\1/y;
  var R_CHAINE = /[rbfuRBFU]{0,2}("|')(?:\\[\s\S]|(?!\1)[^\\\n])*\1?/y;
  var R_NOMBRE = /(?:0[xX][0-9a-fA-F_]+|0[oO][0-7_]+|0[bB][01_]+|(?:\d[\d_]*)?\.\d[\d_]*(?:[eE][+-]?\d+)?|\d[\d_]*\.?(?:[eE][+-]?\d+)?)[jJ]?/y;
  var R_DECORATION = /@[A-Za-z_][\w.]*/y;
  var R_MOT = /[A-Za-z_][A-Za-z0-9_]*/y;
  var R_BLANC = /\s+/y;
  var R_SIGNE = /[-+*\/%=<>!&|^~:,.;?(){}[\]]+/y;

  function lire(regle, source, position) {
    regle.lastIndex = position;
    var trouve = regle.exec(source);
    return trouve && trouve.index === position ? trouve[0] : null;
  }

  function teinterCode(source, langage) {
    var morceaux = [];
    var i = 0;
    var n = source.length;
    while (i < n) {
      var c = source.charAt(i);
      var brut;

      if (c === '#') {
        brut = lire(R_COMMENTAIRE, source, i);
        morceaux.push(marquer('com', brut));
        i += brut.length;
        continue;
      }
      if (c === '@') {
        brut = lire(R_DECORATION, source, i);
        if (brut) { morceaux.push(marquer('deco', brut)); i += brut.length; continue; }
      }
      if (c === '"' || c === "'" || /[rbfuRBFU]/.test(c)) {
        brut = lire(R_TRIPLE, source, i) || lire(R_CHAINE, source, i);
        /* Un prefixe seul n'est pas une chaine: rb sans guillemet est un nom. */
        if (brut && /["']/.test(brut)) {
          morceaux.push(marquer('str', brut));
          i += brut.length;
          continue;
        }
      }
      if (c >= '0' && c <= '9') {
        brut = lire(R_NOMBRE, source, i);
        if (brut) { morceaux.push(marquer('num', brut)); i += brut.length; continue; }
      }
      brut = lire(R_MOT, source, i);
      if (brut) {
        i += brut.length;
        var classe = langage.cles[brut] ? 'kw'
          : langage.soi[brut] ? 'soi'
          : langage.natifs[brut] ? 'nat'
          : source.charAt(i) === '(' ? 'fn'
          : source.charAt(i) === '=' && source.charAt(i + 1) !== '=' ? 'arg'
          : null;
        morceaux.push(marquer(classe, brut));
        continue;
      }
      brut = lire(R_BLANC, source, i);
      if (brut) { morceaux.push(esc(brut)); i += brut.length; continue; }
      brut = lire(R_SIGNE, source, i);
      if (brut) { morceaux.push(marquer('sig', brut)); i += brut.length; continue; }

      morceaux.push(esc(c));
      i += 1;
    }
    return morceaux.join('');
  }

  /* Markdown se lit par ligne: un titre, une citation ou une puce se decident
   * en debut de ligne, et le reste est du texte ou l'emphase compte. */
  function enligne(texte) {
    return esc(texte)
      .replace(/`[^`\n]+`/g, function(m){ return '<span class="t-md-code">' + m + '</span>'; })
      .replace(/(\*\*|__)(?=\S)([\s\S]*?\S)\1/g, function(m){ return '<span class="t-md-gras">' + m + '</span>'; })
      .replace(/~~(?=\S)([\s\S]*?\S)~~/g, function(m){ return '<span class="t-md-barre">' + m + '</span>'; })
      .replace(/(^|[\s(])([*_])(?=[^\s*_])([^*_\n]*[^\s*_])\2(?=$|[\s.,;:!?)])/g,
        function(m, avant, marque, corps){
          return avant + '<span class="t-md-ital">' + marque + corps + marque + '</span>';
        })
      .replace(/\[[^\]\n]*\]\([^)\s]*\)/g, function(m){ return '<span class="t-md-lien">' + m + '</span>'; });
  }

  function teinterMarkdown(source) {
    var dansUnBloc = false;
    return source.split('\n').map(function(ligne){
      if (/^\s*(```|~~~)/.test(ligne)) {
        dansUnBloc = !dansUnBloc;
        return marquer('md-cloture', ligne);
      }
      if (dansUnBloc) return marquer('md-code', ligne);
      if (!ligne.trim()) return esc(ligne);

      var titre = /^(#{1,6})(\s.*)?$/.exec(ligne);
      if (titre) {
        return '<span class="t-md-t' + Math.min(titre[1].length, 4) + '">' + esc(ligne) + '</span>';
      }
      if (/^\s*>/.test(ligne)) return marquer('md-cite', ligne);
      if (/^\s{0,3}([-*_])\s*(\1\s*){2,}$/.test(ligne)) return marquer('md-filet', ligne);

      var puce = /^(\s*(?:[-*+]|\d+[.)])\s+)([\s\S]*)$/.exec(ligne);
      if (puce) return marquer('md-puce', puce[1]) + enligne(puce[2]);

      return enligne(ligne);
    }).join('\n');
  }

  /* Le <pre> doit finir par une ligne vide quand le texte finit par un saut,
   * sinon la derniere ligne du textarea n'a rien en dessous d'elle. */
  function teinter(source, type, langue) {
    var texte = String(source == null ? '' : source);
    if (texte.length > TAILLE_MAX) return esc(texte) + '\n';
    var peint = type === 'markdown'
      ? teinterMarkdown(texte)
      : teinterCode(texte, langue === 'python' ? PYTHON : STUDIO);
    return peint + '\n';
  }

  global.StudioColorier = { teinter: teinter, tailleMax: TAILLE_MAX };
})(typeof window !== 'undefined' ? window : globalThis);
