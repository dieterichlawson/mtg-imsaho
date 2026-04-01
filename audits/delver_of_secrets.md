## Audit — 2026-04-01

**Scryfall Oracle text (front)**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.
**Scryfall Oracle text (back)**: Flying
**Scryfall type line**: Creature — Human Wizard // Creature — Human Insect
**Front P/T**: 1/1
**Back P/T**: 3/2
**Status**: ISSUE

1. **Transform is not optional ("You may reveal")** (`mtg-engine/src/cards/delver_of_secrets.rs`, lines 86-93): Oracle says "You may reveal that card." The code automatically reveals and transforms if an instant/sorcery is on top. While revealing is almost always correct, the player should technically have a choice (e.g., they might not want to reveal information, or in rare cases may not want to transform). This is a minor deviation.
