---
type: skill
name: weather_forecast
description: Report the weather forecast for a city, with its source.
---

# Weather forecast

Answer "what is the weather in X" with real, current data. Your training prior knows
nothing about today, so this always goes through the network. Never answer from memory,
and never soften that by hedging: an invented forecast is worse than "let me check".

## The fast path, which is almost always enough

`wttr.in` returns preformatted text over plain HTTP. No parsing, no HTML, no extraction.

```
web_fetch(url="https://wttr.in/Paris?format=3")
web_fetch(url="https://wttr.in/Paris?format=4")
web_fetch(url="https://wttr.in/Paris")
```

- `?format=3` gives one line: place, symbol, temperature. `Paris: 22 C`. Right for a
  passing question.
- `?format=4` adds wind to the same line, as speed and direction. Not humidity.
- No query parameter gives the full three-day ASCII forecast, in three-hour slots, which
  is what to use when the user asks about tomorrow or the weekend.

Add `?lang=fr` for French output, which only changes the FULL forecast: the one-line
formats carry no words to translate. `?m` forces metric and `?u` forces imperial, which
matters because the default follows the country queried, so a US city answers in
Fahrenheit. Spaces in a city name become `+` or `%20`:
`https://wttr.in/New+York?format=3`.

The full forecast is ANSI-coloured. The escape codes are noise in a chat answer: read the
numbers out of it and write the answer yourself, do not paste the block.

Start here. Reach for the search path below only when this fails, or when the user needs
something wttr.in does not carry, such as an official weather warning.

## The search path

1. `web_search` with `query` set to `<city> weather forecast` (or `meteo <city>` for a
   French city). Optionally pin it with `allowed_domains`, for example
   `["meteofrance.com"]` or `["weather.com"]`.
2. Pick a result from a real meteorological service. Skip aggregators and SEO pages that
   restate the question.
3. `web_fetch` with that `url`. It takes ONE url, as a string, and returns clean text.
4. If the page comes back empty, it is JavaScript-rendered: `web_fetch` again with
   `render: true`.

**`read_extract` is not part of this.** It takes `path` and reads a LOCAL PDF, `.txt` or
`.md` file. Given a URL it answers `File not found`. The tool that reads a web page is
`web_fetch`.

## What to report

Give the user, in their own units and language:

- current conditions and temperature;
- the high and low for the period they asked about;
- wind, when it is strong enough to matter;
- precipitation, or any official warning;
- the source, named.

State the date the forecast is for. "Tomorrow" written on a Sunday means Monday to the
reader and nothing at all in a log read later.

## Traps

- **Ambiguous city names.** Springfield, Cambridge and Toledo each exist several times
  over. wttr.in picks one silently. When the country is not obvious, ask, or query
  `<city>,<country code>`: `https://wttr.in/Cambridge,GB?format=3`.
- **A cached answer is not a current one.** If the user asks twice in a day, fetch again.
- **Do not convert units for them.** Answer in the units of the country asked about,
  unless the user has shown a preference.
- **wttr.in rate limits.** Repeated calls for many cities start returning errors. Space
  them out, or move to the search path.

## Failure modes

**`web_fetch` on wttr.in returns nothing or an error page.** The service is down or is
rate limiting you. Go to the search path; do not retry in a loop.

**The search returns only aggregators.** Re-query with the country and the word
`forecast`, or pin `allowed_domains` to a national service: `meteofrance.com`,
`metoffice.gov.uk`, `weather.gov`.

**The page loads but holds no numbers.** It is JavaScript-rendered. `web_fetch` with
`render: true`, then `browser` with `action: "navigate"` if that still fails.

**The city is not found anywhere.** Ask the user to confirm the spelling and the country,
rather than answering about a same-named place elsewhere.
