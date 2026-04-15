---
id: angel_of_flight_alabaster-01
status: deduped
card: Angel of Flight Alabaster
card_file: mtg-engine/src/cards/isd/angel_of_flight_alabaster.rs
created: 2026-04-14T21:22:40Z
audit_run_id: 2026-04-14-angel_of_flight_alabaster-audit
audit_model: opus
audit_tokens: 20797
audit_duration: 406
deduped_into: merged-your-upkeep-scope-01
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.

**Code:**
> triggers.rs:815-841 — `GameEvent::StepStarted { step }` creates an `UpkeepTrigger` for every battlefield permanent with a `TriggerKind::Upkeep` trigger, regardless of whose upkeep it is. No filter for `controller == active_player`.

**Description:**
The trigger dispatch fires the Angel's upkeep trigger during every player's upkeep, not just the controller's. The oracle text says "At the beginning of YOUR upkeep," meaning it should only trigger during the controller's upkeep. The card's `on_upkeep` handler (angel_of_flight_alabaster.rs:58-59) filters by `state.active_player != controller` and returns early, so the Spirit is not actually returned — but by that point the trigger has already been targeted, placed on the stack, and resolved. This is observable: the controller may be prompted to choose a Spirit target during the opponent's upkeep, the trigger occupies the stack (opponents can respond to it), and "whenever a triggered ability triggers" effects fire incorrectly. The trigger should never be created during the wrong player's upkeep.

**Engine path:**
- triggers.rs:815-841 (dispatch creates trigger for all permanents)
- triggers.rs:1143-1222 (process_pending_trigger_pushes targets and stacks the trigger)
- angel_of_flight_alabaster.rs:58-59 (card-level filter, too late)

**Required check:** 8b

**Affected cards:**
- Angel of Flight Alabaster
- All cards with "At the beginning of your upkeep" triggers: Bloodgift Demon, Endless Ranks of the Dead, Splinterfright, Curse of Oblivion (when checking enchanted player's upkeep), and others
