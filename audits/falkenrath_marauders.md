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

- The counter goes on the creature that dealt the damage, and only combat damage
  to a **player** counts — blocking a creature gives nothing: PASS
- CR 113.7a: the creature trading in the same combat damage step does not counter
  its own trigger: PASS
- "**two** +1/+1 counters", not one: PASS
- Flying and haste, so it can connect the turn it arrives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Two counters on connect: `combat_rules.rs`

## Audit — 2026-08-28 18:15

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Falkenrath Marauders"`, https://scryfall.com/card/isd/141/falkenrath-marauders
**Oracle text**:
```
Flying
Haste (This creature can attack and {T} as soon as it comes under your control.)
Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
```
**Type line**: Creature — Vampire Warrior
**Mana cost**: {3}{R}{R}   **P/T**: 2/2   **Keywords**: Flying, Haste
**Rulings**: none on Scryfall for this card.
**Status**: ISSUE (one redundant guard; behaviour correct)

### Code issues

**A second copy of CR 121.1.** `on_combat_damage_to_player` was
`if state.get_object(self_id).is_some_and(|o| o.zone == Zone::Battlefield) { state.add_counters(.., 2) }`.
`add_counters` already enforces that. Removed — the sixth card in this pass to carry one, after
Rakish Heir, Champion of the Parish, Stromkirk Noble, Lumberknot and Bloodcrazed Neonate.

Everything else is right: `{3}{R}{R}`, `Creature`, `subtypes: ["Vampire", "Warrior"]` — both —
2/2, `keywords: vec![Flying, Haste]`, oracle text verbatim, `TriggerKind::CombatDamageToPlayer`
(the self-scoped kind) matching the hook, and **two** counters rather than one.

### Tricky interactions checked
- **Two counters, not one**: PASS, and the table asserts the exact number.
- **"it" is the Marauders**: PASS.
- **Flying and haste**: declared keywords, pinned set-wide against the checked-in Scryfall cache.
- **Haste and the trigger together**: a Marauders cast and attacking the same turn connects and
  grows. Haste is engine-side; nothing card-specific.
- **Dying in the combat damage step it connected in**: the trigger is still collected (the
  damage-collector fix earlier in this pass) and the counters land nowhere (CR 121.1).
- **Rakish Heir alongside**: the Marauders is a Vampire, so a Heir adds a third counter. Covered
  by the Heir's own tests.

### Test coverage
- two +1/+1 counters on combat damage to a player:
  `cards_combat_damage_triggers.rs:539 a_self_countering_creature_gets_its_counters_on_combat_damage`
  (table row, exact count)
- the keywords match the printed card:
  `card_data_invariants.rs:1808 keywords_say_what_scryfall_says` (set-wide, against the cache)
- the trigger kind matches the printed text, for every card:
  the trigger-declaration invariants in `card_data_invariants.rs`

Mutation-checked: one counter instead of two fails the table; dropping `Keyword::Haste` fails
the keyword sweep.

### Changes made
- `falkenrath_marauders.rs`: dropped the card's own copy of CR 121.1. No test changes — the
  card's two claims were already pinned, one by an exact-count table row and one by a
  registry-wide sweep against Scryfall.
