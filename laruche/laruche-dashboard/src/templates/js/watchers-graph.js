/* The correlation graph: which watcher feeds which.
 *
 * A watcher used to be an island, so a flat list said everything there was to say. Now
 * that a rule can read another watcher's verdict, the list hides the only structure
 * that matters: reading "site-down" and "app-broken" as two unrelated rows tells you
 * nothing, seeing that the second is built on the first tells you the whole design.
 *
 * Drawn only when at least one correlation exists. On a set of independent watchers a
 * graph of disconnected dots is noise, and noise in a diagnostic view is worse than
 * nothing.
 */
(function(){
  'use strict';

  var NODE_W = 150, NODE_H = 34, GAP_X = 74, GAP_Y = 16, PAD = 14;

  function t(key, fallback){
    try {
      var v = window.LaRuche && LaRuche.i18n && LaRuche.i18n.t(key);
      return (v && v !== key) ? v : fallback;
    } catch(e){ return fallback; }
  }
  function esc(s){
    return (window.LaRuche && LaRuche.Utils && LaRuche.Utils.esc)
      ? LaRuche.Utils.esc(s) : String(s == null ? '' : s);
  }

  /* Every watcher name a rule tree correlates on. Mirrors watchers_references() in the
   * Rust side: same traversal, including inside `non` and nested combinators, or the
   * picture would silently omit negated dependencies. */
  function edgesOf(regle, out){
    out = out || [];
    if(!regle || typeof regle !== 'object') return out;
    if(regle.op === 'watcher' && regle.nom) out.push(regle.nom);
    if(Array.isArray(regle.regles)) regle.regles.forEach(function(r){ edgesOf(r, out); });
    if(regle.regle) edgesOf(regle.regle, out);
    return out;
  }

  /* Depth = longest path from a watcher that depends on nothing.
   *
   * Layering by depth is what makes a dependency graph readable: sources on the left,
   * conclusions on the right, every arrow pointing the same way. The visited set guards
   * against a cycle: creation refuses them, but a file edited by hand can still carry
   * one, and a view that hangs the browser would be a poor way to find that out. */
  function depthOf(name, byName, seen){
    seen = seen || {};
    if(seen[name]) return 0;
    seen[name] = true;
    var w = byName[name.toLowerCase()];
    if(!w || !w.regles) return 0;
    var deps = edgesOf(w.regles);
    if(!deps.length) return 0;
    var max = 0;
    deps.forEach(function(d){
      var sub = byName[d.toLowerCase()] ? 1 + depthOf(d, byName, seen) : 1;
      if(sub > max) max = sub;
    });
    return max;
  }

  function render(host, watchers){
    if(!host) return;
    host.innerHTML = '';
    if(!Array.isArray(watchers) || !watchers.length) return;

    var byName = {};
    watchers.forEach(function(w){ if(w.name) byName[w.name.toLowerCase()] = w; });

    var liens = [];
    watchers.forEach(function(w){
      edgesOf(w.regles).forEach(function(source){
        // An edge to an unknown watcher is dropped rather than drawn to nowhere: the
        // engine reads a missing peer as false, so the honest picture is no link.
        if(byName[source.toLowerCase()]) liens.push({ de: source.toLowerCase(), vers: w.name.toLowerCase() });
      });
    });
    if(!liens.length) return;   // nothing to show that the list does not already say

    // Only the connected component: an unrelated watcher on the side adds a column and
    // no information.
    var connectes = {};
    liens.forEach(function(l){ connectes[l.de] = true; connectes[l.vers] = true; });
    var noeuds = watchers.filter(function(w){ return connectes[(w.name||'').toLowerCase()]; });

    var colonnes = {};
    noeuds.forEach(function(w){
      var d = depthOf(w.name, byName, {});
      (colonnes[d] = colonnes[d] || []).push(w);
    });
    var profondeurs = Object.keys(colonnes).map(Number).sort(function(a,b){return a-b;});

    var pos = {};
    var hauteurMax = 0;
    profondeurs.forEach(function(d, i){
      colonnes[d].forEach(function(w, j){
        pos[w.name.toLowerCase()] = {
          x: PAD + i * (NODE_W + GAP_X),
          y: PAD + j * (NODE_H + GAP_Y),
          w: w
        };
        hauteurMax = Math.max(hauteurMax, PAD + (j + 1) * (NODE_H + GAP_Y));
      });
    });
    var largeur = PAD * 2 + profondeurs.length * NODE_W + (profondeurs.length - 1) * GAP_X;
    var hauteur = hauteurMax + PAD;

    var svg = ['<svg viewBox="0 0 '+largeur+' '+hauteur+'" width="100%" height="'+hauteur+'" '+
               'role="img" aria-label="'+esc(t('watchers.graphTitle','Watcher correlations'))+'">'];
    svg.push('<defs><marker id="wg-arrow" viewBox="0 0 10 10" refX="9" refY="5" '+
             'markerWidth="6" markerHeight="6" orient="auto-start-reverse">'+
             '<path d="M0,0 L10,5 L0,10 z" fill="currentColor" opacity=".55"/></marker></defs>');

    liens.forEach(function(l){
      var a = pos[l.de], b = pos[l.vers];
      if(!a || !b) return;
      var x1 = a.x + NODE_W, y1 = a.y + NODE_H/2;
      var x2 = b.x,          y2 = b.y + NODE_H/2;
      var mx = (x1 + x2) / 2;
      // A curve, not a straight line: with several edges converging on one node,
      // straight segments overlap and the picture stops being readable.
      svg.push('<path d="M'+x1+','+y1+' C'+mx+','+y1+' '+mx+','+y2+' '+x2+','+y2+'" '+
               'fill="none" stroke="currentColor" stroke-opacity=".38" stroke-width="1.5" '+
               'marker-end="url(#wg-arrow)"/>');
    });

    noeuds.forEach(function(w){
      var p = pos[w.name.toLowerCase()];
      if(!p) return;
      var vrai = w.verdict === true;
      var couleur = !w.active ? 'var(--text-dim, #777)'
                  : vrai ? 'var(--green, #46c46a)'
                  : 'var(--border, #444)';
      var titre = (w.watcher_type||'?')+' · '+(w.target||'');
      svg.push('<g><title>'+esc(titre)+'</title>');
      svg.push('<rect x="'+p.x+'" y="'+p.y+'" width="'+NODE_W+'" height="'+NODE_H+'" rx="7" '+
               'fill="var(--bg-card, #1b1b1f)" stroke="'+couleur+'" stroke-width="'+(vrai?2:1)+'"/>');
      var nom = (w.name||'').length > 20 ? (w.name||'').slice(0,19)+'…' : (w.name||'');
      svg.push('<text x="'+(p.x+10)+'" y="'+(p.y+15)+'" font-size="11" '+
               'fill="var(--text, #e6e6e6)">'+esc(nom)+'</text>');
      svg.push('<text x="'+(p.x+10)+'" y="'+(p.y+27)+'" font-size="9" '+
               'fill="var(--text-dim, #888)">'+esc(w.watcher_type||'')+
               (w.active ? '' : ' · '+esc(t('watchers.graphInactive','inactive')))+'</text>');
      svg.push('</g>');
    });
    svg.push('</svg>');

    var carte = document.createElement('div');
    carte.className = 'settings-card';
    carte.style.cssText = 'margin-bottom:12px;overflow-x:auto';
    carte.innerHTML = '<div style="font-weight:600;margin-bottom:2px">'+
        esc(t('watchers.graphTitle','Watcher correlations'))+'</div>'+
      '<div style="font-size:10px;color:var(--text-dim);margin-bottom:8px">'+
        esc(t('watchers.graphHint','An arrow means the target reads that watcher\'s verdict. A green border is a verdict currently true.'))+
      '</div>'+svg.join('');
    host.appendChild(carte);
  }

  window.LaRuche = window.LaRuche || {};
  window.LaRuche.WatchersGraph = { render: render };
})();
