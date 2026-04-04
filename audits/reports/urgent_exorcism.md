# Audit: Urgent Exorcism

## Scryfall Reference
- **Name:** Urgent Exorcism
- **Cost:** {1}{W}
- **Type:** Instant
- **Oracle:** Destroy target Spirit or enchantment.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/urgent_exorcism.rs`
- Name: "Urgent Exorcism" -- MATCH
- Cost: {1}{W} -- MATCH
- Types: Instant -- MATCH
- Target: PermanentWithFilter (Spirit subtype OR Enchantment card type) -- MATCH
- is_valid_target: Checks battlefield, enchantment OR Spirit subtype -- MATCH
- on_resolve: Uses resolve_destroy -- CORRECT (destroy, not sacrifice)

## Verdict
**PASS** — Correctly targets Spirits or enchantments and destroys them.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Destroy target Spirit or enchantment.
**Type line**: Instant
**Status**: PASS

### Card Data
- **Name:** Urgent Exorcism -- CORRECT
- **Mana Cost:** {1}{W} -- CORRECT
- **Type:** Instant -- CORRECT

### Code issues
None. Target validation correctly checks for Spirit subtype OR Enchantment card type on battlefield permanents. Uses resolve_destroy helper for destruction. All data and behavior match oracle.
