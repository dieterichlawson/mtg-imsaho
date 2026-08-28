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

## Audit — 2026-08-28 18:22

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Stromkirk Patrol"`, https://scryfall.com/card/isd/118/stromkirk-patrol
**Oracle text**:
```
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```
**Type line**: Creature — Vampire Soldier
**Mana cost**: {4}{B}   **P/T**: 4/3
**Rulings**: none on Scryfall for this card.
**Status**: PASS (one doc comment corrected)

### Code issues
No behavioural issues. `{4}{B}`, `Creature`, `subtypes: ["Vampire", "Soldier"]` — both — 4/3,
oracle text verbatim, `TriggerKind::CombatDamageToPlayer` (the self-scoped kind) matching the
implemented hook, one counter on the Patrol itself.

The redundant CR 121.1 guard this card carried was swept out earlier in this pass, with eight
others and a guard test to keep them out.

**One inaccuracy, in the doc comment rather than the data**: it read
`/// Stromkirk Patrol — {4}{B} 4/3 Vampire.` and the card is a Vampire *Soldier*. The
`subtypes` field was right; the summary above it was not. Corrected — a doc comment that
disagrees with the card is the first place a reader looks and the last place a test does.

### Tricky interactions checked
- **"it" is the Patrol**: PASS.
- **Both subtypes**: PASS, and pinned set-wide against the checked-in Scryfall type line.
- **The self-scoped trigger kind, not the watcher kind**: PASS — it says "this creature", so an
  ally's combat damage does nothing.
- **Rakish Heir alongside**: the Patrol is a Vampire, so a Heir adds a second counter. Covered
  by the Heir's own tests.
- **Dying in the combat damage step it connected in**: the trigger is still collected (the
  damage-collector fix earlier in this pass) and the counter lands nowhere (CR 121.1).
- **A 4/3 that grows to 5/4 on connecting**: nothing special beyond the counter.

### Test coverage
- one +1/+1 counter on combat damage to a player:
  `cards_combat_damage_triggers.rs:539 a_self_countering_creature_gets_its_counters_on_combat_damage`
  (table row, exact count)
- the type line, including both subtypes:
  `card_data_invariants.rs:1587 type_lines_say_what_scryfall_says` (set-wide, against the cache)
- power and toughness, mana cost, oracle text: the sibling invariants in the same file

Mutation-checked: dropping the Soldier subtype fails the type-line sweep by name; changing the
toughness to 4 fails the P/T sweep. No new tests — every claim this card makes is already
pinned, and adding a per-card restatement of a registry-wide invariant is what
`bloodcrazed_neonate`'s deleted test was.

### Changes made
- `stromkirk_patrol.rs`: the doc comment now names both subtypes. No behavioural change, no new
  tests.
