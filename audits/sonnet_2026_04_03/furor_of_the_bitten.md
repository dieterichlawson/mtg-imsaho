## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +2/+2 and attacks each combat if able.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues
- Missing "Enchant creature" prefix in oracle text field (`mtg-engine/src/cards/isd/furor_of_the_bitten.rs` line 22)
  - Oracle text says: `Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able.`
  - Code does: `oracle_text: "Enchanted creature gets +2/+2 and attacks each combat if able.".into()`

### Tricky interactions checked
- Forced attack respects "if able" (tapped, summoning sickness, defender): PASS - ForceAttack only applies to creatures in `eligible` list
- Forced attack with attack costs: PASS - Engine doesn't model attack costs, so ruling about not forcing payment of costs is not violated
- Multiple combat phases: PASS - Engine processes ForceAttack each combat phase if multiple existed
- Continuous effect scope: PASS - EffectScope::Attached correctly applies to enchanted creature only

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- +2/+2 stat modification: `mtg-engine/tests/innistrad_cards.rs:358` AND `mtg-engine/tests/bug_fixes.rs:497`
- ForceAttack continuous effect presence: `mtg-engine/tests/bug_fixes.rs:512-517`
- Attack cost ruling (creatures not forced to pay costs): NOT TESTED (engine doesn't model attack costs)
- Tapped/summoning sick creatures not forced to attack: NOT TESTED (covered by engine-level eligible attacker filtering)