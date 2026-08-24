---
id: trepanation_blade-01
status: fixed
card: Trepanation Blade
audit_run_id: 2026-04-19-trepanation_blade-audit
audit_model: sonnet
audit_tokens: 20596
audit_duration: 350
fixed_sha: f81a55c
fixed_at: 2026-08-24T01:25:42Z
test_file: mtg-engine/tests/trigger_snapshots.rs
fix_note: AttacksTrigger carries the attacker and defending player as AttackInfo; the mill is unconditional and the buff goes to the creature that attacked.
---

## Audit Finding

**Oracle text:**
> Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.

**Code:**
> let creature_id = match equip.attached_to.or_else(|| equip.card_state.get("last_attached_to").copied()) {
    Some(id) => id,
    None => return,
};

**Description:**
The `on_attacks` handler resolves against the equipment's current `attached_to` field rather than the attacking creature captured at trigger-creation time. `PendingTrigger::AttacksTrigger` stores only the equipment's `object_id` — no attacking creature ID, no defending player. If the equipped creature dies between trigger creation (attackers declared) and trigger resolution, SBA at `sba.rs:162` sets `obj.attached_to = None` directly without writing `card_state["last_attached_to"]` (that field is only set by `move_object` when the equipment itself leaves the battlefield). The `last_attached_to` fallback is therefore always absent during attack-trigger resolution and provides no help. The handler hits `None => return` at line 63 and skips ALL effects — including the mandatory mill — even though the trigger was already placed on the stack and must resolve per oracle text. The mill should still occur; only the buff is legitimately skipped when the creature is no longer on the battlefield. A second variant of the same root bug: if the equipment is re-equipped to a different creature between trigger creation and resolution, `attached_to` points to the new creature. Its combat state is unrelated to the original attack, so the buff applies to the wrong creature (the new one) rather than the one that actually attacked.

**Engine path:** mtg-engine/src/cards/isd/trepanation_blade.rs:61

**Required check:** 8b

## Tests

### mill_occurs_when_equipped_creature_dies_before_trigger_resolves
Scenario: Equipped creature attacks; before the Trepanation Blade trigger resolves, opponent kills the creature with an instant; verify the defending player still mills cards until a land (no buff applied since creature is gone).

### buff_targets_attacking_creature_not_current_equipped_creature
Scenario: Equipped creature A attacks (trigger queued); controller re-equips Trepanation Blade to creature B before the trigger resolves; verify the mill targets the original defending player and the +1/+0 buff, if any, applies to creature A (not creature B).

