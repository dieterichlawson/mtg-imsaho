## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Missing `is_valid_target` override to enforce "target opponent" restriction
  - Oracle text says: `"Target opponent sacrifices a creature of their choice."`
  - Code does: `fn target_requirement(&self) -> TargetRequirement { TargetRequirement::PlayerOnly }` with no `is_valid_target` override. The default `is_valid_target` (in `cards/mod.rs:290`) returns `true` for any player, so the caster can legally target themselves. The `PlayerOnly` engine path in `engine.rs:883-898` iterates all players and adds them as valid cast targets if `is_valid_target` returns true. A comparable card, `BumpInTheNight` (`cards/isd/bump_in_the_night.rs:34-40`), correctly restricts with `*pid != caster`. `TributeToHunger` does not.

### Tricky interactions checked

- "Target opponent" restricts casting to non-self targets: FAIL — no `is_valid_target` override; caster can target themselves.
- Toughness captured before sacrifice (ruling: use last-known value on battlefield): PASS — `engine.rs:2316-2318` reads `effective_toughness` before calling `destruction::sacrifice`.
- Opponent (not caster) chooses which creature to sacrifice: PASS — `present_target_choice` is called with `controller: opponent` (`tribute_to_hunger.rs:61`), presenting the choice to the opponent.
- Mandatory sacrifice (no "you may"): PASS — `present_target_choice` is called with `optional: false` (`tribute_to_hunger.rs:69`).
- Auto-apply when opponent has exactly one creature: PASS — `helpers.rs:129-134` auto-applies when `targets.len() == 1 && !optional`, which is correct since there is no real choice.
- No-creature case (opponent has no creatures, spell does nothing): PASS — `tribute_to_hunger.rs:51-55` returns early with `move_spell_after_resolve` when `opp_creatures.is_empty()`.
- Life gain of 0 when toughness is 0: PASS — `engine.rs:2324` checks `if toughness > 0`; gaining 0 life is a no-op per MTG rule 119.3.
- `move_spell_after_resolve` called after resolution: PASS — called in both paths (empty-creature early-return at `tribute_to_hunger.rs:53`, and in the `SacrificeAndGainLife` handler at `engine.rs:2335`).

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Basic sacrifice + life gain equal to toughness: `tier8_cards.rs` — `tribute_to_hunger_opponent_sacs_and_gain_life`
- No creatures does nothing: `tier8_cards.rs` — `tribute_to_hunger_no_creatures_does_nothing`
- "Target opponent" restriction (cannot self-target): NOT TESTED
- Last-known toughness with continuous modifiers (e.g., creature had +X/+X buff): NOT TESTED
- Multiple creatures — opponent presented a real choice: NOT TESTED
- Toughness 0 creature — no life gain event: NOT TESTED
