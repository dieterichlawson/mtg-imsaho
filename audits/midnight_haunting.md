## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/22/midnight-haunting?utm_source=api
**Type line**: `Instant` — {2}{W}
**Oracle text**:
```
Create two 1/1 white Spirit creature tokens with flying.
```
**Status**: PASS

### Code issues
No issues found.

Two 1/1 white Spirit tokens with flying, created with their subtype. Instant speed comes from the card type, not a Flash keyword.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/22/midnight-haunting?utm_source=api
**Type line**: `Instant` — {2}{W}
**Oracle text**:
```
Create two 1/1 white Spirit creature tokens with flying.
```

**Status**: PASS

### Code issues
No issues found.

Two 1/1 white Spirit tokens with flying, same per-token creation as Moan of
the Unhallowed. An instant — castable at end of turn or as a combat trick, which
is the whole point of the card over Moan.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs` — count, flying, and Spirit subtype for Geist-Honored Monk interactions.

## Audit — 2026-08-28 19:44

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Create two 1/1 white Spirit creature tokens with flying.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/midnight_haunting.rs` matches: {2}{W} Instant, on_resolve creates two tokens via `create_token_with_subtypes("", ..., 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit"], ...)`. Name derives to "Spirit Token" (CR 111.4); no self-cleanup.

### Tricky interactions checked
- Instant timing (tokens as surprise blockers / end-of-turn): engine-generic, `instant_interaction.rs` (`can_cast_instant_during_opponent_combat`, `can_cast_instant_during_opponent_main_phase`). PASS
- Parallel Lives: each of the two creations runs the CreatesTokens replacement — four Spirits. PASS
- Spirit tokens visible to subtype-reading effects (no registry face): subtype accessor union, `subtype.rs:456` creates exactly this token shape. PASS
- Untargeted spell — nothing to fizzle; resolves even if the board changes. PASS

### Test coverage
- Main effect: `mtg-engine/tests/cards_death_triggers_and_tokens.rs` `token_making_spells_make_the_tokens_they_print` — asserts two tokens named "Spirit Token", 1/1, white, Spirit subtype, creature type, Flying (color/subtype/card-type assertions added during the Moan of the Unhallowed audit)
- Instant timing: `instant_interaction.rs` (engine-generic)
- No rulings on Scryfall for this card.

Mutation check: removing `Keyword::Flying` from the token creation (with an import sink so it compiles) fails `token_making_spells_make_the_tokens_they_print` ("Midnight Haunting's tokens have Flying"). Bites. A first attempt at this mutation failed to compile (unused import) and was redone — a non-compiling mutation proves nothing.
