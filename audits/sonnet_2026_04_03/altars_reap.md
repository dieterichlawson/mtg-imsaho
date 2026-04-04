## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Must have creature to cast: PASS - Engine checks `creatures.is_empty()` and continues if no creatures (line 536 in engine.rs)
- Sacrifice happens at cast time: PASS - Additional cost paid in submit_action before spell goes on stack (lines 1541-1566 in engine.rs)
- Players can't respond to sacrifice: PASS - Sacrifice occurs during casting process before spell is on stack
- Must sacrifice exactly one creature: PASS - Action generation creates one CastSpell action per eligible creature (lines 576-590 in engine.rs)
- Draws exactly two cards: PASS - `draw_cards(state, controller, 2)` called in on_resolve (line 41 in altars_reap.rs)
- Spell cleanup: PASS - `move_spell_after_resolve` called correctly (line 42 in altars_reap.rs)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Creature sacrifice as additional cost: `tier8_cards.rs:169-193` / TESTED
- Draw two cards: `tier8_cards.rs:187-192` / TESTED  
- Must have creature to cast: NOT TESTED
- Cannot sacrifice additional creatures: NOT TESTED
- Cannot respond to sacrifice: NOT TESTED