## Audit — 2026-04-01

**Scryfall Oracle text**: Deathtouch
When Bitterheart Witch dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
**Scryfall type line**: Creature — Human Shaman
**Status**: ISSUE

- Mana cost {4}{B}: correct
- 1/2 stats: correct
- Subtypes Human, Shaman: correct
- Keyword Deathtouch: correct
- Triggered ability TriggerKind::SelfDies: correct
- Searches library for Curse card: correct
- Shuffles library after: correct (even when no Curse found)

Issues found:
1. **"You may" is not respected**: The Oracle text says "you may search your library" — this is optional. The implementation always searches automatically without giving the player a choice to decline.
2. **"Target player" is not implemented**: The Oracle text says "attached to target player" — the controller should choose which player to attach the Curse to. The implementation always attaches to the opponent automatically.
3. **Curse subtypes check**: The implementation checks `d.subtypes.iter().any(|s| s == "Curse")`. Curses in Innistrad are "Enchantment — Aura Curse", so the subtype is correct.

Test exists in tier15_cards.rs.
