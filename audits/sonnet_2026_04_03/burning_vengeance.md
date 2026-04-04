## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
- **Major bug: Only triggers on flashback, not all spells cast from graveyard** (/Users/dlaw/mtg/mtg-engine/src/cards/isd/burning_vengeance.rs:47-53)
  - Oracle text says: `Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.`
  - Code does: Only checks `cast_with_flashback` flag, which is only set for flashback spells, not for other spells cast from graveyard like Skaab Ruinator (`can_cast_from_graveyard()` cards)

- **Misleading log message** (/Users/dlaw/mtg/mtg-engine/src/cards/isd/burning_vengeance.rs:68)
  - Oracle text says: `to any target` 
  - Code does: Log message says "deals 2 damage to opponent (flashback spell cast)" but target hasn't been chosen yet and should say "from your graveyard" not "flashback"

### Tricky interactions checked
- **Flashback vs other graveyard-cast spells**: FAIL - Only triggers on flashback but should trigger on all graveyard casts
- **Target choice timing**: PASS - Correctly presents "any target" choice to player
- **Controller check**: PASS - Only triggers on caster's own spells
- **Trigger timing**: PASS - Uses SpellCast trigger which fires before spell resolution
- **Copy spells**: NOT TESTED - No test verifies copies don't trigger (which is correct behavior)
- **Activated abilities from graveyard**: NOT TESTED - No test verifies unearth/similar abilities don't trigger (which is correct)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Triggers on flashback spells**: `mtg-engine/tests/tier12_cards.rs:282`
- **Doesn't trigger on non-graveyard casts**: `mtg-engine/tests/tier12_cards.rs:329`
- **Triggers on non-flashback graveyard casts (Skaab Ruinator)**: NOT TESTED
- **Doesn't trigger on activated abilities from graveyard**: NOT TESTED
- **Doesn't trigger on spell copies**: NOT TESTED
- **Target choice presents any target**: INDIRECTLY TESTED - test resolves by targeting opponent but doesn't verify all valid targets available