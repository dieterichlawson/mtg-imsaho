## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Create two 2/2 black Zombie creature tokens.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Flashback exile on resolution**: `move_spell_after_resolve` in `state.rs:1132` checks `obj.cast_with_flashback` and routes to `Zone::Exile` when true. The `cast_with_flashback` flag is set at `engine.rs:1637` when the spell is cast from the graveyard. Path is correct: pass.
- **Flashback exile when countered**: `stack.rs:84` calls `move_spell_after_resolve` when a spell fizzles (all targets illegal). Counter-by-counter-spell goes through `engine.rs` which removes the spell from the stack and calls the countering behavior, which also routes through `move_spell_after_resolve`. Both paths correctly exile a flashback spell: pass.
- **Sorcery timing restriction for flashback**: `engine.rs:692-706` checks `is_sorcery_type` for graveyard casts (including flashback) and gates them behind `is_sorcery_speed` (main phase, empty stack, active player's turn), matching the ruling "you can cast a sorcery using flashback only when you could normally cast a sorcery": pass.
- **Token colors**: `create_token_with_subtypes` call passes `vec![Color::Black]`; `create_token_internal` at `state.rs:376` stores it directly on the `GameObject`: pass.
- **Token subtypes**: Call passes `vec!["Zombie".into()]`; `create_token_internal` at `state.rs:379` stores it in `obj.subtypes`: pass.
- **Token count (two)**: `for _ in 0..2` loop makes exactly two calls to `create_token_with_subtypes`: pass.
- **Token P/T (2/2)**: Arguments `power=2, toughness=2` stored as `Some(2)` / `Some(2)` at `state.rs:374-375`: pass.
- **Parallel Lives doubling**: `create_token_with_subtypes` at `state.rs:325-345` counts Parallel Lives permanents under the same controller and creates extra copies using `(1 << count) - 1` additional tokens per call. Two calls for two tokens gives correct doubling behavior (1 PL → 4 tokens; 2 PL → 8 tokens): pass.
- **Normal cast goes to graveyard (not exile)**: When cast from hand, `in_graveyard` is false so `is_flashback` is false at `engine.rs:1492`, `cast_with_flashback` is never set, and `move_spell_after_resolve` routes to `Zone::Graveyard`: pass.
- **Flashback cost correctness {5}{B}{B}**: `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(5), ManaSymbol::Colored(Color::Black), ManaSymbol::Colored(Color::Black)]))` matches oracle text: pass.
- **Mana cost {2}{B}{B}**: `cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Black), ManaSymbol::Colored(Color::Black)]))` matches oracle text: pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Two 2/2 Zombie tokens created on normal cast: `mtg-engine/tests/tier3_cards.rs:51` (`moan_creates_two_zombies`) — TESTED
- Tokens are 2/2: `mtg-engine/tests/tier3_cards.rs:68-70` — TESTED
- Normal cast spell goes to graveyard: `mtg-engine/tests/tier3_cards.rs:59` — TESTED
- Tokens are black: NOT TESTED (token color not asserted in the test)
- Tokens have Zombie subtype: NOT TESTED (token subtypes not asserted in the test)
- Flashback cast exiles spell: NOT TESTED for this card specifically; covered generically by `mtg-engine/tests/flashback.rs:86` (`flashback_spell_is_exiled_after_resolve`) using Geistflame
- Flashback countered spell still exiled: NOT TESTED for this card specifically; covered generically by `mtg-engine/tests/flashback.rs:129` (`flashback_spell_countered_is_exiled`) using Geistflame
- Sorcery timing restriction for flashback: NOT TESTED
- Flashback cost {5}{B}{B} is correctly required (not {2}{B}{B}): NOT TESTED
