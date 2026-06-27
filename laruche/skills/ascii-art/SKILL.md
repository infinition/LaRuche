---
type: skill
name: ascii-art
description: "ASCII art: banners, cowsay, boxes, image-to-ASCII, QR, weather."
version: 4.1.0
author: 0xbyt4
license: MIT
dependencies: []
platforms: [linux, macos, windows]
tools: [shell_exec, execute_code, web_fetch]
metadata:
  laruche:
    tags: [ASCII, Art, Banners, Creative, Unicode, Text-Art, pyfiglet, figlet, cowsay, boxes]

---

# ASCII Art Skill

Multiple tools for ASCII art needs. All local CLI or free REST APIs — no API keys required.

## Decision Flow

1. **Text banner** → pyfiglet (local, 571 fonts) or asciified API (no install, 250+ fonts)
2. **Message in character art** → cowsay
3. **Decorative border/frame** → boxes (pipeable from pyfiglet/asciified)
4. **Subject art** (cat, dragon, rocket…) → ascii.co.uk via curl + Python parse
5. **Image → ASCII** → ascii-image-converter (PNG/JPEG/GIF/WEBP/URL) or jp2a (JPEG only)
6. **QR code** → qrenco.de via curl
7. **Weather/moon art** → wttr.in via curl
8. **Custom/creative** → LLM generation with Unicode palette
9. **Tool missing** → install it or fall back to next option

---

## Tool 1: pyfiglet (local, 571 fonts)

```bash
pip install pyfiglet --break-system-packages -q
python3 -m pyfiglet "YOUR TEXT" -f slant
python3 -m pyfiglet "TEXT" -f doom -w 80    # set width
python3 -m pyfiglet --list_fonts             # list all fonts
```

| Style | Font | Best for |
|-------|------|----------|
| Clean & modern | `slant` | Project names, headers |
| Bold & blocky | `doom` | Titles, logos |
| Big & readable | `big` | Banners |
| Classic banner | `banner3` | Wide displays |
| Compact | `small` | Subtitles |
| Cyberpunk | `cyberlarge` | Tech themes |
| 3D effect | `3-d` | Splash screens |

**Tips:** Short text (1-8 chars) → `doom`/`block`; long text → `small`/`mini`.

---

## Tool 2: asciified API (remote, no install)

```bash
curl -s "https://asciified.thelicato.io/api/v2/ascii?text=Hello+World"
curl -s "https://asciified.thelicato.io/api/v2/ascii?text=Hello&font=Slant"
curl -s "https://asciified.thelicato.io/api/v2/ascii?text=Hello&font=Star+Wars"
curl -s "https://asciified.thelicato.io/api/v2/fonts"   # list all fonts (JSON array)
```

URL-encode spaces as `+`. Font names are case-sensitive. Response is plain text.

---

## Tool 3: cowsay

```bash
sudo apt install cowsay -y   # Debian/Ubuntu | brew install cowsay (macOS)

cowsay "Hello World"
cowsay -f tux "Linux rules"   # Tux; -f dragon, -f tux, -f sheep…
cowthink "Hmm..."             # thought bubble
cowsay -l                     # list 50+ characters
cowsay -e "OO" -T "U " "Msg" # custom eyes + tongue
```

Eye modifiers: `-b` (borg), `-d` (dead), `-g` (greedy), `-s` (stoned).

---

## Tool 4: boxes (70+ border designs)

```bash
sudo apt install boxes -y   # brew install boxes (macOS)

echo "Hello" | boxes                     # default
echo "Hello" | boxes -d stone
echo "Hello" | boxes -d parchment
echo "Hello" | boxes -d cat
echo "Hello" | boxes -d c-cmt           # C-style comment block
echo "Hello" | boxes -a c              # center text
boxes -l                               # list all designs
```

Combine with pyfiglet or asciified:

```bash
python3 -m pyfiglet "LARUCHE" -f slant | boxes -d stone
curl -s "https://asciified.thelicato.io/api/v2/ascii?text=LARUCHE&font=Slant" | boxes -d stone
```

---

## Tool 5: TOIlet (colored text art)

ANSI color effects. Works in terminals; may not render in plain text contexts.

```bash
sudo apt install toilet toilet-fonts -y   # brew install toilet (macOS)

toilet "Hello World"
toilet -f bigmono12 "Hello"
toilet --gay "Rainbow!"       # rainbow
toilet --metal "Metal!"       # metallic
toilet -F border "Bordered"
toilet -F border --gay "Fancy!"
toilet -F list                # list all filters
```

Filters: `crop`, `gay`, `metal`, `flip`, `flop`, `180`, `left`, `right`, `border`

---

## Tool 6: Image to ASCII

**Option A: ascii-image-converter** (recommended — PNG/JPEG/GIF/WEBP/URLs)

```bash
sudo snap install ascii-image-converter
# OR: go install github.com/TheZoraiz/ascii-image-converter@latest

ascii-image-converter image.png
ascii-image-converter image.png -C               # color
ascii-image-converter image.png -d 60,30         # set dimensions
ascii-image-converter image.png -b               # braille chars
ascii-image-converter https://url/image.jpg      # direct URL
ascii-image-converter image.png --save-txt out   # save as text
```

**Option B: jp2a** (lightweight, JPEG only)

```bash
sudo apt install jp2a -y
jp2a --width=80 image.jpg
jp2a --colors image.jpg
```

---

## Tool 7: Pre-Made Art — ascii.co.uk

URL pattern: `https://ascii.co.uk/art/{subject}` — e.g. `cat`, `dragon`, `rocket`, `skull`, `robot`. Preserve artist signatures.

```bash
curl -s 'https://ascii.co.uk/art/cat' -o /tmp/ascii_art.html
```

```python
import re, html
with open('/tmp/ascii_art.html') as f:
    text = f.read()
arts = re.findall(r'<pre[^>]*>(.*?)</pre>', text, re.DOTALL)
for art in arts:
    clean = re.sub(r'<[^>]+>', '', art)
    clean = html.unescape(clean).strip()
    if len(clean) > 30:
        print(clean)
        print('\n---\n')
```

**GitHub Octocat (bonus):** `curl -s https://api.github.com/octocat`

---

## Tool 8: Fun ASCII via curl

```bash
curl -s "qrenco.de/https://example.com"   # QR code
curl -s "wttr.in/London"                  # weather with ASCII graphics
curl -s "wttr.in/Moon"                    # moon phase
curl -s "v2.wttr.in/London"              # detailed weather
```

---

## Tool 9: LLM-Generated Custom Art (Fallback)

Use when no tool fits. Character palette:

**Box Drawing:** `╔ ╗ ╚ ╝ ║ ═ ╠ ╣ ╦ ╩ ╬ ┌ ┐ └ ┘ │ ─ ├ ┤ ┬ ┴ ┼ ╭ ╮ ╰ ╯`

**Block Elements:** `░ ▒ ▓ █ ▄ ▀ ▌ ▐ ▖ ▗ ▘ ▝ ▚ ▞`

**Geometric & Symbols:** `◆ ◇ ◈ ● ○ ◉ ■ □ ▲ △ ▼ ▽ ★ ☆ ✦ ✧ ◀ ▶ ◁ ▷ ⬡ ⬢ ⌂`

Constraints: max 60 chars wide (terminal-safe), max 15 lines for banners / 25 for scenes. Monospace only.
