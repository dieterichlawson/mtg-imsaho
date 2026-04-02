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

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: {T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)
**Type line**: Creature — Human Wizard
**Status**: PASS

Card data correct: name, mana cost ({1}{U}), type (Creature), subtypes (Human, Wizard), P/T (1/1).

mana_abilities: correctly checks battlefield, untapped, not summoning sick, and library not empty. Produces 1 colorless mana with tap required.

on_activate_mana_ability: mills one card via crate::engine::mill_cards as part of the mana ability cost.

Tests in innistrad_simple_cards.rs cover card data and tapping for colorless mana. No anti-patterns found.

## Audit — 2026-04-02

**Oracle text (Scryfall, cached 2026-04-01):**
> {T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)

**Type line:** Creature — Human Wizard | **P/T:** 1/1 | **Cost:** {1}{U} | **Keywords:** Mill

**Status**: PASS (minor items noted)

### Card Data Verification

All fields match oracle:
- Name: "Deranged Assistant" -- matches
- Mana cost: Generic(1) + Colored(Blue) = {1}{U} -- matches
- Card types: Creature -- matches
- Subtypes: Human, Wizard -- matches
- P/T: 1/1 -- matches
- Oracle text field: "{T}, Mill a card: Add {C}." -- matches

Keywords field is `vec![]` (empty) while Scryfall lists Mill. Mill is a keyword action, not a keyword ability, so this is acceptable.

### Mana Ability

- Tap cost enforced: `requires_tap: true` and `!obj.tapped` guard -- correct
- Summoning sickness: `!obj.summoning_sick` -- correct
- Zone: `Zone::Battlefield` -- correct
- Library guard: `!library_order.is_empty()` prevents activation with empty library -- correct (mill is a cost)
- Produced: `vec![(ManaType::Colorless, 1)]` = {C} -- correct

### Mill Execution

`on_activate_mana_ability` calls `crate::engine::mill_cards(state, controller, 1)` which removes the top card from `library_order` and calls `move_object(card_id, Zone::Graveyard)`. Correct.

### Tests

Both in `mtg-engine/tests/innistrad_simple_cards.rs`, both PASS:
- `deranged_assistant_card_data` -- verifies P/T, MV=2, subtypes
- `deranged_assistant_taps_for_colorless` -- verifies mana ability produces 1 colorless

### Minor Items

1. **TEST GAP:** No test asserts the mill actually occurs (library shrinks / graveyard grows after activation). The existing test only checks mana production.
2. **Keywords field empty:** `keywords: vec![]` vs Scryfall `Mill`. Low functional impact.
