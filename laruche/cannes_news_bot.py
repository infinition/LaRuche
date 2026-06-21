import requests
import os

# Configuration - À remplacer par vos propres informations ou définir via des variables d'environnement
TELEGRAM_TOKEN = os.getenv('TELEGRAM_TOKEN', 'VOTRE_TOKEN_ICI')
CHAT_ID = os.getenv('TELEGRAM_CHAT_ID', 'VOTRE_CHAT_ID_ICI')

def fetch_cannes_news():
    # Cette fonction simule la récupération d'actualités. 
    # Pour une production réelle, utilisez une API de news ou un scraper.
    # Exemple avec une recherche simple (conceptuel) :
    news_summary = "📢 Actualités de Cannes :
    - Événement culturel majeur ce matin.
    - Ouverture de nouvelles expositions.
    - Météo clémente pour les activités en extérieur."
    return news_summary

def send_telegram_message(message):
    url = f"https://api.telegram.org/bot{TELEGRAM_TOKEN}/sendMessage"
    payload = {"chat_id": CHAT_ID, "text": message}
    try:
        response = requests.post(url, json=payload)
        response.raise_for_status()
    except Exception as e:
        print(f"Erreur lors de l'envoi Telegram : {e}")

if __name__ == "__main__":
    news = fetch_cannes_news()
    send_telegram_message(news)