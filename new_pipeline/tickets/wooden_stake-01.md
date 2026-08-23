---
id: wooden_stake-01
status: fixed
card: Wooden Stake
audit_run_id: 2026-04-19-wooden_stake-audit
audit_model: sonnet
audit_tokens: 26131
audit_duration: 436
fixed_at: 2026-08-23T17:06:20Z
fix_note: verified fixed on master by inspection: should_trigger_on_blocks / _on_becomes_blocked implemented and consulted at dispatch; regression coverage present
---

## Audit Finding

**Oracle text:**
> Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.

**Code:**
> for (eq_id, eq_card_id, eq_controller) in attached {
    if registry.get(eq_card_id).is_some() {
        let desc = trigger_description(registry, eq_card_id, &crate::cards::TriggerKind::Blocks, false);
        if !desc.is_empty() {
            let trigger = PendingTrigger::BlocksTrigger {
                object_id: eq_id,
                card_id: eq_card_id,
                controller: eq_controller,
                blocked_attacker: *attacker_id,
                description: desc,
            };

**Description:**
The `BlockersDeclared` handler in `collect_triggers` (triggers.rs:1059-1076 for Blocks, 1106-1124 for BecomesBlocked) creates a `BlocksTrigger` / `BecomesBlockedTrigger` for every block involving equipment that declares those trigger kinds — without checking whether the other creature is a Vampire. The Vampire condition is part of the trigger event itself ('blocks a Vampire', 'becomes blocked by a Vampire'), not an intervening-if clause, so per CR 603.2 the trigger should only go on the stack when a Vampire is actually involved. Instead, the engine fires the trigger unconditionally and defers the Vampire check to `on_blocks` / `on_becomes_blocked` at resolution time. When the equipped creature blocks or is blocked by a non-Vampire, a spurious 'destroy that Vampire' trigger appears on the stack, granting both players an illegal priority window before it resolves with no effect.

**Engine path:** mtg-engine/src/triggers.rs:1059

**Required check:** 8b

**Affected cards:**
- Wooden Stake

## Tests

### equipped_creature_blocks_non_vampire_no_trigger
Scenario: Equipped creature (bearing Wooden Stake) blocks a non-Vampire attacker; verify that no trigger is placed on the stack.

### equipped_creature_blocked_by_non_vampire_no_trigger
Scenario: Equipped creature (bearing Wooden Stake) attacks and is blocked by a non-Vampire; verify that no trigger is placed on the stack.

