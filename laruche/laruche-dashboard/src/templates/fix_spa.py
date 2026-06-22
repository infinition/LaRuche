import sys

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Fix user badge
user_badge_old = """    <div class="user-badge" id="userBadge" style="display:none">
      <div class="user-avatar" id="userAvatar"></div>
      <span class="user-name" id="userName"></span>
      <button class="user-logout" onclick="LaRuche.Auth.logout()" title="Deconnexion">&#x2716;</button>
    </div>"""

user_badge_new = """    <div class="user-badge" id="userBadge" style="display:none; position:relative; cursor:pointer;" onclick="document.getElementById('userDropdown').classList.toggle('show')">
      <div class="user-avatar" id="userAvatar"></div>
      <span class="user-name" id="userName"></span>
      <div id="userDropdown" class="user-dropdown">
        <div class="ud-item" onclick="LaRuche.Auth.logout()">Deconnexion</div>
      </div>
    </div>"""

if user_badge_old in content:
    content = content.replace(user_badge_old, user_badge_new)
else:
    print("WARNING: user badge old not found")

# 2. Fix chat sidebar backtick n
chat_sidebar_old = 'id="chatSidebar">`n        <button'
chat_sidebar_new = 'id="chatSidebar">\n        <button'

if chat_sidebar_old in content:
    content = content.replace(chat_sidebar_old, chat_sidebar_new)
else:
    print("WARNING: chat sidebar old not found")

# 3. Add Console and Audit to tabs
dash_tabs_old = """      <div class="dash-mob-tabs" id="dash-mob-tabs">
        <button class="active" data-tab="0">Swarm</button>
        <button data-tab="1">Models</button>
        <button data-tab="2">Resources</button>
      </div>"""

dash_tabs_new = """      <div class="dash-mob-tabs" id="dash-mob-tabs">
        <button class="active" data-tab="0">Swarm</button>
        <button data-tab="1">Models</button>
        <button data-tab="2">Resources</button>
        <button data-tab="3">Audit</button>
        <button data-tab="4">Console</button>
      </div>"""

if dash_tabs_old in content:
    content = content.replace(dash_tabs_old, dash_tabs_new)
else:
    print("WARNING: dash tabs old not found")


with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Replacement complete.")
