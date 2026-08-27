## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/khc/25/geist-honored-monk?utm_source=api
**Type line**: `Creature — Human Monk` — {3}{W}{W}, */*
**Oracle text**:
```
Vigilance
Geist-Honored Monk's power and toughness are each equal to the number of creatures you control.
When this creature enters, create two 1/1 white Spirit creature tokens with flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "power and toughness are each equal to the number of creatures you control" is
  a characteristic-defining ability — `dynamic_pt`, recomputed every time rather
  than snapshotted, and it counts itself: PASS
- The two Spirit tokens it makes are creatures you control, so they raise its own
  P/T: PASS
- The tokens carry colour, subtype and flying via
  `create_token_with_subtypes`: PASS
- Vigilance: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The CDA and the tokens: `cards_complex_creatures.rs`, `token_is_not_a_card.rs:cda_does_not_count_tokens_in_graveyard`
