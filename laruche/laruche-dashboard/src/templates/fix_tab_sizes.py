import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Replace raw emojis with SVGs
svg_automatisations = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>'
content = content.replace(
    '<span class="tab-icon">&#x2699;</span>',
    f'<span class="tab-icon">{svg_automatisations}</span>'
)

svg_capacites = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path></svg>'
content = content.replace(
    '<span class="tab-icon">&#x1F6E0;</span>',
    f'<span class="tab-icon">{svg_capacites}</span>'
)

# 2. Add strict CSS rules to standardize `.tab-icon` and its SVGs
css_rules = """
/* Uniform sizes for tab icons */
.tab-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  min-width: 18px;
  min-height: 18px;
  margin-right: 6px;
}
.tab-icon svg {
  width: 100% !important;
  height: 100% !important;
  margin: 0 !important;
  display: block;
}
.mobile-tabs a .tab-icon { margin-right: 0; }
@media (max-width: 900px) {
  .header-nav a .tab-icon { margin-right: 0; }
}
"""

content = content.replace('</style>', css_rules + '\n</style>')

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Navigation tab icons standardized successfully.")
