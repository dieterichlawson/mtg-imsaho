## Audit — 2026-04-01

**Scryfall Oracle text**: Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
**Scryfall type line**: Creature — Angel
**Status**: PASS

- Mana cost {4}{W}: correct
- 4/4 stats: correct
- Subtype Angel: correct
- Keyword Flying: correct
- Triggered ability TriggerKind::Upkeep: correct
- on_upkeep checks active_player == controller (your upkeep only): correct
- Filters graveyard for Spirit subtype cards: correct
- Uses present_target_choice with mandatory targeting (optional: false): correct — the Oracle text says "return target Spirit" which is mandatory if valid targets exist
- Uses PendingEffect::ReturnToHand: correct
- Test exists in tier7_cards.rs

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Flying. At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
**Scryfall type line**: Creature — Angel
**Status**: PASS

No issues found. Trigger correctly limited to controller's upkeep. Target choice presented. Spirit subtype check covers both registry and object subtypes.
