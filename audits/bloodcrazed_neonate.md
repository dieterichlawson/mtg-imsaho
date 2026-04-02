# Audit: Bloodcrazed Neonate

## Oracle Text (Scryfall)
- **Name:** Bloodcrazed Neonate
- **Mana Cost:** {1}{R}
- **Type:** Creature — Vampire
- **P/T:** 2/1
- **Oracle Text:** This creature attacks each combat if able. / Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.

## Implementation File
`mtg-engine/src/cards/isd/bloodcrazed_neonate.rs`

## Card Data Checks
- **Name:** Correct
- **Mana Cost:** Correct ({1}{R})
- **Card Types:** Correct (Creature)
- **Subtypes:** Correct (Vampire)
- **P/T:** Correct (2/1)

## Behavior Checks
- **Must attack:** `ContinuousEffect::ForceAttack { scope: EffectScope::OnSelf }` -- correct.
- **+1/+1 counter on combat damage to player:** `on_combat_damage_to_player` adds a PlusOnePlusOne counter. Correct. Zone check is present.

## Verdict: PASS
