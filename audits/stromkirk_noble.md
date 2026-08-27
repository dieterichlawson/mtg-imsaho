## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/164/stromkirk-noble?utm_source=api
**Type line**: `Creature — Vampire Noble` — {R}, 1/1
**Oracle text**:
```
This creature can't be blocked by Humans.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.

- Self-variant trigger, one +1/+1 counter.
- "can't be blocked by Humans" is a separate static restriction, and it is a
  *blocking* restriction rather than an evasion keyword — correctly not menace or
  intimidate.
- All four counter-adders check the creature is still on the battlefield before
  adding, so an ability resolving after its source died does nothing rather than
  putting a counter on a permanent that is not there.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_combat_damage_triggers.rs` — including a table-driven coverage check that every card with this trigger shape in the set is exercised.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/164/stromkirk-noble?utm_source=api
**Type line**: `Creature — Vampire Noble` — {R}, 1/1
**Oracle text**:
```
This creature can't be blocked by Humans.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.

- The counter goes on the creature that dealt the damage, and only combat damage
  to a **player** counts — blocking a creature gives nothing: PASS
- CR 113.7a: the creature trading in the same combat damage step does not counter
  its own trigger: PASS
- "This creature **can't be blocked by Humans**" is a blocking restriction, not
  evasion and not intimidate — a Human simply cannot be declared as its blocker:
  PASS
- `has_subtype` covers a Human token: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The blocking restriction and the counter: `combat_rules.rs`, `subtype.rs`
