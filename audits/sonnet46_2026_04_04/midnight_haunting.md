## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Create two 1/1 white Spirit creature tokens with flying.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Correct token count (two tokens): The `on_resolve` loop runs exactly twice, each iteration calling `create_token_with_subtypes`. PASS
- Token color (white): `vec![Color::White]` passed to `create_token_with_subtypes`. PASS
- Token type/subtype (Spirit creature): `card_types: vec![CardType::Creature]` and `subtypes: vec!["Spirit".into()]`. PASS
- Token P/T (1/1): Arguments `1, 1` passed for power/toughness. PASS
- Token keyword (Flying): `vec![Keyword::Flying]` passed. PASS
- Spell cleanup (instant to graveyard): `move_spell_after_resolve` moves to `Zone::Graveyard` (or `Zone::Exile` if cast with flashback — Midnight Haunting has no flashback, so correctly goes to graveyard). PASS
- Parallel Lives interaction: Each `create_token_with_subtypes` call respects the Parallel Lives doubling logic in `state.rs:314`; with one Parallel Lives in play, each of the two calls creates an extra copy, yielding 4 tokens total. This is correct per MTG rules. PASS
- Controller fallback: `state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0))` — safe default if object is somehow missing; not a practical concern for a resolving spell. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Two Spirit tokens created on resolve: `mtg-engine/tests/tier3_cards.rs:24` — TESTED
- Tokens have P/T 1/1: `mtg-engine/tests/tier3_cards.rs:43-44` — TESTED
- Tokens have Flying: `mtg-engine/tests/tier3_cards.rs:45` — TESTED
- Spell moves to graveyard after resolve: `mtg-engine/tests/tier3_cards.rs:33` — TESTED
- Token color is white: NOT TESTED (test does not assert `colors`)
- Token has Spirit subtype: NOT TESTED (test filters by `name == "Spirit"` but does not assert `subtypes`)
- Parallel Lives doubling: NOT TESTED
