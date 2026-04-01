## Audit — 2026-04-01

**Scryfall Oracle text**: Search your library for a basic land card, reveal it, put it into your hand, then shuffle. Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
**Scryfall type line**: Sorcery
**Status**: ISSUE

1. **Morbid "You may" not presented as a choice** (`mtg-engine/src/cards/caravan_vigil.rs`, line 61): When morbid is active, the code auto-chooses to put the land onto the battlefield. Oracle says "You may put that card onto the battlefield instead of putting it into your hand," making this optional. The player should be given a choice.
2. **Oracle text mismatch** (`mtg-engine/src/cards/caravan_vigil.rs`, line 25): Code says "then shuffle your library" but current Scryfall Oracle text says "then shuffle" (minor templating update).
