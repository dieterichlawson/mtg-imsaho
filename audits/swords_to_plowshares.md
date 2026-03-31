# Audit: Swords to Plowshares

## Oracle (Scryfall)
- **Name:** Swords to Plowshares
- **Cost:** {W}
- **Type:** Instant
- **Oracle:** Exile target creature. Its controller gains life equal to its power.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/swords_to_plowshares.rs`
- **Name:** Swords to Plowshares ✅
- **Cost:** {W} ✅
- **Type:** Instant ✅
- **Target:** TargetRequirement::Creature ✅
- **on_resolve:** exiles target creature from battlefield ✅
- **Life gain:** uses effective_power (accounts for buffs/counters), controller gains that much life ✅
- **Power floor:** .max(0) prevents negative power from causing life loss ✅
- **Events:** emits LifeChanged event ✅
- **Spell cleanup:** move_spell_after_resolve ✅

## Verdict: PASS -- no issues found
