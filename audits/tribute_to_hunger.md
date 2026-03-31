# Audit: Tribute to Hunger

## Scryfall Reference
- **Name:** Tribute to Hunger
- **Cost:** {2}{B}
- **Type:** Instant
- **Oracle:** Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/tribute_to_hunger.rs`
- Name: "Tribute to Hunger" -- MATCH
- Cost: {2}{B} -- MATCH
- Types: Instant -- MATCH
- Target: PlayerOnly (target opponent) -- MATCH
- Behavior: Opponent chooses creature to sacrifice, caster gains life equal to toughness -- MATCH
- Uses PendingEffect::SacrificeAndGainLife -- CORRECT (sacrifice, not destroy)
- Presents choice to opponent (mandatory) -- MATCH

## Verdict
**PASS** — Correctly implements sacrifice (not destroy) with life gain.
