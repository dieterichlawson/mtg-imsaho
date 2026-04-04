## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Werewolf creatures you control get +1/+0 and have trample.
Sacrifice this enchantment: Regenerate all Werewolf creatures you control.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
- Comment on line 8 incorrectly mentions Wolf creatures: `full_moons_rise.rs:8`
  - Oracle text says: `Werewolf creatures you control get +1/+0 and have trample`
  - Code says: `Werewolf and Wolf creatures you control get +1/+0 and have trample`
- Activated ability description on line 57 incorrectly mentions Wolf creatures: `full_moons_rise.rs:57`
  - Oracle text says: `Regenerate all Werewolf creatures you control`
  - Code says: `Regenerate all Wolf and Werewolf creatures you control`
- Variable name on line 74 is misleading: `wolves_and_werewolves` implies both types but filter only checks Werewolf

### Tricky interactions checked
- **Werewolf token subtype checking**: pass (both continuous effects and activated ability correctly check registry data via `CreatureFilter::HasSubtype` and manual `all_subtypes` collection including `creature.subtypes`)
- **Transformed werewolves**: pass (`CreatureFilter::HasSubtype` correctly handles back face subtypes for transformed DFCs per state.rs lines 656-663)
- **Regeneration shield mechanics**: pass (regeneration correctly adds shields to `obj.regeneration_shields += 1` which are consumed during destruction)
- **Combat timing for regeneration**: pass (ability can be activated before combat damage assignment, losing buffs as noted in ruling)
- **Continuous effect re-evaluation**: pass (`ContinuousEffect::ModifyPT` and `ContinuousEffect::GrantKeyword` with `EffectScope::Global` continuously apply to matching creatures)
- **Non-Werewolf exclusion**: pass (filters specifically check for "Werewolf" subtype only, excluding pure Wolf creatures)
- **"All Werewolf creatures" targeting**: pass (no targeting required - activated ability loops through all matching creatures)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic card data (mana cost, type)**: `mtg-engine/tests/innistrad_simple_cards.rs:572` / TESTED
- **+1/+0 power bonus application**: NOT TESTED
- **Trample keyword grant**: NOT TESTED  
- **Sacrifice activated ability functionality**: NOT TESTED
- **Regeneration shield application**: NOT TESTED
- **Combat timing ruling (lose buffs when sacrificed)**: NOT TESTED
- **Werewolf token interaction**: NOT TESTED
- **Transformed werewolf interaction**: NOT TESTED
- **Non-Werewolf creature exclusion**: NOT TESTED