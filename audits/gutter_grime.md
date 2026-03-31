# Audit: Gutter Grime

## Oracle Reference (Scryfall)
- Cost: {4}{G}
- Type: Enchantment
- Oracle: "Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime.""

## Implementation: gutter_grime.rs

## Issues Found

1. **ISSUE: Slime counters stored as PlusOnePlusOne counters** - Line 19 defines `SLIME_COUNTER: CounterType = CounterType::PlusOnePlusOne`. This means the enchantment incorrectly gets +1/+1 counter bonuses (if it were a creature), and effects that interact with +1/+1 counters would incorrectly see these as +1/+1 counters rather than slime counters. Should use a dedicated counter type or a custom tracking mechanism.

2. **ISSUE: Ooze tokens have static P/T instead of dynamic** - The comment on line 14 acknowledges this: "tokens are created with P/T equal to the current slime counter count at creation time (they don't dynamically update)." Per Oracle, ALL Ooze tokens should have P/T equal to the CURRENT number of slime counters on Gutter Grime, meaning they should grow as more slime counters are added. A token created when there's 1 slime counter should become 3/3 when there are 3 slime counters.

Otherwise correct: cost ({4}{G}), type (Enchantment), trigger (nontoken creature you control dies), creates green Ooze tokens.

## Verdict: ISSUES FOUND (2 issues)
