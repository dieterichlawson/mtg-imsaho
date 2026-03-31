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
