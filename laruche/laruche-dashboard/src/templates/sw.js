/* LaRuche service worker: makes the SPA installable + offline-capable.
   Network-first so the UI is never stale; the cached shell is only a fallback. */
var CACHE = 'laruche-shell-v2';
var SHELL = ['/', '/app.css', '/app.js',
  '/vendor/marked.min.js', '/vendor/purify.min.js', '/vendor/highlight.min.js'];

self.addEventListener('install', function(e){
  e.waitUntil(caches.open(CACHE).then(function(c){ return c.addAll(SHELL); }).catch(function(){}));
  self.skipWaiting();
});

self.addEventListener('activate', function(e){
  e.waitUntil(caches.keys().then(function(keys){
    return Promise.all(keys.filter(function(k){ return k !== CACHE; }).map(function(k){ return caches.delete(k); }));
  }));
  self.clients.claim();
});

self.addEventListener('fetch', function(e){
  var req = e.request;
  if(req.method !== 'GET') return;
  var url = new URL(req.url);
  if(url.origin !== self.location.origin) return;
  // Never intercept the API or websockets.
  if(url.pathname.indexOf('/api') === 0 || url.pathname.indexOf('/ws') === 0) return;
  e.respondWith(
    fetch(req).then(function(res){
      if(res && res.ok && (url.pathname === '/' || url.pathname === '/app.css' || url.pathname === '/app.js' || url.pathname.indexOf('/vendor/') === 0)){
        var clone = res.clone();
        caches.open(CACHE).then(function(c){ c.put(req, clone); });
      }
      return res;
    }).catch(function(){
      return caches.match(req).then(function(m){ return m || caches.match('/'); });
    })
  );
});
