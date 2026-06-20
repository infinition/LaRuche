import re

html = open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8').read()

with open('laruche-dashboard/src/templates/out.txt', 'w', encoding='utf-8') as f:
    f.write("--- Providers fetching ---\n")
    for m in re.finditer(r'(async )?function load\w+\(el\)\s*\{.*?api/profiles.*?\}', html, re.DOTALL | re.IGNORECASE):
        f.write(m.group(0) + "\n\n")

    f.write("\n\n--- fetchModels in spa.html ---\n")
    for m in re.finditer(r'(async )?function fetchModels.*?\n  \}', html, re.DOTALL | re.IGNORECASE):
        f.write(m.group(0) + "\n\n")
