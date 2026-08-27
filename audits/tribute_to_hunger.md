## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/119/tribute-to-hunger?utm_source=api
**Type line**: `Instant` — {2}{B}
**Oracle text**:
```
Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Target **opponent** sacrifices a creature **of their choice**" — the choice
  is the opponent's, not the caster's, and `is_valid_target` rejects the caster:
  PASS
- Sacrifice, not destroy, so indestructible does not save it: PASS
- Ruling: "Use the sacrificed creature's toughness **as it last existed on the
  battlefield** to determine how much life to gain" — last known information
  (CR 608.2g): PASS
- The life gain goes through `gain_life`: PASS
- An opponent with no creatures sacrifices nothing and you gain nothing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The opponent's choice and the life gain: `sacrifice_choice.rs`, `cards_removal.rs`
