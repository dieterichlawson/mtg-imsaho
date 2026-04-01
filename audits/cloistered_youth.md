## Audit — 2026-04-01

**Scryfall Oracle text (front)**: At the beginning of your upkeep, you may transform Cloistered Youth.
**Scryfall Oracle text (back)**: At the beginning of your end step, you lose 1 life.
**Scryfall type line**: Creature — Human // Creature — Horror
**Front P/T**: 1/1
**Back P/T**: 3/3
**Status**: ISSUE

1. **Transform is not optional ("you may")** (`mtg-engine/src/cards/cloistered_youth.rs`, line 80-88): The `on_upkeep` handler always transforms Cloistered Youth. Oracle text says "you may transform" — this should present a player choice. The comment on line 10 even notes "Simplified: Automatically transforms at your upkeep" but this deviates from correct behavior.
2. **triggered_abilities TriggerKind mismatch** (`mtg-engine/src/cards/cloistered_youth.rs`, line 39): The second triggered ability uses `TriggerKind::EndStep` with description "lose 1 life", but this is the Unholy Fiend's ability, not Cloistered Youth's. The front face's only trigger is Upkeep (transform). The back face trigger (EndStep) should ideally be declared separately or associated with the back face.
