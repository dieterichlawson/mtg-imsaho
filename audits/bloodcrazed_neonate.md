## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/131/bloodcrazed-neonate?utm_source=api
**Type line**: `Creature — Vampire` — {1}{R}, 2/1
**Oracle text**:
```
This creature attacks each combat if able.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.

- Self-variant trigger, one +1/+1 counter.
- "attacks each combat if able" is a static `ForceAttack`; the "if able" half is
  the engine's, and `combat_rules.rs` covers a forced attacker that a "can't
  attack" effect makes unable.
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/131/bloodcrazed-neonate?utm_source=api
**Type line**: `Creature — Vampire` — {1}{R}, 2/1
**Oracle text**:
```
This creature attacks each combat if able.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.

- The counter goes on the creature that dealt the damage, and only combat damage
  to a **player** counts — blocking a creature gives nothing: PASS
- CR 113.7a: the creature trading in the same combat damage step does not counter
  its own trigger: PASS
- "attacks each combat if able" is a requirement; CR 508.1d means a tapped or
  summoning-sick Neonate does not attack: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The attack requirement and the counter: `combat_requirements.rs`

## Audit — 2026-08-28 18:01

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Bloodcrazed Neonate"`, https://scryfall.com/card/isd/131/bloodcrazed-neonate
**Oracle text**:
```
This creature attacks each combat if able.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```
**Type line**: Creature — Vampire
**Mana cost**: {1}{R}   **P/T**: 2/1
**Rulings**: none on Scryfall for this card.
**Status**: ISSUE (one redundant guard; behaviour correct)

### Code issues

**A second copy of CR 121.1.** `on_combat_damage_to_player` was
`if state.get_object(self_id).is_some_and(|o| o.zone == Zone::Battlefield) { state.add_counters(..) }`.
`add_counters` already enforces that for every card. Removed — the fifth card in this pass to
carry one, after Rakish Heir, Champion of the Parish, Stromkirk Noble and Lumberknot.

Everything else is right: `{1}{R}`, `Creature`, `subtypes: ["Vampire"]`, 2/1, oracle text
verbatim, `ContinuousEffect::ForceAttack { scope: OnSelf }` for the first line, and
`TriggerKind::CombatDamageToPlayer` — the self-scoped kind, not the watcher kind — matching the
implemented hook for the second.

### Tricky interactions checked
- **"attacks each combat if able"**: PASS. Scoped `OnSelf`, so it is the Neonate's own
  requirement rather than one it imposes.
- **Tapped or summoning-sick, it is not forced**: PASS — "if able", and a tapped creature cannot
  attack (CR 508.1a). Shared with Galvanic Juggernaut, tested there.
- **"put a +1/+1 counter on it"** — "it" is the Neonate: PASS.
- **A 2/1 that must attack into anything**: that is the card working, not a bug.
- **The Neonate dying in the combat damage step it connected in**: the trigger is still
  collected (the damage collector fix earlier in this pass) and the counter lands nowhere
  (CR 121.1).
- **Rakish Heir's counter arriving as well**: the Neonate is a Vampire, so a Heir gives it a
  second counter. Covered by the Heir's own tests with a different Vampire.

### Test coverage
- the +1/+1 counter on combat damage to a player:
  `cards_combat_damage_triggers.rs:539 a_self_countering_creature_gets_its_counters_on_combat_damage`
  (table row). The table's own coverage check found this card in the first place: the four
  hand-written tests it replaced never covered the Neonate's counter at all.
- "attacks each combat if able" is declared, for every card in the set that prints it, at the
  right scope:
  `card_data_invariants.rs:2008 attacks_each_combat_in_the_text_means_force_attack_in_the_effects` (NEW)
- a creature whose requirement is its own is really declared as an attacker:
  `combat_rules.rs:259 a_creature_that_forces_itself_to_attack_must_attack`
- and is not, when tapped: `combat_rules.rs:236`

**One test deleted.** `bloodcrazed_neonate_forced_to_attack` read
`state.has_effect(neonate, ForceAttack)` and asserted it was there — a claim about the card's
own `card_data`, restated. Deleting `ForceAttack` from the card leaves `combat_rules.rs`
passing (its tests name the Juggernaut) and fails the new invariant, so the invariant is
strictly stronger than what it replaces. Mutation-checked both ways: removing the effect and
mis-scoping it to `Attached` each fail the invariant.

Removing the test left the file's "Bloodcrazed Neonate" section header with nothing under it,
which `test_suite_guards.rs` caught — twice, in two different guards. The header is gone; the
card stays in the module doc alongside Stromkirk Patrol and Falkenrath Marauders, the other
cards whose coverage in that file is a table row.

### Changes made
- `bloodcrazed_neonate.rs`: dropped the card's own copy of CR 121.1.
- `card_data_invariants.rs`: the new registry-wide ForceAttack invariant.
- `cards_combat_damage_triggers.rs`: the card-data-only test deleted, its section header with
  it, and the reasoning moved to the table that still covers the card.
