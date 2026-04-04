## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
**Type line**: Creature — Wolf
**Status**: ISSUE

### Code issues

- `abilities_activated_this_turn` is never cleared between turns — engine bug causes once-per-turn restriction to become once-per-game permanently
  - Oracle text says: `Activate only once each turn.`
  - Code does: `engine.rs:1778` inserts `ability_index` into `obj.abilities_activated_this_turn` when activated, and `engine.rs:358` uses `activated_this_turn.contains(&ab.ability_index)` to suppress the ability. However, the cleanup step (`engine.rs:3006–3061`) never clears `abilities_activated_this_turn` on any game object — it only clears `until_end_of_turn_effects`, `until_end_of_turn_keywords`, `until_end_of_turn_cant_block`, `until_end_of_turn_protection`, `until_end_of_turn_removed_keywords`, and `prevent_non_wolf_werewolf_combat_damage`. The untap step (`engine.rs:2911–2954`) also does not clear it. As a result, once the ability is activated once in any turn, it is locked out for every subsequent turn of the game.

### Tricky interactions checked

- `until_end_of_turn_effects` cleared at cleanup: PASS — `engine.rs:3021` clears the vec, so the +2/+2 boost correctly expires at end of turn.
- `once_per_turn` enforcement within a turn: PASS — `engine.rs:358` correctly blocks a second activation in the same turn using `activated_this_turn.contains(&ab.ability_index)`.
- `once_per_turn` resets across turns: FAIL — `abilities_activated_this_turn` is never `.clear()`-ed anywhere in the engine between turns; the permanent lock is an engine bug that violates the oracle text.
- Activation only available while on battlefield: PASS — `activated_abilities()` returns `vec![]` unless `zone == Zone::Battlefield` (card file lines 34–50).
- Activation cost {2}{G}: PASS — `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Green)])` matches oracle text.
- `sorcery_speed_only: false` (can activate at instant speed): PASS — oracle text has no "activate only as a sorcery" restriction.
- `requires_tap: false`: PASS — no tap symbol in oracle text.

### Test coverage

- Basic stats (2/2, Wolf subtype): `activated_abilities.rs:180` (`darkthicket_wolf_has_correct_stats`)
- +2/+2 buff applied correctly: `activated_abilities.rs:190` (`darkthicket_wolf_gets_plus_2_plus_2`)
- Once-per-turn restriction blocks second activation in same turn: `activated_abilities.rs:210` (`darkthicket_wolf_once_per_turn`)
- Once-per-turn restriction resets at start of next turn (ability usable again on turn 2): NOT TESTED — this is the scenario that would expose the `abilities_activated_this_turn` not-cleared bug
- `until_end_of_turn_effects` expiry at cleanup: NOT TESTED for this card specifically
