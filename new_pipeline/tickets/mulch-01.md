---
id: mulch-01
status: fixed
card: Mulch
audit_run_id: 2026-04-19-mulch-audit
audit_model: sonnet
audit_tokens: 14567
audit_duration: 325
fixed_sha: e3de0667d9633d33b98ff438075a18b875157cc3
fixed_at: 2026-08-24T00:30:46Z
test_file: mtg-engine/tests/multi_target_and_mill.rs
fix_note: bespoke library-to-graveyard moves now go through mill_one, which emits CreatureCardMilled
---

## Audit Finding

**Oracle text:**
> the rest into your graveyard

**Code:**
> // Non-lands go to graveyard.
for &non_land_id in &non_lands {
    state.move_object(non_land_id, Zone::Graveyard, registry);
}

**Description:**
Mulch moves non-land cards from the library to the graveyard inline via `state.move_object(non_land_id, Zone::Graveyard, registry)` without checking whether each card is a creature and, if so, pushing `GameEvent::CreatureCardMilled`. The centralized `mill_cards()` helper (engine.rs:4303) checks the registry for `CardType::Creature` before each move and emits `CreatureCardMilled { object, milled_player }` when a creature goes to the graveyard from the library. Mulch's bespoke library-to-graveyard path bypasses this emit entirely. As a result, if a non-land creature card is among the top four cards revealed by Mulch, any battlefield permanent watching `TriggerKind::CreatureCardMilled` (currently Undead Alchemist, which exiles the creature and creates a Zombie token) will silently fail to trigger. For example, if an opponent controls Undead Alchemist and the active player casts Mulch, milling a non-land creature to their own graveyard, Undead Alchemist's ability never fires.

**Engine path:** mtg-engine/src/cards/isd/mulch.rs:74

**Required check:** 7

## Tests

### mulch_creature_milled_triggers_undead_alchemist
Scenario: Opponent controls Undead Alchemist; active player casts Mulch and the top four cards include a non-land creature; Undead Alchemist should trigger to exile the creature and create a 2/2 Zombie token, but currently does not because CreatureCardMilled is never emitted.

