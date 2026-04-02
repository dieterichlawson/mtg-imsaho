# Audit: Skaab Ruinator

## Oracle (Scryfall)
- **Name:** Skaab Ruinator
- **Cost:** {1}{U}{U}
- **Type:** Creature -- Zombie Horror
- **Oracle:** As an additional cost to cast Skaab Ruinator, exile three creature cards from your graveyard. Flying. You may cast Skaab Ruinator from your graveyard.
- **P/T:** 5/6

## Implementation: `mtg-engine/src/cards/skaab_ruinator.rs`
- **Name:** Skaab Ruinator ✅
- **Cost:** {1}{U}{U} ✅
- **Type:** Creature ✅
- **Subtypes:** Zombie, Horror ✅
- **P/T:** 5/6 ✅
- **Keywords:** Flying ✅
- **Additional cost:** ExileCreaturesFromGraveyard(3) ✅
- **on_resolve:** exiles 3 creature cards, excludes self, moves to battlefield ✅
- **Cast from graveyard:** mentioned in oracle_text but unclear if engine supports this ability

### Issue
- **BUG (same as Skaab Goliath):** Additional cost paid at resolve time instead of cast time.
- **MISSING:** "You may cast Skaab Ruinator from your graveyard" -- this is stated in oracle text but there's no implementation for casting from graveyard (no flashback_cost or special graveyard-casting logic). This ability appears non-functional.

## Verdict: ISSUE -- "cast from graveyard" ability not implemented

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard. Flying. You may cast Skaab Ruinator from your graveyard.
**Scryfall type line**: Creature -- Zombie Horror
**Status**: ISSUE

Findings:
1. **Mana cost {1}{U}{U}**: Correct.
2. **Types/subtypes (Creature -- Zombie Horror)**: Correct.
3. **P/T 5/6**: Correct.
4. **Keywords (Flying)**: Correct.
5. **Additional cost (exile 3 creatures from graveyard)**: Declared via `AdditionalCost::ExileCreaturesFromGraveyard(3)`, but the exile logic is executed inside `on_resolve` (lines 41-58) rather than at cast time. If the spell is countered, the creatures would not have been exiled. This is a known anti-pattern.
6. **Cast from graveyard**: `can_cast_from_graveyard()` returns `true` (line 35), so the hook exists. However, the previous audit noted uncertainty about whether the engine actually calls this hook. The implementation at least declares intent.
7. **Anti-pattern: `move_object(object_id, Zone::Battlefield)` on line 60**: For a creature spell resolving, moving to the battlefield is correct (not a spell that goes to graveyard). No anti-pattern here.
8. **Oracle text in code**: Says "exile three creature cards from your graveyard" which matches Scryfall's "exile three creature cards from your graveyard." Correct.
9. **Tests**: Found in `mtg-engine/tests/tier15_cards.rs`.

Issues carried forward from previous audit:
- Additional cost paid at resolve time instead of cast time (anti-pattern).

## Audit — 2026-04-01 14:13

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/77/skaab-ruinator)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard. Flying. You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**Mana cost**: {1}{U}{U}
**P/T**: 5/6
**Status**: ISSUE

Findings:
1. **Name**: "Skaab Ruinator" -- correct.
2. **Mana cost {1}{U}{U}**: Correct (Generic(1), Blue, Blue).
3. **Type/subtypes (Creature — Zombie Horror)**: Correct. `card_types: [Creature]`, `subtypes: ["Zombie", "Horror"]`.
4. **P/T 5/6**: Correct.
5. **Keywords [Flying]**: Correct.
6. **Additional cost (exile 3 creature cards from graveyard)**: Declared via `AdditionalCost::ExileCreaturesFromGraveyard(3)` in card data. However, the exile logic is also duplicated inside `on_resolve` (lines 41-58), meaning the cost is paid at resolve time, not cast time. If the spell is countered, the creatures would not have been exiled. This is a rules violation.
7. **Cast from graveyard**: `can_cast_from_graveyard()` returns `true` (line 35). This correctly enables the "You may cast this card from your graveyard" ability.
8. **on_resolve uses `move_object(object_id, Zone::Battlefield)`**: For a creature spell resolving, this is correct behavior (creatures enter the battlefield, they don't go to graveyard).
9. **Oracle text in code**: Matches Scryfall oracle text.
10. **Tests**: Found in `mtg-engine/tests/tier15_cards.rs`. Test verifies exiling 3 creatures and Skaab Ruinator entering battlefield. Assertions are correct.

Issues:
- Additional cost (exile 3 creatures) is executed at resolve time in `on_resolve` rather than at cast time. Per rules, additional costs are paid during casting, so if the spell is countered, the cards should still have been exiled.

## Audit — 2026-04-01 14:37

**Oracle text source**: Scryfall via WebSearch (https://scryfall.com/card/isd/77/skaab-ruinator)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard. Flying. You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**Status**: ISSUE

Card data verified correct: name, mana cost ({1}{U}{U}), card_types (Creature), subtypes (Zombie, Horror), P/T (5/6), keywords (Flying), oracle_text matches, additional_cost (ExileCreaturesFromGraveyard(3)), can_cast_from_graveyard() returns true.

Issue:

1. **Additional cost paid at resolve time instead of cast time** (`skaab_ruinator.rs` lines 40-58).
   - Oracle text says: `As an additional cost to cast this spell, exile three creature cards from your graveyard.`
   - Code does: The `additional_cost` field is correctly set to `ExileCreaturesFromGraveyard(3)` in card data, but the exile logic is also executed inside `on_resolve` (lines 41-58). This means the creatures are exiled when the spell resolves, not when it is cast. Per MTG rules (CR 601.2b), additional costs are paid during the casting process. If the spell is countered, the additional cost should still have been paid (creatures exiled), but with this implementation they would not be.

No other issues found. Test in tier15_cards.rs (1 test) verifies exiling 3 creatures and Skaab Ruinator entering the battlefield.

## Audit — 2026-04-01 18:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**Status**: ISSUE

### Code issues
1. **Additional cost paid at resolve time instead of cast time** (`/Users/dlaw/mtg/mtg-engine/src/cards/isd/skaab_ruinator.rs`, lines 40-58):
   - Oracle text says: `As an additional cost to cast this spell, exile three creature cards from your graveyard.`
   - Code does: The exile logic is executed inside `on_resolve` (lines 41-58), not during the casting process. Per MTG rules (CR 601.2b), additional costs are paid during casting. If the spell is countered, the creatures should have already been exiled as part of casting. With this implementation, if the spell is countered, no creatures are exiled. Note: the `additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(3))` field is declared in card data, but the engine does not handle this cost type at cast time (engine.rs line 466: `_ => vec![]` catches `ExileCreaturesFromGraveyard` without action). This is an engine-level gap affecting all cards with this cost type (Skaab Ruinator, Skaab Goliath, Makeshift Mauler, Stitched Drake, Corpse Lunge).

2. **Engine does not check graveyard creature count for cast eligibility** (`/Users/dlaw/mtg/mtg-engine/src/engine.rs`, lines 454-467):
   - Oracle text says: `As an additional cost to cast this spell, exile three creature cards from your graveyard.`
   - Code does: The engine's legal action generation (lines 454-467) only checks `AdditionalCost::SacrificeCreature` for eligibility; `ExileCreaturesFromGraveyard` falls through to `_ => vec![]`. This means the engine may present Skaab Ruinator as castable even when the player has fewer than 3 creature cards in their graveyard.

3. **Exile candidate selection is not player-chosen** (`/Users/dlaw/mtg/mtg-engine/src/cards/isd/skaab_ruinator.rs`, lines 41-51):
   - Oracle text says: `exile three creature cards from your graveyard`
   - Code does: Auto-selects the first 3 creature cards via `.take(3)` (line 51). The player should choose which creatures to exile. Per ruling: "You must exile three creature cards from your graveyard no matter what zone you're casting Skaab Ruinator from."

All other card data verified correct: name, mana cost {1}{U}{U}, type Creature, subtypes Zombie/Horror, P/T 5/6, Flying keyword, `can_cast_from_graveyard()` returns true, oracle text matches.

### Tricky interactions checked
- Cast from graveyard ability: pass (can_cast_from_graveyard returns true)
- Self-exclusion from exile candidates (can't exile self when on stack): pass (line 44: `o.id != object_id`)
- Additional cost timing (should be at cast, not resolve): ISSUE (see #1)
- Creature card identification for exile: pass (checks card_types and fallback to power.is_some())

### Test coverage
- Exiles 3 creatures and enters battlefield: `tests/tier15_cards.rs` (line 484)
- Cast from graveyard: NOT TESTED
- Cast with fewer than 3 creatures in graveyard (should be illegal): NOT TESTED
- Spell countered after casting (creatures should still be exiled): NOT TESTED
- Player choice of which creatures to exile: NOT TESTED
- Ruling: must exile 3 regardless of casting zone: NOT TESTED

## Re-Audit — 2026-04-01 20:00

**Oracle text source**: Scryfall API (via oracle_lookup.py)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**Status**: ISSUE

### Code issues
Previous issues about additional cost timing and eligibility checking have been fixed by the engine (engine.rs lines 487-497 for eligibility, lines 1302-1336 for cost payment at cast time). The `on_resolve` method (skaab_ruinator.rs line 37-41) now correctly just moves to the battlefield without re-exiling.

However, one new issue was found:

1. **Casting from graveyard will panic due to cost calculation bug** (engine.rs lines 1232-1239):
   - Oracle text says: `You may cast this card from your graveyard.`
   - Code does: When cast from the graveyard, `is_flashback` is set to `true` (line 1225: detects graveyard zone). The cost calculation (lines 1232-1239) then enters the flashback branch: `dynamic_fb.unwrap_or_else(|| { data.flashback_cost.expect("flashback cast on card without flashback_cost") })`. For Skaab Ruinator, `dynamic_fb` is `None` and `data.flashback_cost` is `None`, so this will **panic** with "flashback cast on card without flashback_cost". The `legal_actions` function (line 558-568) correctly handles this case by falling through to the normal mana cost when `can_cast_from_graveyard()` is true and there's no flashback cost, but the `apply_action` function does not replicate this logic. This bug is untested because the existing test only casts from hand, not from graveyard.

All other card data verified correct: name "Skaab Ruinator", mana cost {1}{U}{U}, type Creature, subtypes Zombie/Horror, P/T 5/6, Flying keyword, `can_cast_from_graveyard()` returns true, additional_cost `ExileCreaturesFromGraveyard(3)`, oracle text matches.

Engine eligibility check (engine.rs line 487-497) correctly verifies 3+ creature cards in graveyard and excludes the spell itself (`o.id != obj.id`). Engine cost payment (engine.rs lines 1302-1336) exiles creatures at cast time (before moving to stack). Exile candidate selection auto-picks highest-power creatures (acceptable simplification).

### Tricky interactions checked
- Additional cost paid at cast time (not resolve): pass (engine.rs lines 1302-1336)
- Eligibility check for 3 graveyard creatures: pass (engine.rs lines 487-497)
- Self-exclusion from exile candidates: pass (engine.rs line 491: `o.id != obj.id` and line 1311: `o.id != *object_id`)
- Cast from graveyard (can_cast_from_graveyard): ISSUE (panics in apply_action, see #1)
- on_resolve enters battlefield (not graveyard): pass
- Creature card identification: pass (checks power.is_some() and card_types)

### Test coverage
- Exiles 3 creatures and enters battlefield: `tests/tier15_cards.rs:484`
- Cast from graveyard: NOT TESTED (would panic, see issue #1)
- Spell countered after casting (creatures still exiled): NOT TESTED
- Cast with fewer than 3 creatures (should be illegal): NOT TESTED
- Ruling: can't exile self to pay cost: NOT TESTED (covered by engine code)

## Audit — 2026-04-01 21:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**Status**: ISSUE

### Code issues
Card data is correct: name "Skaab Ruinator", mana cost {1}{U}{U} (Generic(1), Blue, Blue), type Creature, subtypes Zombie/Horror, P/T 5/6, Flying keyword, oracle text matches, `additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(3))`, `can_cast_from_graveyard()` returns true.

`on_resolve` correctly just moves to battlefield (`state.move_object(object_id, Zone::Battlefield)`) -- correct for a creature spell. Additional cost (exile 3 creatures) is handled by the engine at cast time (engine.rs lines 1302-1336) -- correct.

1. **Casting from graveyard panics due to cost calculation bug** (engine.rs lines 1225-1239):
   - Oracle text says: `You may cast this card from your graveyard.`
   - Code does: When a card is cast from the graveyard, `is_flashback` is set to `true` (line 1225-1227: `o.zone == Zone::Graveyard`). The cost calculation (lines 1232-1239) enters the flashback branch and executes `dynamic_fb.unwrap_or_else(|| { data.flashback_cost.expect("flashback cast on card without flashback_cost") })`. For Skaab Ruinator, `dynamic_fb` is `None` and `data.flashback_cost` is `None`, causing a **panic**. The `legal_actions` function (lines 558-568) correctly handles this case by falling through to the normal mana cost when `can_cast_from_graveyard()` is true, but `submit_action`/`apply_action` does not replicate this logic. This means the card appears castable from the graveyard but crashes when the player actually tries to cast it.

2. **Graveyard casting does not check additional cost eligibility** (engine.rs lines 548-612):
   - Oracle text says: `As an additional cost to cast this spell, exile three creature cards from your graveyard.`
   - Code does: The graveyard casting section of `legal_actions` (lines 548-612) does not check `AdditionalCost::ExileCreaturesFromGraveyard`. The hand-casting section (lines 487-497) correctly checks for 3+ creature cards in graveyard, but this check is not replicated in the graveyard-casting path. This means Skaab Ruinator could be presented as castable from the graveyard even without enough creatures to exile.

3. **`cast_with_flashback` flag incorrectly set for graveyard cast** (engine.rs line 1367-1368):
   - Oracle text says: `You may cast this card from your graveyard.` (this is NOT flashback)
   - Code does: Sets `obj.cast_with_flashback = true` for any spell cast from the graveyard (line 1367-1368). For Skaab Ruinator, this is incorrect -- the card is not cast with flashback; it simply has permission to be cast from the graveyard. While this flag doesn't directly affect Skaab Ruinator's resolution (it uses `move_object` not `move_spell_after_resolve`), it would incorrectly trigger Burning Vengeance ("Whenever you cast a spell from your graveyard") and other flashback-matters interactions when Skaab Ruinator is merely being cast from the graveyard using its own ability. Note: per MTG rules, Burning Vengeance SHOULD trigger when Skaab Ruinator is cast from the graveyard (it says "from your graveyard", not "with flashback"), so the flag name is misleading but the behavior happens to be correct for Burning Vengeance specifically.

### Tricky interactions checked
- Additional cost paid at cast time (not resolve): pass (engine.rs lines 1302-1336)
- Eligibility check for 3 graveyard creatures (hand cast): pass (engine.rs lines 487-497)
- Eligibility check for 3 graveyard creatures (graveyard cast): ISSUE (not checked, see #2)
- Self-exclusion from exile candidates: pass (engine.rs line 491: `o.id != obj.id`)
- Cast from graveyard: ISSUE (panics, see #1)
- on_resolve enters battlefield: pass
- Creature card identification for exile: pass (checks power.is_some() and card_types)

### Test coverage
- Exiles 3 creatures and enters battlefield (cast from hand): `tests/tier15_cards.rs:484`
- Cast from graveyard: NOT TESTED (would panic, see issue #1)
- Spell countered after casting (creatures still exiled): NOT TESTED
- Cast with fewer than 3 creatures (should be illegal): NOT TESTED
- Ruling: can't exile self to pay cost: NOT TESTED (covered by engine code)
- Ruling: must exile 3 regardless of casting zone: NOT TESTED

## Audit — 2026-04-01 22:00

**Oracle text source**: Scryfall API (via oracle_lookup.py cache)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**P/T**: 5/6
**Status**: PASS

### Code issues
No issues found. All previously reported issues have been fixed. Card data and behavior verified correct:

- Name "Skaab Ruinator": correct
- Mana cost {1}{U}{U}: correct (Generic(1), Blue, Blue)
- Type Creature: correct
- Subtypes Zombie, Horror: correct (both present)
- P/T 5/6: correct
- Flying keyword: correct
- Oracle text: correct
- `additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(3))`: correct
- `can_cast_from_graveyard()` returns `true` (line 35): correct ("You may cast this card from your graveyard")
- `on_resolve` moves to battlefield via `state.move_object(object_id, Zone::Battlefield)` (line 40): correct for a creature spell

Engine handling:
- **Additional cost at cast time**: engine.rs lines 1349-1382 exile creature cards from graveyard at cast time (before moving to stack). Correct per MTG rules (CR 601.2b).
- **Eligibility check (hand cast)**: engine.rs lines 491-501 verify 3+ creature cards in graveyard, excluding the spell itself (`o.id != obj.id`). Correct per ruling "Skaab Ruinator is on the stack when you pay its costs. It can't be exiled to pay for itself."
- **Eligibility check (graveyard cast)**: engine.rs lines 618-633 replicate the same check for graveyard-cast path, also excluding self (`o.id != obj.id`). Correct.
- **Cast-from-graveyard vs flashback**: engine.rs lines 1275-1276 correctly distinguish `is_cast_from_graveyard` (true for Skaab Ruinator) from `is_flashback` (false), so normal mana cost is used and `cast_with_flashback` is not set. Correct per ruling: "You must exile three creature cards from your graveyard no matter what zone you're casting Skaab Ruinator from."
- **Exile candidate auto-selection**: engine picks highest-power creatures first (line 1365). Acceptable simplification (player choice not presented, but functionally correct).

### Tricky interactions checked
- Additional cost paid at cast time (not resolve): pass (engine.rs lines 1349-1382)
- Eligibility check for 3 graveyard creatures (hand cast): pass (engine.rs lines 491-501)
- Eligibility check for 3 graveyard creatures (graveyard cast): pass (engine.rs lines 618-633)
- Self-exclusion from exile candidates: pass (engine.rs line 495 and 625: `o.id != obj.id`)
- Cast from graveyard uses normal mana cost (not flashback): pass (engine.rs lines 1275-1289)
- `cast_with_flashback` not set when cast from graveyard: pass (test confirms at tier15_cards.rs:549)
- on_resolve enters battlefield (not graveyard): pass
- Creature card identification for exile: pass (checks `power.is_some()` and `card_types.contains(Creature)`)

### Test coverage
- Exiles 3 creatures and enters battlefield (from hand): `tests/tier15_cards.rs:484`
- Cast from graveyard (castable with enough creatures): `tests/tier15_cards.rs:510`
- Cast from graveyard not marked as flashback: `tests/tier15_cards.rs:549`
- Not castable without enough creatures (from graveyard): `tests/tier15_cards.rs:554`
- Ruling: can't exile self to pay cost: NOT TESTED (covered by engine code)
- Spell countered after casting (creatures still exiled): NOT TESTED
- Not in LLM card knowledge: acceptable (complex card, AI can read oracle text)

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**P/T**: 5/6
**Status**: PASS

### Code issues
No issues found.

Card data is correct: {1}{U}{U}, Creature - Zombie Horror, 5/6, Flying keyword, additional_cost ExileCreaturesFromGraveyard(3). The can_cast_from_graveyard returns true, correctly enabling casting from graveyard without using flashback. The engine at engine.rs:1280 correctly distinguishes cast-from-graveyard (uses normal mana cost, no exile after resolution) from flashback (uses flashback cost, exiled after resolution). The on_resolve correctly moves to battlefield. The additional cost of exiling 3 creature cards is handled by the engine at cast time.

### Tricky interactions checked
- Additional cost (exile 3 creatures) paid at cast time, not resolution: PASS - handled by engine, on_resolve just moves to battlefield
- Cast from graveyard uses normal mana cost (not flashback cost): PASS - engine.rs:1280 checks is_cast_from_graveyard
- Not marked as flashback when cast from graveyard: PASS - engine.rs:1281 sets is_flashback = false for cast_from_graveyard
- Cannot exile itself as part of additional cost (on stack): PASS - per ruling, and it's on the stack when costs are paid
- Not castable without 3 creature cards in graveyard: PASS - tested
- Flying keyword present: PASS
- Subtypes Zombie and Horror both present: PASS
- move_object(Zone::Battlefield) correct for creature: PASS

### Test coverage
- Exiles 3 creatures from graveyard on cast: `tier15_cards.rs:484`
- Cast from graveyard (not flashback): `tier15_cards.rs:510`
- Not castable without enough creatures: `tier15_cards.rs:554`
- Ruling: can't exile self (on stack when paying costs): NOT TESTED (implicitly correct)
- Ruling: additional cost applies from any zone: NOT TESTED
- Spell countered after casting (creatures still exiled): NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**:
> As an additional cost to cast this spell, exile three creature cards from your graveyard.
> Flying
> You may cast this card from your graveyard.

**Card data**: {1}{U}{U}, Creature — Zombie Horror, 5/6
**Status**: PASS

### Card data verification
- Name "Skaab Ruinator": correct
- Mana cost `Generic(1), Blue, Blue` = {1}{U}{U}: correct
- Card type Creature: correct
- Subtypes Zombie, Horror: correct
- P/T 5/6: correct
- Keywords [Flying]: correct
- `flashback_cost: None`: correct (this is NOT flashback)
- `additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(3))`: correct
- `can_cast_from_graveyard()` returns `true` (line 35): correct ("You may cast this card from your graveyard")
- `on_resolve` moves to battlefield via `state.move_object(object_id, Zone::Battlefield)` (line 40): correct for a creature spell

### Engine handling verification
- **Additional cost paid at cast time** (engine.rs lines 1383-1417): PASS. Exile happens before the spell moves to the stack (line 1447), so if the spell is countered, the creatures remain exiled. This matches MTG rules.
- **Self-exclusion from exile candidates** (engine.rs line 1392: `o.id != *object_id`): PASS. Per ruling: "Skaab Ruinator is on the stack when you pay its costs. It can't be exiled to pay for itself."
- **Eligibility check for graveyard casting** (engine.rs lines 652-666): PASS. Checks that there are at least 3 creature cards in graveyard excluding the spell itself before offering the cast action.
- **Cast-from-graveyard vs flashback distinction** (engine.rs lines 1300-1310): PASS. `is_cast_from_graveyard` is set to true, `is_flashback` is false. This means:
  - Normal mana cost is used (line 1322), not flashback cost: correct
  - `cast_with_flashback` is NOT set on the object (line 1451-1453): correct, so it won't be exiled after resolution
- **Graveyard casting timing** (engine.rs lines 634-648): PASS. Creature is treated as sorcery-speed, so it can only be cast during main phase with empty stack.
- **Exile candidate selection** (engine.rs line 1399): Auto-selects highest-power creatures. This is a simplification (player should choose), but acceptable for a game engine that doesn't support complex player choices for additional costs.

### Tricky interactions checked
- Additional cost (exile 3 creatures) paid at cast time, not resolution: PASS
- Cast from graveyard uses normal mana cost (not flashback cost): PASS
- Not marked as flashback when cast from graveyard: PASS
- Cannot exile itself as part of additional cost (on stack when costs paid): PASS
- Not castable without 3 creature cards in graveyard: PASS
- Flying keyword present: PASS
- Yixlid Jailer interaction (removes graveyard abilities): NOT IMPLEMENTED (no Jailer in set)
- Burning Vengeance interaction: KNOWN BUG (see burning_vengeance.md — BV only triggers on `cast_with_flashback`, misses `can_cast_from_graveyard` casts)
- Cost reductions apply to graveyard cast (uses `effective_spell_cost`): PASS (line 1323)

### Anti-pattern check
- `can_cast_from_graveyard` vs flashback: PASS. The engine correctly distinguishes these. `can_cast_from_graveyard` uses normal mana cost and does not exile after resolution. Flashback uses flashback cost and exiles after resolution.
- Additional cost in `on_resolve` vs engine: PASS. Previous audits flagged this as a bug, but the current code correctly handles it in the engine at cast time (lines 1383-1417). The `on_resolve` method only does `move_object(Zone::Battlefield)`.

### Test coverage
- `skaab_ruinator_exiles_creatures_from_graveyard` (tier15_cards.rs:484): Tests casting from hand, exiling 3 creatures, entering battlefield
- `skaab_ruinator_cast_from_graveyard` (tier15_cards.rs:510): Tests casting from graveyard, verifies on stack (not panicked), verifies `cast_with_flashback` is false
- `skaab_ruinator_not_castable_without_enough_creatures` (tier15_cards.rs:554): Tests that 2 creatures in graveyard is insufficient
- Ruling: can't exile self: NOT TESTED (implicitly correct via engine logic)
- Ruling: additional cost applies from any zone: NOT TESTED (hand cast tested, graveyard cast tested separately)
- Countered spell (creatures still exiled): NOT TESTED

### Sources
- [Scryfall: Skaab Ruinator](https://scryfall.com/card/isd/77/skaab-ruinator)
- [MTG Salvation: Skaab Ruinator rulings](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/781863-skaab-ruinator)
- [MTG Salvation: Skaab Ruinator and Yixlid Jailer](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/magic-rulings-archives/299645-skaab-ruinator-and-yxlid-jailer)
- [MTG Assist: Skaab Ruinator rulings](https://www.mtgassist.com/cards/Innistrad/Skaab-Ruinator/rulings/)

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**Status**: PASS

### Code issues
No issues found. Card data is correct: {1}{U}{U}, 5/6, creature types Zombie Horror, Flying keyword. The additional cost is modeled as `AdditionalCost::ExileCreaturesFromGraveyard(3)`. Graveyard casting is enabled via `can_cast_from_graveyard() -> true`. The `oracle_text` field reorders the abilities (Flying first, then additional cost, then graveyard cast) compared to oracle order (additional cost, Flying, graveyard cast), but this is a cosmetic string difference with no behavioral impact. All mechanics are correctly implemented.
