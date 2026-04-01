## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Scryfall type line**: Enchantment
**Status**: ISSUE

- Mana cost {4}{G}: correct
- Card type Enchantment: correct
- Trigger on nontoken creature death: correctly checks dead_controller == controller and !is_token
- Adds slime counter: uses PlusOnePlusOne counter type as stand-in (SLIME_COUNTER alias)
- Creates green Ooze token: correct color, subtype, card type
- ISSUE: Slime counters reuse CounterType::PlusOnePlusOne which means any effect that interacts with +1/+1 counters on the enchantment would incorrectly interact with slime counters (and vice versa). Should ideally use a dedicated counter type.
- ISSUE: Token P/T is set at creation time to the current slime count. Oracle says the token's P/T dynamically tracks the number of slime counters on Gutter Grime. If more slime counters are added later, existing tokens should grow. The implementation notes this as a known simplification.
- Tests exist in tier15_cards.rs covering Ooze creation on creature death

## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Scryfall type line**: Enchantment
**Status**: ISSUE

1. **Slime counters stored as PlusOnePlusOne counters**: The code reuses CounterType::PlusOnePlusOne as a stand-in for slime counters (line 19). This could cause confusion if the engine ever checks for +1/+1 counters on enchantments, and is semantically incorrect.
2. **Ooze tokens don't dynamically update P/T**: The code acknowledges this in comments (line 13-14). Oracle text says the tokens have the ability "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime." Tokens should dynamically update as more slime counters are added, but they're created with static P/T. This means earlier tokens are weaker than later ones, which is wrong.
3. **If Gutter Grime leaves, tokens should become 0/0**: Since tokens reference Gutter Grime's slime counters, if it leaves the battlefield, tokens should die. The static P/T implementation doesn't handle this.
