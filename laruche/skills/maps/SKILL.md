---
type: skill
name: maps
description: Geocode an address, find places nearby, or compute a route.
---

# Maps Skill

Location intelligence using free, open data sources. 8 commands, 46 POI
categories, zero dependencies (Python stdlib only), no API key required.

Data sources: OpenStreetMap/Nominatim, Overpass API, OSRM, TimeAPI.io.

## Prerequisites

Python 3.8+ (stdlib only - no pip installs needed).

```bash
MAPS=~/.laruche/skills/maps/scripts/maps_client.py
```

## Commands

### search - Geocode a place name

```bash
python3 $MAPS search "Eiffel Tower"
python3 $MAPS search "1600 Pennsylvania Ave, Washington DC"
```

Returns: lat, lon, display name, type, bounding box, importance score.

### reverse - Coordinates to address

```bash
python3 $MAPS reverse 48.8584 2.2945
```

Returns: full address breakdown (street, city, state, country, postcode).

### nearby - Find places by category

```bash
# By coordinates (e.g. from a location pin)
python3 $MAPS nearby 48.8584 2.2945 restaurant --limit 10
python3 $MAPS nearby 40.7128 -74.0060 hospital --radius 2000

# By address/city/zip/landmark - --near auto-geocodes
python3 $MAPS nearby --near "Times Square, New York" --category cafe
python3 $MAPS nearby --near "90210" --category pharmacy

# Multiple categories merged into one query
python3 $MAPS nearby --near "downtown austin" --category restaurant --category bar --limit 10
```

46 categories: restaurant, cafe, bar, hospital, pharmacy, hotel, guest_house,
camp_site, supermarket, atm, gas_station, parking, museum, park, school,
university, bank, police, fire_station, library, airport, train_station,
bus_stop, church, mosque, synagogue, dentist, doctor, cinema, theatre, gym,
swimming_pool, post_office, convenience_store, bakery, bookshop, laundry,
car_wash, car_rental, bicycle_rental, taxi, veterinary, zoo, playground,
stadium, nightclub.

Each result includes: `name`, `address`, `lat`/`lon`, `distance_m`,
`maps_url` (clickable Google Maps link), `directions_url` (Google Maps
directions from the search point), and promoted tags when available:
`cuisine`, `hours` (opening_hours), `phone`, `website`.

### distance - Travel distance and time

```bash
python3 $MAPS distance "Paris" --to "Lyon"
python3 $MAPS distance "New York" --to "Boston" --mode driving
python3 $MAPS distance "Big Ben" --to "Tower Bridge" --mode walking
```

Modes: `driving` (default), `walking`, `cycling`. Returns road distance,
duration, and straight-line distance for comparison.

### directions - Turn-by-turn navigation

```bash
python3 $MAPS directions "Eiffel Tower" --to "Louvre Museum" --mode walking
python3 $MAPS directions "JFK Airport" --to "Times Square" --mode driving
```

Returns numbered steps with instruction, distance, duration, road name, and
maneuver type (turn, depart, arrive, etc.).

### timezone - Timezone for coordinates

```bash
python3 $MAPS timezone 48.8584 2.2945
python3 $MAPS timezone 35.6762 139.6503
```

Returns timezone name, UTC offset, and current local time.

### area - Bounding box and area for a place

```bash
python3 $MAPS area "Manhattan, New York"
python3 $MAPS area "London"
```

Returns bounding box coordinates, width/height in km, and approximate area.
Useful as input for the `bbox` command.

### bbox - Search within a bounding box

```bash
python3 $MAPS bbox 40.75 -74.00 40.77 -73.98 restaurant --limit 20
```

Finds POIs within a geographic rectangle. Run `area` first to get bounding box
coordinates for a named place.

## Working With Location Pins

When a user shares a location (latitude/longitude), pass coordinates directly
to `nearby`:

```bash
python3 $MAPS nearby 36.17 -115.14 cafe --radius 1500
```

Present results as a numbered list with names, distances, and `maps_url` so
the user gets a tap-to-open link. For "open now?" questions, check the `hours`
field; if missing or unclear, verify with `web_search` since OSM hours are
community-maintained and may be stale.

## Pitfalls

- Nominatim ToS: max 1 req/s - handled automatically by the script
- `nearby` requires lat/lon OR `--near "<address>"` - not both, not neither
- `distance` and `directions` use `--to` for destination (not positional arg)
- OSRM routing coverage is best for Europe and North America
- Overpass API can be slow at peak hours; script auto-falls back between mirrors
  (overpass-api.de → overpass.kumi.systems)
- Zip code alone may be ambiguous globally - include country/state when needed

## Verification

```bash
python3 $MAPS search "Statue of Liberty"
# Expected: lat ~40.689, lon ~-74.044

python3 $MAPS nearby --near "Times Square" --category restaurant --limit 3
# Expected: list of restaurants within ~500m of Times Square
```
