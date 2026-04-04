## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"target creature you control"**: Correctly filters by controller match (`o.controller == controller`) and zone (`Zone::Battlefield`). Uses `o.power.is_some()` to identify creatures.
- **"If that creature is a Human, put two +1/+1 counters on it instead"**: Replacement effect correctly implemented as `if is_human { 2 } else { 1 }` (2 INSTEAD of 1, not 2 PLUS 1). Human check covers both `obj.subtypes` (tokens) and `registry.card_data().subtypes` (normal cards).
- **Self-targeting prevention**: Elder Cathar cannot target itself due to `o.id != object_id` filter and zone check (it's in graveyard when trigger resolves).
- **Auto-targeting vs choice**: Single legal target auto-selected (lines 47-63), multiple targets present choice (lines 64-75) using `PendingEffect::AddCounters` with `human_bonus: true`.
- **Simultaneous death interactions**: Cannot target tokens created by other simultaneous death triggers (e.g., Doomed Traveler's Spirit token) because targets are chosen when trigger goes on stack, before other death triggers resolve.
- **No legal targets**: Correctly handles empty target list by doing nothing (lines 45-46).
- **Replacement effects preventing death**: If Elder Cathar is exiled instead of dying (e.g., Rest in Peace), the death trigger will not fire at all, which is correct per MTG rules.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic death trigger with single target**: `mtg-engine/tests/tier3_cards.rs:404` (elder_cathar_grants_counter_on_death)
- **Human bonus (2 counters)**: `mtg-engine/tests/card_mechanics.rs:412` (elder_cathar_gives_two_counters_to_human)
- **Non-Human gets 1 counter**: `mtg-engine/tests/card_mechanics.rs:432` (elder_cathar_gives_one_counter_to_non_human)
- **Multiple target choice path**: NOT TESTED
- **Zero targets case (no other creatures)**: NOT TESTED
- **Human token receiving 2 counters**: NOT TESTED
- **Simultaneous death timing**: NOT TESTED