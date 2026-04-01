## Audit — 2026-04-01

**Scryfall Oracle text**: Trample\nSplinterfright's power and toughness are each equal to the number of creature cards in your graveyard.\nAt the beginning of your upkeep, mill two cards.
**Scryfall type line**: Creature — Elemental
**Status**: PASS

- Name: correct ("Splinterfright")
- Cost: {2}{G} -- correct
- Type: Creature -- correct
- Subtypes: Elemental -- correct
- P/T: */* (base 0/0) -- correct
- Keywords: Trample -- correct
- `dynamic_pt` counts creature cards in controller's graveyard -- correct
- `on_upkeep` mills 2 cards, only on controller's upkeep -- correct
- Triggered ability registered as TriggerKind::Upkeep -- correct
- Tests exist in `tier7_cards.rs`
- No issues found

## Audit — 2026-04-01

**Scryfall Oracle text**: Trample
Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.
At the beginning of your upkeep, mill two cards.
**Scryfall type line**: Creature — Elemental
**Status**: PASS

No issues found. Uses `dynamic_pt` correctly, mills on upkeep with active player check, has Trample keyword. Triggered ability declaration matches implemented `on_upkeep` hook (TriggerKind::Upkeep).
