## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/29/rebuke?utm_source=api
**Type line**: `Instant` — {2}{W}
**Oracle text**:
```
Destroy target attacking creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **attacking** creature" — read from combat state, so it is only
  castable once attackers are declared: PASS
- CR 506.4: a creature removed from combat stops being an attacking creature, so
  the spell fizzles if that happens in response: PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Attacking-only targeting and the removal-from-combat fizzle: `cards_removal.rs`, `combat_rules.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/29/rebuke?utm_source=api
**Type line**: `Instant` — {2}{W}
**Oracle text**:
```
Destroy target attacking creature.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/29/rebuke
**Oracle text**: Destroy target attacking creature.
**Type line**: Instant
**Mana cost**: {2}{W}
**Rulings**: none (Scryfall returns no rulings for this card)
**Status**: ISSUE (fixed)

### Card data
Matches the fetched text field for field: name, `{2}{W}` as
`[Generic(2), Colored(White)]`, `card_types: [Instant]`, oracle text verbatim,
no P/T, no keywords, no subtypes, no flashback.

### Code issues

1. `is_valid_target` restated `target_requirement` (`rebuke.rs:28-40`, removed).
   - Oracle text says: `Destroy target attacking creature.`
   - `target_requirement` says: `CreatureWithFilter(TargetFilter::Attacking)`.
   - The override's whole body was:
     `let is_attacking = state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(id)); is_attacking`
   - `matches_target_filter` says, for that filter (`targeting.rs:483`):
     `TargetFilter::Attacking => { state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&obj.id)) }`
     — the same expression.
   - Its preamble was `if obj.zone != Zone::Battlefield || !state.is_creature(obj.id, registry) { return false; }`.
     Both halves are the callers' job now: the spell-cast enumerator reads
     `state.all_objects_in_zone(Zone::Battlefield)` (`targeting.rs:273`), and
     `stack::is_target_legal` re-checks the zone and, since the Ranger's Guile
     audit added the `CreatureWithFilter` arm (`stack.rs:77-81`), creature-ness.
     This is one of the six cards that audit listed as still carrying the
     redundant preamble; unlike the other five it had no further restriction
     underneath, so the whole override went.
   - Removed; the full workspace is green with it gone.

2. The resolution-time half of "attacking" was untested (`resolution_time_checks.rs`, test added).
   - `rebuke_only_targets_a_creature_that_is_attacking`
     (`cards_removal_and_bounce.rs:198`) checks what `legal_actions` offers and
     stops there. Whether CR 608.2b re-checks the filter — the thing that makes
     removing the override safe — had no Rebuke row.
   - Being an attacker is a combat status (CR 506.4), not a characteristic on
     the object, so this cannot join the parametric
     `a_target_that_stops_qualifying_makes_the_spell_fizzle`: that table stands a
     creature up in a main phase, and this needs a declared attacker.
   - Added `rebuke_fizzles_when_its_target_stops_attacking`. It pulls the
     creature out through `destruction::remove_from_combat` — the engine's own
     CR 506.4 path, used by regeneration — rather than editing `state.combat` by
     hand, and asserts the spell is *countered* (no `SpellResolved` event) rather
     than resolving and finding nothing to destroy.

### Tricky interactions checked
- Blocked attacker: still attacking (CR 506.4 removes a creature from combat
  only on specific events; being blocked is not one), and
  `combat.attackers` still holds it, so it stays a legal target. PASS.
- A *blocker* is not an attacking creature: `attackers.contains_key` is keyed on
  attackers only; `blocker_assignments` is a separate map. PASS.
- A creature that stopped attacking before resolution: countered by game rules.
  PASS — this is the test added above.
- Cast outside combat: `state.combat` is `None`, `is_some_and` is false, no
  legal target, so the spell cannot be cast at all (CR 601.2c). Covered by the
  bystander half of `rebuke_only_targets_a_creature_that_is_attacking`.
- Indestructible: `resolve_destroy` routes to `destruction::try_destroy`, the
  "destroy" pipeline (CR 701.7b) — the oracle text says destroy, not sacrifice.
  PASS.
- Hexproof/protection on the attacker: `can_be_targeted_by` at enumeration and
  `is_target_legal` at resolution; never this card's job. PASS.
- Target dies in response: leaves the battlefield, sole target illegal, spell
  countered (CR 608.2b) in `stack::resolve_spell`. PASS.
- Self-cleanup: `on_resolve` moves nothing; the engine owns the spell
  (CR 608.2m). PASS.

### UI presentation
`TargetFilter::Attacking` renders as `"attacking"` (`cards/mod.rs:325`), so the
prompt reads "attacking creature". No choices beyond the target, nothing to
present.

### Test coverage
- Attacking creature is offered and destroyed: `cards_removal_and_bounce.rs:198`
  (`rebuke_only_targets_a_creature_that_is_attacking`).
- A creature that stayed home is not offered: same test.
- Target stops attacking before resolution → countered: `resolution_time_checks.rs`
  (`rebuke_fizzles_when_its_target_stops_attacking`) — **added this audit**.
- Target leaves the battlefield → countered: covered generically in `fizzle.rs`
  (`a_spell_whose_only_target_became_illegal_is_countered_by_game_rules`).
- Target stops being a creature → not legal: `fizzle.rs:495`
  (`a_target_creature_that_stopped_being_a_creature_is_no_longer_legal`).
- No rulings exist for this card, so there is no per-ruling row to fill.

### Mutations run
| mutation | result |
| --- | --- |
| `is_target_legal`: skip the resolution-time filter re-check | fails `a_target_that_stops_qualifying_makes_the_spell_fizzle` **and** the new Rebuke test (before it was added: only the former) |
| `matches_target_filter`: `TargetFilter::Attacking => true` | fails both Rebuke tests, and nothing else |

Suite after: 1440 passing, exit 0, zero warnings.

