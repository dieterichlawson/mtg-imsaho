## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
Creatures enchanted player controls get -1/-1.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Target selection**: PASS - TargetRequirement::PlayerOnly allows targeting any player (including self) as expected for "enchant player"
- **Attachment mechanism**: PASS - resolve_curse helper correctly sets attached_to_player field on the curse object
- **Continuous effect scope**: PASS - CreatureFilter::AttachedPlayer correctly identifies creatures controlled by the cursed player via effect_applies_to checking source.attached_to_player
- **Effect timing**: PASS - Continuous effects recalculated each time effective_power/effective_toughness called, handles creatures entering after curse and control changes
- **State-based death**: PASS - Creatures with effective_toughness <= 0 die immediately via SBA rule 704.5f, curse's -1/-1 properly kills 1-toughness creatures
- **Effect selectivity**: PASS - Only affects creatures controlled by cursed player, curse controller's creatures unaffected
- **"As long as" continuous evaluation**: PASS - Effect continuously evaluates, not just snapshot at ETB

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic -1/-1 effect on cursed player's creatures**: `mtg-engine/tests/tier7_cards.rs:197`
- **Selectivity (only cursed player affected)**: `mtg-engine/tests/tier7_cards.rs:197`
- **Targeting any player**: NOT TESTED
- **Creatures dying from -1/-1**: NOT TESTED
- **Creatures entering after curse**: NOT TESTED
- **Control changes**: NOT TESTED
- **Attachment persistence**: NOT TESTED