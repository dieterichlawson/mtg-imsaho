# Audit: Blasphemous Act

## Oracle Text (Scryfall)
- **Name:** Blasphemous Act
- **Mana Cost:** {8}{R}
- **Type:** Sorcery
- **Oracle Text:** This spell costs {1} less to cast for each creature on the battlefield. / Blasphemous Act deals 13 damage to each creature.

## Implementation File
`mtg-engine/src/cards/isd/blasphemous_act.rs`

## Card Data Checks
- **Name:** Correct
- **Mana Cost:** Correct ({8}{R})
- **Card Types:** Correct (Sorcery)
- **Oracle Text:** Correct

## Behavior Checks
- **Cost reduction:** `modified_cost` counts creatures on the battlefield and reduces generic cost by that amount, capped at 8 (cannot reduce below {R}). Correct per rulings.
- **On resolve:** Deals 13 damage to each creature on the battlefield. Correctly marks damage and emits `NonCombatDamageDealt` events. Correctly moves spell to graveyard after resolve.

## Verdict: PASS
