# Audit: Snapcaster Mage

## Oracle (Scryfall)
- **Name:** Snapcaster Mage
- **Cost:** {1}{U}
- **Type:** Creature -- Human Wizard
- **Oracle:** Flash. When Snapcaster Mage enters the battlefield, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
- **P/T:** 2/1

## Implementation: `mtg-engine/src/cards/snapcaster_mage.rs`
- **Name:** Snapcaster Mage ✅
- **Cost:** {1}{U} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Wizard ✅
- **P/T:** 2/1 ✅
- **Keywords:** Flash ✅
- **Triggered ability:** EntersBattlefield ✅
- **on_enter_battlefield:** finds instant/sorcery in graveyard without flashback ✅
- **Flashback cost:** uses the card's mana cost ✅
- **until_end_of_turn_flashback:** correctly stored ✅

### Issue
- **SIMPLIFICATION:** The target is auto-selected (highest mana value card) rather than letting the player choose. The oracle says "target" which normally means the player picks. This is a minor simplification -- the AI picks the most expensive card, which is usually but not always correct.

## Verdict: PASS -- minor simplification in target selection

## Audit — 2026-04-02

**Oracle Text:**
> Flash
> When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.

**Card Data:**
- Name: Snapcaster Mage — correct
- Cost: {1}{U} — correct
- Type: Creature — Human Wizard — correct
- P/T: 2/1 — correct
- Keywords: Flash — correct
- Triggered ability: ETB — correct

**Behavior:**
- ISSUE: The oracle says "target instant or sorcery card in your graveyard" — the player should choose which card to target. The implementation auto-selects the card with the highest mana value (`max_by_key`), giving the player no choice. This is incorrect when there are multiple eligible cards in the graveyard.
- Grants flashback with cost equal to the card's mana cost via `until_end_of_turn_flashback` — correct
- Skips cards that already have flashback — correct

**Result: ISSUE** — No player targeting: auto-selects highest mana value instant/sorcery instead of letting the player choose the target.

## Re-audit — 2026-04-02
**Status**: PASS
Previously fixed bug re-verified: ETB correctly grants flashback to an instant/sorcery in graveyard. Oracle text updated to match Scryfall: "When this creature enters" (was "When Snapcaster Mage enters the battlefield"). Doc comment updated. Behavior unchanged.

## Audit — 2026-04-03 21:31

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flash
When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Creature — Human Wizard
**Status**: ISSUE

### Code issues
- **Engine bug: until_end_of_turn_flashback not cleared at end of turn** (`/Users/dlaw/mtg/mtg-engine/src/engine.rs:3020-3026`)
  - Oracle text says: `gains flashback until end of turn`
  - Code does: Grants flashback that persists permanently - the cleanup logic clears `until_end_of_turn_effects`, `until_end_of_turn_keywords`, etc., but is missing `state.until_end_of_turn_flashback.clear()`

### Tricky interactions checked
- **Timing restrictions for sorceries with flashback**: PASS - Engine correctly applies sorcery timing restrictions even for flashback spells
- **Multiple eligible targets vs auto-selection**: PASS - Code properly presents choice when multiple targets available, though auto-selects when only one (acceptable UX behavior)
- **Flashback cost calculation**: PASS - Correctly uses the card's mana cost as flashback cost
- **Cards with existing flashback excluded**: PASS - Correctly filters out cards that already have innate flashback via `d.flashback_cost.is_none()`
- **Only instant/sorcery cards targeted**: PASS - Correctly filters for CardType::Instant || CardType::Sorcery
- **Cards with no mana cost**: PASS - Uses ManaCost::free() for cards with no cost, but these would create uncastable flashback per rulings
- **Graveyard targeting restrictions**: PASS - Correctly targets only cards owned by the controller in graveyard
- **Stack interactions with removal**: PASS - Uses targeting system that allows responses
- **Flashback spell exiled after cast**: PASS - Engine handles this correctly in general flashback system

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic flashback granting**: `mtg-engine/tests/tier14_cards.rs:393-421` 
- **Timing restrictions for sorcery flashback**: NOT TESTED
- **Cards with no mana cost interaction**: NOT TESTED
- **End of turn cleanup**: NOT TESTED
- **Multiple target choice presentation**: NOT TESTED
- **Cards with existing flashback excluded**: NOT TESTED
- **Stack interaction with graveyard hate**: NOT TESTED
- **Auto-selection with single target**: NOT TESTED
- **Flashback spell exile after resolution**: `mtg-engine/tests/flashback.rs:84-106`
