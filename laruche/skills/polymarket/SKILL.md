---
type: skill
name: polymarket
description: "Query Polymarket prediction markets: search, prices, orderbooks, history."
version: 1.0.0
author: Teknium
tags: [polymarket, prediction-markets, market-data, trading]
platforms: [linux, macos, windows]
tools: [shell_exec, execute_code]
scripts: [scripts/polymarket.py]
metadata:
  laruche:
    category: finance
    homepage: https://polymarket.com
---

# Polymarket - Prediction Market Data

Query prediction market data from Polymarket using their public REST APIs.
All endpoints are read-only and require **zero authentication**.

Use the bundled `scripts/polymarket.py` via `execute_code` (Python) or `shell_exec`.

## Key Concepts

- **Events** contain one or more **Markets** (1:many)
- **Markets** are binary - prices 0.00–1.00 ARE probabilities (0.65 → 65% likely)
- `outcomePrices` / `outcomes` / `clobTokenIds` are **double-encoded** JSON strings - parse with `json.loads(market['outcomePrices'])`
- `clobTokenIds`: [Yes token ID, No token ID] - used for price/orderbook queries
- `conditionId`: hex string - used for price history queries
- Volume is in USDC (US dollars)

## APIs

| API | Base URL | Purpose |
|-----|----------|---------|
| Gamma | `gamma-api.polymarket.com` | Discovery, search, browse |
| CLOB | `clob.polymarket.com` | Real-time prices, orderbooks, history |
| Data | `data-api.polymarket.com` | Trades, open interest |

Rate limits: 1,000–9,000 req/10s - no throttling needed.

## Script Commands

```bash
# Search markets by keyword
python3 scripts/polymarket.py search "bitcoin"

# Trending events by volume
python3 scripts/polymarket.py trending --limit 10

# Market details by slug
python3 scripts/polymarket.py market <slug>

# Full event with all nested markets
python3 scripts/polymarket.py event <slug>

# Current price for a token (clobTokenIds from market/event results)
python3 scripts/polymarket.py price <token_id>

# Orderbook depth
python3 scripts/polymarket.py book <token_id>

# Price history (conditionId from market results)
python3 scripts/polymarket.py history <condition_id> --interval all --fidelity 50

# Recent trades (optionally filtered by market conditionId)
python3 scripts/polymarket.py trades --limit 10 --market <condition_id>
```

## Standard Workflow

1. **Search** - `search "<query>"` → get events, slugs, current prices
2. **Present** - format `outcomePrices` as percentages, show volume
3. **Drill down** (if asked):
   - `market <slug>` → get `clobTokenIds` and `conditionId`
   - `price <token_id>` or `book <token_id>` for real-time depth
   - `history <condition_id>` for trend

## Output Format

The script auto-formats output:
- Prices as percentages: `Yes: 65.2% / No: 34.8%`
- Volumes as `$1.2M`, `$450K`
- History as ASCII bar chart

When relaying to user: preserve the question, probability, and volume.
Example: `"Will X happen?" - 65.2% Yes ($1.2M volume)`

## Limitations

- Read-only - placing trades requires EIP-712 wallet signatures (not supported)
- Some new markets may have empty price history
- Geographic restrictions apply to trading; read-only data is globally accessible
