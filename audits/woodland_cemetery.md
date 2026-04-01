## Audit — 2026-04-01

**Scryfall Oracle text**: Woodland Cemetery enters the battlefield tapped unless you control a Swamp or a Forest.\n{T}: Add {B} or {G}.
**Scryfall type line**: Land
**Scryfall mana cost**: (none)
**Status**: PASS

Findings:
- Name: Correct.
- Type: Land — correct. No cost — correct.
- Enters tapped condition: Checks for Swamp or Forest subtypes on other permanents controlled by the same player. Correctly excludes itself (`o.id != object_id`). Correct.
- Mana abilities: Two abilities producing {B} and {G} respectively, both requiring tap. Correct.
- Only shows mana abilities when untapped and on battlefield. Correct.
- Tests: `woodland_cemetery_card_data` and `woodland_cemetery_enters_untapped_with_swamp` in innistrad_simple_cards.rs.

No issues found.
