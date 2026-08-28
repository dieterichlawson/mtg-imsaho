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

## Audit — 2026-08-28 17:46

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Stromkirk Noble"`, https://scryfall.com/card/isd/164/stromkirk-noble
**Oracle text**:
```
This creature can't be blocked by Humans.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
```
**Type line**: Creature — Vampire Noble
**Mana cost**: {R}   **P/T**: 1/1
**Rulings**: none on Scryfall for this card.
**Status**: ISSUE (one redundant guard removed; behaviour was correct)

### Code issues

**A second copy of CR 121.1.** `on_combat_damage_to_player` was
`if state.get_object(self_id).is_some_and(|o| o.zone == Zone::Battlefield) { state.add_counters(..) }`.
`GameState::add_counters` already refuses anything not on the battlefield, for every card at
once. Removed, as on Rakish Heir earlier in this pass.

Everything else is right. `{R}`, `Creature`, `subtypes: ["Vampire", "Noble"]` — both, which is
the subtype trap this card is the example of — 1/1, oracle text verbatim,
`TriggerKind::CombatDamageToPlayer` (the self-scoped one) matching the implemented hook, and
"it" is the Noble.

"Can't be blocked by Humans" is modelled as
`CanOnlyBeBlockedBy { allowed_blockers: Not(HasSubtype("Human")) }`. The double negative reads
oddly but is exactly equivalent: a blocker must be a non-Human, and two such restrictions
compose the same way two "can't be blocked by" clauses would. It is not menace and does not
pretend to be.

### Tricky interactions checked
- **A Human cannot block it; a non-Human can**: PASS.
- **A transformed Human Werewolf CAN block it**: PASS, and this is the one that matters in this
  set — half its Humans are the front face of a werewolf. `CreatureFilter::HasSubtype` goes
  through `state.has_subtype`, which reads the ACTIVE face, so a transformed Village Ironsmith
  is a Werewolf and the restriction stops reaching it.
- **A Human granted to a non-Human**: `has_subtype` reads `obj.subtypes` before the registry, so
  a runtime-granted Human would count. Nothing in this pool grants Human.
- **"it" is the Noble, not the damaged player**: PASS.
- **The Noble dying in the combat damage step that connected**: the trigger is still collected
  (fixed earlier this pass in the damage collector) and the counter lands nowhere via CR 121.1.
- **Blocked, then the blocker stops being a Human**: CR 509.1a checks the restriction when
  blockers are declared, so the block stands. Not tested — no line of play in this pool reaches
  it at instant speed after blockers are declared.

### Test coverage
- a Human cannot block, a non-Human can:
  `cards_combat_damage_triggers.rs:369 stromkirk_noble_cant_be_blocked_by_humans`
- a transformed Human can:
  `cards_combat_damage_triggers.rs:392 stromkirk_noble_can_be_blocked_by_a_human_that_has_transformed` (NEW)
- the +1/+1 counter on combat damage to a player:
  `cards_combat_damage_triggers.rs:~520 a_self_countering_creature_gets_its_counters_on_combat_damage` (table row)
- an anthem stacking on top of its counters: `snapshot_anthems.rs:51`, `snapshot_anthems.rs:81`
- Rakish Heir's counter arriving on it as well: `cards_combat_damage_triggers.rs:322`

Mutation-checked: dropping the `Not(..)` (so it could only be blocked BY Humans) kills both
blocking tests; filtering on any creature instead of Humans kills both; and making
`CreatureFilter::HasSubtype` read the printed front face instead of the active one kills **only
the new test** — which is the point of it. The existing test uses Unruly Mob, a Human with no
other face, so it cannot tell those two reads apart.

### Changes made
- `stromkirk_noble.rs`: dropped the card's own copy of CR 121.1.
- `cards_combat_damage_triggers.rs`: the transformed-blocker test.
