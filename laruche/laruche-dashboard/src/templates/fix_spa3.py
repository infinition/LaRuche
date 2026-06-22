import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Delete .status-indicator CSS
status_css_old = """.status-indicator { display:flex; align-items:center; gap:6px; }
.status-dot {
  width:11px; height:11px; border-radius:50%; background:var(--red);
  display:inline-block; transition:background var(--transition-fast); flex-shrink:0;
}
.status-dot.connected { background:var(--green); }
.status-dot.reconnecting { background:var(--amber); animation:pulse 1.5s infinite; }
.status-text { font-size:11px; color:var(--text-muted); transition:color var(--transition-fast); }
.status-dot.connected + .status-text { display:none; }
.status-dot.reconnecting + .status-text { color:var(--amber); }"""
content = content.replace(status_css_old, "")

# 2. Add offline animation for honeycomb
pulse_hex_old = """@keyframes pulse-hex { 0%, 100% { opacity: 0.2; transform: scale(0.8); } 50% { opacity: 1; transform: scale(1.1); } }"""
pulse_hex_new = """@keyframes pulse-hex { 0%, 100% { opacity: 0.2; transform: scale(0.8); } 50% { opacity: 1; transform: scale(1.1); } }
@keyframes pulse-hex-offline { 0%, 100% { opacity: 0.8; transform: scale(0.9); background: var(--red); } 50% { opacity: 1; transform: scale(1.1); background: var(--amber); } }"""
content = content.replace(pulse_hex_old, pulse_hex_new)

# Add CSS class
css_inject = """.honeycomb-loader.hc-sm.offline .hexagon { animation: pulse-hex-offline 1.5s infinite ease-in-out; }"""
content = content.replace('.honeycomb-loader.hc-sm .hexagon { width:8px; height:9px; }', '.honeycomb-loader.hc-sm .hexagon { width:8px; height:9px; }\n' + css_inject)

# 3. Update HTML
html_old = """    <div class="honeycomb-loader hc-sm" onclick="LaRuche.Router.go('settings')" title="Settings">"""
html_new = """    <div class="honeycomb-loader hc-sm" id="statusHoneycomb" onclick="LaRuche.Router.go('settings')" title="Settings">"""
content = content.replace(html_old, html_new)

html_del_old = """    <div class="status-indicator">
      <span class="status-dot" id="statusDot"></span>
      <span class="status-text" id="statusText">Deconnecte</span>
    </div>"""
content = content.replace(html_del_old, "")

# 4. Update JS

content = content.replace("document.getElementById('statusDot').className = 'status-dot connected';", "document.getElementById('statusHoneycomb').className = 'honeycomb-loader hc-sm';")
content = content.replace("document.getElementById('statusText').textContent = 'Connecte';", "")

content = content.replace("document.getElementById('statusDot').className = 'status-dot';", "document.getElementById('statusHoneycomb').className = 'honeycomb-loader hc-sm offline';")
content = content.replace("document.getElementById('statusText').textContent = 'Deconnecte';", "")

content = content.replace("document.getElementById('statusText').textContent = 'Erreur';", "")

content = content.replace("document.getElementById('statusText').textContent = 'Serveur indisponible';", "")

content = content.replace("document.getElementById('statusDot').className = 'status-dot reconnecting';", "document.getElementById('statusHoneycomb').className = 'honeycomb-loader hc-sm offline';")
# we must use regex for the reconnect text because it has a variable
content = re.sub(r"document\.getElementById\('statusText'\)\.textContent = 'Reconnexion \('\+reconnectAttempts\+'\)\.\.\.';", "", content)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Updates applied.")
