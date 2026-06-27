import sys

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    html = f.read()

toggle_func = """
  function toggleVisibility(id, providerType, currentVis) {
    var newVis = currentVis === 'public_proxy' ? 'prive' : 'public_proxy';
    if(newVis === 'public_proxy' && (providerType === 'openai' || providerType === 'anthropic' || providerType === 'codex')) {
      if(!confirm("Public = le mesh utilise ce provider VIA ce node (ta clé reste locale, ce node relaie et exécute les appels). N'expose jamais une clé que tu ne veux pas voir consommée par le réseau. Continuer ?")) {
        return;
      }
    }
    fetch('/api/profiles/'+id+'/visibility', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({visibility: newVis})})
    .then(function(r){return r.json();})
    .then(function(d){
      if(d.status==='ok') {
        LaRuche.Toast.show('Visibilité modifiée avec succès','ok');
        loadTab('providers');
      } else {
        LaRuche.Toast.show('Erreur: '+(d.error||'?'),'err');
      }
    }).catch(function(e){LaRuche.Toast.show('Erreur: '+e,'err');});
  }

"""

if "function toggleVisibility(id, providerType, currentVis)" not in html:
    html = html.replace("  function deleteCredential(provider, apiKey) {", toggle_func + "  function deleteCredential(provider, apiKey) {")

with open('laruche-dashboard/src/templates/spa.html', 'w', encoding='utf-8') as f:
    f.write(html)
