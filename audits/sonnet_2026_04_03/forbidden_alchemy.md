## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Oracle text field incomplete (`mtg-engine/src/cards/isd/forbidden_alchemy.rs:25`)
  - Oracle text says: `Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.\nFlashback {6}{B}`
  - Code does: `oracle_text: "Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.".into(),` (missing flashback text)

### Tricky interactions checked

- Flashback exile mechanism: PASS - `move_spell_after_resolve` correctly checks `cast_with_flashback` and moves to exile
- Empty library handling: PASS - Code handles `revealed.is_empty()` case 
- Single card library handling: PASS - Code auto-selects when `revealed.len() == 1`
- Player choice presentation: PASS - `ResolutionChoiceKind::ChooseFromRevealed` presents all revealed cards as options
- Flashback cost mechanism: PASS - `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(6), ManaSymbol::Colored(Color::Black)]))` matches oracle {6}{B}
- Card selection and graveyard placement: PASS - Engine correctly moves chosen card to hand and rest to graveyard in choice handler

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Basic Forbidden Alchemy functionality (choose 1 from 4, rest to graveyard): `card_mechanics.rs:724`
- Card selection and library manipulation: `flashback.rs:380`
- Flashback exile mechanism: `flashback.rs:86` (general flashback test, not Forbidden Alchemy specific)
- Forbidden Alchemy with flashback: NOT TESTED
- Empty library edge case: NOT TESTED  
- Single card library edge case: NOT TESTED
- Flashback cost validation: NOT TESTED