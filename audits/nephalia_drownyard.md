## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/245/nephalia-drownyard?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{1}{U}{B}, {T}: Target player mills three cards.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}{U}{B}, {T}: Target player mills three cards" — through `mill_cards`, so
  creature cards among them emit `CreatureCardMilled`: PASS
- The mill happens on resolution, not on activation (CR 602.2a): PASS
- Its mana ability and its activated ability are both offered while it is
  untapped, and neither after: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mill on resolution: `activated_no_stack.rs:nephalia_drownyard_mills_on_resolution`
