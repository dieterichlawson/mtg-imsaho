## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target land. You gain 4 life.
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: Maw of the Mire -- correct
- Cost: {4}{B} -- correct
- Type: Sorcery -- correct
- Target: land permanent -- correctly implemented with PermanentWithFilter(HasCardType(Land))
- Effect: destroys land and gains 4 life -- correctly implemented
- Life gain properly tracks old/new life and emits event -- correct
- Tests exist in innistrad_simple_cards.rs

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target land. You gain 4 life.
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. Life gain happens unconditionally in on_resolve, but fizzle system prevents on_resolve from being called if target is illegal. Uses try_destroy correctly. Emits LifeChanged event. Uses move_spell_after_resolve.
