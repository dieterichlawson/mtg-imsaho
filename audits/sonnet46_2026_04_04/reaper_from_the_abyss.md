## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying\nMorbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.
**Type line**: Creature — Demon
**Status**: ISSUE

### Code issues

- Intervening-if clause not enforced at trigger collection time (`mtg-engine/src/triggers.rs:604–641`, `mtg-engine/src/cards/isd/reaper_from_the_abyss.rs:34–37,47–49`)
  - Oracle text says: `"Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature."`
  - Code does: In `collect_triggers`, the `StepStarted { step: Step::EndStep }` handler (triggers.rs lines 597–642) unconditionally queues an `EndStepTrigger` for any permanent whose `TriggerKind::EndStep` description is non-empty. The Reaper's `TriggeredAbilityDef` has `description: "destroy target non-Demon creature"` (non-empty), so the trigger is placed on the stack at every end step regardless of whether a creature died this turn. The morbid condition check (`if !state.creature_died_this_turn { return; }`) only occurs in `on_end_step` at resolution time (reaper_from_the_abyss.rs line 47). Per CR 603.4, "At the beginning of each end step, if a creature died this turn" is an intervening-if clause: the trigger should not be placed on the stack at all unless the condition is true when the trigger event occurs. As implemented, in turns where no creature died, the trigger appears on the stack, giving all players an opportunity to respond with instants/activated abilities that they would not have in a real game, before fizzling harmlessly.

### Tricky interactions checked

- Intervening-if clause (condition checked at trigger time vs. resolution time): FAIL — trigger always queued unconditionally; condition only checked at resolution (see issue above).
- Mandatory targeting (optional = false, matching ruling "the morbid ability is mandatory"): PASS — `present_target_choice` called with `optional: false`; single-target case auto-applies without player choice.
- Reaper excluded as a valid target of its own ability: PASS — `o.id != self_id` filter plus Demon subtype check double-excludes the Reaper itself.
- Demon subtype check covers both registry data and runtime object subtypes (for tokens): PASS — code checks `registry.card_data(o.card_id).map(...).unwrap_or(false) || o.subtypes.iter().any(|s| s == "Demon")`.
- No valid targets (all creatures are Demons): PASS — `targets.is_empty()` causes early return with no effect; correct for a targeted triggered ability that fizzles.
- `creature_died_this_turn` flag lifecycle (set on death, cleared on new turn): PASS — set in `destruction.rs:100` and `sba.rs:96,144`; cleared at new turn start in `engine.rs:2888`.
- Trigger fires on EACH end step (not just active player's): PASS — `StepStarted { step: Step::EndStep }` fires for every player's end step; trigger collection scans all battlefield permanents.
- Reaper leaves battlefield before trigger resolves: PASS — `resolve_next_trigger` in `triggers.rs:961–967` checks `o.zone == Zone::Battlefield` before calling `on_end_step`; also confirmed in `on_end_step` itself (reaper_from_the_abyss.rs:42–45).
- `try_destroy` used (respects indestructible and regeneration): PASS — `apply_pending_effect` for `DestroyCreature` calls `crate::destruction::try_destroy` (engine.rs:2274).
- Mana cost {3}{B}{B}{B}: PASS — `ManaCost::new(vec![Generic(3), Colored(Black), Colored(Black), Colored(Black)])`.
- P/T 6/6: PASS — `power: Some(6), toughness: Some(6)`.
- Flying keyword: PASS — `keywords: vec![Keyword::Flying]`.
- Subtype Demon: PASS — `subtypes: vec!["Demon".into()]`.

### Test coverage

- Morbid condition met, destroys non-Demon creature: `tier7_cards.rs:133` (`reaper_destroys_non_demon_on_morbid_end_step`) TESTED
- Morbid condition not met, no destruction: `tier7_cards.rs:155` (`reaper_no_trigger_without_morbid`) TESTED (but test only validates end result — does not catch that trigger incorrectly appears on stack when morbid is absent)
- Intervening-if: trigger must NOT appear on stack when no creature died: NOT TESTED
- Multiple non-Demon targets (player must choose): NOT TESTED
- Controller's own non-Demon creature is a valid target: NOT TESTED
- Demon token immune to destruction (object-level subtype check): NOT TESTED
- No valid targets (all non-Reaper creatures are Demons): NOT TESTED
- Reaper leaves battlefield before trigger resolves (trigger suppressed): NOT TESTED
- Target is indestructible (try_destroy returns Indestructible): NOT TESTED
