import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Make chat-main a container
if '.chat-main { flex:1; display:flex; flex-direction:column; overflow:hidden; position:relative; }' in content:
    content = content.replace(
        '.chat-main { flex:1; display:flex; flex-direction:column; overflow:hidden; position:relative; }',
        '.chat-main { flex:1; display:flex; flex-direction:column; overflow:hidden; position:relative; container-type: inline-size; }'
    )
else:
    # Fallback if already modified or slightly different
    content = content.replace('.chat-main {', '.chat-main { container-type: inline-size;')

# Convert @media (max-width: 768px) to @container (max-width: 650px) for the tray logic
tray_media_start = content.find('@media (max-width: 768px) {\n  .actions-tray {')
if tray_media_start != -1:
    content = content[:tray_media_start] + content[tray_media_start:].replace(
        '@media (max-width: 768px) {',
        '@container (max-width: 650px) {',
        1
    )

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Container query applied for actions tray.")
