## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/83/think-twice?utm_source=api
**Type line**: `Instant` — {1}{U}
**Oracle text**:
```
Draw a card.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

Draws one card; the flashback cost is declared in card data and was verified exact set-wide.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier D)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/83/think-twice?utm_source=api
**Type line**: `Instant` — {1}{U}
**Oracle text**:
```
Draw a card.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

Draw a card; flashback {2}{U}. The draw goes through `engine::draw_cards`, so
an empty library is handled by the engine (and by Laboratory Maniac's
replacement) rather than by the card.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`flashback.rs` — cast from hand, then from the graveyard, then exiled.

## Audit — 2026-08-28 19:47

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Draw a card.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found in the card. `mtg-engine/src/cards/isd/think_twice.rs` matches: {1}{U} Instant, flashback {2}{U}, on_resolve draws one via `crate::engine::draw_cards` (empty-library loss applies). No self-cleanup.

One test weakness found and fixed: `think_twice_draws_from_graveyard` stocked the library with a single card, so a card that drew TWO passed — the second draw just hit an empty library. Library now holds two cards; the exact-count assertion is falsifiable.

### Tricky interactions checked
- All six Scryfall rulings are the generic flashback rules: alternative cost, exile on leaving the stack however it leaves, timing by card type (instant — castable any time priority allows, including from the graveyard on an opponent's turn), castable even if it reached the graveyard without being cast (mill). Engine-generic, tested in `flashback.rs`. PASS
- Mana value stays 2 regardless of the flashback cost paid: MV computed from `cost`, not the paid cost. PASS
- Draw through `draw_cards`: draw-from-empty is the engine's loss rule, not a panic. PASS

### Test coverage
- Flashback cast draws one and is exiled: `mtg-engine/tests/flashback.rs` `think_twice_draws_from_graveyard` (now with a 2-card library)
- Offered from graveyard: `flashback.rs` `every_flashback_card_is_offered_from_the_graveyard`
- Countered flashback still exiled: `flashback.rs` `flashback_spell_countered_is_exiled` (engine-generic)
- Used as the stock instant in `cards_shortcuts_taken.rs` and exile-zone tests in `cards_lands_and_mana_sources.rs` (incidental coverage of the from-hand cast)

Mutation check: `draw_cards(..., 1, ...)` -> `2`: with the 1-card library the test PASSED (vacuous — recorded above, fixed); after stocking two cards it FAILS. Bites now.
