## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- Auto-selects first basic land in library order instead of presenting a player search choice (`mtg-engine/src/cards/isd/caravan_vigil.rs` lines 39–50)
  - Oracle text says: `"Search your library for a basic land card, reveal it, put it into your hand, then shuffle."`
  - Code does: `let basic_land = player.library_order.iter().find(|&&obj_id| { ... }).copied();` — this blindly picks the first matching basic land in library order, never presenting the player with a choice. When a player has multiple different basic land types in their library (e.g., Forest, Mountain, Island), they cannot choose which one to fetch. The engine already has `ResolutionChoiceKind::ChooseFromLibrary` for exactly this purpose (used correctly in `garruk_relentless.rs`).

### Tricky interactions checked

- "You may" morbid choice is optional: PASS — code presents a `ResolutionChoiceKind::YesNo` choice; answering "No" puts the card in hand.
- Ruling — player can choose hand even if morbid active: PASS — YesNo default of "No" puts the land in hand.
- `creature_died_this_turn` checked at resolution time (not at cast time): PASS — condition is read inside `on_resolve` when the spell actually resolves.
- `creature_died_this_turn` resets at turn boundary: PASS — `engine.rs:2888` clears it at the start of the next player's turn.
- Library shuffled when no basic land found: PASS — `else` branch at lines 87–93 shuffles regardless.
- Library shuffled after player makes morbid choice: PASS — `on_yes_no_choice` shuffles at lines 120–123 before calling `move_spell_after_resolve`.
- `on_yes_no_choice` dispatch in engine: PASS — `engine.rs:1995–2001` correctly looks up the `source_card` object's `card_id` and calls `on_yes_no_choice`.
- `move_spell_after_resolve` called on all exit paths: PASS — called at line 96 for non-morbid path; called in `on_yes_no_choice` line 125 for morbid path; morbid early-return does not call it (correctly deferred).
- Flashback exile vs graveyard: PASS — `move_spell_after_resolve` checks `cast_with_flashback` and exiles accordingly.
- Player choice of "search and fail to find" when no basic land exists: PASS — code handles empty result naturally (nothing happens, library still shuffled).
- Multiple basic lands in library — player cannot choose: FAIL — see Code Issues above.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Non-morbid path (land goes to hand): `mtg-engine/tests/tier11_cards.rs:170` (`caravan_vigil_finds_basic_land`)
- Morbid path with player accepting battlefield: `mtg-engine/tests/tier11_cards.rs:188` (`caravan_vigil_morbid_choose_battlefield`)
- Morbid path with player declining (land goes to hand): NOT TESTED
- Multiple basic land types in library — player can choose which to fetch: NOT TESTED
- `creature_died_this_turn` correctly false when no creature died: NOT TESTED (covered indirectly by non-morbid test)
- Library shuffled when no basic land in library: NOT TESTED
- Flashback cast exiles the spell: NOT TESTED
