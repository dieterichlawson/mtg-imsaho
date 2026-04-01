# Audit: Gutter Grime

## Oracle Reference (Scryfall)
- Cost: {4}{G}
- Type: Enchantment
- Oracle: "Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime.""

## Implementation: gutter_grime.rs

## Issues Found

1. **FIXED: Slime counters stored as PlusOnePlusOne counters** - Added CounterType::Slime to properly track slime counters separately from +1/+1 counters.

2. **FIXED: Ooze tokens have static P/T instead of dynamic** - Tokens now have base 0/0 P/T with dynamic lookup via card_state "pt_source_counter" linking to the source Gutter Grime. effective_power/toughness dynamically reads slime counter count.

Otherwise correct: cost ({4}{G}), type (Enchantment), trigger (nontoken creature you control dies), creates green Ooze tokens.

## Verdict: ALL ISSUES FIXED

## Audit — 2026-04-01 06:10

**Scryfall Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Scryfall type line**: Enchantment
**Status**: PASS

No issues found. Dynamic P/T for Ooze tokens now correctly tracks slime counter count on the source Gutter Grime. Each token stores its source Gutter Grime ObjectId via card_state, and effective_power/toughness dynamically look up the counter count. If Gutter Grime leaves the battlefield, tokens become 0/0. Token deaths and opponent creature deaths correctly do not trigger the ability.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Scryfall type line**: Enchantment
**Status**: PASS

Mana cost {4}{G}: correct. Type Enchantment: correct. No subtypes: correct. Trigger on nontoken creature you control dying: correct (checks `is_token` and `dead_controller`). Adds slime counter via `CounterType::Slime`: correct. Creates green Ooze creature token with dynamic P/T linked to slime counters on source Gutter Grime: correct. Token created with subtypes `["Ooze"]`: correct. Uses `AnyCreatureDies` trigger kind with `triggered_abilities` declaration: correct. Tests present in `tests/gutter_grime.rs` and `tests/tier15_cards.rs`. No anti-patterns found (no `move_object` to graveyard for spells, no `CombatDamageDealt` misuse).

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Type line**: Enchantment
**Status**: PASS

Mana cost {4}{G}: correct (Generic(4) + Green). Type Enchantment: correct. No supertypes or subtypes: correct. No P/T: correct. Oracle text in code says "This creature's power and toughness" while Scryfall says "This token's power and toughness" -- minor text discrepancy, but functionally identical since the token IS a creature. Trigger condition: `AnyCreatureDies` trigger kind, filters for nontoken (`is_token` check) and controller-owned (`dead_controller`): correct. Adds slime counter via `CounterType::Slime`: correct. Creates green Ooze creature token with base 0/0 and dynamic P/T linked to source Gutter Grime via `card_state["pt_source_counter"]`: correct per rulings (tokens dynamically track counter count). Token has subtypes `["Ooze"]` and colors `[Green]`: correct. `triggered_abilities` declaration matches the `on_any_creature_dies` hook: correct. Tests in `tests/gutter_grime.rs` cover: basic trigger, multiple deaths growing all tokens, token death ignored, opponent death ignored, Gutter Grime removal makes tokens 0/0. No anti-patterns found.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Type line**: Enchantment
**Status**: PASS

Mana cost {4}{G}: correct. Type Enchantment: correct. No supertypes or subtypes: correct.

Triggered ability: triggers on `AnyCreatureDies`, checks that dead creature was nontoken (`is_token` check) and controlled by the enchantment's controller: correct. Adds slime counter via `CounterType::Slime`: correct. Creates a green Ooze creature token with base 0/0 and dynamic P/T linked to slime counter count on source Gutter Grime via `card_state["pt_source_counter"]`: correct per rulings that tokens update dynamically. Token has correct subtypes `["Ooze"]` and color green: correct.

Oracle text in code says "This creature's power and toughness" while Scryfall says "This creature's power and toughness" (Scryfall also shows "This token's power and toughness" in some entries -- minor wording variant, functionally identical).

Tests: 4 tests in `tests/gutter_grime.rs` covering creation, growth, token death exclusion, opponent death exclusion, and source removal causing 0/0. Good coverage. No anti-patterns found.
