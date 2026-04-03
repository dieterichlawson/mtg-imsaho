## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

Card data verified against oracle text:
- Name: "Cackling Counterpart" -- matches
- Mana cost: {1}{U}{U} -- matches
- Type: Instant -- matches
- Oracle text field: matches (reminder text for Flashback omitted, consistent with codebase convention)
- Flashback cost: {5}{U}{U} -- matches
- Target: `CreatureWithFilter(TargetFilter::YouControl)` -- correctly implements "target creature you control"

### Tricky interactions checked
- Target removed before resolution: PASS -- `on_resolve` checks `o.zone == Zone::Battlefield` before creating the copy (line 45); if the target left, no token is created and the spell still goes to graveyard/exile
- Flashback exile: PASS -- `move_spell_after_resolve` checks `cast_with_flashback` flag and exiles if true, sends to graveyard otherwise (state.rs:1132-1141)
- Token is a copy (not just a vanilla creature): PASS -- `create_token_copy` copies name, power, toughness, card_types, subtypes, keywords from the source, and sets `card_id` so the token gets the same `CardBehavior` (state.rs:413-448)
- Parallel Lives doubling: PASS -- `create_token_with_subtypes` (called by `create_token_copy`) checks for Parallel Lives and creates extra copies (state.rs:325-335)

### Test coverage
- Creates token copy with correct name/P/T: `tier12_cards.rs:487` (cackling_counterpart_creates_token_copy)
- Flashback cost present and correct (mana value 7): `tier12_cards.rs:510` (cackling_counterpart_has_flashback)
- Target fizzle (creature removed before resolution): NOT TESTED
- Copying a legendary creature (legend rule): NOT TESTED
- Token copy of a token: NOT TESTED
