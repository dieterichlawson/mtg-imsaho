## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/26/paraselene?utm_source=api
**Type line**: `Sorcery` — {2}{W}
**Oracle text**:
```
Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
```
**Status**: PASS

### Code issues
No issues found.

- "Destroy all enchantments" goes through `try_destroy_all`, one event
  (CR 700.2c), so each indestructible check is made against the battlefield as it
  stood before any of them died.
- "You gain 1 life for each enchantment destroyed **this way**" counts only
  `DestroyResult::Died`, so a regenerated or indestructible enchantment does not
  pay out. That distinction is why the card says "this way".

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/26/paraselene?utm_source=api
**Type line**: `Sorcery` — {2}{W}
**Oracle text**:
```
Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "You gain 1 life **for each enchantment destroyed this way**" — only the ones
  that actually died, counted from `DestroyResult::Died`, so an indestructible
  enchantment neither dies nor pays: PASS
- "Destroy **all** enchantments" — both players', and Auras and Curses count: PASS
- `try_destroy_all`, so they die simultaneously: PASS
- The life gain goes through `change_life`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The count of what actually died: `cards_removal.rs`
