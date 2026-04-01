## Audit — 2026-04-01

**Scryfall Oracle text**: Activated abilities of artifacts can't be activated.
**Scryfall type line**: Enchantment
**Status**: ISSUE

- Name: correct ("Stony Silence")
- Cost: {1}{W} -- correct
- Type: Enchantment -- correct
- Oracle text: matches

**Issue: Static ability is not implemented.** The code comment explicitly acknowledges this: "the engine doesn't have an ability restriction system." The card is registered for deck building purposes only. Its static ability (preventing activated abilities of artifacts) is never enforced. This means Sol Ring, equipment equip abilities, Traveler's Amulet, etc. can all still be activated while Stony Silence is on the battlefield.

- Tests exist in `innistrad_simple_cards.rs` (card_data test only, no functional test)

## Audit — 2026-04-01

**Scryfall Oracle text**: Activated abilities of artifacts can't be activated.
**Scryfall type line**: Enchantment
**Status**: ISSUE

1. **Static ability not enforced** (stony_silence.rs:7-11): The code comment acknowledges this is a known limitation. The card is registered but its ability does nothing. Artifact activated abilities can still be activated while Stony Silence is on the battlefield.
