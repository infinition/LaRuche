import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. We need to wrap the action buttons in <div class="actions-tray">
# The buttons to wrap: micBtn, upload-btn, cwdToggleBtn, codingModeToggle label

html_to_find = """<button class="input-action-btn mic-btn" id="micBtn" title="Cliquer pour enregistrer / arreter"><svg width="1.2em" height="1.2em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: middle;"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3z"></path><path d="M19 10v2a7 7 0 0 1-14 0v-2"></path><line x1="12" y1="19" x2="12" y2="22"></line></svg></button>
          <div class="honey-wave" id="honeyWave" style="display:none"><div class="bar"></div><div class="bar"></div><div class="bar"></div><div class="bar"></div><div class="bar"></div></div>
          /* .upload-btn { display:none; } */
          <label class="input-action-btn upload-btn" title="Joindre un fichier">"""

# Wait, `/* .upload-btn { display:none; } */` is NOT in the HTML here, it was replaced in the CSS section earlier. 
# Let's extract the exact block from the HTML using regex to be safe.

# We look for `<div class="input-area">` and replace what's inside.

start_marker = '<div class="input-area">'
end_marker = '<textarea id="userInput"'

# We find the slice between start_marker and end_marker
idx_start = content.find(start_marker)
idx_end = content.find(end_marker)

if idx_start != -1 and idx_end != -1:
    inner_content = content[idx_start + len(start_marker):idx_end]
    
    # We want to extract honeyWave from the inner_content because it should NOT be inside the tray
    honey_wave_match = re.search(r'<div class="honey-wave"[^>]*>.*?</div></div>', inner_content)
    honey_wave_str = honey_wave_match.group(0) if honey_wave_match else ''
    
    # Remove honeyWave from inner_content
    if honey_wave_str:
        inner_content = inner_content.replace(honey_wave_str, '')

    # Build the new HTML structure
    new_html = f"""<div class="input-area" id="mainInputArea">
          <button class="tray-toggle-btn" id="trayToggleBtn" onclick="document.getElementById('mainInputArea').classList.toggle('show-tray'); this.classList.toggle('active')" title="Afficher les outils">
            <svg width="1.2em" height="1.2em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: middle;"><polyline points="9 18 15 12 9 6"></polyline></svg>
          </button>
          <div class="actions-tray">
            {inner_content.strip()}
          </div>
          {honey_wave_str}
          """
    
    # Replace it in content
    content = content[:idx_start] + new_html + content[idx_end:]


# Add the CSS for actions-tray and tray-toggle-btn
css_to_add = """
.tray-toggle-btn { display: none; }
.actions-tray { display: contents; }

@media (max-width: 768px) {
  .actions-tray {
    display: none;
    position: absolute;
    bottom: calc(100% + 8px);
    left: 16px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    padding: 8px;
    border-radius: var(--radius);
    gap: 8px;
    box-shadow: 0 -4px 12px rgba(0,0,0,0.3);
    z-index: 100;
  }
  #mainInputArea.show-tray .actions-tray {
    display: flex;
  }
  .tray-toggle-btn {
    display: flex; align-items: center; justify-content: center;
    width: 44px; height: 44px; min-width: 44px; font-size: 20px;
    background: var(--bg-input); border: 1px solid var(--border); border-radius: var(--radius);
    color: var(--text-dim); cursor: pointer; transition: all 0.2s;
  }
  .tray-toggle-btn.active {
    color: var(--amber); border-color: var(--amber); background: var(--amber-glow);
    transform: rotate(90deg);
  }
}
"""

# Insert CSS before `</style>`
content = content.replace('</style>', css_to_add + '\n</style>')


with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Chevron and action tray added successfully.")
