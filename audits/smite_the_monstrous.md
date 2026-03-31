# Audit: Smite the Monstrous

## Oracle (Scryfall)
- **Name:** Smite the Monstrous
- **Cost:** {3}{W}
- **Type:** Instant
- **Oracle:** Destroy target creature with power 4 or greater.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/smite_the_monstrous.rs`
- **Name:** Smite the Monstrous ✅
- **Cost:** {3}{W} ✅
- **Type:** Instant ✅
- **Target:** CreatureWithFilter(PowerAtLeast(4)) ✅
- **is_valid_target:** uses effective_power to account for buffs/debuffs ✅
- **on_resolve:** uses resolve_destroy helper ✅

## Verdict: PASS -- no issues found
