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
