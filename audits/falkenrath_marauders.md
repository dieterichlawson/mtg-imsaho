# Audit: Falkenrath Marauders

## Reference (Scryfall)
- **Name:** Falkenrath Marauders
- **Cost:** {3}{R}{R}
- **Type:** Creature -- Vampire Warrior
- **Oracle:** Flying, haste. Whenever Falkenrath Marauders deals combat damage to a player, put two +1/+1 counters on it.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{R}{R})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Vampire, Warrior)
- Oracle text: CORRECT
- P/T: CORRECT (2/2)
- Keywords: CORRECT (Flying, Haste)
- Combat damage trigger: CORRECT (TriggerKind::CombatDamageToPlayer)
- Two +1/+1 counters: CORRECT (add_counters with PlusOnePlusOne, 2)

## Issues
None found.
