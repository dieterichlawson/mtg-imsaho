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
