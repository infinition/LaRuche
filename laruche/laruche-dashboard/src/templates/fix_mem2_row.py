import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Fix .mem2-row display: flex
content = content.replace(
    '.mem2-row { align-items:center;',
    '.mem2-row { display:flex; align-items:center;'
)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Added display:flex to .mem2-row.")
