## Audit — 2026-04-01

**Scryfall Oracle text**: When Armored Skaab enters the battlefield, mill four cards.
**Scryfall type line**: Creature — Zombie Warrior
**Status**: PASS

- Mana cost {2}{U}: correct
- 1/4 stats: correct
- Subtypes Zombie, Warrior: correct
- Triggered ability TriggerKind::EntersBattlefield: correct
- on_enter_battlefield mills 4 cards via crate::engine::mill_cards: correct

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: When this creature enters, mill four cards.
**Scryfall type line**: Creature — Zombie Warrior
**Status**: ISSUE

1. **No tests**: No test files reference Armored Skaab. Missing test for ETB mill trigger.
2. **Not in LLM card knowledge**: Missing from mtg-player/src/llm.rs.
