---
type: skill
name: ascii-art
description: Render text or an image as ASCII art for terminal-friendly output.
---

# ASCII art

Text that has to survive a terminal, a log file, a commit message or a plain-text channel,
where an image cannot go. A banner for a CLI, a QR code somebody can scan off the screen,
a picture reduced to characters.

Everything here is a local CLI or a keyless HTTP endpoint. Nothing needs an account.

## Pick by what you were asked for

| You want | Use |
|---|---|
| A text banner | `pyfiglet` locally, or the asciified endpoint with no install |
| A character saying something | `cowsay` |
| A frame around existing output | `boxes`, piped |
| Coloured terminal text | `toilet` |
| An image turned into characters | `ascii-image-converter`, or `jp2a` for JPEG |
| A QR code | `qrenco.de` |
| Weather or a moon phase | `wttr.in` |
| Something no tool covers | draw it yourself, palette at the end |

## Banners

Locally, which is fastest and works offline:

```bash
pip install pyfiglet
python -m pyfiglet "LARUCHE" -f slant
python -m pyfiglet "LARUCHE" -f doom -w 80
python -m pyfiglet --list_fonts | head -40
```

Without installing anything:

```bash
curl -s "https://asciified.thelicato.io/api/v2/ascii?text=LaRuche&font=Slant"
curl -s "https://asciified.thelicato.io/api/v2/fonts"
```

Spaces become `+` in the query string. Font names are case sensitive: `Slant` works,
`slant` does not. The response is plain text and **arrives without a trailing newline**,
so the next thing you print lands on the same line. Add one.

Choosing a font is mostly about length. Short and punchy takes a heavy face; anything long
needs a narrow one, or it wraps and becomes unreadable:

| Text | Font |
|---|---|
| 1 to 8 characters | `doom`, `block`, `3-d` |
| a normal word | `slant`, `big` |
| a phrase | `small`, `mini` |

Check the width before showing it. Terminal art over 80 columns wraps, and wrapped ASCII
art is just noise.

## Frames, characters, colour

```bash
cowsay "the build is green"
cowsay -f tux "hello"
cowthink "hmm"
cowsay -l                     # every available character
```

```bash
echo "release 2.4.1" | boxes -d stone
echo "note" | boxes -d c-cmt          # a C comment block
boxes -l                              # every design
```

They compose, which is the point:

```bash
python -m pyfiglet "LARUCHE" -f slant | boxes -d stone
```

`toilet` adds ANSI colour:

```bash
toilet -f bigmono12 --metal "LARUCHE"
toilet -F border "framed"
```

**Colour is escape codes, not characters.** In a terminal it looks good; in a file, a commit
message or a chat message it becomes `\x1b[35m` litter. Use `toilet` only when the
destination is a terminal, and `pyfiglet` everywhere else.

On Debian and Ubuntu these come from `apt install cowsay boxes toilet`, on macOS from
`brew install cowsay boxes toilet`. All three need root to install. If you do not have it,
the asciified endpoint covers banners and there is no local substitute for the others: say
so rather than trying to install for ten minutes.

## Images

```bash
ascii-image-converter image.png
ascii-image-converter image.png -C            # colour
ascii-image-converter image.png -d 60,30      # columns,rows
ascii-image-converter image.png -b            # braille, much finer
ascii-image-converter https://example.com/photo.jpg
```

Install with `go install github.com/TheZoraiz/ascii-image-converter@latest`, or
`snap install ascii-image-converter`. For JPEG only, `jp2a --width=80 image.jpg` is
smaller and usually already packaged.

Set the width explicitly, always. The default guesses from the terminal, and the terminal
it guesses is not the one the user is reading in.

Photographs convert badly. High contrast with a clear silhouette (a logo, an icon, a
diagram) survives; a group photo becomes grey mush. Look at the result before sending it,
and if it is unreadable say so instead of shipping it.

## QR codes and weather

```bash
curl -s "https://qrenco.de/https://example.com"
curl -s "https://wttr.in/London"
curl -s "https://wttr.in/Moon"
```

A QR code needs its aspect ratio to survive. Anything that reflows lines, or renders in a
proportional font, breaks it. Send it inside a fenced code block, and tell the user it must
be viewed in a monospace font to scan.

For an actual weather answer rather than the picture, use the `weather_forecast` skill.

## Drawing it yourself

When nothing above fits, compose it from a palette. Keep every line the same visual width,
because misalignment is the only thing a reader notices.

```
box drawing   ╔ ╗ ╚ ╝ ║ ═ ╠ ╣ ╦ ╩ ╬ ┌ ┐ └ ┘ │ ─ ├ ┤ ┬ ┴ ┼ ╭ ╮ ╰ ╯
blocks        ░ ▒ ▓ █ ▄ ▀ ▌ ▐ ▖ ▗ ▘ ▝ ▚ ▞
shapes        ◆ ◇ ● ○ ◉ ■ □ ▲ △ ▼ ▽ ★ ☆ ◀ ▶ ⬡ ⬢
```

Sixty columns maximum, fifteen lines for a banner and twenty-five for a scene. Monospace
is assumed by every character above: in a proportional font the alignment collapses
entirely.

## Traps

- **ANSI colour outside a terminal.** See above. This is the most common way this output
  gets ruined.
- **Width.** Over 80 columns it wraps. Check before sending.
- **Wide characters.** Box-drawing and block characters are single-width, but emoji and
  CJK are double-width and will not line up with them. Do not mix.
- **Fenced code blocks are mandatory.** Outside one, markdown renderers eat the spacing
  and collapse the art.
- **Fonts are case sensitive** in the asciified API.
- **Credit stays on the art.** Pre-made ASCII art often carries the artist's initials in a
  corner. Keep them.

## Failure modes

**`pyfiglet: No module named`.** Not installed. `pip install pyfiglet`, or fall back to the
asciified endpoint, which needs nothing.

**`cowsay: command not found` and no root.** No local fallback exists. Say so; do not spend
the turn attempting installs that will not work.

**The banner is wrapped and unreadable.** The font is too wide for the text. Move down the
font table, or shorten the text.

**The asciified endpoint returns an empty body.** The font name is wrong, or wrongly cased.
List the fonts and copy one exactly.

**The image came out as grey mush.** Too little contrast for the medium. Try `-b` for
braille, raise the dimensions, or tell the user this image will not reduce to characters.

**It looked right in the terminal and broken in the chat.** Colour codes, or no code fence.
Re-render without `toilet` and wrap it in a fence.
