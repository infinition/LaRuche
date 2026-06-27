---
type: skill
name: openhue
description: "Control Philips Hue lights, scenes, rooms via OpenHue CLI."
version: 1.0.0
author: community
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec]
metadata:
  laruche:
    tags: [Smart-Home, Hue, Lights, IoT, Automation]
    homepage: https://www.openhue.io/cli
prerequisites:
  commands: [openhue]
---

# OpenHue CLI

Control Philips Hue lights and scenes via a Hue Bridge from the terminal.

## Install

```bash
# Linux (pre-built binary)
curl -sL https://github.com/openhue/openhue-cli/releases/latest/download/openhue-linux-amd64 \
  -o ~/.local/bin/openhue && chmod +x ~/.local/bin/openhue

# macOS
brew install openhue/cli/openhue-cli
```

First run: press the button on your Hue Bridge to pair. The bridge must be on the same local network.

## Discovery

```bash
openhue get light    # list all lights with exact names
openhue get room     # list rooms
openhue get scene    # list scenes
```

Light and room names are case-sensitive — always verify with `openhue get`.

## Control Lights

```bash
openhue set light "Bedroom Lamp" --on
openhue set light "Bedroom Lamp" --off
openhue set light "Bedroom Lamp" --on --brightness 50        # 0–100
openhue set light "Bedroom Lamp" --on --temperature 300      # 153 (warm) – 500 (cool) mirek
openhue set light "Bedroom Lamp" --on --color red
openhue set light "Bedroom Lamp" --on --rgb "#FF5500"        # color-capable bulbs only
```

## Control Rooms

```bash
openhue set room "Bedroom" --off
openhue set room "Bedroom" --on --brightness 30
```

## Scenes

```bash
openhue set scene "Relax" --room "Bedroom"
openhue set scene "Concentrate" --room "Office"
```

## Useful Presets

```bash
# Bedtime: dim warm
openhue set room "Bedroom" --on --brightness 20 --temperature 450

# Work: bright cool
openhue set room "Office" --on --brightness 100 --temperature 250

# Movie: dim living room
openhue set room "Living Room" --on --brightness 10

# All off (adapt room names to your setup)
for room in "Bedroom" "Office" "Living Room"; do openhue set room "$room" --off; done
```

## Notes

- Bridge and the machine running LaRuche must share the same local network.
- Colors (`--color`, `--rgb`) only work on color-capable bulbs, not white-only models.
- Pair well with `cron_create` for scheduled lighting (dim at bedtime, bright at sunrise).
