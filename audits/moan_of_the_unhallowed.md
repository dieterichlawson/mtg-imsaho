## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/109/moan-of-the-unhallowed?utm_source=api
**Type line**: `Sorcery` — {2}{B}{B}
**Oracle text**:
```
Create two 2/2 black Zombie creature tokens.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

Creates its two Zombie tokens through `create_token_with_subtypes` with ['Zombie'], so they are Zombies for Unbreathing Horde and the rest of the tribal set rather than nameless 2/2s.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/109/moan-of-the-unhallowed?utm_source=api
**Type line**: `Sorcery` — {2}{B}{B}
**Oracle text**:
```
Create two 2/2 black Zombie creature tokens.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

Two 2/2 black Zombie tokens, created one at a time through
`create_token_with_subtypes` so each is separately offered to Parallel Lives
(CR 614.5 — the doubler applies once per creation event, and there are two).
Zombie subtype supplied, which matters for the set's Zombie tribal.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs` — count, P/T, colour and subtype.

## Audit — 2026-08-28 19:43

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Create two 2/2 black Zombie creature tokens.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/moan_of_the_unhallowed.rs` matches: {2}{B}{B} Sorcery, `flashback_cost` {5}{B}{B}, on_resolve creates two tokens via `create_token_with_subtypes("", ..., vec![Color::Black], vec![CardType::Creature], vec![], vec!["Zombie"], ...)`. The helper derives the name "Zombie Token" (CR 111.4) and runs the token-count replacement (Parallel Lives) per token, so "create two" doubles to four. No self-cleanup; the engine moves the sorcery to the graveyard (or exile, on a flashback cast).

### Tricky interactions checked
- Flashback rulings (all six are generic flashback rules): engine-level — offered from graveyard, exiled whether it resolves or is countered, sorcery timing respected, cast even if it got to the graveyard without being cast (mill), alternative-cost interaction. PASS
- Parallel Lives with "create two": each token creation runs the CreatesTokens replacement, four tokens total. PASS
- Tokens are real Zombies for subtype-reading effects (obj.subtypes, not registry — tokens have no registry face): covered by the subtype accessor union. PASS
- Test gap found and closed: the token table test asserted name/count/P/T/keywords but never "black" or the Zombie subtype — a colorless or subtype-less token passed. Added color/subtype/card-type columns (also strengthens Midnight Haunting).

### Test coverage
- Main effect (two 2/2 black Zombie tokens, sorcery to graveyard): `mtg-engine/tests/cards_death_triggers_and_tokens.rs` `token_making_spells_make_the_tokens_they_print` (now asserts color, subtype, card type too)
- Flashback offered from graveyard: `mtg-engine/tests/flashback.rs` `every_flashback_card_is_offered_from_the_graveyard` (covers all flashback cards including this one)
- Flashback exile after resolve / when countered: `flashback.rs` `flashback_spell_is_exiled_after_resolve`, `flashback_spell_countered_is_exiled`
- Flashback data invariants: `card_data_invariants.rs` `flashback_is_only_on_instants_and_sorceries_and_says_so`
- Parallel Lives doubling: `mtg-engine/tests/cards_death_triggers_and_tokens.rs` (Parallel Lives audit, card 195)

Mutation check: removing `Color::Black` from the token creation fails `token_making_spells_make_the_tokens_they_print` ("Moan of the Unhallowed's tokens are Black"). Bites.
