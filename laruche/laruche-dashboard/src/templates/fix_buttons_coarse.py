import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Replace the generic button styling for coarse pointers
content = content.replace(
    'button, select, .mic-btn, .send-btn { min-height:44px; min-width:44px; }',
    '.mic-btn, .send-btn, .input-action-btn, .sidebar-toggle, .sidebar-close { min-height:44px; min-width:44px; }'
)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Updated coarse pointer CSS rules.")
