## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/33/smite-the-monstrous?utm_source=api
**Type line**: `Instant` — {3}{W}
**Oracle text**:
```
Destroy target creature with power 4 or greater.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "power **4 or greater**" read through `effective_power`, so a pumped 2/2 is a
  legal target and a debuffed 5/5 is not: PASS
- The check runs again on resolution: shrinking the creature in response makes
  it fizzle (CR 608.2b): PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The power threshold at cast and at resolution: `cards_removal.rs`, `resolution_time_checks.rs:a_target_that_stops_qualifying_makes_the_spell_fizzle`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/33/smite-the-monstrous?utm_source=api
**Type line**: `Instant` — {3}{W}
**Oracle text**:
```
Destroy target creature with power 4 or greater.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/33/smite-the-monstrous
**Oracle text**:
```
Destroy target creature with power 4 or greater.
```
**Type line**: `Instant` · **Mana cost**: `{3}{W}`
**Rulings**: none published for this card.
**Status**: ISSUE (fixed) — a restated target requirement, and a filter whose "power" nothing tested as
*current* power.

### Card data
| field | oracle | `smite_the_monstrous.rs` | |
|---|---|---|---|
| cost | `{3}{W}` | `Generic(3) + White` | ok |
| types | Instant | `vec![CardType::Instant]` | ok |
| oracle_text | as above | byte-identical | ok |
| targeting | "creature with power 4 or greater" | `CreatureWithFilter(PowerAtLeast(4))` | ok |
| effect | destroy | `helpers::resolve_destroy` → `destruction::try_destroy` | ok |

### Code issues

**1. `is_valid_target` restated the target requirement.** Removed.

It checked zone, `is_creature`, and `state.effective_power(..) >= 4` — the last being the identical call the
filter's `PowerAtLeast` arm makes. The creature-ness half stopped being the card's job during the Ranger's Guile
audit, when `stack::is_target_legal` gained its own `is_creature` re-check for `CreatureWithFilter`. Confirmed
dead by deleting it and running the whole suite.

**2. Nothing tested that "power" means power *now*.** Fixed.

- Target table (`cards_removal_and_bounce.rs:157`): a printed 5/5 legal, a printed 2/2 illegal.
- Resolution-time re-check (`resolution_time_checks.rs:273`): shrinks the target by writing
  `state.get_object_mut(id).unwrap().power = Some(2)` — the *printed* value.

Every case is a printed number, so an implementation reading `obj.power` instead of `effective_power` passes all
of them. That is the exact anti-pattern the audit procedure lists.

### Rules check
- **CR 608.2b** — a creature shrunk below 4 in response stops being a legal target and the spell is countered by
  game rules. Already covered at `resolution_time_checks.rs:266`, and it works because `is_target_legal` re-runs
  the filter.
- **"Destroy"** through `destruction::try_destroy`, so indestructible and regeneration apply.
- **Effective power** includes counters, `ModifyPT` effects and anything else in the layers — which is what
  makes a Gavony Township counter or a Ranger's Guile pump turn a 3/3 into a legal target.

### Changes made
- `mtg-engine/src/cards/isd/smite_the_monstrous.rs` — `is_valid_target` removed, with a comment naming what
  applies the rule.
- `mtg-engine/tests/cards_removal_and_bounce.rs` — `smite_the_monstrous_reads_power_as_it_is_now`, in both
  directions: a printed 3/3 buffed to 4 is offered and destroyed; a printed 5/5 shrunk to 3 is not offered.
  Either direction alone is explained by the other reading, so both are asserted.

### Mutation checks (both discriminating, and both **only** against the new test)
1. `PowerAtLeast` reading `obj.power` instead of `state.effective_power(..)` →
   `smite_the_monstrous_reads_power_as_it_is_now` FAILED, and nothing else did. That is the measure of the gap.
2. Threshold changed from 4 to 3 → the same single test FAILED. Worth noting why nothing else caught it: the
   table's 2/2 is still illegal at a threshold of 3 and its 5/5 still legal, so only the shrunk-to-3 case
   separates them.

### Tricky interactions checked
- Printed 5/5 legal, printed 2/2 illegal: **pass** (`cards_removal_and_bounce.rs:157`).
- Buffed to 4 → legal: **pass** (new).
- Debuffed to 3 → illegal: **pass** (new).
- Shrunk below 4 after being targeted → spell countered, not resolved into nothing: **pass**
  (`resolution_time_checks.rs:266`, which asserts on `SpellResolved` rather than on the battlefield).
- Indestructible: covered at the pipeline level by `bramblecrush_respects_indestructible`; same
  `resolve_destroy` call.

### Test coverage
- what it may and may not target: `cards_removal_and_bounce.rs:157`
- power as it is now, both directions: `cards_removal_and_bounce.rs:322` (new)
- stops qualifying before resolution: `resolution_time_checks.rs:266`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1439 passing.

