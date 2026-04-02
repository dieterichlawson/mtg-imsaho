# Audit: Grasp of Phantoms

## Oracle Text (Scryfall)
> **Grasp of Phantoms** {3}{U}
> Sorcery
> Put target creature on top of its owner's library.
> Flashback {7}{U} *(You may cast this card from your graveyard for its flashback cost. Then exile it.)*

## Implementation: `mtg-engine/src/cards/isd/grasp_of_phantoms.rs`

### Card Data
- **Name**: "Grasp of Phantoms" — CORRECT
- **Mana cost**: `Generic(3), Colored(Blue)` — CORRECT ({3}{U})
- **Card type**: `Sorcery` — CORRECT
- **Flashback cost**: `Generic(7), Colored(Blue)` — CORRECT ({7}{U})
- **Keywords**: `vec![]` — Acceptable; flashback is modeled via the `flashback_cost` field, not the `Keyword` enum (which has no `Flashback` variant).

### Targeting
- `target_requirement()` returns `TargetRequirement::Creature` — CORRECT. Oracle says "target creature."

### on_resolve
- Checks `targets.first()` for `Target::Object(target_id)` — CORRECT
- Verifies target creature is on the battlefield (`obj.zone == Zone::Battlefield`) — CORRECT
- Uses `obj.owner` to determine which library to place on top of — CORRECT ("its owner's library")
- Calls `state.move_object(*target_id, Zone::Library)` then `state.get_player_mut(owner).library_order.insert(0, *target_id)` — CORRECT. Verified that `move_object` only sets `obj.zone` and does NOT modify `library_order`, so there is no double-add. The explicit `insert(0, ...)` correctly places the card on top.
- Calls `state.move_spell_after_resolve(object_id)` at end — CORRECT (sends spell to graveyard, or exile if cast with flashback)

### Previous Audit Concern Resolved
The prior audit flagged a "POTENTIAL ISSUE: Library insertion may double-add." This is NOT an issue. `move_object()` (state.rs:443) only changes `obj.zone`, `zone_change_count`, and battlefield-specific state. It does not touch `library_order`. The manual `library_order.insert(0, ...)` is the sole insertion point, consistent with the pattern used elsewhere in the codebase (e.g., `engine.rs:2109`).

### Tests (`mtg-engine/tests/tier11_cards.rs`)
- `grasp_of_phantoms_puts_creature_on_top_of_library` — verifies creature ends in library zone and is at position 0 of owner's library_order. PASSES.
- `grasp_of_phantoms_has_flashback` — verifies flashback_cost is `Some`. PASSES.

## Verdict
**NO ISSUES FOUND.** The implementation correctly matches the oracle text.
