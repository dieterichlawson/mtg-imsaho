## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- Control change is never reverted at end of turn — engine cleanup step does not process `until_end_of_turn_control_changes`
  - Oracle text says: `"Gain control of target creature until end of turn."`
  - Code does: `state.until_end_of_turn_control_changes.push((*creature_id, original));` records the original controller, but `mtg-engine/src/engine.rs` lines 3020–3025 (the cleanup step) only clears `until_end_of_turn_effects`, `until_end_of_turn_keywords`, `until_end_of_turn_cant_block`, `until_end_of_turn_protection`, and `until_end_of_turn_removed_keywords`. It never iterates `until_end_of_turn_control_changes` to restore `obj.controller = original_controller`. No other location in the engine processes this field. The creature is permanently stolen.

### Tricky interactions checked

- **"Until end of turn" for control**: FAIL — `until_end_of_turn_control_changes` is populated in `traitorous_blood.rs:45` but the engine's cleanup step (`engine.rs:3020–3025`) never reads it to revert the controller. The creature remains under the caster's control indefinitely.
- **"Until end of turn" for haste and trample**: PASS — both keywords are pushed to `until_end_of_turn_keywords` (lines 52–59) which is cleared in cleanup (`engine.rs:3022`). The keyword grants expire correctly.
- **Untap**: PASS — `obj.tapped = false` (line 49) unconditionally untaps the creature on resolution.
- **Targeting any creature (including tapped or own)**: PASS — `TargetRequirement::Creature` generates targets from all creatures on the battlefield via `all_objects_in_zone`, filtered only by hexproof (checked in `can_be_targeted`). No restriction to opponent's creatures, and no filter for tapped status.
- **Target creature must still be on the battlefield at resolution**: PASS — line 41 guards with `o.zone == Zone::Battlefield` before any effect is applied.
- **move_spell_after_resolve (sorcery goes to graveyard)**: PASS — called at line 65; handles flashback exile too.
- **Test for actual control revert at end of turn**: FAIL — `traitorous_blood_reverts_at_end_of_turn` (tier12_cards.rs:384) only asserts that `until_end_of_turn_control_changes` is non-empty and has the right entries. It does not advance to the cleanup step to verify the controller is actually restored. The test gives a false sense of correctness.

### Test coverage

- Control change applied on resolution: `mtg-engine/tests/tier12_cards.rs:358` (tested)
- Untap on resolution: `mtg-engine/tests/tier12_cards.rs:373` (tested)
- Haste granted: `mtg-engine/tests/tier12_cards.rs:376` (tested)
- Trample granted: `mtg-engine/tests/tier12_cards.rs:378` (tested)
- Control reverted at end of turn: NOT TESTED — the existing test (`tier12_cards.rs:384`) only checks that the revert data is recorded, not that the revert executes.
- Targeting a tapped creature: NOT TESTED
- Targeting a creature you already control: NOT TESTED
- Creature leaving battlefield while stolen (does stale entry in control_changes cause problems): NOT TESTED
