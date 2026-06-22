import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# We need to insert a rule for .toast-container in the mobile media query
# Or just right after the toast CSS block if we define a generic media query
toast_css_addition = """
@media (max-width: 768px) {
  .toast-container {
    bottom: calc(76px + var(--safe-bottom, env(safe-area-inset-bottom, 0px)));
    right: 12px;
    left: 12px;
    align-items: center;
  }
}
"""

# Let's insert this right after `.toast-err { ... }`
if '.toast-err { background:rgba(239,68,68,.12); border-color:rgba(239,68,68,.3); color:var(--red); }' in content:
    content = content.replace(
        '.toast-err { background:rgba(239,68,68,.12); border-color:rgba(239,68,68,.3); color:var(--red); }',
        '.toast-err { background:rgba(239,68,68,.12); border-color:rgba(239,68,68,.3); color:var(--red); }\n' + toast_css_addition
    )

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Toast CSS fixed for mobile tabs avoidance.")
