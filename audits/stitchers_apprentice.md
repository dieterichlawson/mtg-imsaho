## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/81/stitchers-apprentice?utm_source=api
**Type line**: `Creature — Homunculus` — {1}{U}, 1/2
**Oracle text**:
```
{1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The creature you sacrifice ... could be the Homunculus you've just
  created. It could also be Stitcher's Apprentice itself" — the sacrifice is
  part of the *effect* (after the colon), so the token is on the battlefield and
  eligible: PASS
- Ruling: "You create a token and sacrifice a creature all while the activated
  ability is resolving. Nothing can happen between the two" — both happen inside
  one `resolve_activated_ability`: PASS
- Ruling: "Any abilities that trigger on the Homunculus token entering the
  battlefield will resolve after you've sacrificed a creature" — triggers are
  collected and resolved after the ability finishes: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Token then sacrifice, in that order: `cards_complex_creatures.rs:stitchers_apprentice_creates_token_then_sacrifices`
- The token is a 2/2 Homunculus: `cards_complex_creatures.rs:stitchers_apprentice_token_is_2_2_homunculus`
- ETB triggers fire after the sacrifice: `phantom_triggers.rs`
