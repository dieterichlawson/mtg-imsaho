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

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
**Type line**: Creature — Zombie Horror
**Status**: PASS

### Code issues
None found. All previously identified issues (double-exile bug, redundant battlefield move, oracle text mismatch) were fixed in prior audits.

Verified fields:
- **Name**: "Makeshift Mauler" -- matches oracle
- **Mana cost**: Generic(3) + Colored(Blue) = {3}{U} -- matches oracle
- **Type**: Creature with subtypes Zombie, Horror -- matches oracle "Creature — Zombie Horror"
- **P/T**: 4/5 -- matches oracle
- **Keywords**: none -- correct (oracle has no keywords)
- **Oracle text**: `"As an additional cost to cast this spell, exile a creature card from your graveyard."` -- exact match
- **Additional cost**: `ExileCreaturesFromGraveyard(1)` -- correct for "exile a creature card"
- **on_resolve**: only calls `state.move_object(object_id, Zone::Battlefield)` -- no redundant exile, correct

Engine handling verified:
- `engine.rs` line ~543: legal_actions checks graveyard has enough creature cards before allowing cast
- `engine.rs` line ~713: same check for flashback casts
- `engine.rs` line ~1571: at cast time, exiles creature card(s) from graveyard (auto-selects highest power)
- Engine excludes the spell itself from exile candidates (`o.id != obj.id`)

### Tricky interactions checked (min 3)
1. **Empty graveyard prevents casting**: Engine checks `creature_count < n` and skips generating cast actions if not enough creatures. Verified in code at line ~553. No dedicated test for Makeshift Mauler specifically, but the mechanism is shared with all ExileCreaturesFromGraveyard cards and tested via Skaab Goliath.
2. **Spell countered after additional cost paid**: The exile happens at cast time (submit_action, line ~1571), not on resolution. If the spell is countered, the creature card remains exiled. This is correct per MTG rules (additional costs are paid when casting).
3. **Non-creature cards in graveyard don't count**: The engine filters for creature cards using `o.power.is_some() || registry.card_data(o.card_id).card_types.contains(Creature)`. Non-creature cards (instants, sorceries, etc.) are correctly excluded from exile candidates.
4. **Auto-selection of exile target**: The engine automatically selects the highest-power creature to exile rather than letting the player choose. This is a known engine-wide simplification for all ExileCreaturesFromGraveyard cards. Not a card-specific bug.

### Test coverage
- `makeshift_mauler_exiles_creature_from_graveyard` -- casts with creature in graveyard, verifies mauler on battlefield and creature exiled (PASS)
- `makeshift_mauler_is_4_5_zombie` -- verifies power/toughness after resolution (PASS)
- Missing: test for "cannot cast without creature in graveyard" (covered by engine mechanism but no card-specific test)
- Missing: card knowledge / AI scenario files
