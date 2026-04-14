---
id: geist_of_saint_traft-01
status: new
card: Geist of Saint Traft
card_file: mtg-engine/src/cards/isd/geist_of_saint_traft.rs
created: 2026-04-14T21:28:51Z
audit_run_id: 2026-04-14-geist_of_saint_traft-audit
audit_model: opus
audit_tokens: 20037
audit_duration: 493
---

## Audit Finding

**Oracle text:**
> Exile that token at end of combat.

**Code:**
> `state.end_of_combat_exiles.push(token_id);` (geist_of_saint_traft.rs:86)
> `let exiles: Vec<_> = state.end_of_combat_exiles.drain(..).collect(); for exile_id in exiles { ... state.move_object(exile_id, Zone::Exile, registry); }` (combat.rs:638-641)

**Description:**
Per CR 603.7d, "Exile that token at end of combat" creates a delayed triggered ability. Delayed triggered abilities use the same trigger mechanism as other triggered abilities — they go on the stack at the beginning of the end of combat step and can be responded to (CR 603.7). The current implementation uses `end_of_combat_exiles`, a Vec processed as a turn-based action inside `combat::end_combat()`, which is called by `perform_turn_based_actions` at the start of the EndCombat step (engine.rs:4540). This executes BEFORE `collect_triggers` runs (engine.rs:4748), meaning the tokens are exiled before any EndCombat triggers fire and before players receive priority. Players cannot respond to the exile (e.g., sacrificing the token for value before it's exiled, or using Sundial of the Infinite to end the turn). Additionally, the card registers a `TriggerKind::EndCombat` triggered ability (line 37-40) whose `on_end_combat` handler is the default no-op — this puts a phantom "exile the Angel token" trigger on the stack that does nothing because the exile already happened.

**Engine path:**
- geist_of_saint_traft.rs:86 — pushes token ID to `end_of_combat_exiles`
- combat.rs:635-644 — `end_combat()` drains and processes exiles as turn-based action
- engine.rs:4539-4541 — `perform_turn_based_actions` calls `end_combat()` before triggers
- engine.rs:4748 — `collect_triggers` runs after turn-based actions
- geist_of_saint_traft.rs:37-40 — phantom EndCombat trigger with no handler

**Required check:** 8b

**Affected cards:**
- Geist of Saint Traft
- Any future card using `end_of_combat_exiles` mechanism

