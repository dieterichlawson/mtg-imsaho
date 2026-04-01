## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. That player mills those cards. Equipped creature gets +1/+0 until end of turn for each card put into that graveyard this way.\nEquip {2}
**Scryfall type line**: Artifact — Equipment
**Status**: ISSUE

- Name: correct ("Trepanation Blade")
- Cost: {3} -- correct
- Type: Artifact -- correct
- Subtypes: Equipment -- correct
- Equip cost: {2} -- correct (sorcery_speed_only: true)
- Attack trigger: mills from defending player's library until a land is found -- correct
- Pump: +1/+0 per card milled until end of turn -- correct

- Land card is included in the mill count (increment happens before the land check break), which matches Oracle text ("each card put into that graveyard this way" includes the land) -- correct
- Tests exist in `tier9_cards.rs`
- No issues found
