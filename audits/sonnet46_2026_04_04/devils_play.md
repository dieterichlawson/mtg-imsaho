## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **X=0 handling**: When x_value=0, `on_resolve` skips `resolve_damage` and calls `move_spell_after_resolve` directly. Dealing 0 damage is functionally identical; spell still reaches the correct destination zone (graveyard or exile). PASS
- **Flashback exile on resolution**: `move_spell_after_resolve` in `state.rs:1132` checks `obj.cast_with_flashback`; when true, moves to `Zone::Exile`. The `cast_with_flashback` flag is set in `engine.rs` during `Action::CastSpell` when `is_flashback` is true. Correct for both X>0 path (via `resolve_damage`) and X=0 path (direct call). PASS
- **Flashback exile on counter**: Counterspell (`counterspell.rs:50`) and Lost in the Mist (`lost_in_the_mist.rs:56`) both call `move_spell_after_resolve` on the targeted spell, which correctly exiles a `cast_with_flashback` spell. The `PayOrNot` path in `engine.rs:1962` also uses `move_spell_after_resolve`. PASS
- **Flashback exile on fizzle**: `stack.rs:84` calls `move_spell_after_resolve` when all targets are illegal; correctly exiles a flashback-cast Devil's Play. PASS
- **X value computed correctly for flashback cost**: `engine.rs:1515–1521` strips the X symbol from the flashback cost `{X}{R}{R}{R}` to get non_x_cost `{R}{R}{R}` (mana_value=3); X = total_mana_in_pool - 3. Correct. PASS
- **X value computed correctly for normal cost**: non_x_cost of `{X}{R}` is `{R}` (mana_value=1); X = total_mana_in_pool - 1. Correct. PASS
- **`can_pay` for flashback cost with X symbol**: `mana.rs:try_auto_pay` skips the X symbol (not Colored, not Colorless, not Generic); effectively checks only the `{R}{R}{R}` portion. Equivalent to the explicit non-X check used for activated abilities. PASS
- **x_value readable during on_resolve**: The object is still in `state.objects` when `on_resolve` is called (only the stack entry is popped, not the object); `state.get_object(object_id).and_then(|o| o.x_value)` returns the correct value. PASS
- **Flashback offered only from graveyard**: `engine.rs:667` iterates `objects_in_zone(Zone::Graveyard, player)` for flashback actions; no flashback is offered from hand. PASS
- **Sorcery timing restriction on flashback**: `engine.rs:692–706` checks `is_sorcery_speed` before offering a sorcery flashback cast. Devil's Play as a Sorcery is covered. PASS
- **"may" optionality of flashback**: Flashback is an offered action the player may choose to take or not; nothing forces the cast. PASS
- **Target legality re-check at resolution**: `stack.rs:79–86` verifies targets before calling `on_resolve`; fizzles if all targets illegal, sending the spell to the correct zone. PASS
- **Spell cast from GY without having been cast first**: Engine checks for `Zone::Graveyard` only; no restriction on how the card arrived there. PASS

### Test coverage
- Normal cast dealing X>0 damage: `tier14_cards.rs:298` (`devils_play_deals_x_damage`) TESTED
- X=0 deals no damage: `tier14_cards.rs:318` (`devils_play_x_zero`) TESTED
- Flashback cast from graveyard (Devil's Play specific): NOT TESTED
- Flashback exile after resolution (Devil's Play specific): NOT TESTED
- Flashback exile after being countered (Devil's Play specific): NOT TESTED (generic case tested in `flashback.rs:129` with Geistflame)
- Flashback exile on fizzle (Devil's Play specific): NOT TESTED
- Correct X computation when casting via flashback: NOT TESTED
- Targeting a creature (not a player): NOT TESTED
