import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Replace max-width so 90% actually applies
content = content.replace(
    'width: 90% !important; max-width: 400px;',
    'width: 90% !important; max-width: none !important;'
)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Removed max-width constraint for mobile sidebars.")
