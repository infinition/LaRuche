import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Replace width: 90% !important; with width: 90vw !important; min-width: 90vw !important;
content = content.replace(
    'width: 90% !important; max-width: none !important;',
    'width: 90vw !important; min-width: 90vw !important; max-width: 90vw !important;'
)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Updated sidebar width to use 90vw.")
