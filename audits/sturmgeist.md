## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/82/sturmgeist?utm_source=api
**Type line**: `Creature — Spirit` — {3}{U}{U}, */*
**Oracle text**:
```
Flying
Sturmgeist's power and toughness are each equal to the number of cards in your hand.
Whenever this creature deals combat damage to a player, draw a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "power and toughness are each equal to the number of cards in your hand" is a
  characteristic-defining ability — `dynamic_pt`, recomputed live, so casting a
  card shrinks it mid-combat: PASS
- "Whenever **this creature** deals combat damage to a player, draw a card" —
  its own damage only, and the draw then grows it: PASS
- Flying: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The CDA and the draw trigger: `cards_complex_creatures.rs`, `combat_rules.rs`
