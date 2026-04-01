# Oracle Text Lookup

Look up the current oracle text and rulings for a Magic: The Gathering card.

## Arguments
- `$ARGUMENTS` — One or more card names, comma-separated (e.g., "Lightning Bolt" or "Lightning Bolt, Doom Blade")

## How this works

We maintain a local oracle text cache at `data/oracle_cache.json` managed by `scripts/oracle_lookup.py`. This cache is the **single source of truth** for audits — it eliminates WebSearch unreliability and prevents oracle text hallucination.

## Procedure (repeat for each card)

### Step 1: Check the local cache

```bash
python3 scripts/oracle_lookup.py lookup "Card Name"
```

If the card is cached, output the cached data and you're done. The cache includes the source URL for verification.

### Step 2: If not cached, fetch via WebSearch

Search for the card's oracle text. Try in order:

1. `{card name} scryfall oracle text` (restrict to scryfall.com)
2. `{card name} MTG oracle text gatherer` (broader search)
3. `{card name} MTG card text type line` (any source)

**You MUST find the oracle text.** Do not give up after one search. Try different queries and sources until you have:
- Exact card name
- Mana cost
- Type line (including ALL subtypes)
- Full oracle text (verbatim, not paraphrased)
- Power/toughness (if creature)
- Keywords (if any)
- The source URL where you found this information

### Step 3: Cache the oracle text

After fetching, immediately cache it so we never need to fetch again:

```bash
python3 scripts/oracle_lookup.py add-card "Card Name" \
  --mana-cost "{1}{R}" \
  --type-line "Enchantment — Aura Curse" \
  --oracle-text "Enchant player\nAt the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls." \
  --source "Scryfall via WebSearch" \
  --source-url "https://scryfall.com/card/isd/138/curse-of-the-pierced-heart"
```

For creatures, add `--power` and `--toughness`. For cards with keywords, add `--keywords "Flying,Hexproof"`.

For double-faced cards, also add the back face:
```bash
python3 scripts/oracle_lookup.py add-back-face "Front Face Name" \
  --back-name "Back Face Name" \
  --type-line "Creature — Werewolf" \
  --oracle-text "Back face oracle text..." \
  --power "4" --toughness "4"
```

### Step 4: Fetch and cache rulings

Search for rulings:
1. `{card name} scryfall rulings` (restrict to scryfall.com)
2. `{card name} MTG rulings gatherer`

For EACH ruling found, cache it with a source citation:

```bash
python3 scripts/oracle_lookup.py add-ruling "Card Name" \
  --date "2011-09-22" \
  --text "The ruling text here..." \
  --source "Scryfall rulings via WebSearch" \
  --source-url "https://scryfall.com/card/isd/138/curse-of-the-pierced-heart"
```

**Every cached ruling MUST have a source URL.** Do not cache rulings without citing where you found them.

### Step 5: Output the result

Print the full card data and all rulings for the user.

## Important rules
- **NEVER use your training data as a source for oracle text.** Only cache text from external sources.
- **Every cache entry has a source URL** — this is mandatory, not optional.
- **Rulings need individual source citations** — each ruling must link back to where it was found.
- **Do not paraphrase** — oracle text must be verbatim from the source.
- **The cache is append-only** — do not delete entries. If oracle text needs updating (e.g., new errata), add a new entry with the updated text; the latest entry wins.
