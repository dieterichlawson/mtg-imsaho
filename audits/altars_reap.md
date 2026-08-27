## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/86/altars-reap?utm_source=api
**Type line**: `Instant` — {1}{B}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
```
**Status**: PASS

### Code issues
No issues found.

Draws two. The sacrifice is an additional cost paid at cast time (CR 601.2f), so it happens even if the spell is later countered — correctly not part of resolution.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/86/altars-reap?utm_source=api
**Type line**: `Instant` — {1}{B}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
```

**Status**: PASS

### Code issues
No issues found.

Both rulings hold structurally. "You must sacrifice exactly one creature" —
`AdditionalCost::SacrificeCreature` is a fixed one, and `legal_actions` will not
offer the cast with no creature to sacrifice. "Players can only respond once
this spell has been cast and all its costs have been paid" — the sacrifice is
paid in `pay_additional_cost` during the cast, before the spell is on the stack
and before anyone gets priority; `on_resolve` only draws.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_sacrifice_and_additional_costs.rs` — the creature is gone before the spell can be responded to.
