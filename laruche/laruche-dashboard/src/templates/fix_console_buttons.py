import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Remove .filter-btn from the touch target rule
content = content.replace(
    '.session-item, .suggested-prompt, .filter-btn { min-height:44px; }',
    '.session-item, .suggested-prompt { min-height:44px; }'
)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Removed .filter-btn from coarse touch target sizing.")
