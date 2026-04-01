## Audit — 2026-04-01

**Scryfall Oracle text**: (Front — Delver of Secrets) At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.
(Back — Insectile Aberration) Flying
**Scryfall type line**: (Front) Creature — Human Wizard // (Back) Creature — Human Insect
**Status**: ISSUE

### Findings

1. **Reveal is not optional (ISSUE)**: Oracle says "You **may** reveal that card. If an instant or sorcery card is **revealed** this way, transform..." The implementation auto-reveals and auto-transforms if the top card is instant/sorcery (lines 86-93). The player should have the option to not reveal (e.g., to avoid transforming if they want to keep Delver as a Human for Champion of the Parish triggers, or to hide information).

2. **Card data correct**: Name, cost ({U}), type (Creature), subtypes (Human, Wizard front / Human, Insect back), P/T (1/1 front / 3/2 back) all match.

3. **Back face keywords correct**: Flying on Insectile Aberration.

4. **Upkeep trigger correct**: Only triggers on front face during controller's upkeep.

5. **Tests**: No dedicated tests found.
