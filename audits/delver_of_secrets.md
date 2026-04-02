# Audit: Delver of Secrets // Insectile Aberration

## Scryfall Reference
- **Front Face: Delver of Secrets**
  - **Cost:** {U}
  - **Type:** Creature -- Human Wizard
  - **Oracle:** At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
  - **P/T:** 1/1

- **Back Face: Insectile Aberration**
  - **Cost:** (none)
  - **Type:** Creature -- Human Insect
  - **Oracle:** Flying
  - **P/T:** 3/2

## Implementation: `delver_of_secrets.rs`
- **Front face name:** Delver of Secrets -- CORRECT
- **Cost:** {U} -- CORRECT
- **Front subtypes:** ["Human", "Wizard"] -- CORRECT
- **Front P/T:** 1/1 -- CORRECT
- **Back face name:** Insectile Aberration -- CORRECT
- **Back subtypes:** ["Human", "Insect"] -- CORRECT
- **Back P/T:** 3/2 -- CORRECT
- **Back keywords:** [Flying] -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Checks top card of library for instant/sorcery, transforms if found -- CORRECT

## Issues
None

## Audit — 2026-04-01 15:12

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.
**Oracle text (back)**: Flying
**Type line (front)**: Creature — Human Wizard
**Type line (back)**: Creature — Human Insect
**Ruling**: [2011-09-22] You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library.
**Status**: ISSUE

### Code issues

1. **"You may" choice is not presented to the player** (`mtg-engine/src/cards/isd/delver_of_secrets.rs:86`)
   - Oracle text says: `You may reveal that card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.`
   - Code does: `if is_instant_or_sorcery { ... obj.is_transformed = true; }` — the code automatically transforms Delver whenever the top card is an instant or sorcery, without giving the player the choice to decline the reveal.
   - The "You may" is strategically relevant: a player might want to avoid revealing information to their opponent, or might not want Delver to transform in certain situations (e.g., if they have equipment or auras that benefit Human creatures specifically).
   - The ruling explicitly confirms: "You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library."

### Tricky interactions checked
- Only triggers on controller's upkeep (not each upkeep): PASS — code checks `state.active_player != controller`
- Only triggers on front face: PASS — code checks `is_transformed` and returns if true
- Empty library: PASS — `top_card_id` would be `None`, gracefully handled
- Card stays on top of library after checking: PASS — code only reads the top card, never moves it
- Back face has Flying keyword: PASS
- Dynamic P/T for back face (3/2): PASS

### Test coverage
- Transform when top card is instant: `tier15_cards.rs:delver_transforms_when_top_card_is_instant` — TESTED
- Does not transform when top card is creature: `tier15_cards.rs:delver_does_not_transform_when_top_card_is_creature` — TESTED
- Player choosing NOT to reveal (you may decline): NOT TESTED (bug — choice not implemented)
- Transform when top card is sorcery: NOT TESTED
- Empty library (no crash): NOT TESTED
- Multiple Delvers checking same top card: NOT TESTED
- Back face does not trigger on upkeep: NOT TESTED (implicit from code structure)
