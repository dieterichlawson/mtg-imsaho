## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/141/falkenrath-marauders?utm_source=api
**Type line**: `Creature — Vampire Warrior` — {3}{R}{R}, 2/2
**Oracle text**:
```
Flying
Haste (This creature can attack and {T} as soon as it comes under your control.)
Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
```

**Status**: PASS

### Code issues
No issues found.

- Self-variant trigger, and **two** +1/+1 counters — the quantity distinguishes
  it from the three other vampires with the same trigger shape in this set.
- All four counter-adders check the creature is still on the battlefield before
  adding, so an ability resolving after its source died does nothing rather than
  putting a counter on a permanent that is not there.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_combat_damage_triggers.rs` — including a table-driven coverage check that every card with this trigger shape in the set is exercised.
