---
type: skill
name: telegram_encoding_fix
description: Fix UTF-8 encoding issues in Telegram bot scripts (UnicodeEncodeError, garbled text, HTTP 400).
tools: [file_list, file_read, file_write, shell_exec, execute_code]
---

## When to use

Use when a Telegram bot script fails to send messages containing non-ASCII characters (accents, emoji, special symbols) - manifesting as `UnicodeEncodeError`, garbled text, or HTTP 400 errors from the Telegram API.

---

## Procedure

### 1. Locate the script

```
file_list("<bot_scripts_directory>")
```

Look for `send_telegram.py`, `telegram_bot.py`, `notify.py`, or similar. Check `skills/`, `scripts/`, or the project root if unsure.

### 2. Read and identify the defect

```
file_read("<path/to/send_telegram.py>")
```

| Pattern | Problem |
|---|---|
| `json.dumps(payload)` | Missing `ensure_ascii=False` - escapes non-ASCII as `\uXXXX` |
| `requests.post(..., data=json_str)` | No `Content-Type: application/json; charset=utf-8` header |
| `open(file, "w")` | Missing `encoding="utf-8"` |
| `str.encode()` with no arg | Defaults to system locale, not UTF-8 |

### 3. Apply the fix

Key patches to write back with `file_write`:

**Canonical send function:**
```python
import requests, json

def send_telegram(token, chat_id, text):
    url = f"https://api.telegram.org/bot{token}/sendMessage"
    payload = {"chat_id": chat_id, "text": text, "parse_mode": "HTML"}
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    headers = {"Content-Type": "application/json; charset=utf-8"}
    response = requests.post(url, data=body, headers=headers)
    response.raise_for_status()
    return response.json()
```

**File writes:**
```python
with open("output.txt", "w", encoding="utf-8") as f:
    f.write(text)
```

```
file_write("<path/to/send_telegram.py>", "<corrected_content>")
```

### 4. Verify

Test with a non-ASCII payload:

```
execute_code("""
import sys
sys.path.insert(0, "<script_dir>")
from send_telegram import send_telegram
send_telegram("<TOKEN>", "<CHAT_ID>", "Test: café, emoji 🎉, accents àéîõü")
""")
```

Or via shell:
```
shell_exec("python <path/to/send_telegram.py> --test")
```

Expected: HTTP 200, message appears correctly in Telegram.

---

## Pitfalls

- **Windows locale trap**: `sys.stdout` may default to a non-UTF-8 locale. Add `sys.stdout.reconfigure(encoding="utf-8")` at script top, or set `PYTHONIOENCODING=utf-8` in the environment before running.
- **Double-encoding**: Do not call `.encode("utf-8")` on a value that is already `bytes`.
- **Message length limit**: Telegram rejects messages over 4096 characters - split before sending.
- **`requests` shortcut**: `requests.post(url, json=payload)` auto-serializes but uses `ensure_ascii=True` on some older versions. Prefer the explicit `data=body` pattern above for maximum safety.
- **Token exposure**: Never hardcode the bot token. Use `${TELEGRAM_BOT_TOKEN}` (substituted from the secrets vault at execution time).
