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

## Audit — 2026-04-02

**Oracle Text:**
> Destroy target creature with power 4 or greater.

**Card Data:**
- Name: Smite the Monstrous — correct
- Cost: {3}{W} — correct
- Type: Instant — correct

**Behavior:**
- target_requirement: CreatureWithFilter(PowerAtLeast(4)) — correct
- is_valid_target checks effective_power >= 4 (accounts for buffs/counters) — correct
- on_resolve calls resolve_destroy helper — correct

**Result: PASS**
