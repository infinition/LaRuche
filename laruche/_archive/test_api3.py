import urllib.request, json; print(json.loads(urllib.request.urlopen('http://localhost:8419/api/skills').read().decode('utf-8')))
