import re

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    html = f.read()

# 1. Inject payload.capability = 'code'
old_payload = "if(noThinkEnabled) payload.no_think = true;"
new_payload = "if(noThinkEnabled) payload.no_think = true;\n    var codingTog = document.getElementById('codingModeToggle');\n    if(codingTog && codingTog.checked) payload.capability = 'code';"

html = html.replace(old_payload, new_payload)

# 2. Add the toggle in the chat-input-wrapper
old_input = """<textarea id="userInput" placeholder="Ecrivez votre message..." rows="1"></textarea>"""
new_input = """<label class="think-toggle" title="Mode Coding (envoie le message au modele de code au lieu du LLM de chat)" style="margin-right:8px;">
            <input type="checkbox" id="codingModeToggle">
            <span>Code</span>
          </label>
          <textarea id="userInput" placeholder="Ecrivez votre message..." rows="1"></textarea>"""

html = html.replace(old_input, new_input)

with open('laruche-dashboard/src/templates/spa.html', 'w', encoding='utf-8') as f:
    f.write(html)
