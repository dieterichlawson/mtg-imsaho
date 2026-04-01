#!/usr/bin/env python3
"""Oracle text lookup and cache management for MTG card auditing.

Usage:
  python3 scripts/oracle_lookup.py lookup "Card Name"
  python3 scripts/oracle_lookup.py add-card "Card Name" --mana-cost "{1}{R}" --type-line "Enchantment — Aura" --oracle-text "The oracle text..." --source "Scryfall via WebSearch" --source-url "https://..."
  python3 scripts/oracle_lookup.py add-card-json '{"name": "Card Name", "mana_cost": "{1}{R}", ...}'
  python3 scripts/oracle_lookup.py add-ruling "Card Name" --date "2011-09-22" --text "The ruling..." --source "Scryfall rulings via WebSearch" --source-url "https://..."
  python3 scripts/oracle_lookup.py list
  python3 scripts/oracle_lookup.py lookup-dfc "Card Name" (for double-faced cards, shows both faces)
"""

import argparse
import json
import os
import sys
import time
import urllib.request
import urllib.error
import urllib.parse
from datetime import date

CACHE_PATH = os.path.join(os.path.dirname(__file__), "..", "data", "oracle_cache.json")
SCRYFALL_HEADERS = {
    "User-Agent": "MTG-Imsaho/1.0",
    "Accept": "application/json",
}
SCRYFALL_DELAY = 0.1  # 100ms between requests per Scryfall rate limit guidelines


def load_cache():
    with open(CACHE_PATH, "r") as f:
        return json.load(f)


def save_cache(cache):
    with open(CACHE_PATH, "w") as f:
        json.dump(cache, f, indent=2, ensure_ascii=False)
        f.write("\n")


def normalize_name(name):
    """Normalize card name for case-insensitive lookup."""
    return name.strip().lower()


def find_card(cache, name):
    """Find a card by name (case-insensitive)."""
    norm = normalize_name(name)
    for card_name, data in cache["cards"].items():
        if normalize_name(card_name) == norm:
            return card_name, data
    return None, None


def find_rulings(cache, name):
    """Find rulings by card name (case-insensitive)."""
    norm = normalize_name(name)
    for card_name, rulings in cache["rulings"].items():
        if normalize_name(card_name) == norm:
            return card_name, rulings
    return None, None


def cmd_lookup(args):
    cache = load_cache()
    canonical_name, card = find_card(cache, args.name)
    if not card:
        print(f"NOT FOUND: '{args.name}' is not in the oracle cache.")
        print("Use 'add-card' to cache it after fetching from an external source.")
        sys.exit(1)

    print(f"Name: {card['name']}")
    print(f"Mana Cost: {card.get('mana_cost', 'N/A')}")
    print(f"Type Line: {card.get('type_line', 'N/A')}")
    if card.get("power") or card.get("toughness"):
        print(f"P/T: {card.get('power', '?')}/{card.get('toughness', '?')}")
    print(f"Oracle Text: {card.get('oracle_text', 'N/A')}")
    if card.get("keywords"):
        print(f"Keywords: {', '.join(card['keywords'])}")
    print(f"Source: {card.get('source', 'unknown')}")
    print(f"Source URL: {card.get('source_url', 'N/A')}")
    print(f"Cached: {card.get('cached_at', 'unknown')}")

    # Show back face if present
    if card.get("back_face"):
        bf = card["back_face"]
        print(f"\n--- Back Face ---")
        print(f"Name: {bf['name']}")
        print(f"Type Line: {bf.get('type_line', 'N/A')}")
        if bf.get("power") or bf.get("toughness"):
            print(f"P/T: {bf.get('power', '?')}/{bf.get('toughness', '?')}")
        print(f"Oracle Text: {bf.get('oracle_text', 'N/A')}")

    # Show rulings if any
    _, rulings = find_rulings(cache, args.name)
    if rulings:
        print(f"\n--- Rulings ({len(rulings)}) ---")
        for r in rulings:
            print(f"[{r.get('date', '?')}] {r['text']}")
            print(f"  Source: {r.get('source', '?')} — {r.get('source_url', 'N/A')}")


def cmd_add_card(args):
    cache = load_cache()

    card_data = {
        "name": args.name,
        "mana_cost": args.mana_cost,
        "type_line": args.type_line,
        "oracle_text": args.oracle_text,
        "power": args.power,
        "toughness": args.toughness,
        "source": args.source,
        "source_url": args.source_url,
        "cached_at": str(date.today()),
    }
    if args.keywords:
        card_data["keywords"] = [k.strip() for k in args.keywords.split(",")]

    cache["cards"][args.name] = card_data
    save_cache(cache)
    print(f"Cached: {args.name}")


def cmd_add_card_json(args):
    cache = load_cache()
    card_data = json.loads(args.json)
    name = card_data["name"]
    card_data.setdefault("cached_at", str(date.today()))
    cache["cards"][name] = card_data
    save_cache(cache)
    print(f"Cached: {name}")


def cmd_add_back_face(args):
    cache = load_cache()
    canonical_name, card = find_card(cache, args.name)
    if not card:
        print(f"NOT FOUND: '{args.name}' — add the front face first.")
        sys.exit(1)

    card["back_face"] = {
        "name": args.back_name,
        "type_line": args.type_line,
        "oracle_text": args.oracle_text,
        "power": args.power,
        "toughness": args.toughness,
    }
    save_cache(cache)
    print(f"Added back face '{args.back_name}' to {canonical_name}")


def cmd_add_ruling(args):
    cache = load_cache()

    ruling = {
        "date": args.date,
        "text": args.text,
        "source": args.source,
        "source_url": args.source_url,
        "cached_at": str(date.today()),
    }

    if args.name not in cache["rulings"]:
        cache["rulings"][args.name] = []

    # Deduplicate by text
    existing_texts = {r["text"] for r in cache["rulings"][args.name]}
    if ruling["text"] not in existing_texts:
        cache["rulings"][args.name].append(ruling)
        save_cache(cache)
        print(f"Added ruling for {args.name}")
    else:
        print(f"Ruling already cached for {args.name} (duplicate)")


def cmd_list(args):
    cache = load_cache()
    cards = sorted(cache["cards"].keys())
    if not cards:
        print("Cache is empty.")
        return
    print(f"{len(cards)} cards cached:")
    for name in cards:
        card = cache["cards"][name]
        rulings_count = len(cache.get("rulings", {}).get(name, []))
        r_str = f" ({rulings_count} rulings)" if rulings_count else ""
        print(f"  {name} — {card.get('type_line', '?')}{r_str}")


def scryfall_get(url):
    """Make a GET request to Scryfall with required headers and rate limiting."""
    req = urllib.request.Request(url, headers=SCRYFALL_HEADERS)
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        print(f"Scryfall HTTP error {e.code}: {e.reason}", file=sys.stderr)
        if e.code == 404:
            return None
        raise
    except urllib.error.URLError as e:
        print(f"Network error: {e.reason}", file=sys.stderr)
        raise


def parse_scryfall_card(data):
    """Parse a Scryfall card object into our cache format."""
    scryfall_url = data.get("scryfall_uri", "")

    # DFC: card_faces present
    faces = data.get("card_faces", [])
    if faces:
        front = faces[0]
        card_data = {
            "name": front["name"],
            "mana_cost": front.get("mana_cost", ""),
            "type_line": front.get("type_line", ""),
            "oracle_text": front.get("oracle_text", ""),
            "power": front.get("power"),
            "toughness": front.get("toughness"),
            "source": "Scryfall API",
            "source_url": scryfall_url,
            "cached_at": str(date.today()),
        }
        if len(faces) > 1:
            back = faces[1]
            card_data["back_face"] = {
                "name": back["name"],
                "type_line": back.get("type_line", ""),
                "oracle_text": back.get("oracle_text", ""),
                "power": back.get("power"),
                "toughness": back.get("toughness"),
            }
    else:
        card_data = {
            "name": data["name"],
            "mana_cost": data.get("mana_cost", ""),
            "type_line": data.get("type_line", ""),
            "oracle_text": data.get("oracle_text", ""),
            "power": data.get("power"),
            "toughness": data.get("toughness"),
            "source": "Scryfall API",
            "source_url": scryfall_url,
            "cached_at": str(date.today()),
        }

    # Extract keywords from oracle text keywords array if present
    keywords = data.get("keywords", [])
    if keywords:
        card_data["keywords"] = keywords

    return card_data


def fetch_rulings(card_name, rulings_uri):
    """Fetch rulings from Scryfall and return as list of ruling dicts."""
    if not rulings_uri:
        return []
    time.sleep(SCRYFALL_DELAY)
    data = scryfall_get(rulings_uri)
    if not data:
        return []
    rulings = []
    for r in data.get("data", []):
        rulings.append({
            "date": r.get("published_at", ""),
            "text": r.get("comment", ""),
            "source": "Scryfall API",
            "source_url": rulings_uri,
            "cached_at": str(date.today()),
        })
    return rulings


def cmd_fetch(args):
    """Fetch a card from Scryfall API and cache it (card data + rulings)."""
    cache = load_cache()

    # Check cache first unless --force
    if not args.force:
        _, existing = find_card(cache, args.name)
        if existing:
            print(f"Already cached: {existing['name']} (use --force to re-fetch)")
            cmd_lookup_by_name(cache, args.name)
            return

    # Fetch card data
    encoded = urllib.parse.quote(args.name)
    url = f"https://api.scryfall.com/cards/named?fuzzy={encoded}"
    print(f"Fetching from Scryfall: {args.name}...")
    data = scryfall_get(url)
    if not data:
        print(f"NOT FOUND on Scryfall: '{args.name}'")
        sys.exit(1)

    card_data = parse_scryfall_card(data)
    cache["cards"][card_data["name"]] = card_data
    print(f"Cached card: {card_data['name']}")

    # Fetch rulings
    rulings_uri = data.get("rulings_uri", "")
    if rulings_uri:
        print(f"Fetching rulings...")
        rulings = fetch_rulings(card_data["name"], rulings_uri)
        if rulings:
            cache["rulings"][card_data["name"]] = rulings
            print(f"Cached {len(rulings)} rulings")
        else:
            print("No rulings found")

    save_cache(cache)
    print("---")
    cmd_lookup_by_name(cache, card_data["name"])


def cmd_fetch_set(args):
    """Fetch all cards in a set from Scryfall and cache them."""
    cache = load_cache()
    set_code = args.set_code.lower()

    url = f"https://api.scryfall.com/cards/search?q=set%3A{set_code}&unique=cards&order=name"
    total = 0
    skipped = 0
    page = 1

    while url:
        print(f"Fetching page {page} of set '{set_code}'...")
        data = scryfall_get(url)
        if not data:
            print(f"No results for set '{set_code}'")
            break

        for card in data.get("data", []):
            name = card.get("card_faces", [{}])[0].get("name", card["name"]) if card.get("card_faces") else card["name"]

            if not args.force:
                _, existing = find_card(cache, name)
                if existing:
                    skipped += 1
                    continue

            card_data = parse_scryfall_card(card)
            cache["cards"][card_data["name"]] = card_data

            # Fetch rulings for each card
            rulings_uri = card.get("rulings_uri", "")
            if rulings_uri:
                time.sleep(SCRYFALL_DELAY)
                rulings = fetch_rulings(card_data["name"], rulings_uri)
                if rulings:
                    cache["rulings"][card_data["name"]] = rulings

            total += 1
            print(f"  {card_data['name']}")
            time.sleep(SCRYFALL_DELAY)

        if data.get("has_more"):
            url = data.get("next_page")
            page += 1
        else:
            url = None

    save_cache(cache)
    print(f"\nDone. Cached {total} cards, skipped {skipped} already-cached.")


def cmd_lookup_by_name(cache, name):
    """Print card info from cache (helper, no args object needed)."""
    _, card = find_card(cache, name)
    if not card:
        return
    print(f"Name: {card['name']}")
    print(f"Mana Cost: {card.get('mana_cost', 'N/A')}")
    print(f"Type Line: {card.get('type_line', 'N/A')}")
    if card.get("power") or card.get("toughness"):
        print(f"P/T: {card.get('power', '?')}/{card.get('toughness', '?')}")
    print(f"Oracle Text: {card.get('oracle_text', 'N/A')}")
    if card.get("keywords"):
        print(f"Keywords: {', '.join(card['keywords'])}")
    print(f"Source: {card.get('source', 'unknown')}")
    print(f"Source URL: {card.get('source_url', 'N/A')}")
    if card.get("back_face"):
        bf = card["back_face"]
        print(f"\n--- Back Face ---")
        print(f"Name: {bf['name']}")
        print(f"Type Line: {bf.get('type_line', 'N/A')}")
        if bf.get("power") or bf.get("toughness"):
            print(f"P/T: {bf.get('power', '?')}/{bf.get('toughness', '?')}")
        print(f"Oracle Text: {bf.get('oracle_text', 'N/A')}")
    _, rulings = find_rulings(cache, name)
    if rulings:
        print(f"\n--- Rulings ({len(rulings)}) ---")
        for r in rulings:
            print(f"[{r.get('date', '?')}] {r['text']}")


def main():
    parser = argparse.ArgumentParser(description="Oracle text cache manager")
    sub = parser.add_subparsers(dest="command")

    # lookup
    p = sub.add_parser("lookup", help="Look up a card")
    p.add_argument("name", help="Card name")

    # add-card
    p = sub.add_parser("add-card", help="Add a card to the cache")
    p.add_argument("name", help="Card name")
    p.add_argument("--mana-cost", required=True)
    p.add_argument("--type-line", required=True)
    p.add_argument("--oracle-text", required=True)
    p.add_argument("--power", default=None)
    p.add_argument("--toughness", default=None)
    p.add_argument("--keywords", default=None, help="Comma-separated keywords")
    p.add_argument("--source", required=True)
    p.add_argument("--source-url", required=True)

    # add-card-json
    p = sub.add_parser("add-card-json", help="Add a card from JSON string")
    p.add_argument("json", help="JSON object with card fields")

    # add-back-face
    p = sub.add_parser("add-back-face", help="Add back face to a DFC")
    p.add_argument("name", help="Front face card name")
    p.add_argument("--back-name", required=True)
    p.add_argument("--type-line", required=True)
    p.add_argument("--oracle-text", required=True)
    p.add_argument("--power", default=None)
    p.add_argument("--toughness", default=None)

    # add-ruling
    p = sub.add_parser("add-ruling", help="Add a ruling")
    p.add_argument("name", help="Card name")
    p.add_argument("--date", required=True, help="Ruling date (YYYY-MM-DD)")
    p.add_argument("--text", required=True, help="Ruling text")
    p.add_argument("--source", required=True)
    p.add_argument("--source-url", required=True)

    # fetch (from Scryfall API)
    p = sub.add_parser("fetch", help="Fetch a card from Scryfall API and cache it")
    p.add_argument("name", help="Card name (fuzzy match)")
    p.add_argument("--force", action="store_true", help="Re-fetch even if already cached")

    # fetch-set (bulk fetch entire set)
    p = sub.add_parser("fetch-set", help="Fetch all cards in a set from Scryfall API")
    p.add_argument("set_code", help="Set code (e.g., 'isd' for Innistrad)")
    p.add_argument("--force", action="store_true", help="Re-fetch even if already cached")

    # list
    sub.add_parser("list", help="List all cached cards")

    args = parser.parse_args()
    if not args.command:
        parser.print_help()
        sys.exit(1)

    cmds = {
        "lookup": cmd_lookup,
        "add-card": cmd_add_card,
        "add-card-json": cmd_add_card_json,
        "add-back-face": cmd_add_back_face,
        "add-ruling": cmd_add_ruling,
        "fetch": cmd_fetch,
        "fetch-set": cmd_fetch_set,
        "list": cmd_list,
    }
    cmds[args.command](args)


if __name__ == "__main__":
    main()
