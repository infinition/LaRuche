/* ── Boot ── */
(function(){
  LaRuche.Header.init();
  // FR/EN language toggle in the header (injected on the right).
  (function(){
    var hr = document.querySelector('.header-right');
    if(!hr || document.getElementById('langToggle')) return;
    var cur = LaRuche.i18n.get();
    var btn = document.createElement('button');
    btn.id = 'langToggle';
    btn.title = 'Language';
    btn.style.cssText = 'background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:6px;padding:3px 8px;cursor:pointer;font-size:11px;font-weight:600;margin-right:8px';
    btn.textContent = cur.toUpperCase();
    btn.onclick = function(){ LaRuche.i18n.setLang(cur === 'fr' ? 'en' : 'fr'); };
    hr.insertBefore(btn, hr.firstChild);
  })();
  /* Le selecteur de theme, juste a cote de la langue.
     Un menu et non un cycle: la liste grandit avec les themes que l'utilisateur
     fabrique, et faire seize clics pour revenir au premier n'est pas une
     interface. Hauteur bornee et defilement, donc, et un apercu au survol qui
     peint pour de vrai: un pave de couleur ne dit pas ce que donne un theme, une
     interface entiere si. */
  (function(){
    var hr = document.querySelector('.header-right');
    if(!hr || !window.LaRuche || !LaRuche.Themes || document.getElementById('themeToggle')) return;
    var btn = document.createElement('button');
    btn.id = 'themeToggle';
    btn.title = LaRuche.i18n.t('theme.titre');
    btn.style.cssText = 'background:none;border:1px solid var(--border);color:var(--text-dim);border-radius:6px;padding:3px 7px;cursor:pointer;font-size:12px;line-height:1;margin-right:8px;display:flex;align-items:center';
    btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/><circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/><circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/><circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/></svg>';

    var menu = null, ouvert = false;
    function fermer(){
      if(!menu) return;
      menu.remove(); menu = null; ouvert = false;
      LaRuche.Themes.apercuFin();
      document.removeEventListener('click', dehors, true);
    }
    function dehors(e){ if(menu && !menu.contains(e.target) && e.target !== btn) fermer(); }

    function ouvrir(){
      menu = document.createElement('div');
      menu.id = 'themeMenu';
      var r = btn.getBoundingClientRect();
      menu.style.cssText = 'position:fixed;top:'+(r.bottom+6)+'px;right:'+Math.max(8, window.innerWidth-r.right)+'px;'+
        'background:var(--bg-panel);border:1px solid var(--border);border-radius:10px;padding:5px;'+
        'z-index:9999;min-width:190px;max-height:min(320px,60vh);overflow-y:auto;'+
        'box-shadow:0 12px 32px rgba(0,0,0,.45)';
      var actif = LaRuche.Themes.actif();
      LaRuche.Themes.catalogue().forEach(function(t){
        var o = document.createElement('button');
        o.type = 'button';
        o.style.cssText = 'display:flex;align-items:center;gap:9px;width:100%;text-align:left;background:'+
          (t.id===actif ? 'var(--bg-hover)' : 'none')+';border:none;color:var(--text);padding:6px 8px;'+
          'border-radius:7px;cursor:pointer;font-size:12.5px';
        o.innerHTML = '<span style="flex:0 0 auto;width:17px;height:17px;border-radius:5px;border:1px solid var(--border);'+
          'background:'+t.fond+';display:inline-flex;align-items:center;justify-content:center">'+
          '<span style="width:7px;height:7px;border-radius:50%;background:'+t.point+'"></span></span>'+
          '<span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">'+LaRuche.Utils.esc(t.nom)+'</span>'+
          (t.id===actif ? '<span style="color:var(--amber)">&#10003;</span>' : '');
        o.onmouseenter = function(){ LaRuche.Themes.apercuSur(t.id); };
        o.onclick = function(){ LaRuche.Themes.appliquer(t.id); fermer(); };
        menu.appendChild(o);
      });
      // La sortie du MENU, et non celle d'une ligne: passer d'une ligne a l'autre
      // ne doit pas repeindre le theme retenu entre les deux.
      menu.onmouseleave = function(){ LaRuche.Themes.apercuFin(); };
      document.body.appendChild(menu);
      ouvert = true;
      setTimeout(function(){ document.addEventListener('click', dehors, true); }, 0);
    }

    btn.onclick = function(e){ e.stopPropagation(); if(ouvert) fermer(); else ouvrir(); };
    hr.insertBefore(btn, hr.firstChild);
    LaRuche.Themes.charger();
    // Translate the navigation labels.
    document.querySelectorAll('.header-nav a[data-page], .mobile-tabs a[data-page]').forEach(function(a){
      var key = 'nav.' + a.dataset.page;
      var span = a.querySelector('.tab-text');
      if(span && LaRuche.i18n.DICT[key]) span.textContent = LaRuche.i18n.t(key);
    });
  })();
  // Translate the whole static shell (spa.html) via data-i18n attributes.
  LaRuche.i18n.applyStatic();
  // The tab labels just changed length: re-measure whether they still fit on one line.
  if(LaRuche.Header.fitNav) LaRuche.Header.fitNav();
  LaRuche.Voice.init();
  LaRuche.Feed.init();
  if(LaRuche.Secrets && LaRuche.Secrets.init) LaRuche.Secrets.init();
  if(LaRuche.Mesh && LaRuche.Mesh.init) LaRuche.Mesh.init();
  // reposition mesh windows on page change and resize
  window.addEventListener('hashchange', function(){ if(LaRuche.Mesh) LaRuche.Mesh.repositionWindows(); });
  window.addEventListener('resize', function(){ if(LaRuche.Mesh) LaRuche.Mesh.repositionWindows(); });
  // Check auth first, then init router
  LaRuche.Auth.init(function(authenticated){
    LaRuche.WS.connect();
    LaRuche.Router.init();
    LaRuche.Console.log('info','SPA','LaRuche SPA initialized');
    // Inside the auth callback, and only when signed in: greeting the login screen
    // with a setup checklist would be both useless and a small disclosure.
    if(authenticated && LaRuche.Accueil) LaRuche.Accueil.init();
    if(LaRuche.TableRonde) LaRuche.TableRonde.init();

    // iOS virtual keyboard handler - adjust layout when keyboard opens/closes
    if(window.visualViewport) {
      var inputArea = document.querySelector('.input-area');
      var mobileTabs = document.getElementById('mobileTabs');
      window.visualViewport.addEventListener('resize', function(){
        var offset = window.innerHeight - window.visualViewport.height;
        if(offset > 100) {
          // Keyboard is open
          if(mobileTabs) mobileTabs.style.display = 'none';
          document.body.style.height = window.visualViewport.height + 'px';
        } else {
          // Keyboard is closed
          if(mobileTabs) mobileTabs.style.display = '';
          document.body.style.height = '';
        }
      });
      window.visualViewport.addEventListener('scroll', function(){
        // Prevent viewport scroll offset on iOS
        window.scrollTo(0,0);
      });
    }
  });
  // PWA: register the service worker so the SPA is installable (add to home screen) + offline.
  if('serviceWorker' in navigator){
    window.addEventListener('load', function(){ navigator.serviceWorker.register('/sw.js').catch(function(){}); });
  }
})();
