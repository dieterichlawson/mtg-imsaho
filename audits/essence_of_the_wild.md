## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/178/essence-of-the-wild?utm_source=api
**Type line**: `Creature — Avatar` — {3}{G}{G}{G}, 6/6
**Oracle text**:
```
Creatures you control enter as a copy of this creature.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- Excludes itself, non-creatures, and creatures an opponent controls — all
  three required by "**Creatures you control** enter as a copy of **this**
  creature".
- `e.copy_of.is_some()` guard stops the effect applying twice to one event
  (CR 614.5).
- Uses `state.is_creature`, so a token entering is covered.

### Test coverage
`copy_effects.rs`
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/178/essence-of-the-wild?utm_source=api
**Type line**: `Creature — Avatar` — {3}{G}{G}{G}, 6/6
**Oracle text**:
```
Creatures you control enter as a copy of this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Creatures you control **enter as a copy of** this creature" is a replacement
  effect applied as the creature enters (CR 614), not something done afterwards:
  PASS
- It excludes **itself** — the Essence is not on the battlefield when it is
  entering, so it cannot copy itself: PASS
- "Creatures **you control**", so an opponent's creatures are unaffected: PASS
- It applies to tokens too, since a token entering is a creature entering: PASS
- `copy_of.is_some()` guards against applying twice when another copy effect has
  already claimed the entry: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The entering copy: `cards_complex_creatures.rs`, `enters_tapped.rs`
