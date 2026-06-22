import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# We want to change the CSS for .header-nav a
# Original:
# .header-nav a {
#   padding:6px 14px; font-size:12px; font-weight:600; color:var(--text-muted);
#   text-decoration:none; border-bottom:2px solid transparent;
#   transition:all var(--transition-fast); text-transform:uppercase; letter-spacing:.5px;
# }

if '.header-nav a {\n  padding:6px 14px;' in content:
    content = content.replace(
        '.header-nav a {\n  padding:6px 14px;',
        '.header-nav a {\n  display:flex; align-items:center; justify-content:center;\n  padding:6px 14px;'
    )

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Flex alignment added to .header-nav a")
