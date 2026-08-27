## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/80/stitched-drake?utm_source=api
**Type line**: `Creature — Zombie Drake` — {1}{U}{U}, 3/4
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
Flying
```
**Status**: ISSUE

### Code issues
See below.

Same dead `on_resolve`; removed.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/80/stitched-drake?utm_source=api
**Type line**: `Creature — Zombie Drake` — {1}{U}{U}, 3/4
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
Flying
```

**Status**: PASS

### Code issues
No issues found.

Rulings: "exactly one creature card", and "players can only respond once ...
all its costs have been paid". `AdditionalCost::ExileCreaturesFromGraveyard(1)`
is a fixed count paid during the cast. Data-only card otherwise — 3/4 Zombie
Drake with flying, both subtypes.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_sacrifice_and_additional_costs.rs` — the exile happens at cast; `card_data_invariants.rs` covers the printed characteristics.
