# Audit: Skirsdag Cultist

## Oracle (Scryfall)
- **Name:** Skirsdag Cultist
- **Cost:** {2}{R}{R}
- **Type:** Creature -- Human Shaman
- **Oracle:** {R}, {T}, Sacrifice a creature: Skirsdag Cultist deals 2 damage to any target.
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/skirsdag_cultist.rs`
- **Name:** Skirsdag Cultist ✅
- **Cost:** {2}{R}{R} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Shaman ✅
- **P/T:** 2/2 ✅
- **Activated ability:** {R}, tap, sacrifice a creature ✅
- **sacrifice_cost:** SacrificeCost::SacrificeCreature ✅
- **Target:** AnyTarget ✅
- **Damage to creature:** marks 2 damage, emits NonCombatDamageDealt event ✅
- **Damage to player:** reduces life by 2, emits NonCombatDamageDealt + LifeChanged events ✅

## Verdict: PASS -- no issues found
