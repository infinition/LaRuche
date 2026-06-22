import re

filepath = 'C:/Users/infinition/Desktop/laruche-v2/laruche/laruche-dashboard/src/templates/spa.html'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update CSS for mobile sizes to make all input-action-btns the same size
content = content.replace(
    '.mic-btn { width:44px; height:44px; font-size:18px; }',
    '.input-action-btn { width:44px; height:44px; font-size:18px; min-width:44px; min-height:44px; }'
)

# 2. Show upload-btn on mobile by removing `.upload-btn { display:none; }`
content = content.replace(
    '.upload-btn { display:none; }',
    '/* .upload-btn { display:none; } */'
)

# 3. Make cwd-bar hidden by default
content = content.replace(
    '.cwd-bar {\n  display:flex;',
    '.cwd-bar {\n  display:none;'
)

# 4. Add the cwd-toggle-btn next to upload-btn
new_btn = """<button class="input-action-btn cwd-toggle-btn" id="cwdToggleBtn" onclick="const b=document.getElementById('cwdBar'); b.style.display = (b.style.display==='flex') ? 'none' : 'flex'; this.classList.toggle('active')" title="Afficher/Masquer le dossier de travail">
            <svg width="1.2em" height="1.2em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: middle;"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>
          </button>"""

# Find where to insert it (after upload-btn label)
# Let's find the closing label of upload-btn
upload_btn_end = """          </label>
          <label class="input-action-btn code-toggle-label" """

replacement_with_cwd_btn = f"""          </label>
          {new_btn}
          <label class="input-action-btn code-toggle-label" """

content = content.replace(upload_btn_end, replacement_with_cwd_btn)

# 5. Add active state CSS for cwd-toggle-btn (put it near code-toggle-label.active)
css_active = """.code-toggle-label.active { border-color:var(--amber); color:var(--amber); background:var(--amber-glow); }
.cwd-toggle-btn.active { border-color:var(--amber); color:var(--amber); background:var(--amber-glow); }"""

content = content.replace(
    '.code-toggle-label.active { border-color:var(--amber); color:var(--amber); background:var(--amber-glow); }',
    css_active
)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Mobile layout and cwd toggle fixes applied.")
