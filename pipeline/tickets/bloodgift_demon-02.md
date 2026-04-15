---
id: bloodgift_demon-02
status: closed-duplicate
card: Bloodgift Demon
card_file: mtg-engine/src/cards/isd/bloodgift_demon.rs
created: 2026-04-14T21:19:54Z
audit_run_id: 2026-04-14-bloodgift_demon-audit
audit_model: opus
audit_tokens: 12882
audit_duration: 241
duplicate_of: merged-your-upkeep-scope-02
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, target player draws a card and loses 1 life.

**Code:**
> triggers.rs:815-859 — `StepStarted { step: Upkeep }` handler iterates ALL battlefield permanents with `TriggerKind::Upkeep` and queues triggers for each, regardless of whose upkeep step it is.
> bloodgift_demon.rs:44-46 — `if state.active_player != controller { return; }` filters at resolution time.

**Description:**
The oracle says "At the beginning of **your** upkeep," meaning the trigger event is specifically the controller's upkeep step beginning. Per CR 603.2, the trigger should only fire when its trigger event occurs. However, the engine's `StepStarted::Upkeep` handler (triggers.rs:822-859) queues an `UpkeepTrigger` for every battlefield permanent with `TriggerKind::Upkeep`, regardless of whether the current upkeep belongs to that permanent's controller. The card compensates by checking `state.active_player != controller` in `on_upkeep` and returning early, but the trigger still goes on the stack during opponents' upkeeps. This is observable: opponents see a "Bloodgift Demon's upkeep trigger" on the stack during their own upkeep, may waste responses on it (e.g., killing the Demon), and then the trigger resolves with no effect. The engine has no mechanism to distinguish `TriggerKind::YourUpkeep` from `TriggerKind::EachUpkeep` — this is an engine-level limitation.

**Engine path:**
- mtg-engine/src/triggers.rs:815-859 (upkeep trigger queuing — no controller filtering)
- mtg-engine/src/cards/isd/bloodgift_demon.rs:44-46 (resolution-time active_player check)

**Required check:** 8b

**Affected cards:**
- Bloodgift Demon
- All cards using `TriggerKind::Upkeep` with "your upkeep" text (e.g., Angel of Flight Alabaster, Curse of the Pierced Heart, Curse of Oblivion, Endless Ranks of the Dead, Splinterfright, and all DFC transform-check triggers)
