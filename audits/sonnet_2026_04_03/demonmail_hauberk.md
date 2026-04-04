## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature gets +4/+2. Equip—Sacrifice a creature.
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues
- Engine auto-selects creature to sacrifice instead of presenting player choice (mtg-engine/src/engine.rs:1751-1760)
  - Oracle text says: `Equip—Sacrifice a creature.`
  - Code does: Auto-sacrifices first eligible creature with comment "For now, auto-sacrifice the first eligible creature. // TODO: Present choice to player when there are multiple options."

### Tricky interactions checked
- Sacrifice cost vs effect resolution: PASS (cost paid before ability resolves)
- Can sacrifice equipped creature per ruling: PASS (target chosen before cost, ability fizzles correctly)
- Targeting "you control" restriction: PASS (uses TargetFilter::YouControl)
- Sorcery speed timing: PASS (sorcery_speed_only: true)
- Continuous effect application: PASS (EffectScope::Attached with +4/+2 ModifyPT)
- Equipment attachment mechanics: PASS (uses attached_to field properly)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic equip functionality: `mtg-engine/tests/tier9_cards.rs:93-136`
- Card data validation: `mtg-engine/tests/tier9_cards.rs:83-90`
- Creature sacrifice cost: `mtg-engine/tests/tier9_cards.rs:93-136` (only tests that one creature dies, not player choice)
- Power/toughness bonus: `mtg-engine/tests/tier9_cards.rs:132-135`
- Can sacrifice equipped creature ruling: NOT TESTED
- Player choice in sacrifice: NOT TESTED
- Multiple creature sacrifice scenarios: NOT TESTED