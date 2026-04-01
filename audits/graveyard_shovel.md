## Audit — 2026-04-01

**Scryfall Oracle text**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Scryfall type line**: Artifact
**Status**: ISSUE

- Mana cost {2}: correct
- Card type Artifact: correct
- Activated ability {2}, {T}: correct cost and tap requirement
- ISSUE: Oracle says "Target player exiles a card from their graveyard" — the target is a player who then chooses a card to exile. The implementation instead targets a graveyard card directly (TargetRequirement::GraveyardCard) and can target any player's graveyard card. This changes the targeting semantics: Oracle targets a player, then that player exiles a card of their choice; the implementation targets a specific card.
- Creature check and 2 life gain: correctly checks if exiled card was a creature and gains 2 life
- Life gain emits LifeChanged event: correct
- Tests exist in innistrad_simple_cards.rs covering exile and life gain
