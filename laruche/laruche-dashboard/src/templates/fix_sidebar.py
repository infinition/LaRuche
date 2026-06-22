import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Hide sidebar toggle globally
if '.sidebar-toggle { display: none !important; }' not in content:
    content = content.replace(
        '.sidebar-toggle { display:flex; }',
        '.sidebar-toggle { display: none !important; }'
    )
    # also add it to generic CSS if it's not already there
    content = content.replace(
        '.header-brand { font-size:14px; }',
        '.header-brand { font-size:14px; }\n  .sidebar-toggle { display: none !important; }'
    )

# 2. Add Mobile sidebar logic for chat, memory, missions
css_mobile_sidebar = """
@media (max-width: 900px) {
  .chat-sidebar, .mem2-side, .mis-side {
    position: fixed !important; left: 0; top: 0; bottom: 0; z-index: 120 !important;
    width: 90% !important; max-width: 400px;
    transform: translateX(-100%);
    transition: transform var(--transition-med);
    box-shadow: 4px 0 24px rgba(0,0,0,0.6);
    background: var(--bg-panel);
    border-right: 1px solid var(--border) !important;
  }
  .chat-sidebar.open, .mem2-side.open, .mis-side.open {
    transform: translateX(0);
  }
  .mis-layout, .mem2-layout {
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .mem2-main, .mem2-detail, .mis-dossier-body {
    flex: 1;
    min-height: 50vh;
  }
}
"""
if '.chat-sidebar, .mem2-side, .mis-side {' not in content:
    content = content.replace(
        '.sidebar-overlay.open { display:block; z-index:119; }',
        '.sidebar-overlay.open { display:block; z-index:119; }\n' + css_mobile_sidebar
    )

# 3. Change onclick of statusHoneycomb
content = content.replace(
    'onclick="LaRuche.Router.go(\'settings\')"',
    'onclick="LaRuche.Chat.toggleSidebar()"'
)

# 4. Replace toggleSidebar and closeSidebarMobile in JS
js_old = """  function toggleSidebar() {
    document.getElementById('chatSidebar').classList.toggle('open');
    document.getElementById('sidebarOverlay').classList.toggle('open');
  }
  function closeSidebarMobile() {
    if(window.innerWidth<=768){
      document.getElementById('chatSidebar').classList.remove('open');
      document.getElementById('sidebarOverlay').classList.remove('open');
    }
  }"""

js_new = """  function toggleSidebar() {
    if(window.innerWidth > 900) return;
    var activePage = document.querySelector('.page.active');
    if(!activePage) return;
    var side;
    if(activePage.id === 'page-chat') side = document.getElementById('chatSidebar');
    else if(activePage.id === 'page-missions') side = activePage.querySelector('.mis-side');
    else if(activePage.id === 'page-memory') side = activePage.querySelector('.mem2-side');
    
    if(side) {
      side.classList.toggle('open');
      document.getElementById('sidebarOverlay').classList.toggle('open');
    }
  }
  function closeSidebarMobile() {
    if(window.innerWidth<=900){
      var activePage = document.querySelector('.page.active');
      if(activePage) {
        var side;
        if(activePage.id === 'page-chat') side = document.getElementById('chatSidebar');
        else if(activePage.id === 'page-missions') side = activePage.querySelector('.mis-side');
        else if(activePage.id === 'page-memory') side = activePage.querySelector('.mem2-side');
        if(side) side.classList.remove('open');
      }
      var overlay = document.getElementById('sidebarOverlay');
      if(overlay) overlay.classList.remove('open');
    }
  }"""

if 'function toggleSidebar() {' in content:
    content = content.replace(js_old, js_new)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Modifications applied successfully.")
