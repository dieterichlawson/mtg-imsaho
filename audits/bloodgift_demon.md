## Audit — 2026-04-01

**Scryfall Oracle text**: Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
**Scryfall type line**: Creature — Demon
**Status**: PASS

- Mana cost {3}{B}{B}: correct
- 5/4 stats: correct
- Subtype Demon: correct
- Keyword Flying: correct
- Triggered ability TriggerKind::Upkeep: correct
- on_upkeep checks active_player == controller (your upkeep only): correct
- Presents target player choice to controller: correct
- Uses PendingEffect::DrawAndLoseLife: correct
- optional: false — correct, this is mandatory
- Test exists in tier7_cards.rs

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Flying. At the beginning of your upkeep, target player draws a card and loses 1 life.
**Scryfall type line**: Creature — Demon
**Status**: PASS

No issues found. Target player choice correctly presented. Upkeep trigger limited to controller's upkeep.
