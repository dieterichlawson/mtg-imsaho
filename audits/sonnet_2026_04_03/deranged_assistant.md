## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)
**Type line**: Creature — Human Wizard
**Status**: PASS

### Code issues

No issues found.

### Tricky interactions checked

- **Non-reversibility ruling**: PASS - Mill happens immediately in `on_activate_mana_ability`, before spell validation. Even if spell casting fails later, milled card stays in graveyard.
- **Empty library prevention**: PASS - `mana_abilities()` checks `!library_order.is_empty()` before offering the ability.
- **Summoning sickness check**: PASS - `mana_abilities()` includes `!obj.summoning_sick` condition.
- **Tapping requirement**: PASS - `requires_tap: true` and engine taps the creature when activating.
- **Mill timing**: PASS - Card is milled when mana ability activates (via `mill_cards(state, controller, 1)`), not when mana is spent.
- **Mana production**: PASS - Produces exactly 1 colorless mana as specified.
- **Cost structure**: PASS - Mill is part of the activation cost, correctly handled in callback rather than separate additional cost.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- **Basic card data (P/T, cost, types)**: `mtg-engine/tests/innistrad_simple_cards.rs:300`
- **Mana ability produces colorless**: `mtg-engine/tests/innistrad_simple_cards.rs:312`
- **Library requirement check**: `mtg-engine/tests/innistrad_simple_cards.rs:318` (setup shows requirement understood)
- **Summoning sickness**: NOT TESTED
- **Empty library prevention**: NOT TESTED
- **Mill actually happens**: NOT TESTED (test only checks mana production)
- **Non-reversibility ruling**: NOT TESTED