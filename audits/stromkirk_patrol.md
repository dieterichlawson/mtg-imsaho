## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/118/stromkirk-patrol?utm_source=api
**Type line**: `Creature — Vampire Soldier` — {4}{B}, 4/3
**Oracle text**:
```
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.

Self-variant trigger, one +1/+1 counter.
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/118/stromkirk-patrol?utm_source=api
**Type line**: `Creature — Vampire Soldier` — {4}{B}, 4/3
**Oracle text**:
```
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.

- The counter goes on the creature that dealt the damage, and only combat damage
  to a **player** counts — blocking a creature gives nothing: PASS
- CR 113.7a: the creature trading in the same combat damage step does not counter
  its own trigger: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The counter on connect: `combat_rules.rs`

## Audit — 2026-08-28 18:17

**Follow-up to the audit above, not a re-audit.**

Swept out with eight other cards: the `if ... zone == Zone::Battlefield` in front of
`add_counters` was a second copy of CR 121.1, which `GameState::add_counters` enforces for every
card. Behaviour is unchanged — mutation-checking the guard on each card showed it never fired.

`test_suite_guards.rs::no_card_re_checks_the_battlefield_before_adding_counters` now fails the
build if one comes back.
