## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Upkeep trigger timing**: PASS - correctly checks `state.active_player != cursed_player` to only trigger on enchanted player's upkeep
- **Target choice controller**: PASS - curse controller chooses target (not enchanted player), which is correct per MTG rules
- **Planeswalker targeting scope**: PASS - correctly limits to planeswalkers controlled by the enchanted player on battlefield  
- **Auto-targeting when no choice**: PASS - when no planeswalkers present, automatically deals damage to player (only legal target)
- **Choice optionality**: PASS - correctly sets `optional: false` since oracle has no "may" clause
- **Damage type**: PASS - uses `NonCombatDamageDealt` correctly
- **Source leaving battlefield**: PASS - trigger generates immediate damage (no planeswalkers) or uses PendingEffect with stored source info (with planeswalkers)
- **Trigger system integration**: PASS - verified trigger dispatch in `triggers.rs` correctly processes upkeep triggers for battlefield permanents

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic upkeep damage**: `mtg-engine/tests/tier7_cards.rs:176` (curse_of_pierced_heart_deals_damage_on_upkeep)  
- **Planeswalker targeting choice**: NOT TESTED
- **Auto-targeting when no planeswalkers**: NOT TESTED  
- **Curse controller vs enchanted player upkeep**: NOT TESTED
- **Multiple planeswalkers present**: NOT TESTED
- **Enchantment targeting (Bitterheart Witch interaction)**: `mtg-engine/tests/tier15_cards.rs:183,224,259` (indirect testing via witch tutoring)

Sources:
- [Curse of the Pierced Heart · Innistrad (ISD) #138](https://scryfall.com/card/isd/138/curse-of-the-pierced-heart)
- [Curse of the Pierced Heart MTG - Innistrad #138 (English) | Magic: The Gathering](https://gatherer.wizards.com/ISD/en-us/138/curse-of-the-pierced-heart)
- [Curse of the Pierced Heart • Enchantment — Aura Curse (Innistrad) - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Curse-of-the-Pierced-Heart/)
- [Cuses on Planeswalkers? - Magic Rulings Archives - Magic Rulings - Magic Fundamentals - MTG Salvation Forums - MTG Salvation](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/magic-rulings-archives/296647-cuses-on-planeswalkers)
- [Curse of the Pierced Heart and Planeswalkers — MTG Q&A](https://tappedout.net/mtg-questions/curse-of-the-pierced-heart-and-planeswalkers/)
- [Curse of the Pierced Heart rulings - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Curse-of-the-Pierced-Heart/rulings?price_type=mtgo)
- [Curse of the Pierced Heart - Enchantment - Cards - MTG Salvation](https://www.mtgsalvation.com/cards/innistrad/19036-curse-of-the-pierced-heart)
- [Curse of the Pierced Heart | Innistrad | Modern | Card Kingdom](https://www.cardkingdom.com/mtg/innistrad/curse-of-the-pierced-heart)
- [Curse of the Pierced Heart • Innistrad (isd) • Cards • MTG Arena Zone](https://mtgazone.com/cards/curse-of-the-pierced-heart/)
- [Curse of the Pierced Heart | Innistrad (ISD) #138 | Kairosmithy | MTG Card Discovery](https://kairosmithy.com/cards/lxMPaJiUIc2E-curse-of-the-pierced-heart)