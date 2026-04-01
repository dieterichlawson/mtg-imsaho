## Audit — 2026-04-01

**Scryfall Oracle text**: At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
**Scryfall type line**: Creature — Devil
**Status**: ISSUE

1. **on_spell_cast does not filter for instant/sorcery** (`mtg-engine/src/cards/charmbreaker_devils.rs`, line 75-92): The `on_spell_cast` handler triggers on ANY spell cast by the controller, but Oracle text says "Whenever you cast an instant or sorcery spell." The code should check if `_spell_id` refers to an instant or sorcery before applying the +4/+0 buff. Currently, casting a creature or enchantment spell would also trigger the buff, which is incorrect.
