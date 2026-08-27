## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/148/infernal-plunge?utm_source=api
**Type line**: `Sorcery` — {R}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Add {R}{R}{R}.
```
**Status**: PASS

### Code issues
No issues found.

Adds {R}{R}{R} to the pool. Sacrifice is an additional cost paid at cast time.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/148/infernal-plunge?utm_source=api
**Type line**: `Sorcery` — {R}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Add {R}{R}{R}.
```

**Status**: PASS

### Code issues
No issues found.

Same additional-cost shape as Altar's Reap, same two rulings, same engine
path. `on_resolve` adds {R}{R}{R} to the controller's pool. A sorcery, so the
mana arrives during a main phase with an empty stack — the usual ritual timing.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_sacrifice_and_additional_costs.rs` — sacrifice at cast, three red mana on resolution.
