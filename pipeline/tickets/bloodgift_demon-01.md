---
id: bloodgift_demon-01
status: new
card: Bloodgift Demon
card_file: mtg-engine/src/cards/isd/bloodgift_demon.rs
created: 2026-04-14T21:19:54Z
audit_run_id: 2026-04-14-bloodgift_demon-audit
audit_model: opus
audit_tokens: 12882
audit_duration: 241
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, target player draws a card and loses 1 life.

**Code:**
> `target_requirement: None` (bloodgift_demon.rs:34)
> `fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], ...)` (bloodgift_demon.rs:40)
> The `on_upkeep` method ignores `chosen_targets` (parameter prefixed with `_`) and instead presents its own `ResolutionChoice::ChooseTarget` at resolution time (bloodgift_demon.rs:57-66).

**Description:**
The oracle text says "target player," which per CR 603.3d means the target must be chosen when the triggered ability is put on the stack — not when it resolves. The implementation sets `target_requirement: None` in the `TriggeredAbilityDef`, causing the trigger to go on the stack untargeted via `process_pending_trigger_pushes` (triggers.rs:1163-1166). The target is then chosen at resolution inside `on_upkeep`. This has three consequences: (1) opponents get priority to respond without knowing who the target is, (2) the CR 603.3c rule that a trigger with no legal targets is never placed on the stack cannot apply (the trigger always goes on the stack), and (3) the CR 608.2b resolution-time legality recheck does not occur. The engine already supports targeted triggers — `TargetRequirement::PlayerOnly` exists and `process_pending_trigger_pushes` (triggers.rs:1169-1218) correctly handles target selection at trigger-queue time with hexproof filtering, auto-pick for single targets, and fizzle for zero targets.

**Engine path:**
- mtg-engine/src/cards/isd/bloodgift_demon.rs:34 (`target_requirement: None`)
- mtg-engine/src/cards/isd/bloodgift_demon.rs:40 (`_chosen_targets` unused)
- mtg-engine/src/cards/isd/bloodgift_demon.rs:57-66 (manual target selection at resolution)
- mtg-engine/src/triggers.rs:1163-1166 (untargeted triggers bypass target selection)
- mtg-engine/src/triggers.rs:1169-1218 (existing targeted trigger infrastructure)

**Required check:** 8b, 8f

**Affected cards:**
- Bloodgift Demon
- Any other card with a targeted triggered ability that uses `target_requirement: None` and implements targeting manually in its on_resolve/on_upkeep handler

