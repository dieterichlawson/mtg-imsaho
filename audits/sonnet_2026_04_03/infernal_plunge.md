## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Add {R}{R}{R}.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Counterspell interaction**: If Infernal Plunge is countered, the creature is already sacrificed (at cast time, before spell goes on stack) and the {R}{R}{R} is never produced. Implementation is correct: sacrifice handled by engine during cast action, mana addition in `on_resolve`.
- **Cannot cast without creatures**: Engine's `legal_actions` in engine.rs lines 529-537 checks for creatures with power on battlefield and only generates CastSpell actions if at least one exists. Correctly implements ruling "you cannot cast it without sacrificing a creature".
- **Multiple sacrifice candidates**: Engine.rs lines 580-587 generates one distinct CastSpell action per eligible creature with `sacrifice: Some(creature_id)`, giving player real choice of which creature to sacrifice.
- **Sacrifice timing**: AdditionalCost::SacrificeCreature is processed at cast time (engine.rs lines 1541-1546) before spell goes on stack, not at resolution. This is correct per MTG rules and matches oracle text "As an additional cost to cast this spell".
- **Exactly one creature required**: Engine enforces exactly one sacrifice through action generation and cast execution, matching ruling "You must sacrifice exactly one creature... you cannot sacrifice additional creatures".

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Cannot cast without creatures: `mtg-engine/tests/infernal_plunge.rs:20` (cannot_cast_without_creature)
- Can cast with creatures: `mtg-engine/tests/infernal_plunge.rs:40` (can_cast_with_creature)  
- Sacrifice timing (cast time not resolution): `mtg-engine/tests/infernal_plunge.rs:62` (sacrifice_at_cast_time)
- Adds {R}{R}{R} on resolution: `mtg-engine/tests/infernal_plunge.rs:91` (adds_three_red_mana)
- Multiple sacrifice candidates: `mtg-engine/tests/infernal_plunge.rs:122` (one_action_per_sacrifice_target)
- Ruling: sacrifice exactly one creature: NOT TESTED (enforced by engine design but no explicit test)
- Ruling: sacrifice can't be prevented once spell cast: NOT TESTED (timing-based ruling)