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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.
**Type line**: Instant
**Status**: ISSUE

### Card Data
- **Name:** Tribute to Hunger -- CORRECT
- **Mana Cost:** {2}{B} -- CORRECT
- **Type:** Instant -- CORRECT
- **P/T:** N/A -- CORRECT

### Code issues
1. **Oracle text omits "of their choice"**: Code oracle_text says "Target opponent sacrifices a creature." but oracle says "Target opponent sacrifices a creature of their choice." The behavior is correct (opponent chooses via PendingEffect::SacrificeAndGainLife), so this is a cosmetic text-only issue.
   - Oracle: "sacrifices a creature of their choice"
   - Code: "sacrifices a creature"
2. **Behavior correct**: Opponent chooses creature, life gain equals toughness. Implementation uses SacrificeAndGainLife pending effect with opponent as chooser.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Target opponent sacrifices a creature of their choice." (was "Target opponent sacrifices a creature."). Doc comment updated. Behavior unchanged.
