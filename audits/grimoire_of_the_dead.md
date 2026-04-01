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

## Audit — 2026-04-01

**Scryfall Oracle text**: {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead. / {T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
**Scryfall type line**: Legendary Artifact
**Status**: ISSUE

1. **Study counters stored in card_state instead of proper counter type**: The code stores study counters via card_state using ObjectId as a stand-in for a count, rather than using a proper CounterType. This is fragile and non-standard compared to other counter implementations. (Lines 51-53, 116)
2. **Discard auto-picks first card**: The discard as additional cost auto-picks the first card in hand rather than presenting a choice to the player. (Line 100)
3. **Ability 1 available even when tapped**: The discard ability requires tap but the activated_abilities method doesn't check if the artifact is already tapped before offering ability 0. (Actually, checking line 47: it checks zone == Battlefield but not tapped status for ability availability. The requires_tap field should handle this at the engine level.)
