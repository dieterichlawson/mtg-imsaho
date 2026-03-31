# Audit: Woodland Cemetery

## Scryfall Reference
- **Name:** Woodland Cemetery
- **Cost:** *(none)*
- **Type:** Land
- **Oracle:** This land enters tapped unless you control a Swamp or a Forest. {T}: Add {B} or {G}.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/woodland_cemetery.rs`
- Name: "Woodland Cemetery" -- MATCH
- Cost: None -- MATCH
- Types: Land -- MATCH
- on_enter_battlefield: Checks if controller has a Swamp or Forest (by subtype), enters tapped if not -- MATCH
- Excludes self from the check (o.id != object_id) -- CORRECT
- Mana abilities: Two options: Add {B} or Add {G}, both require tap -- MATCH

## Verdict
**PASS** — Dual land with conditional ETB tapped correctly implemented.
