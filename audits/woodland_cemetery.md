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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: This land enters tapped unless you control a Swamp or a Forest. / {T}: Add {B} or {G}.
**Mana cost**: (none — land)
**Type line**: Land
**Status**: ISSUE
### Code issues
1. **Oracle text string mismatch**: Oracle says `"This land enters tapped unless you control a Swamp or a Forest."` but code has `"Woodland Cemetery enters the battlefield tapped unless you control a Swamp or a Forest."`. The oracle template was updated to use "This land enters tapped" instead of the old "[Card Name] enters the battlefield tapped" wording.
### Behavior
Behavior is correct: on_enter_battlefield checks for Swamp or Forest subtypes among other battlefield permanents (excluding self), and enters tapped if none found. Mana abilities correctly offer {B} or {G} with tap cost, only when untapped on battlefield. All logic is sound.
