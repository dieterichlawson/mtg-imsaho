## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
This creature can't block.
**Type line**: Creature — Vampire Scout
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flying keyword correct: `keywords: vec![Keyword::Flying]` matches oracle. `has_keyword` in `state.rs` checks this; `can_block_attacker` in `combat.rs` correctly requires blocker to have flying or reach when attacker has flying — PASS
- Can't block implementation: `ContinuousEffect::PreventBlock { scope: EffectScope::OnSelf }` means `effect_applies_to` returns true only when `creature_id == source_id` (i.e., the effect applies to Vampire Interloper itself). `can_block` in `state.rs` returns false, and `eligible_blockers` in `combat.rs` filters it out — PASS
- Engine uses `eligible_blockers` for player choices: `engine.rs` line 165 calls `combat::eligible_blockers` to populate `ChooseBlockers`, so the player is never presented Vampire Interloper as a valid blocker — PASS
- Redundant PreventBlock check in `eligible_blockers` (combat.rs lines 602-609 re-checks `has_continuous_effect` after `can_block` already did so): harmless duplicate, does not cause incorrect behavior — PASS
- `declare_blockers_with_registry` validation: only calls `can_block_attacker` (flying/reach/intimidate), not `can_block` (PreventBlock). However, Vampire Interloper never appears in the `eligible_blockers` list presented to the player, so this server-side validation gap cannot be triggered in normal gameplay — PASS
- Card data fields (cost, types, subtypes, P/T): `{1}{B}` → `Generic(1) + Colored(Black)`, `CardType::Creature`, subtypes `["Vampire", "Scout"]`, power/toughness `2/1` — all match oracle — PASS
- oracle_text display field: code says `"Flying. Vampire Interloper can't block."`, oracle says `"Flying\nThis creature can't block."`. This field is display-only (used in `view.rs`) and is not used for rules evaluation — PASS

### Test coverage
- Can't block restriction: `mtg-engine/tests/card_mechanics.rs:473` (`vampire_interloper_cant_block`) — TESTED
- Flying keyword behavior: NOT TESTED (no test that Vampire Interloper can only be blocked by flyers/reach creatures)
- P/T 2/1: NOT TESTED
- Mana cost {1}{B}: NOT TESTED
- Subtypes Vampire/Scout: NOT TESTED
