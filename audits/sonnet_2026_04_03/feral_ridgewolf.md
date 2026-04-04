## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Trample
{1}{R}: This creature gets +2/+0 until end of turn.
**Type line**: Creature — Wolf
**Status**: ISSUE

### Code issues
- oracle_text field mismatch at line 23
  - Oracle text says: `{1}{R}: This creature gets +2/+0 until end of turn.`
  - Code does: `{1}{R}: Feral Ridgewolf gets +2/+0 until end of turn.`

### Tricky interactions checked
- Multiple activations stacking: pass (each activation adds a separate UntilEndOfTurnEffect)
- Until-end-of-turn cleanup: pass (cleanup step clears until_end_of_turn_effects vector)
- Self-targeting ("this creature"): pass (ability affects the source object directly)
- Activated ability only works on battlefield: pass (zone check in activated_abilities)
- Mana cost payment: pass (auto_pay handles {1}{R} cost correctly)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic stats (1/2, trample, Wolf): `mtg-engine/tests/activated_abilities.rs:121`
- +2/+0 effect from single activation: `mtg-engine/tests/activated_abilities.rs:132`
- Multiple activations stack correctly: `mtg-engine/tests/activated_abilities.rs:156`
- Until-end-of-turn cleanup: `mtg-engine/tests/engine_bugs.rs:41` (general cleanup test)
- Mana cost verification: NOT TESTED (implicitly tested in activation tests)