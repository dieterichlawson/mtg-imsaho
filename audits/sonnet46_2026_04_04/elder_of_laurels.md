## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
**Type line**: Creature — Human Advisor
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Creature count at resolution vs. activation**: The count is computed inside `on_activate_ability` (called at line 1802 of `engine.rs`), which is resolution time in this engine. The oracle ruling states "The number of creatures you control is counted as the ability resolves." Correct — PASS.
- **Fixed bonus after resolution**: The bonus is stored as fixed `power_mod`/`toughness_mod` i32 values in a `UntilEndOfTurnEffect` struct (elder_of_laurels.rs:63-69). `effective_power`/`effective_toughness` in `state.rs:886-891, 928-932` sum those fixed values. Matches ruling: "Once the ability has resolved, the bonus won't change if the number of creatures you control changes later in the turn." — PASS.
- **Until end of turn cleanup**: `state.until_end_of_turn_effects.clear()` is called at `engine.rs:3021` in the `Step::Cleanup` handler. Effects correctly expire at end of turn — PASS.
- **Target requirement (any creature, no controller restriction)**: `TargetRequirement::Creature` (elder_of_laurels.rs:45) causes `engine.rs:867-878` to scan `all_objects_in_zone(Zone::Battlefield)` (all controllers), matching oracle text "Target creature" with no restriction — PASS.
- **No tap cost**: `requires_tap: false` (line 43) is correct; the oracle text has no tap symbol — PASS.
- **No once-per-turn restriction**: `once_per_turn: false` (line 47) is correct; the oracle text has no such restriction — PASS.
- **No sorcery-speed restriction**: `sorcery_speed_only: false` (line 48) is correct; the oracle text has no timing restriction — PASS.
- **Creature count proxy (`power.is_some()`)**: The code uses `o.power.is_some()` to identify creatures (elder_of_laurels.rs:58), consistent with the engine-wide convention used throughout `engine.rs` (lines 840, 869, 1052, etc.) — PASS.
- **Elder of Laurels itself included in X**: Since the ability costs only mana (no sacrifice, no tap), the Elder is still on the battlefield when `on_activate_ability` fires, so it counts toward X. Oracle text says "creatures you control" with no exclusion of the source — PASS.
- **Target fizzle check**: The code verifies the target is still on the battlefield before applying the effect (elder_of_laurels.rs:62). Since abilities resolve synchronously in this engine this check is redundant but not wrong — PASS.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Creature count at resolution: `tier10_cards.rs:33` (`elder_of_laurels_pumps_by_creature_count`) — TESTED (3 creatures present, asserts +3/+3 on target)
- Fixed bonus after resolution (bonus doesn't change with subsequent creature count changes): NOT TESTED
- Until end of turn cleanup (effects clear at end of turn): NOT TESTED
- Target can be any creature (including opponent's): NOT TESTED
- No tap requirement: `tier10_cards.rs:33` — implicitly tested (ability activated without tapping)
- Card data (mana cost, P/T, subtypes): `tier10_cards.rs:20` (`elder_of_laurels_card_data`) — TESTED
- Elder itself counted in X: `tier10_cards.rs:33` — TESTED (elder is one of the 3 counted)
- Multiple activations stacking: NOT TESTED
