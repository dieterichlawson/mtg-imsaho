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
