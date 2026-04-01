## Audit — 2026-04-01

**Scryfall Oracle text**: Morbid — When Woodland Sleuth enters the battlefield, if a creature died this turn, return a creature card at random from your graveyard to your hand.
**Scryfall type line**: Creature — Human Scout
**Scryfall mana cost**: {3}{G}
**Scryfall P/T**: 2/3
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {3}{G} — correct.
- Types: Creature — Human Scout — correct.
- P/T: 2/3 — correct.
- Morbid condition: Checks `state.creature_died_this_turn`. Correct.
- ETB effect: Finds creature cards in controller's graveyard, shuffles randomly, returns one to hand. Correct.
- Tests: `woodland_sleuth_morbid_returns_creature` and `woodland_sleuth_no_morbid_no_return` in tier11_cards.rs.

No issues found.

## Audit — 2026-04-01

**Scryfall Oracle text**: Morbid — When Woodland Sleuth enters the battlefield, if a creature died this turn, return a creature card at random from your graveyard to your hand.
**Scryfall type line**: Creature — Human Scout
**P/T**: 2/3, **Mana cost**: {3}{G}
**Status**: PASS

No issues found. Morbid check, random creature selection, and return-to-hand logic all correct.
