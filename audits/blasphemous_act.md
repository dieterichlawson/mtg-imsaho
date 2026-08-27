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
## Audit — 2026-08-27 (Tier B)

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


### Tricky interactions checked
- Ruling: "Blasphemous Act's ability **can't reduce the total cost to cast the
  spell below {R}**." The reduction is `creature_count.min(8)`, so the {8}
  generic can go to zero but the coloured pip always remains: PASS
- "for each creature on the battlefield" — **all** creatures, not just yours:
  PASS
- Ruling: "The total cost is locked in before you pay that cost" — the count is
  taken during casting, so sacrificing a creature for mana afterwards does not
  raise the price back: PASS
- "deals 13 damage to **each** creature" — no targeting, so hexproof does not
  save anything, and the damage goes through `deal_damage` so protection,
  prevention and Unbreathing Horde's replacement all apply: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The cost reduction and the sweep: `cards_burn_and_damage.rs`, `inline_damage.rs`
