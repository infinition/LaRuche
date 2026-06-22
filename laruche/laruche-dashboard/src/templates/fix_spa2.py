import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update CSS for Feed toggle button
feed_css_old = """.feed-toggle-btn {
  align-items:center; gap:5px;
  background:rgba(245,158,11,0.08); border:1px solid rgba(245,158,11,0.25);
  color:var(--amber); border-radius:8px; cursor:pointer;
  padding:5px 10px; font-size:12px; font-weight:600; line-height:1;
  transition:background .15s, border-color .15s; min-height: auto !important; min-width: auto !important;
}
.feed-toggle-btn:hover { background:rgba(245,158,11,0.16); border-color:rgba(245,158,11,0.45); }
.feed-toggle-btn .feed-toggle-ico { font-size:13px; }"""

feed_css_new = """.feed-toggle-btn {
  align-items:center; gap:5px;
  background:transparent; border:1px solid var(--border);
  color:var(--text-dim); border-radius:8px; cursor:pointer;
  padding:5px 10px; font-size:12px; font-weight:600; line-height:1;
  transition:background .15s, border-color .15s, color .15s; min-height: auto !important; min-width: auto !important;
}
.feed-toggle-btn:hover { color:var(--text); background:rgba(255,255,255,0.05); }
.feed-toggle-btn.active { background:rgba(245,158,11,0.08); border:1px solid rgba(245,158,11,0.25); color:var(--amber); }
.feed-toggle-btn.active:hover { background:rgba(245,158,11,0.16); border-color:rgba(245,158,11,0.45); }
.feed-toggle-btn .feed-toggle-ico { font-size:13px; }"""

content = content.replace(feed_css_old, feed_css_new)

# 2. Update JS for Feed openDrawer
open_drawer_old = """  function openDrawer(){
    var ov = document.getElementById('feedDrawerOverlay');
    if(!ov) return;
    open = true;
    ov.classList.add('open');"""

open_drawer_new = """  function openDrawer(){
    var ov = document.getElementById('feedDrawerOverlay');
    if(!ov) return;
    open = true;
    ov.classList.add('open');
    var btn = document.getElementById('feedToggleBtn');
    if(btn) btn.classList.add('active');"""

content = content.replace(open_drawer_old, open_drawer_new)

# 3. Update JS for Feed closeDrawer
close_drawer_old = """  function closeDrawer(){
    var ov = document.getElementById('feedDrawerOverlay');
    if(!ov) return;
    open = false;
    ov.classList.remove('open');"""

close_drawer_new = """  function closeDrawer(){
    var ov = document.getElementById('feedDrawerOverlay');
    if(!ov) return;
    open = false;
    ov.classList.remove('open');
    var btn = document.getElementById('feedToggleBtn');
    if(btn) btn.classList.remove('active');"""

content = content.replace(close_drawer_old, close_drawer_new)

# 4. Wrap text in tab-text
# I will use a regex to wrap the trailing text of the <a> tags inside .header-nav and .mobile-tabs
def wrap_tab_text(match):
    prefix = match.group(1)
    text = match.group(2).strip()
    return f'{prefix}<span class="tab-text">{text}</span></a>'

content = re.sub(r'(<a href="#[^"]+" data-page="[^"]+">(?:<span class="tab-icon">.*?</span>)?)([^<]+)</a>', wrap_tab_text, content, flags=re.DOTALL)

# Add Automatisations and Capacites tab-icon wrapper since they don't have it
content = content.replace('&#x2699; <span class="tab-text">', '<span class="tab-icon">&#x2699;</span> <span class="tab-text">')
content = content.replace('&#x1F6E0; <span class="tab-text">', '<span class="tab-icon">&#x1F6E0;</span> <span class="tab-text">')

# 5. Add CSS to hide tab-text on small screens
# I'll inject it into the main CSS block before `.header-nav`
css_inject = """@media (max-width: 1150px) {
  .header-nav a .tab-text { display: none; }
  .header-nav a { padding: 6px 10px; }
}
.mobile-tabs a .tab-text { display: none; }
"""
content = content.replace('.header-nav { display:flex; gap:0; }', css_inject + '.header-nav { display:flex; gap:0; }')

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Updates applied.")
