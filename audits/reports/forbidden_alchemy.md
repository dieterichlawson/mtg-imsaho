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

## Audit — 2026-04-02 20:58

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/55/forbidden-alchemy)
**Oracle text**: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

1. **UI: Revealed cards invisible in views (CLI + LLM player)** — When `on_resolve` runs, `library_order.drain(..count)` removes the top 4 cards from the library order, but their `.zone` field stays as `Zone::Library`. The `GameView` builds `your_library_cards` from `library_order`, so these cards disappear from all views. Both `cli.rs::perm_name` and `llm.rs::obj_name` search battlefield, hand, stack, and graveyards but not a "revealed" zone, so they fall through to displaying raw `ObjectId(N)` strings. The `library_search_ui` (triggered because there are >3 ChosenCard actions) likewise shows empty card info (no name, type, cost, oracle text). **Impact**: Players (human and AI) see meaningless IDs instead of card names when making the Forbidden Alchemy choice.

2. **Misleading LLM prompt description** — `mtg-player/src/llm.rs` line 115 describes Forbidden Alchemy as `"Draw 1 card, mill 3"`. The actual behavior is look-at-4-choose-1, which is strictly better than draw+mill because the player gets to select the best card. This misdescription may cause the AI to undervalue the card's selection power.

3. **Stale code comment** — `forbidden_alchemy.rs` line 9 says `"Simplified: draw 1 card, mill 3"` but the implementation correctly presents a full ChooseFromRevealed choice. The comment contradicts the actual behavior and should be removed.

### Tricky interactions checked (min 3)

1. **Fewer than 4 cards in library**: Handled correctly via `min(4, library_order.len())`. If 0 cards, does nothing. If 1 card, auto-puts it in hand. Matches the 2011-09-22 ruling.
2. **Flashback + exile**: `move_spell_after_resolve` checks `cast_with_flashback` flag. When cast via flashback, the spell is exiled instead of going to graveyard. Confirmed correct in engine.rs handler.
3. **Choice resolution with ChooseFromRevealed**: Engine handler (engine.rs ~line 2025) moves chosen card to Hand and all other revealed cards to Graveyard, then calls `move_spell_after_resolve`. The spell itself is cleaned up only after the choice, not before. Correct sequencing.
4. **Cards removed from library order during reveal**: The drain ensures drawn/revealed cards cannot be drawn again by other effects while the choice is pending. However, this creates the UI visibility bug noted above.

### Test coverage

1. `flashback::forbidden_alchemy_draws_and_mills` — 5-card library, reveals 4, player chooses 1 for hand, 3 go to graveyard, 1 remains in library. PASSES.
2. `card_mechanics::forbidden_alchemy_choice_from_top_4` — 4 named cards (Lightning Bolt, Grizzly Bears, Forest, Giant Growth), choice presented, Lightning Bolt chosen goes to hand, other 3 go to graveyard. PASSES.
- **Missing**: No test for the fewer-than-4-cards edge case (e.g., 2 cards in library).
- **Missing**: No test for Forbidden Alchemy cast via flashback (flashback is tested generically, but not specifically for this card's reveal+choose flow when cast from graveyard).

## Audit — 2026-04-03 22:21

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

1. **Misleading LLM description** — `mtg-player/src/llm.rs` line 115 describes Forbidden Alchemy as `"Draw 1 card, mill 3"`. 
   - Oracle text says: `Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.`
   - Code does: Presents a choice of 4 revealed cards, but LLM description says it just draws the top card, causing AI to undervalue the card's selection power.

2. **UI: Revealed cards show as ObjectId instead of names** — When choice is presented, `cli.rs::perm_name` function only searches battlefield, hand, and stack zones.
   - Oracle text says: `Look at the top four cards of your library. Put one of them into your hand`
   - Code does: Revealed cards are drained from `library_order` but stay in `Zone::Library`, so `perm_name` falls back to displaying raw `ObjectId(N)` strings instead of card names during choice presentation.

3. **Stale code comment** — `forbidden_alchemy.rs` line 9 comment says `"Simplified: draw 1 card, mill 3"`.
   - Oracle text says: `Look at the top four cards of your library. Put one of them into your hand`  
   - Code does: Correctly implements full choice mechanism, but comment contradicts the actual behavior.

### Tricky interactions checked
- Fewer than 4 cards in library: PASS - Uses `std::cmp::min(4, library_order.len())` to handle correctly per 2011-09-22 ruling
- Empty library (0 cards): PASS - Handled with `revealed.is_empty()` branch, calls `move_spell_after_resolve`
- Single card library: PASS - Auto-selects the only card without choice UI
- Flashback mechanics: PASS - Engine correctly identifies flashback and uses `flashback_cost` field
- Choice resolution: PASS - Engine moves chosen card to hand, rest to graveyard via `ChooseFromRevealed` handler
- Spell cleanup: PASS - Uses `move_spell_after_resolve` correctly in all code paths

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Top 4 cards choice mechanism: `card_mechanics.rs:724` (forbidden_alchemy_choice_from_top_4) 
- Library manipulation with multiple cards: `flashback.rs:380` (forbidden_alchemy_draws_and_mills)
- Fewer than 4 cards ruling: NOT TESTED
- Empty library case: NOT TESTED  
- Single card library case: NOT TESTED
- Flashback casting from graveyard: NOT TESTED

## Audit — 2026-04-03 22:21 (independent re-audit)

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

1. **Revealed cards invisible in UI views** — `forbidden_alchemy.rs` line 37: `player.library_order.drain(..count)` removes the top 4 cards from `library_order` but their `.zone` remains `Zone::Library`. The `GameView` builds `your_library_cards` only from `library_order` (`view.rs` line 97), so these cards vanish from all views. When the choice is presented, both `cli.rs::perm_name` (line 931) and `llm.rs::obj_name` (line 599) search only battlefield, hand, stack, and graveyards — not library. They fall through to the fallback `format!("{}", id)` which shows `obj#N` instead of card names. The oracle text says "Look at the top four cards" — if the player cannot see the card names during the choice, they cannot meaningfully look at them.
   - Oracle text says: `Look at the top four cards of your library. Put one of them into your hand`
   - Code does: Cards are drained from `library_order` but remain in `Zone::Library`; `perm_name`/`obj_name` cannot resolve their names, displaying `obj#N` instead

2. **Misleading LLM card knowledge** — `mtg-player/src/llm.rs` line 115 describes Forbidden Alchemy as `"Draw 1 card, mill 3"`. The oracle text says "Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard." This is wrong in two ways: (a) "Draw" is mechanically different from "put into hand" (draw triggers draw-related abilities; put into hand does not), and (b) it omits the player's choice of which card to keep, which is the card's primary strategic value.
   - Oracle text says: `Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.`
   - Code does: `"Draw 1 card, mill 3"` at `llm.rs:115`

### Tricky interactions checked
- Fewer than 4 cards in library: PASS — `min(4, library_order.len())` at line 36 handles this correctly per the 2011-09-22 ruling
- 0 cards in library: PASS — empty revealed vec triggers early return with `move_spell_after_resolve`
- 1 card in library: PASS — auto-puts single card in hand, no choice needed, correct behavior
- Flashback exile: PASS — `move_spell_after_resolve` checks `cast_with_flashback` flag; exiles if true, graveyard if false
- Choice resolution sequencing: PASS — spell cleanup via `move_spell_after_resolve` happens only after the choice is resolved (engine.rs line 2035), not before
- Put into hand vs draw: PASS — implementation uses `state.move_object(card_id, Zone::Hand)` (line 45), not `draw_cards()`, correctly matching "put... into your hand" semantics
- Mana cost {2}{U}: PASS — `Generic(2), Colored(Blue)` matches oracle
- Flashback cost {6}{B}: PASS — `Generic(6), Colored(Black)` matches oracle
- Card type Instant: PASS — `CardType::Instant` matches oracle
- Keywords vec empty: PASS — no `Keyword::Flashback` variant exists in engine; flashback is modeled via `flashback_cost` field, consistent with all other flashback cards

### Test coverage
- Basic reveal-4-choose-1 flow: `card_mechanics.rs:724` (`forbidden_alchemy_choice_from_top_4`)
- Reveal from 5-card library, 1 kept, 3 graveyard, 1 remains: `flashback.rs:380` (`forbidden_alchemy_draws_and_mills`)
- Fewer than 4 cards edge case (e.g., 2 cards in library): NOT TESTED
- 0 cards in library edge case: NOT TESTED
- 1 card in library edge case: NOT TESTED
- Flashback-specific flow (reveal+choose when cast from graveyard): NOT TESTED
- UI visibility of revealed cards during choice: NOT TESTED
