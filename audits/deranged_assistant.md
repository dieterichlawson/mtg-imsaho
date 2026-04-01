# Audit: Deranged Assistant

## Scryfall Reference
- **Name:** Deranged Assistant
- **Cost:** {1}{U}
- **Type:** Creature -- Human Wizard
- **Oracle:** {T}, Mill a card: Add {C}.
- **P/T:** 1/1
- **Keywords:** Mill

## Implementation: `deranged_assistant.rs`
- **Name:** Deranged Assistant -- CORRECT
- **Cost:** {1}{U} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Human", "Wizard"] -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Keywords:** none -- ACCEPTABLE (Mill is a keyword action, not a keyword ability)
- **Mana ability:** {T}, add {C} -- CORRECT
- **Mill as cost:** Checks library not empty -- CORRECT
- **Produces:** Colorless 1 -- CORRECT
- **Summoning sickness check:** Yes -- CORRECT

## Issues
1. **ISSUE: Mill cost is not actually implemented.** The mana_abilities method checks that the library is not empty (which implies the mill cost exists), but the actual milling of the top card is not performed when the ability activates. There is no on_activate_mana_ability or similar hook that puts the top card into the graveyard. The description says "Mill a card, add {C}" but the code only checks the precondition, it doesn't move the card.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: {T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)
**Scryfall type line**: Creature -- Human Wizard
**Status**: PASS

Previous mill cost issue has been fixed. The implementation now has an `on_activate_mana_ability` hook that calls `crate::engine::mill_cards(state, controller, 1)` to mill a card as part of the mana ability activation.

Verified correct:
- Mana cost: {1}{U} -- matches
- Types: Creature -- matches
- Subtypes: Human, Wizard -- matches
- P/T: 1/1 -- matches
- Mana ability: requires tap, library not empty, produces 1 colorless -- correct
- Mill cost: performed during `on_activate_mana_ability` -- correct
- Summoning sickness check: yes (`!obj.summoning_sick`) -- correct
- No anti-patterns detected
- Tests found in `mtg-engine/tests/innistrad_simple_cards.rs`
