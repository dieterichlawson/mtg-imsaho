## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target creature you control gets +1/+1 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Hexproof prevents targeting by opponents only**: `can_be_targeted` in `engine.rs:758-768` checks `controller != caster` before blocking on hexproof, so the caster can still target their own hexproof creature with their own spells. Correct.
- **"Until end of turn" expiry**: `until_end_of_turn_effects` and `until_end_of_turn_keywords` are both cleared at `engine.rs:3021-3022` in `Step::Cleanup`. Correct.
- **Fizzle if target leaves battlefield**: `stack.rs:8-41` (`is_target_legal`) returns false if the object is no longer on the battlefield; spell fizzles per CR 608.2b. Correct.
- **Hexproof doesn't block resolution of already-targeted spells**: `is_target_legal` (called at resolution) only checks zone, not hexproof. Hexproof is only enforced at target-selection time (`can_be_targeted`). Correct per MTG rules.
- **Target restriction "you control"**: `target_requirement` is `CreatureWithFilter(YouControl)`. Engine filters by `can_be_targeted` then `is_valid_target`; `is_valid_target` checks `o.controller == caster`. Correct.
- **+1/+1 applied via `UntilEndOfTurnEffect`**: `effective_power` and `effective_toughness` in `state.rs:851-935` sum all `until_end_of_turn_effects` entries matching the target id. Correct.
- **`move_spell_after_resolve` called**: Called at end of `on_resolve`; `stack.rs` only redundantly moves if object is still in Zone::Stack, which it isn't after the first call. No double-move issue.
- **Opponent's hexproof creature as target**: `is_valid_target` checks `o.controller == caster`, so opponent's creatures are excluded regardless of hexproof status. Correct.

### Test coverage
- +1/+1 pump and hexproof grant: `innistrad_cards.rs:201` (`rangers_guile_gives_hexproof_and_pump`) — TESTED
- Cannot target opponent's creature: `card_fixes.rs:85` (`rangers_guile_cannot_target_opponent_creature`) — TESTED
- Hexproof expires at end of turn: NOT TESTED
- Fizzle when target leaves battlefield in response: NOT TESTED
- Can target own creature that already has hexproof (from prior Ranger's Guile): NOT TESTED
