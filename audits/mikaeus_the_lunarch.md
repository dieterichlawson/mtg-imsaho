## Audit — 2026-04-01

**Scryfall Oracle text**: Mikaeus, the Lunarch enters the battlefield with X +1/+1 counters on it.\n{T}: Put a +1/+1 counter on Mikaeus, the Lunarch.\n{T}, Remove a +1/+1 counter from Mikaeus, the Lunarch: Put a +1/+1 counter on each other creature you control.
**Scryfall type line**: Legendary Creature — Human Cleric
**Status**: PASS

- Name: Mikaeus, the Lunarch -- correct
- Cost: {X}{W} -- correct
- Type: Creature -- correct
- Supertypes: Legendary -- correct
- Subtypes: Human, Cleric -- correct
- P/T: 0/0 -- correct
- Enters with X +1/+1 counters -- correctly implemented using x_value
- Ability 0: {T} to add +1/+1 counter -- correct
- Ability 1: {T}, remove a +1/+1 counter to put +1/+1 on each other creature you control -- correct
- Ability 1 only available when Mikaeus has a +1/+1 counter -- correct
- Both abilities require tap -- correct
- Tests: no dedicated test found, but card logic is straightforward

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01

**Scryfall Oracle text**: Mikaeus enters with X +1/+1 counters on it. {T}: Put a +1/+1 counter on Mikaeus. {T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.
**Scryfall type line**: Legendary Creature — Human Cleric
**Status**: PASS

No issues found. X counters on ETB, tap abilities, counter distribution all correct. Legendary supertype set. Both activated abilities correctly require tap.
