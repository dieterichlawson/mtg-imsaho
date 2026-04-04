## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
**Type line**: Legendary Artifact
**Status**: ISSUE

### Code issues
- Counter removal not implemented (mtg-engine/src/cards/isd/grimoire_of_the_dead.rs:131-136)
  - Oracle text says: `{T}, Remove three study counters from Grimoire of the Dead and sacrifice it:`
  - Code does: Checks for 3+ counters in `activated_abilities()` but never removes them in `on_activate_ability()`. Comment on line 135 incorrectly states "counter removal is moot" but oracle text explicitly requires removing counters as part of the cost.

### Tricky interactions checked
- Counter removal as cost: FAIL - Oracle requires removing 3 study counters as part of cost, but code only checks availability and never removes them
- Creature detection from all graveyards: PASS - Uses `o.power.is_some() || o.card_types.contains(&CardType::Creature)` which correctly handles both regular creatures and artifact creatures per ruling
- Color/type addition "in addition to": PASS - Code adds Black color and Zombie subtype without removing existing colors/types
- Controller change for reanimated creatures: PASS - Sets `obj.controller = controller` correctly
- First ability discard choice: PASS - Presents choice when multiple cards, auto-discards when one card
- Exclude Grimoire from reanimation targets: PASS - Filters `o.id != object_id` to prevent self-targeting

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- First ability discard and counter addition: `mtg-engine/tests/tier15_cards.rs:2585-2653`
- Accumulating 3 study counters: `mtg-engine/tests/tier15_cards.rs:2656-2696`  
- Mass reanimation from all graveyards: `mtg-engine/tests/tier15_cards.rs:2699-2737`
- Black Zombie type addition: `mtg-engine/tests/tier15_cards.rs:2731-2734`
- Controller change to caster: `mtg-engine/tests/tier15_cards.rs:2727-2729`
- Ability 1 unavailable without 3 counters: `mtg-engine/tests/tier15_cards.rs:2740-2755`
- Counter removal when ability 1 activates: NOT TESTED