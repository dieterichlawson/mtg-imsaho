## Audit — 2026-04-01

**Scryfall Oracle text**: {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
**Scryfall type line**: Legendary Artifact
**Status**: ISSUE

- Mana cost {4}: correct
- Card type Artifact, supertype Legendary: correct
- Ability 0 (study counter): correct — costs {1}, requires tap, discards a card, adds study counter
- Ability 1 (reanimate): correctly sacrifices Grimoire, returns all graveyard creatures under controller's control, makes them black Zombies
- ISSUE: Study counters are tracked via card_state with ObjectId encoding rather than using the actual counter system. This is a non-standard approach that works but means counter-interaction cards won't see them.
- ISSUE: The discard auto-picks the first card in hand rather than allowing player choice
- ISSUE: Ability 0 discard is not enforced as a cost — it happens during resolution, so the ability could technically be activated even if the check for cards in hand happens only at ability-listing time
- Black Zombie addition to returned creatures: correctly adds "Zombie" subtype and Black color if not present
- Tests exist in tier15_cards.rs covering study counter accumulation and reanimation
