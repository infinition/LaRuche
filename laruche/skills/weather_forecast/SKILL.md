---
type: skill
name: weather-forecast
description: Fetch and summarize weather forecast for any city via web search + extraction.
tools: [web_search, web_fetch, read_extract]
---

# Weather Forecast

## Procedure

1. **Search** (`web_search`): query `"<city> weather forecast"` or `"météo <city>"` for French cities. Prefer official or well-known sources in results (e.g. meteofrance.com, weather.com, accuweather.com, bbc.co.uk/weather).

2. **Pick the best URL**: select the first result from a recognized meteorological source. Skip aggregators, ads, or SEO-spam sites.

3. **Extract content** (`read_extract` on the chosen URL): this strips HTML boilerplate and returns clean text. Fall back to `web_fetch` if `read_extract` fails or returns empty.

4. **Parse key data** from the extracted text:
   - Current conditions (sun, rain, clouds, storms)
   - Min/max temperatures per period (today, tonight, next days)
   - Wind speed and direction
   - Alerts / vigilance warnings if any
   - Forecast horizon (hours or days covered)

5. **Present** a structured summary to the user: conditions, temperatures, wind, alerts, and cite the source URL.

## Failure handling

- If `read_extract` returns garbled/empty content, retry with `web_fetch` on the same URL and parse manually.
- If the top result is paywalled or JS-heavy, try the second result or a different source (e.g. `wttr.in/<city>` — plain-text weather, no parsing needed: `web_fetch("https://wttr.in/<city>?format=3")` returns a one-liner).
- For non-Latin city names, URL-encode or use the English transliteration in the query.

## Quick path (plain-text fallback)

For a fast, no-parse answer use wttr.in directly:
```
web_fetch("https://wttr.in/Paris?format=v2")
```
Returns a preformatted ASCII forecast — no extraction needed.
