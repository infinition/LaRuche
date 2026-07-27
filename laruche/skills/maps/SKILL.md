---
type: skill
name: maps
description: Geocode an address, find places nearby, or compute a route.
---

# Maps

Turn a place name into coordinates, coordinates into an address, and either into what is
around it or how to get from one to the other. Everything runs against free open data:
no API key, no account, no Python package to install.

One bundled script does all of it and answers in JSON on stdout, so you read fields
rather than parse prose.

## Prerequisites

Python 3.8 or later, standard library only. Nothing to install.

The script lives inside this skill folder, and skills sit under `skills/` relative to the
node's working directory. Resolve it once, to an ABSOLUTE path, and reuse it:

```bash
python skills/maps/scripts/maps_client.py search "Statue of Liberty"
```

Verify before anything else. Success is JSON containing latitude near `40.689` and
longitude near `-74.044`. An empty answer or a traceback means the path is wrong: find
the real one with `file_search` on `maps_client.py` rather than guessing another prefix.

## The eight commands

| Command | Answers | Needs |
|---|---|---|
| `search "<place>"` | where is this | a name or a full address |
| `reverse <lat> <lon>` | what is here | coordinates |
| `nearby` | what is around this | coordinates OR `--near "<place>"` |
| `distance "<a>" --to "<b>"` | how far, how long | two places |
| `directions "<a>" --to "<b>"` | how do I get there | two places |
| `timezone <lat> <lon>` | what time is it there | coordinates |
| `area "<place>"` | how big is it | a name |
| `bbox <s> <w> <n> <e> <category>` | what is inside this rectangle | four coordinates |

```bash
python skills/maps/scripts/maps_client.py search "1600 Pennsylvania Ave, Washington DC"
python skills/maps/scripts/maps_client.py reverse 48.8584 2.2945
python skills/maps/scripts/maps_client.py nearby 48.8584 2.2945 restaurant --limit 10
python skills/maps/scripts/maps_client.py nearby --near "90210" --category pharmacy
python skills/maps/scripts/maps_client.py distance "Paris" --to "Lyon" --mode driving
python skills/maps/scripts/maps_client.py directions "Big Ben" --to "Tower Bridge" --mode walking
python skills/maps/scripts/maps_client.py timezone 35.6762 139.6503
python skills/maps/scripts/maps_client.py area "Manhattan, New York"
```

`--mode` is `driving` (default), `walking` or `cycling`. `--radius` is metres, `--limit`
caps the result count. `--category` repeats to merge several searches into one query:

```bash
python skills/maps/scripts/maps_client.py nearby --near "downtown austin" \
  --category restaurant --category bar --limit 10
```

The 46 categories: restaurant, cafe, bar, hospital, pharmacy, hotel, guest_house,
camp_site, supermarket, atm, gas_station, parking, museum, park, school, university,
bank, police, fire_station, library, airport, train_station, bus_stop, church, mosque,
synagogue, dentist, doctor, cinema, theatre, gym, swimming_pool, post_office,
convenience_store, bakery, bookshop, laundry, car_wash, car_rental, bicycle_rental,
taxi, veterinary, zoo, playground, stadium, nightclub.

## What a `nearby` result carries

`name`, `address`, `lat`, `lon`, `distance_m`, a `maps_url` the user can tap, a
`directions_url` from the search point, and, when OpenStreetMap has them, `cuisine`,
`hours`, `phone` and `website`.

Present them as a numbered list with the name, the distance and the link. The distance is
what the user actually decides on, so lead with it, not with the address.

## Procedure for a location pin

When the user shares coordinates, do not geocode anything. Pass them straight through:

```bash
python skills/maps/scripts/maps_client.py nearby 36.17 -115.14 cafe --radius 1500
```

For "is it open now", read the `hours` field. When it is missing, say it is unknown and
offer to check: OpenStreetMap hours are contributed by volunteers and go stale silently.
Never state that a place is open on the strength of an OSM `hours` string alone.

## Traps

- **`nearby` needs coordinates OR `--near`, never both and never neither.** With neither
  it has no centre and cannot search.
- **`distance` and `directions` take the destination with `--to`**, not as a second
  positional argument. `distance "Paris" "Lyon"` does not do what it looks like.
- **A postcode alone is ambiguous worldwide.** `90210` resolves in the United States;
  a bare `75001` may not resolve where you expect. Add the city or the country.
- **Straight-line distance is not road distance.** The output gives both. Quote the road
  one unless the user asked how far apart they are as the crow flies.
- **OSRM coverage is strongest in Europe and North America.** A route across a region it
  models poorly returns something plausible and wrong. Sanity-check the duration.
- **Nominatim allows one request per second** and the script paces itself. Do not
  parallelise calls to it: the ban is on the IP, and it outlives the session.
- **The script identifies itself in its user agent**, which Nominatim's terms require.
  Leave `USER_AGENT` alone; a generic or absent one gets the whole machine blocked.

## Failure modes

**`nearby` returns `All Overpass mirrors failed`.** Both public Overpass servers timed
out, which happens at peak hours. It is one JSON error object, not a crash. Wait and
retry once, or answer from `search` and `reverse`, which use Nominatim and are unaffected.
Do not present the outage as "there are no cafes there".

**`search` returns nothing for a place that exists.** The query is too specific or too
local. Drop the building number, or add the city and country: OpenStreetMap matches on
the name it holds, not on the name a user would say.

**Coordinates come back for the wrong continent.** Latitude and longitude were passed in
the wrong order, or a sign was dropped. Longitude is second and is negative west of
Greenwich.

**Accented place names print as mojibake.** The script forces UTF-8 on stdout, so this
means something downstream re-encoded the output. Read the JSON directly rather than
piping it through another tool.

**A traceback instead of JSON.** The Python on PATH is older than 3.8, or the path to the
script is wrong. Check with `python --version`, then locate the script rather than
retrying the same command.
