## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever Selhoff Occultist or another creature dies, target player mills a card.
**Scryfall type line**: Creature — Human Rogue
**Mana cost**: {2}{U}
**P/T**: 2/3
**Status**: PASS

Implementation correctly models:
- Name, mana cost {2}{U}, type Creature, subtypes Human/Rogue, P/T 2/3
- Two triggered abilities: SelfDies and AnyCreatureDies
- `on_dies` and `on_any_creature_dies` both present a mill-1 choice targeting any player
- Target player choice is presented to the controller
- Tests: No dedicated test found, but the trigger infrastructure is tested elsewhere.

No issues found.
## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever Selhoff Occultist or another creature dies, target player mills a card.
**Scryfall type line**: Creature — Human Rogue
**Status**: ISSUE

- **No test coverage**: No tests found for Selhoff Occultist in the test suite. Should have at least one test for the mill-on-death trigger and one for the self-dies trigger.
