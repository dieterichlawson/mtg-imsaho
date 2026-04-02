# Audit: Brimstone Volley

## Oracle Reference
- **Name:** Brimstone Volley
- **Mana Cost:** {2}{R}
- **Type:** Instant
- **Oracle Text:** Brimstone Volley deals 3 damage to any target. / Morbid -- Brimstone Volley deals 5 damage instead if a creature died this turn.

## Card Data Audit
- **Name:** Correct ("Brimstone Volley")
- **Mana Cost:** Correct (Generic(2), Red)
- **Type:** Correct (Instant)
- **Subtypes:** Correct (none)
- **P/T:** Correct (None)

## Behavior Audit
- **Targeting:** `TargetRequirement::AnyTarget`. Correct.
- **Morbid check:** `if state.creature_died_this_turn { 5 } else { 3 }`. Correctly checks morbid condition and switches between 3 and 5 damage. Correct.
- **Damage delivery:** Uses `resolve_damage` helper. Correct.

## Result: PASS
