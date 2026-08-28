## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/166/traitorous-blood?utm_source=api
**Type line**: `Sorcery` — {1}{R}{R}
**Oracle text**:
```
Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Gain control of target creature **until end of turn**" — a temporary control
  change that reverts at cleanup, and reverts to the *original* controller
  rather than to its owner: PASS
- "**Untap** it. It gains trample and **haste**" — haste is what makes the
  stolen creature able to attack, and all three effects end together: PASS
- Control changing back does not untap or remove it from combat: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The temporary control change and the granted keywords: `control_durations.rs`, `control_change.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/166/traitorous-blood?utm_source=api
**Type line**: `Sorcery` — {1}{R}{R}
**Oracle text**:
```
Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.
```

**Rulings fetched**:
- [2011-09-22] Traitorous Blood can target any creature, even one that’s tapped or one you already control.
- [2011-09-22] Gaining control of a creature doesn’t cause you gain control of any Auras or Equipment attached to it.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.`
**Type line**: `Sorcery` — {1}{R}{R}
**Status**: ISSUE (fixed) — engine-level, found through this card

### Rulings (both 2011-09-22)
1. "Traitorous Blood can target any creature, even one that's tapped or one you already control."
2. "Gaining control of a creature doesn't cause you gain control of any Auras or Equipment attached to it."

### Code issues

- `mtg-engine/src/cards/isd/traitorous_blood.rs:42` and five other cards — "Untap it" was performed by writing the field.
  - Oracle text says: `Untap it.`
  - Code did: `if let Some(obj) = state.get_object_mut(*creature_id) { obj.tapped = false; }`
  - `GameEvent::Untapped` had exactly one producer, the untap step. Six card sites untapped something and emitted nothing: Traitorous Blood, Spidery Grasp, Village Bell-Ringer, Grimgrin Corpse-Born, Civilized Scholar (twice) and Galvanic Juggernaut. Nothing in this pool watches for an untap, so nothing was visibly broken — the first card that does would have seen one untap in seven. The same shape as the hand-written `damage_marked` writes the suite already guards against. Fixed with `GameState::untap` (clears the flag, emits the event) plus a source guard, `test_suite_guards.rs::only_the_untap_helper_untaps_a_permanent`. Setting the flag to `true` stays allowed and is deliberately a different act: a permanent entering tapped was never untapped (CR 614.1c).

- `mtg-engine/src/stack.rs:93` — CR 608.2b did not re-check creature-ness for bare "target creature".
  - `is_target_legal` did: `if matches!(inner_req, TargetRequirement::CreatureWithFilter(_)) && !state.is_creature(*id, registry)`
  - `CreatureWithFilter` got the re-check when seven cards' duplicated preambles were collapsed into it; bare `TargetRequirement::Creature` asks the identical question and was left out. That is most "target creature" spells, Traitorous Blood among them. Extended to both. Nothing in this set turns a creature into a non-creature, so no card can stage it — the existing `fizzle.rs` test builds the state directly for exactly that reason, and now runs over both shapes.

- `mtg-engine/src/cards/isd/traitorous_blood.rs:10` — the behavior struct was spelled `TraiterousBlood`. Renamed.

The card is otherwise right: `{1}{R}{R}`, Sorcery, oracle text verbatim, `TargetRequirement::Creature` with no restriction (ruling 1), control change through `change_control` (which sets summoning sickness, CR 302.6) with a `TemporaryEffect::ChangeControl` recording the revert, and haste and trample as `until_end_of_turn` grants.

### Tricky interactions checked

- Ruling 1, a creature you already control is a legal target: PASS. `TargetRequirement::Creature` carries no controller restriction, so `legal_actions` offers your own. Untested until this audit.
- Ruling 1, a tapped creature: PASS, tested.
- Ruling 2, attached Auras and Equipment do not change hands: PASS. `change_control` sets one object's `controller` and touches nothing attached. Untested through this card until this audit.
- "until end of turn" for the control change: PASS, and reverted by the engine's cleanup rather than by the card — `control_durations.rs:124` runs the game to cleanup rather than replaying its body.
- Summoning sickness on the stolen creature, and haste covering it: PASS, `control_change.rs:64`.
- Order of the three clauses: the control change happens before the untap, which matters only in that the creature must exist for either; no observable difference here.
- Targeting your own creature and it staying yours: PASS — `original == controller`, so the end-of-turn revert is a no-op and the untap and grants still land.
- Redundant preamble: `on_resolve` guards `o.zone == Zone::Battlefield`. Dead code — CR 608.2b is applied by `stack::resolve_spell` before `on_resolve` is called, and with the fix above the re-check now also requires creature-ness. Left in place, deliberately and consistently with the 29 similar `activated_abilities` gates recorded in the Mirror-Mad Phantasm entry: harmless, and removing it is churn rather than correctness.

### Test coverage

- Steals, untaps, grants haste and trample: `cards_spells_and_enchantments.rs:529` `traitorous_blood_steals_untaps_and_grants_keywords`
- Haste beats the summoning sickness the steal causes: `control_change.rs:64` `traitorous_blood_creature_can_still_attack_via_haste`
- Control reverts at cleanup: `control_durations.rs:124` `bug_control_change_not_reverted_at_eot`
- Ruling 1, your own creature is offered and works: `cards_spells_and_enchantments.rs:553` `traitorous_blood_can_target_a_creature_you_already_control`, added this audit
- Ruling 2, the Equipment stays with its controller: `cards_spells_and_enchantments.rs:578` `traitorous_blood_leaves_the_equipment_with_its_owner`, added this audit
- CR 608.2b creature-ness re-check for bare `Creature`: `fizzle.rs:495` `a_target_creature_that_stopped_being_a_creature_is_no_longer_legal`, second row added this audit
- Haste and trample wearing off at end of turn: NOT TESTED per card — `until_end_of_turn` expiry is engine-general and covered there; not duplicated here

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 drop the trample grant | `traitorous_blood_steals_untaps_and_grants_keywords` FAILED | (unchanged) |
| M2 drop the untap | 2 tests FAILED | (unchanged) |
| M3 restrict targets to creatures you do not control | n/a | `traitorous_blood_can_target_a_creature_you_already_control` FAILED |
| M4 also take control of attached Equipment | passed whole workspace | `traitorous_blood_leaves_the_equipment_with_its_owner` FAILED |
| M5 revert the bare-`Creature` re-check in `stack.rs` | n/a (was the gap) | `a_target_creature_that_stopped_being_a_creature_is_no_longer_legal` FAILED |

M3 needed two attempts: the first wrote `is_valid_target(&self, state, source_id: ObjectId, ...)` and did not compile — the trait's second parameter is the caster's `PlayerId`, not the source object. Recorded because a mutation that fails to compile proves nothing and must not be counted as a pass.

Source restored from `/tmp/tb.bak`, `/tmp/tb2.bak` and `/tmp/stack.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1477 passing (was 1474). `cargo check --workspace --all-targets` clean, zero warnings.
