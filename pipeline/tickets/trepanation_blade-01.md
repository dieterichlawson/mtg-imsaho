---
id: trepanation_blade-01
status: closed-duplicate
card: Trepanation Blade
card_file: mtg-engine/src/cards/isd/trepanation_blade.rs
created: 2026-04-15T03:52:30Z
audit_run_id: 2026-04-14-trepanation_blade-audit
audit_model: opus
audit_tokens: 28121
audit_duration: 580
duplicate_of: merged-trigger-source-zone-gate-02
---

## Audit Finding

**Oracle text:**
> Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card.

**Code:**
> `triggers.rs:1332-1337`:
> ```rust
> PendingTrigger::AttacksTrigger { object_id, card_id, chosen_targets, .. } => {
>     if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
>         if let Some(behavior) = registry.get(card_id) {
>             behavior.on_attacks(state, object_id, &chosen_targets, registry);
>         }
>     }
> ```

**Description:**
The `AttacksTrigger` resolution path gates execution on the source object (`object_id`, which is the equipment for Trepanation Blade) still being on the battlefield. Per CR 113.7a, "Once activated or triggered, an ability exists on the stack independently of its source." If Trepanation Blade is destroyed in response to its own attack trigger (e.g., an opponent casts Shatter), the triggered ability should still resolve — the defending player should still mill cards and the creature should still get the buff. Instead, the zone gate silently drops the entire trigger. This is engine-wide: the same gate applies to all `AttacksTrigger`, `BlocksTrigger`, and similar trigger kinds.

**Engine path:**
- triggers.rs:1332-1337

**Required check:** 8b

**Affected cards:**
- Trepanation Blade
- All cards with equipment/permanent-based attack triggers (Geist of Saint Traft, Kessig Cagebreakers, Grimgrin Corpse-Born, Hamlet Captain, etc.)

## Tests

### equipment_attack_trigger_resolves_after_equipment_destroyed
Source ticket: (new)
Implementation: (not yet written)
Scenario: Equip Trepanation Blade to a creature. Set up defender's library with [nonland, land]. Declare the creature as an attacker, which puts the Blade's attack trigger on the stack. Before the trigger resolves, destroy the Blade (move it to graveyard). Resolve the trigger. Assert that the defender's library was milled (both cards in graveyard) and the creature received +2/+0 until end of turn, despite the equipment no longer being on the battlefield.
