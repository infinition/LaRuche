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
