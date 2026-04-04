## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}, {T}: Tap target non-Human creature.
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "target" creature selection: Player must choose a valid target, not auto-selected (PASS)
- "non-Human" subtype checking: Correctly checks both registry data and runtime object subtypes for tokens (PASS)
- Activated ability targeting: Engine properly generates targets for TargetRequirement::Creature and calls is_valid_target (PASS)
- Can target tapped creatures: No restriction against this in oracle text or code (PASS)
- Tap cost requirement: Prevents activation when source is already tapped (PASS)
- Mana cost payment: {1} generic mana cost properly handled by engine (PASS)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Correct stats (1/2 Human Cleric): `mtg-engine/tests/activated_abilities.rs:273`
- Taps non-Human creatures: `mtg-engine/tests/activated_abilities.rs:284`
- Cannot target Human creatures: `mtg-engine/tests/activated_abilities.rs:312`
- Requires tap (cannot activate when tapped): `mtg-engine/tests/activated_abilities.rs:333`
- Subtype checking for tokens: NOT TESTED
- Targeting hexproof/shroud creatures: NOT TESTED