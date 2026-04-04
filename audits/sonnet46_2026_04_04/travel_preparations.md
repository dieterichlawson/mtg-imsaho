## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Put a +1/+1 counter on each of up to two target creatures.
Flashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- LLM card knowledge in `mtg-player/src/llm.rs` line 111 describes one target instead of up to two
  - Oracle text says: `"Put a +1/+1 counter on each of up to two target creatures."`
  - Code does: `"- Travel Preparations ({1}{G} sorcery, flashback {1}{W}): Put a +1/+1 counter on target creature."` — says "target creature" (singular, no "up to two"), so the LLM player will never plan to target two creatures and will never cast this spell for full value.

### Tricky interactions checked

- **Mana cost** ({1}{G} normal, {1}{W} flashback): Correctly encoded as `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Green)])` for main cost and `Some(ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)]))` for flashback_cost — PASS
- **oracle_text field**: Correctly set to `"Put a +1/+1 counter on each of up to two target creatures."` — PASS
- **Card type (Sorcery)**: Correctly declared as `vec![CardType::Sorcery]` — PASS
- **UpToTargets 0–2 combinations**: `generate_cast_actions_with_targets` for `UpToTargets(2, Creature)` generates combinations for k in 0..=min(2, options.len()), including casting with 0 targets (valid per "up to" rules) and 1 or 2 distinct targets — PASS
- **No duplicate targets**: `target_combinations` generates C(n,k) combinations (not permutations), advancing the slice index after each pick, so the same target can never appear twice in one cast action — correctly enforces the ruling "You can't target the same creature twice" — PASS
- **Partial target invalidation (one of two targets becomes illegal)**: Stack resolution in `stack.rs` line 80 uses `any_legal` (at least one valid target is sufficient). `on_resolve` individually checks `o.zone == Zone::Battlefield` for each target before adding the counter, so the remaining legal target still gets a counter — correctly implements "If one target is illegal by resolution time, you'll still put a +1/+1 counter on the other creature" — PASS
- **All targets illegal (fizzle)**: When all targets are illegal at resolution, `any_legal` is false, `move_spell_after_resolve` is called, and the spell correctly goes to exile (flashback) or graveyard (normal cast) without resolving — PASS
- **Flashback exile on resolution**: `move_spell_after_resolve` checks `cast_with_flashback` flag (set at cast time via `engine.rs` line 1637) and routes to `Zone::Exile` instead of `Zone::Graveyard` — PASS
- **Flashback exile on counter/fizzle**: Fizzle path also calls `move_spell_after_resolve`, so flashback-cast Travel Preparations is correctly exiled even when countered or fizzled — matches ruling "A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way" — PASS
- **Sorcery timing enforced for flashback**: `engine.rs` lines 692–704 check `is_sorcery_speed` for sorcery-type spells cast from graveyard — PASS
- **on_resolve uses move_spell_after_resolve (not move_object)**: Correctly uses `state.move_spell_after_resolve(object_id)` so flashback cast goes to exile, not graveyard — PASS

### Test coverage

For each ruling and tricky interaction:
- Basic one-target case (adds +1/+1 counter): `flashback.rs:259` (`travel_preparations_adds_counter`) — TESTED
- Two-targets case (both creatures get counters): NOT TESTED
- Zero-targets case (cast with no creatures on battlefield, spell resolves doing nothing): NOT TESTED
- Partial target invalidation (one target leaves battlefield before resolution, other still gets counter): NOT TESTED
- No-duplicate-targets enforcement: NOT TESTED
- Flashback cast exiles the spell: covered by system test `flashback_spell_is_exiled_after_resolve` (using Geistflame), but not tested specifically for Travel Preparations: NOT TESTED for this card
- Flashback countered/fizzled → still exiled: covered by `flashback_spell_countered_is_exiled` (Geistflame), not Travel Preparations specifically: NOT TESTED for this card
- Sorcery timing restriction for flashback: NOT TESTED for this card
- LLM description accuracy (up to two targets): NOT TESTED
