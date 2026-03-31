# Audit: Corpse Lunge

## Scryfall Reference
- **Name:** Corpse Lunge
- **Cost:** {2}{B}
- **Type:** Instant
- **Oracle:** As an additional cost to cast this spell, exile a creature card from your graveyard. Corpse Lunge deals damage equal to the exiled card's power to target creature.
- **P/T:** N/A
- **Keywords:** none

## Implementation: `corpse_lunge.rs`
- **Name:** Corpse Lunge -- CORRECT
- **Cost:** {2}{B} -- CORRECT
- **Type:** Instant -- CORRECT
- **Subtypes:** none -- CORRECT
- **P/T:** N/A -- CORRECT
- **Additional cost:** ExileCreaturesFromGraveyard(1) -- CORRECT
- **Target:** TargetRequirement::Creature -- CORRECT
- **Damage:** Uses NonCombatDamageDealt event -- CORRECT
- **Behavior:** Exiles creature from graveyard, deals damage equal to power to target creature -- CORRECT

## Issues
None
