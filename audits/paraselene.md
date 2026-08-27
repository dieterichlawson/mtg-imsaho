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
