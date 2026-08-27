## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/130/blasphemous-act?utm_source=api
**Type line**: `Sorcery` — {8}{R}
**Oracle text**:
```
This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
```
**Status**: PASS

### Code issues
No issues found.

- 13 damage to each creature through `apply_pending_effect`, so the damage
  pipeline applies protection, prevention and replacements (Unbreathing Horde's
  among them).
- The creature list is snapshotted before any damage lands; nothing dies
  mid-resolution anyway, since state-based actions do not run until it finishes
  (CR 704.3).
- The cost reduction is declared in card data and handled by the cost pipeline.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
