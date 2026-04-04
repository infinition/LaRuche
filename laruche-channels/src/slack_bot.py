"""LaRuche Slack Channel — connects a Slack bot to the Essaim agent.

Uses Slack's Events API via Socket Mode (no public URL needed).

Usage:
    export SLACK_BOT_TOKEN="xoxb-..."
    export SLACK_APP_TOKEN="xapp-..."
    pip install slack-bolt httpx
    python -m src.slack_bot
"""

import os
import sys

try:
    from slack_bolt import App
    from slack_bolt.adapter.socket_mode import SocketModeHandler
except ImportError:
    print("[Slack] ERROR: Install slack-bolt: pip install slack-bolt")
    sys.exit(1)

import httpx

BOT_TOKEN = os.environ.get("SLACK_BOT_TOKEN", "")
APP_TOKEN = os.environ.get("SLACK_APP_TOKEN", "")
LARUCHE_URL = os.environ.get("LARUCHE_URL", "http://127.0.0.1:8419")

if not BOT_TOKEN or not APP_TOKEN:
    print("[Slack] ERROR: Set SLACK_BOT_TOKEN and SLACK_APP_TOKEN")
    sys.exit(1)

app = App(token=BOT_TOKEN)


def query_agent(text: str) -> str:
    """Synchronous query to LaRuche agent."""
    try:
        resp = httpx.post(
            f"{LARUCHE_URL}/infer",
            json={"prompt": text, "capability": "llm", "max_tokens": 4096, "temperature": 0.7},
            timeout=120,
        )
        if resp.status_code == 200:
            return resp.json().get("response", "No response")
        return f"Error: {resp.status_code}"
    except Exception as e:
        return f"Error: {e}"


@app.event("app_mention")
def handle_mention(event, say):
    """Handle @bot mentions in channels."""
    text = event.get("text", "")
    # Remove the bot mention
    text = " ".join(w for w in text.split() if not w.startswith("<@"))
    if not text.strip():
        say("How can I help?")
        return

    print(f"[Slack] Mention: {text[:80]}")
    response = query_agent(text.strip())
    say(response)
    print(f"[Slack] -> Replied ({len(response)} chars)")


@app.event("message")
def handle_dm(event, say):
    """Handle direct messages."""
    # Only respond to DMs (not channel messages without mention)
    if event.get("channel_type") != "im":
        return

    text = event.get("text", "")
    if not text.strip():
        return

    print(f"[Slack] DM: {text[:80]}")
    response = query_agent(text.strip())
    say(response)
    print(f"[Slack] -> Replied ({len(response)} chars)")


def main():
    print(f"[Slack] Starting LaRuche Slack channel...")
    print(f"[Slack] LaRuche URL: {LARUCHE_URL}")
    handler = SocketModeHandler(app, APP_TOKEN)
    handler.start()


if __name__ == "__main__":
    main()
