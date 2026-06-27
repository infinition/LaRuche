import sys

with open('laruche-dashboard/src/templates/spa.html', 'r', encoding='utf-8') as f:
    lines = f.readlines()

fetch_models_start = -1
fetch_models_end = -1
panel_models_start = -1

for i, line in enumerate(lines):
    if 'async function fetchModels(){' in line:
        fetch_models_start = i
    if fetch_models_start != -1 and 'function setPreferredModel' in line:
        fetch_models_end = i
    if '<div class="panel" id="panel-models">' in line:
        panel_models_start = i

print(f"fetchModels: {fetch_models_start} to {fetch_models_end}")
print(f"panel-models: {panel_models_start}")

print("".join(lines[panel_models_start:panel_models_start+10]))
print("...")
print("".join(lines[fetch_models_start:fetch_models_end]))
