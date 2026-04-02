# Audit: Makeshift Mauler

## Oracle (Official)
- **Name:** Makeshift Mauler
- **Cost:** {3}{U}
- **Type:** Creature — Zombie Horror
- **Oracle:** As an additional cost to cast this spell, exile a creature card from your graveyard.
- **P/T:** 4/5

## Implementation
- Name: "Makeshift Mauler" -- CORRECT
- Cost: {3}{U} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Zombie", "Horror"] -- CORRECT
- P/T: 4/5 -- CORRECT
- Oracle text matches -- CORRECT
- Additional cost: ExileCreaturesFromGraveyard(1) -- CORRECT
- On resolve: exiles a creature card from graveyard, then enters battlefield -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Makeshift Mauler
- **Cost:** {3}{U}
- **Type:** Creature — Zombie Horror
- **P/T:** 4/5
- **Oracle Text:** As an additional cost to cast this spell, exile a creature card from your graveyard.

### Card Data Checks
- [x] Name: "Makeshift Mauler" — correct
- [x] Cost: {3}{U} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Zombie, Horror — correct
- [x] P/T: 4/5 — correct
- [x] Additional cost: ExileCreaturesFromGraveyard(1) — correct
- [ ] Oracle text: minor mismatch (cosmetic)
  - **Oracle:** `"As an additional cost to cast this spell, exile a creature card from your graveyard."`
  - **Implementation:** `"As an additional cost to cast Makeshift Mauler, exile a creature card from your graveyard."`
  - Note: Scryfall uses modern "this spell" templating; implementation uses card name. Functionally equivalent.

### Behavior Checks
- [ ] **ISSUE: `on_resolve` performs a redundant exile.** The engine already handles `AdditionalCost::ExileCreaturesFromGraveyard(1)` at cast time (see `engine.rs` line ~1491). The card's `on_resolve` method (lines 33-55) exiles another creature from the graveyard on resolution, causing a double-exile. Other cards with the same additional cost (Skaab Ruinator, Skaab Goliath) do NOT implement `on_resolve`.
- [ ] **ISSUE: `on_resolve` manually moves to battlefield (line 54).** Creature spells are normally placed on the battlefield by the engine's resolution logic. This explicit `move_object(object_id, Zone::Battlefield)` may conflict with or duplicate the engine's default creature resolution.

### Result: ISSUE

**Issues found:**
1. **Double exile bug:** `on_resolve` exiles an additional creature card from the graveyard, but the engine already exiles one at cast time via `AdditionalCost::ExileCreaturesFromGraveyard(1)`.
2. **Redundant battlefield move:** `on_resolve` explicitly moves the creature to the battlefield, which may conflict with default creature resolution.

## Re-audit — 2026-04-02
**Status**: PASS
Previously fixed bug re-verified: AdditionalCost::ExileCreaturesFromGraveyard(1) correctly requires exiling a creature card from graveyard. Oracle text already matches Scryfall. Doc comment updated to use "this spell". Behavior unchanged.
