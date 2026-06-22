import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Fix Automatisations
content = content.replace(
    '<span class="tab-text">&#x2699; Automatisations</span>',
    '<span class="tab-icon">&#x2699;</span><span class="tab-text">Automatisations</span>'
)

# Fix Capacites
content = content.replace(
    '<span class="tab-text">&#x1F6E0; Capacites</span>',
    '<span class="tab-icon">&#x1F6E0;</span><span class="tab-text">Capacites</span>'
)

# Fix Memoire (add opening span)
content = content.replace(
    '<a href="#memory" data-page="memory"><svg',
    '<a href="#memory" data-page="memory"><span class="tab-icon"><svg'
)

# Fix Memoire (add closing span and wrap text)
content = content.replace(
    '</svg> Memoire</a>',
    '</svg></span><span class="tab-text">Memoire</span></a>'
)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Updates applied.")
