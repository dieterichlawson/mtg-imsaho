## Audit — 2026-04-01

**Scryfall Oracle text**: {3}{R}: Choose target creature or player. Reveal the top three cards of your library. Heretic's Punishment deals damage to that creature or player equal to the highest mana value among the revealed cards. Put the revealed cards on the bottom of your library in any order.
**Scryfall type line**: Enchantment
**Status**: ISSUE

- Mana cost {4}{R}: correct
- Card type Enchantment: correct
- Activated ability {3}{R}: correct cost, no tap required: correct
- Target AnyTarget (creature or player): correct
- Reveals top 3 cards and finds greatest mana value: correct
- Damage dealing to creature or player: correct
- ISSUE: Oracle says "Put the revealed cards on the bottom of your library in any order." The implementation moves them to the graveyard instead (line 116: `state.move_object(card_id, Zone::Graveyard)`). The comment says "per current Oracle errata" but this is incorrect — the current Oracle text still says bottom of library.
- ISSUE: When max_mv is 0 (all revealed cards are lands), no damage is dealt. This is technically correct (0 damage), but the revealed cards should still be put on the bottom of the library regardless.
- Tests exist in tier15_cards.rs
