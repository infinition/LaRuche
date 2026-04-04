"""LaRuche Telegram Channel — connects a Telegram bot to the Essaim agent.

Forwards messages from Telegram to LaRuche via WebSocket, and sends
agent responses back to the Telegram chat.

Usage:
    export TELEGRAM_BOT_TOKEN="your-bot-token"
    python -m src.telegram
"""

import asyncio
import json
import os
import sys

import httpx

BOT_TOKEN = os.environ.get("TELEGRAM_BOT_TOKEN", "")
LARUCHE_URL = os.environ.get("LARUCHE_URL", "http://127.0.0.1:8419")
ALLOWED_CHAT_IDS = os.environ.get("TELEGRAM_ALLOWED_CHATS", "").split(",")

if not BOT_TOKEN:
    print("[Telegram] ERROR: Set TELEGRAM_BOT_TOKEN environment variable")
    sys.exit(1)

API = f"https://api.telegram.org/bot{BOT_TOKEN}"


async def send_telegram(chat_id: int, text: str, parse_mode: str = "Markdown"):
    """Send a message to Telegram."""
    async with httpx.AsyncClient() as client:
        # Try Markdown first, fallback to plain text
        try:
            resp = await client.post(f"{API}/sendMessage", json={
                "chat_id": chat_id,
                "text": text,
                "parse_mode": parse_mode,
            })
            if resp.status_code != 200:
                # Fallback: send without parse_mode
                await client.post(f"{API}/sendMessage", json={
                    "chat_id": chat_id,
                    "text": text,
                })
        except Exception as e:
            print(f"[Telegram] Send error: {e}")


async def send_typing(chat_id: int):
    """Send typing indicator."""
    async with httpx.AsyncClient() as client:
        await client.post(f"{API}/sendChatAction", json={
            "chat_id": chat_id,
            "action": "typing",
        })


async def query_agent(text: str, session_id: str = None) -> str:
    """Send a message to the LaRuche agent via HTTP and get the response."""
    # Use the /infer endpoint for simplicity (avoids WebSocket complexity)
    async with httpx.AsyncClient(timeout=120) as client:
        try:
            resp = await client.post(f"{LARUCHE_URL}/infer", json={
                "prompt": text,
                "capability": "llm",
                "max_tokens": 4096,
                "temperature": 0.7,
            })
            if resp.status_code == 200:
                data = resp.json()
                return data.get("response", "No response")
            else:
                return f"Error: {resp.status_code}"
        except Exception as e:
            return f"Error connecting to LaRuche: {e}"


async def poll_updates():
    """Long-poll Telegram for new messages."""
    offset = 0
    print(f"[Telegram] Bot started. Listening for messages...")
    print(f"[Telegram] LaRuche URL: {LARUCHE_URL}")

    if ALLOWED_CHAT_IDS and ALLOWED_CHAT_IDS[0]:
        print(f"[Telegram] Allowed chat IDs: {ALLOWED_CHAT_IDS}")
    else:
        print(f"[Telegram] WARNING: No chat ID restriction — anyone can use the bot!")

    async with httpx.AsyncClient(timeout=60) as client:
        while True:
            try:
                resp = await client.get(f"{API}/getUpdates", params={
                    "offset": offset,
                    "timeout": 30,
                })

                if resp.status_code != 200:
                    print(f"[Telegram] API error: {resp.status_code}")
                    await asyncio.sleep(5)
                    continue

                data = resp.json()
                if not data.get("ok"):
                    print(f"[Telegram] API not ok: {data}")
                    await asyncio.sleep(5)
                    continue

                for update in data.get("result", []):
                    offset = update["update_id"] + 1

                    message = update.get("message", {})
                    chat_id = message.get("chat", {}).get("id")
                    text = message.get("text", "")
                    user = message.get("from", {}).get("first_name", "Unknown")

                    if not text or not chat_id:
                        continue

                    # Check allowlist
                    if ALLOWED_CHAT_IDS and ALLOWED_CHAT_IDS[0]:
                        if str(chat_id) not in ALLOWED_CHAT_IDS:
                            await send_telegram(chat_id, "Access denied. Your chat ID is not authorized.")
                            continue

                    print(f"[Telegram] {user} ({chat_id}): {text[:80]}")

                    # Send typing indicator
                    await send_typing(chat_id)

                    # Query agent
                    response = await query_agent(text)

                    # Send response (split if too long for Telegram's 4096 char limit)
                    if len(response) <= 4000:
                        await send_telegram(chat_id, response)
                    else:
                        # Split into chunks
                        for i in range(0, len(response), 4000):
                            chunk = response[i:i + 4000]
                            await send_telegram(chat_id, chunk)

                    print(f"[Telegram] -> Replied ({len(response)} chars)")

            except httpx.ReadTimeout:
                continue  # Normal for long-polling
            except Exception as e:
                print(f"[Telegram] Error: {e}")
                await asyncio.sleep(5)


def main():
    print("[Telegram] Starting LaRuche Telegram channel...")
    asyncio.run(poll_updates())


if __name__ == "__main__":
    main()
