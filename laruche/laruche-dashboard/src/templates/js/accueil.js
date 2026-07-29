/* ── Accueil: the first-launch welcome ─────────────────────────────────────
   A fresh install used to open on an empty chat with no indication of what was
   missing. The diagnosis already existed - /api/onboarding probes the backend,
   the model, embeddings, voice and Chrome - but it was buried in Settings >
   General, which is exactly where someone who has never seen the app will not
   look.

   So this modal renders THAT endpoint, and nothing of its own: no second list of
   steps to keep in sync, no duplicated wording. Each step carries a `section`
   field naming where to act, which becomes a button here and a printed path in
   the CLI's /configure. One source, several renderings.

   It also offers the other route: hand the whole thing to the agent, which owns
   the `configure-laruche` skill. Reading state, explaining it, and pointing at
   the right screen is what that skill does - the modal just opens the door. */
LaRuche.Accueil = (function(){
  var VU = 'laruche_accueil_vu';
  var _echap = null;

  function _fermer(){
    var ov = document.getElementById('accueilModal');
    if(ov) ov.remove();
    if(_echap){ document.removeEventListener('keydown', _echap); _echap = null; }
  }

  // "Seen" is per browser, not per install: on a shared node each person meets the
  // welcome once, and clearing site data brings it back rather than stranding them.
  function _marquerVu(){ try{ localStorage.setItem(VU, '1'); }catch(e){} }
  function _dejaVu(){ try{ return localStorage.getItem(VU) === '1'; }catch(e){ return false; } }

  function _puce(s){
    if(s.done) return '<span class="acc-puce acc-puce--ok" aria-hidden="true">&#x2713;</span>';
    if(s.optional) return '<span class="acc-puce acc-puce--opt" aria-hidden="true">&#x25CB;</span>';
    return '<span class="acc-puce acc-puce--ko" aria-hidden="true">&#x2717;</span>';
  }

  function _etapeHtml(s){
    var esc = LaRuche.Utils.esc;
    // Only unmet steps get an action button. A green line with "Open" invites a
    // detour to a screen where there is nothing left to do.
    var bouton = (!s.done && s.section)
      ? '<button class="tl-btn acc-go" data-section="'+esc(s.section)+'">'+esc(LaRuche.i18n.t('accueil.open'))+'</button>'
      : '';
    return '<li class="acc-etape'+(s.done?' acc-etape--ok':'')+'">'+
      _puce(s)+
      '<div class="acc-etape-txt">'+
        '<div class="acc-etape-titre">'+esc(s.title||'')+
          (s.optional && !s.done ? ' <span class="acc-opt">'+esc(LaRuche.i18n.t('accueil.optional'))+'</span>' : '')+
        '</div>'+
        (s.instruction ? '<div class="acc-etape-aide">'+esc(s.instruction)+'</div>' : '')+
      '</div>'+
      bouton+
    '</li>';
  }

  function _rendre(d){
    var esc = LaRuche.Utils.esc;
    var steps = d.steps || [];
    var pret = !!d.complete;
    var sup = (d.optional_total|0) ? LaRuche.i18n.t('accueil.extras', {
      done: (d.optional_done|0), total: (d.optional_total|0)
    }) : '';

    var ov = document.createElement('div');
    ov.id = 'accueilModal';
    ov.className = 'lr-modal-ov';
    ov.innerHTML = '<div class="lr-modal acc-modal" role="dialog" aria-modal="true" aria-labelledby="accTitre">'+
      '<h3 id="accTitre">'+esc(LaRuche.i18n.t('accueil.title'))+'</h3>'+
      '<p class="lr-modal-sub">'+esc(LaRuche.i18n.t(pret ? 'accueil.subReady' : 'accueil.subTodo'))+'</p>'+
      '<div class="acc-etat'+(pret?' acc-etat--ok':'')+'">'+
        '<strong>'+esc(d.progress||'?')+'</strong> '+esc(LaRuche.i18n.t('accueil.required'))+
        (sup ? ' <span class="acc-extras">'+esc(sup)+'</span>' : '')+
      '</div>'+
      '<ul class="acc-liste">'+steps.map(_etapeHtml).join('')+'</ul>'+
      '<label class="acc-encore"><input type="checkbox" id="accPlusJamais"> '+esc(LaRuche.i18n.t('accueil.dontShow'))+'</label>'+
      '<div class="lr-modal-actions">'+
        '<button class="tl-btn" id="accPlusTard">'+esc(LaRuche.i18n.t('accueil.later'))+'</button>'+
        '<button class="tl-btn tl-btn--active" id="accAgent">'+esc(LaRuche.i18n.t('accueil.askAgent'))+'</button>'+
      '</div></div>';
    document.body.appendChild(ov);

    // The checkbox is read at close time, whichever way the modal is dismissed -
    // ticking it and then pressing Escape has to count as "don't show again".
    function sortir(){
      var cb = document.getElementById('accPlusJamais');
      if(cb && cb.checked) _marquerVu();
      _fermer();
    }
    ov.addEventListener('click', function(e){ if(e.target === ov) sortir(); });
    _echap = function(e){ if(e.key === 'Escape') sortir(); };
    document.addEventListener('keydown', _echap);
    document.getElementById('accPlusTard').onclick = sortir;

    ov.querySelectorAll('.acc-go').forEach(function(b){
      b.onclick = function(){
        // Jumping to the fix means the welcome has served its purpose.
        _marquerVu(); _fermer();
        LaRuche.Settings.ouvrirSection(b.dataset.section);
      };
    });

    document.getElementById('accAgent').onclick = function(){
      _marquerVu(); _fermer();
      LaRuche.Router.go('chat');
      // Let the view settle before writing into it, otherwise the message lands in
      // a chat pane that is about to be repainted.
      setTimeout(function(){
        var q = LaRuche.i18n.t('accueil.agentPrompt');
        var box = document.getElementById('userInput');
        if(box) box.value = q;
        if(LaRuche.Chat && LaRuche.Chat.sendMessage) LaRuche.Chat.sendMessage(q);
      }, 120);
    };
  }

  // Open on demand (Settings > General), regardless of the "seen" flag.
  async function ouvrir(){
    _fermer();
    try{
      var r = await fetch('/api/onboarding');
      _rendre(await r.json());
    }catch(e){
      LaRuche.Toast.show(LaRuche.i18n.t('accueil.failed'), 'error');
    }
  }

  // Auto-open at boot, once. Silent on failure: a probe that times out must not
  // greet a working install with an error box.
  function init(){
    if(_dejaVu()) return;
    fetch('/api/onboarding')
      .then(function(r){ return r.json(); })
      .then(function(d){ _rendre(d); })
      .catch(function(){});
  }

  return { init:init, ouvrir:ouvrir, fermer:_fermer };
})();

LaRuche.i18n.add({
  'accueil.title':      { fr:'Bienvenue dans LaRuche', en:'Welcome to LaRuche' },
  'accueil.subReady':   { fr:"Tout est en place. Voici l'état de ce nœud, pour information.",
                          en:'Everything is in place. Here is this node’s state, for reference.' },
  'accueil.subTodo':    { fr:'Il reste quelques réglages avant que ce nœud soit pleinement opérationnel.',
                          en:'A few settings are still missing before this node is fully operational.' },
  'accueil.required':   { fr:'réglages indispensables', en:'required steps' },
  'accueil.extras':     { fr:'· {done}/{total} options actives', en:'· {done}/{total} extras enabled' },
  'accueil.optional':   { fr:'facultatif', en:'optional' },
  'accueil.open':       { fr:'Régler', en:'Set up' },
  'accueil.later':      { fr:'Plus tard', en:'Later' },
  'accueil.askAgent':   { fr:"Demander à l'agent", en:'Ask the agent' },
  'accueil.dontShow':   { fr:'Ne plus afficher au démarrage', en:'Don’t show this at startup' },
  'accueil.failed':     { fr:"État indisponible", en:'State unavailable' },
  'accueil.reopen':     { fr:"Revoir l'accueil", en:'Show the welcome again' },
  // Sent verbatim to the agent. Naming the skill removes the guesswork; the skill
  // itself is what reads the live configuration and explains what to change.
  'accueil.agentPrompt':{ fr:"Aide-moi à configurer cette LaRuche. Utilise la capacité configure-laruche : lis l'état réel du nœud, dis-moi ce qui manque et guide-moi écran par écran.",
                          en:'Help me configure this LaRuche. Use the configure-laruche skill: read the node’s actual state, tell me what is missing, and walk me through it screen by screen.' }
});
