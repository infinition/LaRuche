import urllib.request, json; print(json.loads(urllib.request.urlopen('http://localhost:3000/api/skills/web-research').read().decode('utf-8')))
