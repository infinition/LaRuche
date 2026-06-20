import re

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    lines = f.readlines()

print("--- PAGE CONTAINERS ---")
for i, line in enumerate(lines):
    if 'id="' in line and ('chat' in line or 'memory' in line) and ('class="page"' in line or 'class="view"' in line or 'class="panel"' in line):
        print(f"Line {i+1}: {line.strip()}")

print("\n--- WEBSOCKET HANDLER ---")
for i, line in enumerate(lines):
    if 'ws.onmessage' in line or 'onmessage =' in line:
        print(f"Line {i+1}: {line.strip()}")
        
print("\n--- NAVIGATOR ---")
for i, line in enumerate(lines):
    if 'function showPage' in line or 'function switchTab' in line or 'const pages' in line:
        print(f"Line {i+1}: {line.strip()}")
