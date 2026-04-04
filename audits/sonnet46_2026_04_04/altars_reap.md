## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Cannot cast without a creature**: The `legal_actions` generator (engine.rs ~line 530–536) filters `AdditionalCost::SacrificeCreature` cases and executes `if creatures.is_empty() { continue; }`, preventing any cast action from being generated — pass.
- **Player chooses which creature to sacrifice**: `legal_actions` generates one separate `CastSpell { sacrifice: Some(sac_id), .. }` action per eligible creature controlled by the caster (engine.rs ~lines 576–590), giving the player a choice — pass.
- **Exactly one creature sacrificed, not zero or more**: Sacrifice happens in `submit_action` before the spell is placed on the stack (engine.rs ~line 1544); no mechanism allows sacrificing additional creatures — pass.
- **Sacrifice is an additional cost, paid at cast time**: Sacrifice is executed during `submit_action` (before stack entry), not during resolution — pass.
- **Draw two cards goes to caster**: `on_resolve` looks up `o.controller` on the spell object (altars_reap.rs line 35–37) and passes it to `draw_cards`, correctly attributing the draw to the caster — pass.
- **Spell moves to graveyard after resolution**: `state.move_spell_after_resolve(object_id)` (altars_reap.rs line 42) sends non-flashback spells to the graveyard, which is correct for a standard Instant — pass.
- **"Backward compatibility" auto-sacrifice fallback**: engine.rs ~lines 1548–1565 contains a fallback that auto-sacrifices the first creature when `sacrifice: None` is submitted. This path is hit by the test helper (`cast_and_resolve` always passes `sacrifice: None`) but not by normally-generated legal actions. The fallback does not skip the sacrifice entirely — pass.
- **Mana cost {1}{B}**: Declared as `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Black)])` — matches oracle — pass.
- **Card type Instant**: Declared as `card_types: vec![CardType::Instant]` — pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic sacrifice-a-creature + draw-two behavior: `tier8_cards.rs:169` (`altars_reap_sacrifices_and_draws_two`) — TESTED
- Cannot cast without controlling a creature: NOT TESTED (shared engine logic with other AdditionalCost::SacrificeCreature cards, but no specific test for Altar's Reap)
- Player choice of which creature to sacrifice (multiple creatures available): NOT TESTED (the only test has exactly one creature, so the choice is trivial)
- Spell goes to graveyard (not exile) after resolution: NOT TESTED explicitly (covered implicitly by test not failing on zone check)
- Ruling [2013-04-15] — must sacrifice exactly one creature, cannot sacrifice additional creatures: NOT TESTED
- Ruling [2013-04-15] — no response window before spell is cast and costs paid: NOT TESTED
