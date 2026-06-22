import requests
from xml.etree import ElementTree

def fetch_and_parse():
    url = 'https://export.arxiv.org/api/query?search_query=all:JEPa&max_results=50'
    response = requests.get(url)
    if response.status_code != 200:
        print(f'Error: {response.status_code}')
        return

    root = ElementTree.fromstring(response.content)
    # Namespaces
    ns = {'atom': 'http://www.w3.org/2005/Atom'}
    
    with open('jepa_results.md', 'w', encoding='utf-8') as f:
        f.write('# Résultats de recherche arXiv pour JEPa\n\n')
        
        for entry in root.findall('.//atom:entry', ns):
            title = entry.find('atom:title', ns).text
            summary = entry.find('atom:summary', ns).text
            link = entry.find('atom:link', ns).attrib.get('href')
            
            # Check for withdrawn
            if 'withdrawn' in summary.lower() or 'retracted' in summary.lower():
                continue
                
            f.write(f'## {title}\n')
            f.write(f'- **Lien**: {link}\n')
            f.write(f'- **Résumé**: {summary}\n\n')
            f.write('---\n\n')

if __name__ == '__main__':
    fetch_and_parse()