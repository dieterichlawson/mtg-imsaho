## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Draw two cards, then discard a card at random.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues
- Inconsistent oracle text formatting compared to other flashback cards at mtg-engine/src/cards/isd/desperate_ravings.rs:25
  - Oracle text says: `Draw two cards, then discard a card at random. Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`  
  - Code does: `oracle_text: "Draw two cards, then discard a card at random.".into()` (missing flashback text)
  - Note: Other flashback cards like Think Twice include flashback text in oracle_text, while Geistflame doesn't. This inconsistency should be resolved.

### Tricky interactions checked
- Atomic resolution (draw then discard with no interruptions): pass - single on_resolve function handles both actions
- Random discard implementation: pass - uses `hand.choose(&mut rand::thread_rng()).copied()` for proper randomization
- Empty hand after drawing edge case: pass - code checks `if let Some(discard_id) = to_discard` before discarding
- Flashback exile vs graveyard: pass - `move_spell_after_resolve` correctly checks `cast_with_flashback` flag and exiles accordingly
- Controller consistency: pass - draws cards for spell controller and discards from same controller's hand
- Flashback cost accuracy: pass - {2}{U} correctly implemented as `ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Blue)`
- Spell timing restrictions for flashback: pass - handled by engine casting system, not card-specific code

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic functionality (draw 2, discard 1, net +1 hand): `mtg-engine/tests/flashback.rs:347-376`
- Flashback general mechanics (exile after resolve): `mtg-engine/tests/flashback.rs:84-106`  
- Flashback cost checking: `mtg-engine/tests/flashback.rs:22-40`
- Flashback exile when countered: `mtg-engine/tests/flashback.rs:128-162`
- Random discard specificity: NOT TESTED
- Flashback cast of Desperate Ravings specifically: NOT TESTED
- Atomic resolution timing (no interruptions): NOT TESTED
- Edge cases (0 cards, 1 card after drawing): NOT TESTED