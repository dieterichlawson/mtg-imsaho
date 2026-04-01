## Audit — 2026-04-01

**Scryfall Oracle text**: Flash\nWhen Snapcaster Mage enters the battlefield, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
**Scryfall type line**: Creature — Human Wizard
**Mana cost**: {1}{U}
**P/T**: 2/1
**Status**: ISSUE

**Issue: Target selection is automated instead of player-chosen.** The implementation automatically selects the instant/sorcery with the highest mana value from the graveyard rather than presenting a choice to the player. The Oracle text says "target instant or sorcery card," meaning the player should choose which card gets flashback. The current "pick the most expensive" heuristic may not always match optimal play.

**Positive aspects**:
- Flash keyword correctly implemented
- Flashback grant uses `until_end_of_turn_flashback` (correct duration)
- Flashback cost equals mana cost (correct)
- Only targets cards without existing flashback (correct)

- Tests: `snapcaster_mage_grants_flashback` in tier14_cards.rs

## Audit — 2026-04-01

**Scryfall Oracle text**: Flash
When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
**Scryfall type line**: Creature — Human Wizard
**Status**: ISSUE

1. **Auto-selects target instead of presenting choice** (snapcaster_mage.rs:46-61): Oracle text says "target instant or sorcery card in your graveyard" which requires the player to choose. The code auto-selects the highest mana value card. This should present a target choice to the player.
