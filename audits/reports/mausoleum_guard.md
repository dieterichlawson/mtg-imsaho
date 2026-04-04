# Audit: Mausoleum Guard

## Reference (Scryfall/API)
- **Name:** Mausoleum Guard
- **Mana Cost:** {3}{W}
- **Type:** Creature — Human Scout
- **Oracle:** When this creature dies, create two 1/1 white Spirit creature tokens with flying.
- **P/T:** 2/2

## Implementation: `mausoleum_guard.rs`
- **Name:** Mausoleum Guard -- CORRECT
- **Mana Cost:** {3}{W} -- CORRECT
- **Type:** Creature — Human Scout -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Keywords:** None -- CORRECT
- **Triggered ability:** SelfDies, creates two 1/1 white Spirit tokens with Flying -- CORRECT
- **Token details:** name "Spirit", power 1, toughness 1, color White, type Creature, keyword Flying, subtype "Spirit" -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: When this creature dies, create two 1/1 white Spirit creature tokens with flying.
**Type line**: Creature — Human Scout
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Mausoleum Guard", cost {3}{W}, 2/2, type Creature with subtypes Human/Scout, triggered ability on SelfDies creates two 1/1 white Spirit creature tokens with flying. Behavior is correct.

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/20/mausoleum-guard)
**Oracle text**: When this creature dies, create two 1/1 white Spirit creature tokens with flying.
**Type line**: Creature — Human Scout
**Status**: PASS
### Code issues
None. All card data is correct:
- Name: "Mausoleum Guard" matches
- Mana cost: Generic(3) + Colored(White) = {3}{W} matches
- Type: Creature with subtypes ["Human", "Scout"] matches "Creature — Human Scout"
- P/T: 2/2 matches
- Triggered ability: SelfDies trigger calls `on_dies` which creates two 1/1 white Spirit creature tokens with flying via `create_token_with_subtypes`
- Controller is correctly used (not owner) for token creation, so stolen creatures produce tokens for the correct player
- Minor cosmetic note: oracle_text field says "When Mausoleum Guard dies" while Scryfall now uses "When this creature dies" (modern templating). No gameplay impact.
### Tricky interactions checked (min 3)
1. **Parallel Lives interaction**: `create_token_with_subtypes` respects Parallel Lives doubling. With one Parallel Lives, each of the two token-creation calls would produce 2 tokens instead of 1, for 4 total. Correct.
2. **Stolen creature / controller vs owner**: `on_dies` uses `state.get_object(object_id).map(|o| o.controller)` which correctly reads the controller (preserved through zone change), so if Control Magic is on Mausoleum Guard, the opponent gets the Spirit tokens. Correct.
3. **SelfDies trigger resolution**: The trigger system captures the controller from the `CreatureDied` event (recorded before zone change) and passes `dead_id` to `on_dies`. The card then reads controller from the graveyard object (controller field is not cleared by `move_object`). This works correctly.
4. **Multiple deaths in same SBA check**: If Mausoleum Guard dies alongside other creatures, all `CreatureDied` events are collected, and triggers are queued in APNAP order. Mausoleum Guard's trigger correctly fires independently.
### Test coverage
- `tier3_cards::mausoleum_guard_creates_two_spirits_on_death`: Kills Mausoleum Guard with lethal damage (2 damage marked), verifies it moves to graveyard, processes triggers, and asserts exactly two 1/1 Spirit tokens with flying are created. PASSES.
