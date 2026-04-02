# Audit: Forbidden Alchemy

## Reference (Scryfall)
- **Name:** Forbidden Alchemy
- **Cost:** {2}{U}
- **Type:** Instant
- **Oracle:** Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard. Flashback {6}{B}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{U})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Flashback cost: CORRECT ({6}{B})
- Looks at top 4 cards: CORRECT (drains 4 from library_order)
- Player chooses one for hand: CORRECT (ChooseFromRevealed choice)
- Rest go to graveyard: CORRECT
- P/T: CORRECT (N/A)

## Issues
None found.

---

## Audit 2026-04-02

### Oracle Text (Scryfall)
```
Name: Forbidden Alchemy
Mana Cost: {2}{U}
Type Line: Instant
Oracle Text: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
Keywords: Flashback
```

### Card Data
- **Name:** Correct. `"Forbidden Alchemy"`
- **Cost:** Correct. `{2}{U}` implemented as `Generic(2), Colored(Blue)`.
- **Type:** Correct. `Instant`.
- **Oracle text field:** Correct. Matches oracle verbatim.
- **Flashback cost:** Correct. `{6}{B}` implemented as `Generic(6), Colored(Black)`.
- **Keywords vec:** Empty, but Flashback is declared via `flashback_cost: Some(...)`. Consistent with other flashback cards in the codebase. No functional issue.

### Effect: Look at top 4, put 1 in hand, rest in graveyard
- **Look at top 4:** Correct. `player.library_order.drain(..count)` where `count = min(4, library_order.len())`.
- **Fewer than 4 cards ruling:** Handled correctly via the `min(4, len)` logic, matching the 2011-09-22 ruling.
- **0 cards case:** Handled -- does nothing, calls `move_spell_after_resolve`.
- **1 card case:** Handled -- auto-puts the single card into hand without prompting. Correct behavior.
- **2+ cards case:** Presents a `ChooseFromRevealed` choice to the controller. Player picks one card; the engine handler (engine.rs lines 1828-1838) moves the chosen card to Hand and all others to Graveyard. Correct.

### Player Choice Presentation
- The `ChooseFromRevealed` variant correctly provides the list of revealed `ObjectId`s.
- The engine's `legal_actions` generates one `ResolvedChoice::ChosenCard(id)` per revealed card, allowing the player to pick exactly one.
- Description text is clear: `"choose a card to put into your hand (rest go to graveyard)"`.

### move_spell_after_resolve
- Called in the 0-card and 1-card early-return paths.
- Called by the `ChooseFromRevealed` handler in engine.rs for the 2+ card path (line 1838).
- `move_spell_after_resolve` checks `cast_with_flashback`: if true, exiles the spell; otherwise moves to graveyard. Correct per flashback rules.

### Test Coverage
1. `flashback::forbidden_alchemy_draws_and_mills` -- verifies 4 cards revealed from 5-card library, 1 chosen goes to hand, 3 go to graveyard, 1 remains in library. Passes.
2. `card_mechanics::forbidden_alchemy_choice_from_top_4` -- verifies choice presentation with 4 named cards, chosen card goes to hand, other 3 go to graveyard. Passes.

### Verdict
**PASS** -- No issues found. Implementation matches oracle text exactly.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.\nFlashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.
