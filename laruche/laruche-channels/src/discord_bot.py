"""LaRuche Discord Channel — connects a Discord bot to the Essaim agent.

Usage:
    export DISCORD_BOT_TOKEN="your-bot-token"
    pip install discord.py httpx
    python -m src.discord_bot
"""

import asyncio
import os
import sys

try:
    import discord
except ImportError:
    print("[Discord] ERROR: Install discord.py with: pip install discord.py")
    sys.exit(1)

import httpx

LARUCHE_URL = os.environ.get("LARUCHE_URL", "http://127.0.0.1:8419")
BOT_TOKEN = os.environ.get("DISCORD_BOT_TOKEN", "")
ALLOWED_CHANNEL_IDS = os.environ.get("DISCORD_ALLOWED_CHANNELS", "").split(",")

if not BOT_TOKEN:
    for p in ["channels-config.json", "../channels-config.json"]:
        if os.path.exists(p):
            try:
                dc = json.load(open(p)).get("discord", {})
                BOT_TOKEN = dc.get("bot_token", "")
                if dc.get("allowed_channels"): ALLOWED_CHANNEL_IDS = dc["allowed_channels"].split(",")
                if BOT_TOKEN: print(f"[Discord] Loaded token from {p}"); break
            except: pass
    if not BOT_TOKEN:
        try:
            dc = httpx.get(f"{LARUCHE_URL}/api/config/channels", timeout=5).json().get("discord", {})
            BOT_TOKEN = dc.get("bot_token", "")
            if dc.get("allowed_channels"): ALLOWED_CHANNEL_IDS = dc["allowed_channels"].split(",")
            if BOT_TOKEN: print("[Discord] Loaded token from LaRuche API")
        except: pass

if not BOT_TOKEN:
    print("[Discord] ERROR: Set DISCORD_BOT_TOKEN environment variable")
    sys.exit(1)

intents = discord.Intents.default()
intents.message_content = True
client = discord.Client(intents=intents)


async def query_agent(text: str) -> str:
    """Send a message to the LaRuche agent (full tools + memory)."""
    import re
    async with httpx.AsyncClient(timeout=120) as http:
        try:
            resp = await http.post(f"{LARUCHE_URL}/api/webhook", json={"prompt": text})
            if resp.status_code == 200:
                data = resp.json()
                if data.get("error"): return f"Error: {data['error']}"
                response = data.get("response", "")
                response = re.sub(r'<tool_call>[\s\S]*?</tool_call>', '', response)
                response = re.sub(r'<plan>[\s\S]*?</plan>', '', response)
                return response.strip() or "Done."
            return f"Error: {resp.status_code}"
        except Exception as e:
            return f"Error: {e}"


@client.event
async def on_ready():
    print(f"[Discord] Bot connected as {client.user}")
    print(f"[Discord] LaRuche URL: {LARUCHE_URL}")


@client.event
async def on_message(message):
    # Don't respond to self
    if message.author == client.user:
        return

    # Check if the bot is mentioned or if it's a DM
    is_dm = isinstance(message.channel, discord.DMChannel)
    is_mentioned = client.user in message.mentions

    if not is_dm and not is_mentioned:
        return

    # Check allowed channels
    if ALLOWED_CHANNEL_IDS and ALLOWED_CHANNEL_IDS[0]:
        if str(message.channel.id) not in ALLOWED_CHANNEL_IDS and not is_dm:
            return

    # Clean the message (remove bot mention)
    text = message.content
    for mention in message.mentions:
        text = text.replace(f"<@{mention.id}>", "").replace(f"<@!{mention.id}>", "")
    text = text.strip()

    if not text:
        return

    print(f"[Discord] {message.author}: {text[:80]}")

    # Show typing
    async with message.channel.typing():
        response = await query_agent(text)

    # Send response (Discord limit: 2000 chars)
    if len(response) <= 1900:
        await message.reply(response)
    else:
        # Split into chunks
        for i in range(0, len(response), 1900):
            chunk = response[i:i + 1900]
            if i == 0:
                await message.reply(chunk)
            else:
                await message.channel.send(chunk)

    print(f"[Discord] -> Replied ({len(response)} chars)")


def main():
    print("[Discord] Starting LaRuche Discord channel...")
    client.run(BOT_TOKEN)


if __name__ == "__main__":
    main()
