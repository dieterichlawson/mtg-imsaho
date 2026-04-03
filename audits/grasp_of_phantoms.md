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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Put target creature on top of its owner's library.\nFlashback {7}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found. Card data matches oracle: name, mana cost {3}{U}, type Sorcery, flashback cost {7}{U}. The on_resolve correctly moves target creature to Zone::Library and inserts at position 0 in library_order (top of library). Follows established codebase pattern for library placement (same as engine.rs PutOnTopOfLibrary handler). move_spell_after_resolve called. Target requirement is Creature. No anti-patterns.

## Audit — 2026-04-02 21:09

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/58/grasp-of-phantoms)
**Oracle text**: Put target creature on top of its owner's library.\nFlashback {7}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
None. All card data fields match oracle text exactly:
- Name: "Grasp of Phantoms" matches
- Mana cost: Generic(3) + Colored(Blue) matches {3}{U}
- Type: Sorcery matches
- Flashback cost: Generic(7) + Colored(Blue) matches {7}{U}
- Target: TargetRequirement::Creature matches "target creature"
- on_resolve uses `obj.owner` (not controller) for library placement, matching "its owner's library"
- `move_object` to Zone::Library + `library_order.insert(0, ...)` correctly places on top without duplication
- `move_spell_after_resolve` correctly handles graveyard/exile based on flashback flag

### Tricky interactions checked (min 3)
1. **Target removed before resolution**: on_resolve checks `obj.zone == Zone::Battlefield` (line 41). If the creature was bounced/killed before resolution, the spell does nothing. Correct -- the target is illegal and the spell fizzles.
2. **Owner vs controller (stolen creature)**: Uses `obj.owner` (line 43), not `obj.controller`. If a creature is controlled by player A but owned by player B, it correctly goes on top of player B's library. Matches oracle "its owner's library."
3. **Flashback exile**: `move_spell_after_resolve` (line 52) checks `cast_with_flashback` flag. If cast via flashback ({7}{U}), the spell is exiled instead of going to graveyard. Verified in state.rs lines 1132-1141.
4. **No duplicate library_order entry**: Confirmed `move_object` (state.rs:452-517) does NOT modify `library_order`. The explicit `insert(0, ...)` is the sole insertion, consistent with engine.rs:2309-2311 pattern.

### Test coverage
- `grasp_of_phantoms_puts_creature_on_top_of_library` (tier11_cards.rs:280) -- verifies creature moves to Library zone and is at position 0 in owner's library_order. PASSES.
- `grasp_of_phantoms_has_flashback` (tier11_cards.rs:296) -- verifies flashback_cost is Some. PASSES.
- Missing: no test for casting via flashback (verifying exile after resolution), no test for fizzling when target is removed. These are engine-level behaviors tested elsewhere.
