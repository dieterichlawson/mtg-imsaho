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
