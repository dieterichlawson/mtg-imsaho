## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Up to two target creatures can't block this turn.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Up to two" allows 0 targets**: Engine generates cast actions with k starting at 0 in `generate_cast_actions_with_targets` (`engine.rs:1011`), so casting with 0 targets is valid. Correct per MTG rules.
- **"This turn" — cleanup**: `until_end_of_turn_cant_block.clear()` is called in the Cleanup step at `engine.rs:3023`. Effect correctly expires at end of turn.
- **Combat enforcement**: `combat.rs:611` filters `eligible_blockers` by `!state.until_end_of_turn_cant_block.contains(&id)`. Creatures targeted by the spell are excluded from legal blockers.
- **Flashback exile**: `state.move_spell_after_resolve` (`state.rs:1132`) checks `obj.cast_with_flashback` and sends the card to Exile if true, Graveyard otherwise. Correct for both normal and flashback casts.
- **Flashback timing (sorcery speed)**: Engine at `engine.rs:692-706` checks `is_sorcery_speed` for sorcery-type cards in the graveyard flashback loop. Correct — you cannot flash back a sorcery at instant speed.
- **Target validity at resolution**: `on_resolve` checks `o.zone == Zone::Battlefield` before pushing to `until_end_of_turn_cant_block` (`nightbirds_clutches.rs:37`). If a target leaves the battlefield in response, it is skipped cleanly.
- **`Keyword::Flashback` absent from `keywords: vec![]`**: No `Keyword::Flashback` exists in the engine's `Keyword` enum (`types.rs:289`); flashback is represented solely via `flashback_cost` in `CardData`. This is the correct engine convention — not an issue.
- **Sorcery countered/leaves-stack exile**: Per rulings, a flashback spell is exiled even if countered. The engine handles this in `stack.rs:84` and `stack.rs:109` which call `move_spell_after_resolve` for spells removed from the stack via fizzle or counter. Correct.
- **Same creature targeted twice**: `target_combinations` in the engine generates true combinations (no duplicates by index), so the same creature cannot be selected twice for the two target slots. Correct.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Normal cast prevents blocking: `mtg-engine/tests/card_mechanics.rs:175` (`nightbirds_clutches_prevents_blocking`)
- `until_end_of_turn_cant_block` set on target: `mtg-engine/tests/flashback.rs:452` (`nightbirds_clutches_taps_creature`) — note: test is misnamed (says "taps" but card does not tap; test correctly checks `cant_block`)
- Eligible blockers correctly excludes targeted creature: `mtg-engine/tests/card_mechanics.rs:175` (calls `combat::eligible_blockers` and asserts target absent)
- Flashback cast of Nightbird's Clutches exiles the card: NOT TESTED (general flashback exile is tested for Bump in the Night at `flashback.rs:469`, but no test casts Nightbird's Clutches specifically via flashback from the graveyard)
- "Up to two" — casting with 0 targets: NOT TESTED
- "Up to two" — casting with 1 target: both test files cast with exactly 1 target, so partial coverage exists
- "This turn" cleanup (effect clears at end of turn): NOT TESTED for this card specifically
