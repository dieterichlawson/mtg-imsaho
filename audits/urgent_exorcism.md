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
