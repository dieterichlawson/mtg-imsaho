## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creatures you control enter as a copy of this creature.
**Type line**: Creature — Avatar
**Status**: ISSUE

### Code issues
- Multiple Essence of the Wild handling in `/Users/dlaw/mtg/mtg-engine/src/state.rs:537` 
  - Oracle text says: `If you control more than one Essence of the Wild, creatures you control will enter as a copy of the one whose copy effect you apply last.`
  - Code does: Uses `.find()` to get the first matching copy source instead of allowing player choice for order of replacement effect application. This violates the official ruling that the player should choose which copy effect applies last when multiple replacement effects modify the same event.

### Tricky interactions checked
- Controller restriction ("you control"): pass - correctly checks `o.controller == controller`
- Creature filtering: pass - uses `o.power.is_some()` to detect creatures entering battlefield
- Replacement effect timing: pass - implemented correctly via `entering_copy_source` during `move_object` before ETB event
- Copy characteristics: pass - copies name, power, toughness, colors, card_types, subtypes, keywords, oracle_text
- Self-exclusion: pass - correctly excludes source permanent with `o.id != entering_id`
- Copy source propagation: pass - copies `entering_copy_source` flag allowing effect chaining
- Opponent creature exclusion: pass - replacement effect only affects same controller's creatures
- Multiple Essence of the Wild: fail - uses first-found instead of player choice for last-applied wins rule

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic copy replacement effect: `mtg-engine/tests/tier15_cards.rs:2506`  
- Controller restriction (opponent unaffected): `mtg-engine/tests/tier15_cards.rs:2532`
- Multiple Essence of the Wild order: NOT TESTED
- ETB abilities being overridden by copy effect: NOT TESTED  
- External replacement effects interaction (e.g., "enters tapped"): NOT TESTED
- Copy effect with Clone-type creatures: NOT TESTED
- Copy source leaving battlefield between trigger and resolution: NOT TESTED