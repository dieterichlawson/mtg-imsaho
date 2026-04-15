---
id: bitterheart_witch-01
status: deduped
card: Bitterheart Witch
card_file: mtg-engine/src/cards/isd/bitterheart_witch.rs
created: 2026-04-14T21:20:46Z
audit_run_id: 2026-04-14-bitterheart_witch-audit
audit_model: opus
audit_tokens: 13463
audit_duration: 293
deduped_into: merged-target-as-choose-01
---

## Audit Finding

**Oracle text:**
> When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to **target** player, then shuffle.

**Code:**
> `target_requirement: None` (bitterheart_witch.rs:59)
> Player choice presented during resolution via `present_player_choice` (bitterheart_witch.rs:14-34) and `on_yes_no_choice` (bitterheart_witch.rs:79-130)

**Description:**
The oracle text says "target player", which per CR 603.3d requires the target to be chosen when the triggered ability is put on the stack — not during resolution. The implementation declares `target_requirement: None` in its `TriggeredAbilityDef`, so the trigger goes on the stack untargeted (triggers.rs:596-601, `chosen_targets: Vec::new()`). The player selection happens entirely during resolution: first a YesNo prompt for "you may", then a library search, then a `ChooseTarget` prompt for the player. This means (a) the trigger uses "choose" semantics rather than "target" semantics, bypassing CR 115 targeting rules; (b) opponents cannot respond to the specific target before the trigger resolves; (c) the trigger will not fizzle if all legal player targets become illegal between stacking and resolution (e.g., gaining hexproof); (d) the `_chosen_targets` parameter passed to `on_dies` is always empty and is ignored. The engine already supports targeted SelfDies triggers — Falkenrath Noble (falkenrath_noble.rs:33) uses `target_requirement: Some(TargetRequirement::PlayerOnly)` with the same `SelfDies` trigger kind.

**Engine path:**
- bitterheart_witch.rs:55-61 (TriggeredAbilityDef with target_requirement: None)
- triggers.rs:1163-1166 (untargeted trigger pushed directly to stack)
- triggers.rs:1251-1253 (on_dies called with empty chosen_targets)
- bitterheart_witch.rs:65-77 (on_dies ignores chosen_targets, presents YesNo)
- bitterheart_witch.rs:107-129 (player choice deferred to resolution)

**Required check:** 8f

**Affected cards:**
- Bitterheart Witch
