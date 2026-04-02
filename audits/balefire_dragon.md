# Audit: Balefire Dragon

## Oracle Text (Scryfall)
- **Name:** Balefire Dragon
- **Mana Cost:** {5}{R}{R}
- **Type:** Creature — Dragon
- **P/T:** 6/6
- **Oracle Text:** Flying / Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.

## Implementation File
`mtg-engine/src/cards/isd/balefire_dragon.rs`

## Card Data Checks
- **Name:** Correct
- **Mana Cost:** Correct ({5}{R}{R})
- **Card Types:** Correct (Creature)
- **Subtypes:** Correct (Dragon)
- **P/T:** Correct (6/6)
- **Keywords:** Correct (Flying)
- **Triggered ability:** Correctly registered as `CombatDamageToPlayer`

## Behavior Checks
- **on_combat_damage_to_player:** Correctly receives `damaged_player` and `amount`, deals `amount` damage to each creature that player controls.
- **Non-combat damage:** Correctly emits `NonCombatDamageDealt` events (matching ruling that this damage is not combat damage).
- **Zone check:** Checks self is on battlefield before triggering -- correct.

## Verdict: PASS
