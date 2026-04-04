## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Draw two cards, then discard a card at random.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Draw and discard atomicity (ruling: "Nothing can happen between the two"): PASS — both actions occur synchronously inside `on_resolve` with no `awaiting_action` set between them; no player action can intervene.
- Discard is mandatory (no "may"): PASS — code performs the discard unconditionally via `hand.choose(&mut rand::thread_rng())` with no player choice prompt.
- Discard is at random (not player's choice): PASS — code uses `rand::thread_rng()` / `SliceRandom::choose()`, which picks uniformly at random without any player input.
- Newly drawn cards are eligible for random discard: PASS — `draw_cards` moves cards to `Zone::Hand` before the discard pool is built; drawn cards are included in the random selection.
- Spell itself is not in the discard pool: PASS — at resolution time the spell's zone is `Zone::Stack` (not `Zone::Hand`), so it is excluded by the filter `o.zone == Zone::Hand && o.owner == controller`.
- Flashback cost {2}{U}: PASS — `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Blue)]))` matches oracle.
- Mana cost {1}{R}: PASS — `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Red)])` matches oracle.
- Flashback spell is exiled after resolution: PASS — `move_spell_after_resolve` checks `cast_with_flashback` flag; `is_flashback` is set to `true` in `submit_action` when cast from graveyard, which sets `obj.cast_with_flashback = true`.
- Flashback spell is exiled when countered: PASS — `resolve_spell` calls `state.move_spell_after_resolve(object_id)` on fizzle/counter; engine test `flashback_spell_countered_is_exiled` confirms this path.
- Double-move guard after `on_resolve`: PASS — `on_resolve` calls `move_spell_after_resolve`; `resolve_spell` lines 107-111 then check `obj.zone == Zone::Stack` before calling it again, preventing a double move since it's already been moved to Graveyard/Exile.
- `Discarded` event `player` field: PASS — code uses `player: owner`, and because the hand is filtered by `o.owner == controller`, `owner` is always equal to `controller`; semantically correct.
- Flashback not offered without sufficient mana: PASS — `legal_actions` uses `generate_cast_actions_with_targets` which checks mana affordability; engine test `flashback_not_offered_without_mana` confirms.
- Instant timing (castable at instant speed): PASS — card type is `CardType::Instant`; engine enforces instant timing correctly.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Draw 2 cards, discard 1 (net hand size +1): `flashback.rs:348` (`desperate_ravings_draws_two_discards_one`) — TESTED
- Discard is at random (not player's choice): NOT TESTED (randomness is hard to deterministically test; no test verifies the discard bypasses player choice)
- Newly drawn cards included in discard pool: NOT TESTED explicitly (implied by net hand size test)
- Flashback cast of Desperate Ravings specifically (exiled after resolution): NOT TESTED — only generic flashback tests use Geistflame/Think Twice/Bump in the Night
- Flashback spell exiled when countered (generic): `flashback.rs:129` (`flashback_spell_countered_is_exiled`) — TESTED (Geistflame, not Desperate Ravings specifically)
- Flashback not offered without mana: `flashback.rs:65` (`flashback_not_offered_without_mana`) — TESTED
- Flashback offered from graveyard: `flashback.rs:23` (`flashback_offered_from_graveyard`) — TESTED
- Atomicity of draw-then-discard (nothing can happen between): NOT TESTED explicitly
- LLM card knowledge description omits "at random": `mtg-player/src/llm.rs` — description reads "Draw 2 cards, discard 1" without noting the random discard; this is a documentation gap but does not affect engine behavior
