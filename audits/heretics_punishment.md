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

## Audit — 2026-04-01

**Scryfall Oracle text**: {3}{R}: Choose any target, then mill three cards. Heretic's Punishment deals damage to that permanent or player equal to the greatest mana value among the milled cards.
**Scryfall type line**: Enchantment
**Status**: ISSUE

1. **Oracle text in code uses old wording**: The code's oracle_text says "Reveal the top 3 cards of your library... Put the revealed cards on the bottom of your library" but the current Oracle says "mill three cards" (which puts them into graveyard, not bottom of library). The code correctly moves cards to graveyard (line 116: move_object to Zone::Graveyard), so the behavior is correct but the oracle_text string is outdated. (Line 26)
2. **Missing damaged_by tracking**: When dealing damage to a creature target, the code does not push to obj.damaged_by (line 84: only sets damage_marked). This means damage tracking for effects like "whenever a creature is dealt damage" won't work. (Line 82-85)
3. **Damage dealt even when max_mv is 0**: If all milled cards have mana value 0 (lands), the code skips damage (line 78: `if max_mv > 0`). But the Oracle says the enchantment deals damage equal to the greatest mana value — dealing 0 damage is still dealing damage (relevant for triggers). This is a minor edge case.
