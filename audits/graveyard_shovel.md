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

## Audit — 2026-04-01

**Scryfall Oracle text**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Scryfall type line**: Artifact
**Status**: ISSUE

1. **Targeting is wrong**: Oracle says "Target player exiles a card from their graveyard" — the ability targets a PLAYER, and that player chooses which card to exile on resolution. The code instead targets a specific card in any graveyard (TargetRequirement::GraveyardCard), which is a different mechanic. The targeted player should choose which card to exile, not the controller of Graveyard Shovel. (Lines 48, 57 in graveyard_shovel.rs)
2. **Oracle text in code is wrong**: The code's oracle_text says "Exile target card from a graveyard" which doesn't match the current Oracle text "Target player exiles a card from their graveyard." (Line 23)
