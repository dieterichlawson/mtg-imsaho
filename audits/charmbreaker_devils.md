## Audit — 2026-04-01

**Scryfall Oracle text**: At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
Whenever you cast an instant or sorcery spell, Charmbreaker Devils gets +4/+0 until end of turn.
**Scryfall type line**: Creature — Devil
**Status**: ISSUE

### Findings

1. **on_spell_cast does not check spell type (ISSUE)**: The Oracle text says "Whenever you cast an instant or sorcery spell" but `on_spell_cast` (line 75) does not verify that the cast spell is an instant or sorcery. It triggers on ANY spell cast by the controller, including creatures, artifacts, enchantments, etc. The `_spell_id` parameter is ignored; it should be used to look up the spell's card types and filter to instant/sorcery only.

2. **Card data correct**: Name, cost ({5}{R}), type (Creature), subtype (Devil), P/T (4/4) all match.

3. **Upkeep ability correct**: Correctly finds instant/sorcery cards in graveyard, picks one at random, and returns to hand.

4. **Until-end-of-turn effect correct**: Uses `UntilEndOfTurnEffect` with +4/+0.

5. **Tests**: No dedicated tests found.
