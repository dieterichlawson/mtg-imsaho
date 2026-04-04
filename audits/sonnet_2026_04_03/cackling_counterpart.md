## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues
- Color copying bug in `/Users/dlaw/mtg/mtg-engine/src/state.rs` lines 424-431
  - Oracle text says: `The token copies exactly what was printed on the original creature and nothing else`
  - Code does: `Vec::new(), // colors TODO` - hardcoded empty vector instead of copying colors from source creature's mana cost
- Token copying completely broken in `/Users/dlaw/mtg/mtg-engine/src/state.rs` lines 424-431
  - Oracle text says: `If the copied creature is a token, the token that's created copies the original characteristics of that token as stated by the effect that created the token`
  - Code does: When copying tokens, `registry.card_data(CardId(0))` returns None, so `.unwrap_or_default()` gives empty vectors for keywords, card_types, and subtypes, losing all token characteristics

### Tricky interactions checked
- Colors matter for game rules: FAIL - tokens created without colors when source has colored mana cost
- Token-to-token copying: FAIL - all characteristics lost when copying tokens due to CardId(0) lookup failure
- Flashback exile functionality: PASS - `move_spell_after_resolve` correctly checks `cast_with_flashback` flag and exiles spell
- Base vs modified P/T copying: PASS - correctly copies base power/toughness from object (tokens store base values, continuous effects applied separately)
- X in mana cost handling: PASS - registry data handles X=0 correctly
- ETB triggers on token copy: PASS - token gets same card_id so triggers should fire correctly

### Test coverage
- Basic token copy creation: `tier12_cards.rs:487` - tests that token is created with correct P/T and is marked as token
- Flashback cost existence: `tier12_cards.rs:510` - tests that flashback_cost field exists and has correct mana value
- Color copying: NOT TESTED - no tests verify token has correct colors when copying colored creatures
- Token characteristics copying (keywords/types/subtypes): NOT TESTED - no tests verify keywords like Flying are copied to tokens
- Token-to-token copying: NOT TESTED - no tests copy tokens created by other spells
- X in mana cost copying: NOT TESTED - no tests copy creatures with X in cost
- Complex copy layering (copying creatures that are copying other things): NOT TESTED - likely not implemented in engine yet