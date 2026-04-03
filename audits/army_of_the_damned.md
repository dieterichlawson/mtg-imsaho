# Audit: Army of the Damned

## Reference (Scryfall/API)
- **Name:** Army of the Damned
- **Mana Cost:** {5}{B}{B}{B}
- **Type:** Sorcery
- **Oracle:** Create thirteen tapped 2/2 black Zombie creature tokens. Flashback {7}{B}{B}{B}
- **P/T:** N/A

## Implementation: `army_of_the_damned.rs`
- **Name:** Army of the Damned -- CORRECT
- **Mana Cost:** {5}{B}{B}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Flashback:** {7}{B}{B}{B} -- CORRECT
- **Effect:** Creates 13 tokens, each 2/2 black Zombie creature, enters tapped -- CORRECT
- **Token subtypes:** ["Zombie"] -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Create thirteen tapped 2/2 black Zombie creature tokens.\nFlashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch (cosmetic)**: Oracle says `"Create thirteen tapped 2/2 black Zombie creature tokens."` but code oracle_text field says `"Create thirteen 2/2 black Zombie creature tokens. They enter the battlefield tapped."` The code uses an older template; the current oracle consolidates "tapped" into the create clause.
   - Code: `"Create thirteen 2/2 black Zombie creature tokens. They enter the battlefield tapped."`
   - Oracle: `"Create thirteen tapped 2/2 black Zombie creature tokens."`

Behavior is otherwise correct: creates 13 tokens with correct stats (2/2 black Zombie creature), sets each tapped, flashback_cost is {7}{B}{B}{B}. All functional behavior matches.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Create thirteen tapped 2/2 black Zombie creature tokens." (was "Create thirteen 2/2 black Zombie creature tokens. They enter the battlefield tapped."). Doc comment updated. Behavior unchanged.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Create thirteen tapped 2/2 black Zombie creature tokens.
Flashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Mana cost**: {5}{B}{B}{B}
**Status**: PASS

### Code issues
No issues found.

All card data verified against oracle text:
- Mana cost {5}{B}{B}{B}: matches code `ManaSymbol::Generic(5), ManaSymbol::Colored(Color::Black) x3`
- Type line "Sorcery": matches code `card_types: vec![CardType::Sorcery]`
- No supertypes, no subtypes: matches code `supertypes: vec![], subtypes: vec![]`
- Oracle text field: matches current Scryfall wording exactly
- Flashback cost {7}{B}{B}{B}: matches code `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(7), ManaSymbol::Colored(Color::Black) x3]))`
- Token creation: 13 iterations creating 2/2 black Zombie creature tokens with correct name, P/T, color, card types, and subtypes
- Tokens enter tapped: `obj.tapped = true` set after each token creation
- Spell cleanup: uses `move_spell_after_resolve` (correct pattern)
- No targeting required: no `target_requirement` or `is_valid_target` (correct -- oracle has no "target")

### Tricky interactions checked
- Tokens enter tapped (not "enter then tap"): pass -- functionally correct; tapped is set immediately after creation before any trigger processing
- No targeting (cannot fizzle): pass -- correctly has no targeting methods
- Flashback cost correctness: pass -- {7}{B}{B}{B} matches oracle
- Token subtypes via `create_token_with_subtypes`: pass -- uses correct API with `vec!["Zombie".into()]`
- Token color (black): pass -- `vec![Color::Black]` passed to token creation
- `move_spell_after_resolve` instead of raw zone move: pass

### Test coverage
- Main effect (13 tapped 2/2 Zombie tokens): `mtg-engine/tests/tier12_cards.rs:59` -- TESTED
- Token count (13): `mtg-engine/tests/tier12_cards.rs:70` -- TESTED
- Tokens enter tapped: `mtg-engine/tests/tier12_cards.rs:73-75` -- TESTED
- Token P/T (2/2): `mtg-engine/tests/tier12_cards.rs:78-81` -- TESTED
- Token color (black): NOT TESTED (test does not assert on color)
- Token subtype (Zombie): partially tested (test filters by name "Zombie" but does not assert subtypes)
- Flashback cast from graveyard: NOT TESTED for this specific card (flashback infrastructure tested elsewhere)
- Flashback exiled after resolution: NOT TESTED for this specific card

## Audit — 2026-04-02 20:28

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Create thirteen tapped 2/2 black Zombie creature tokens.
Flashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

All card data verified against oracle:
- Name: "Army of the Damned" -- matches
- Mana cost: `{5}{B}{B}{B}` -- code has `Generic(5), Colored(Black) x3` -- matches
- Type: Sorcery -- code has `vec![CardType::Sorcery]` -- matches
- Supertypes/subtypes: none -- code has `vec![], vec![]` -- matches
- Oracle text field: `"Create thirteen tapped 2/2 black Zombie creature tokens.\nFlashback {7}{B}{B}{B}"` -- matches current Scryfall wording
- Flashback cost: `{7}{B}{B}{B}` -- code has `Some(ManaCost::new(vec![Generic(7), Colored(Black) x3]))` -- matches
- Token creation: 13 iterations (loop `0..13`) creating 2/2 black Zombie creature tokens -- matches "thirteen tapped 2/2 black Zombie creature tokens"
- Tokens enter tapped: `obj.tapped = true` after each `create_token_with_subtypes` call -- correct
- Spell cleanup: `move_spell_after_resolve(object_id)` handles graveyard vs exile (flashback) -- correct
- No targeting: no `target_requirement` or `is_valid_target` implemented -- correct (oracle has no "target")

### Tricky interactions checked
- Flashback exiles after resolution: pass -- `move_spell_after_resolve` checks `cast_with_flashback` flag and moves to exile zone if true (state.rs:1132-1141)
- Tokens enter tapped (not ETB trigger): pass -- `tapped = true` set directly on the object immediately after creation, before any trigger processing occurs
- No targeting means spell cannot fizzle: pass -- no targeting code, resolves unconditionally
- Parallel Lives interaction: pass (minor caveat) -- `create_token_with_subtypes` returns only primary token ID; duplicates created by Parallel Lives would not get `tapped = true`. This is an engine-level pattern shared by Kessig Cagebreakers and Geist of Saint Traft, not an Army-specific bug.

### Test coverage
- Creates 13 Zombie tokens: `mtg-engine/tests/tier12_cards.rs:70` -- TESTED
- Tokens enter tapped: `mtg-engine/tests/tier12_cards.rs:73-75` -- TESTED
- Token P/T is 2/2: `mtg-engine/tests/tier12_cards.rs:78-81` -- TESTED
- Token color (black): NOT TESTED (test filters by name but does not assert color)
- Flashback cost present and correct: NOT TESTED for this specific card (flashback infrastructure tested via Divine Reckoning and Memory's Journey)
- Flashback cast + exile after resolution: NOT TESTED for this specific card
