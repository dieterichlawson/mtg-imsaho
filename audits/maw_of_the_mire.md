## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/108/maw-of-the-mire?utm_source=api
**Type line**: `Sorcery` — {4}{B}
**Oracle text**:
```
Destroy target land. You gain 4 life.
```

**Status**: ISSUE

### Code issues
See below.


- The life gain was written out by hand rather than going through
  `GameState::change_life`. Collapsed in the set-wide sweep; see the guard
  `test_suite_guards.rs::only_change_life_writes_a_life_total`.

### Tricky interactions checked
- Ruling: "If the targeted land is an illegal target by the time Maw of the Mire
  resolves, it won't resolve and none of its effects will occur. **You won't
  gain 4 life.**" The life gain is gated on the target still being on the
  battlefield — *not* on the destroy succeeding, so an indestructible land
  survives and you still gain 4, which is what "the spell resolved" means:
  PASS
- `try_destroy`, so indestructible and regeneration apply: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The life gain gated on target legality: `cards_removal.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/108/maw-of-the-mire?utm_source=api
**Type line**: `Sorcery` — {4}{B}
**Oracle text**:
```
Destroy target land. You gain 4 life.
```

**Rulings fetched**:
- [2011-09-22] If the targeted land is an illegal target by the time Maw of the Mire resolves, it won’t resolve and none of its effects will occur. You won’t gain 4 life.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/108/maw-of-the-mire
**Oracle text**:
```
Destroy target land. You gain 4 life.
```
**Type line**: `Sorcery` · **Mana cost**: `{4}{B}`
**Ruling** (2011-09-22, https://api.scryfall.com/cards/90b34a03-3270-412c-90ca-03c1b3e61222/rulings):
"If the targeted land is an illegal target by the time Maw of the Mire resolves, it won't resolve and none of
its effects will occur. You won't gain 4 life."

**Status**: ISSUE (fixed) — two checks that restated engine rules, one of them narrower than what it restated.

### Card data
| field | oracle | `maw_of_the_mire.rs` | |
|---|---|---|---|
| cost | `{4}{B}` | `Generic(4) + Black` | ok |
| types | Sorcery | `vec![CardType::Sorcery]` | ok |
| oracle_text | as above | byte-identical | ok |
| targeting | "target land" | `PermanentWithFilter(HasCardType([Land]))` | ok |

### Code issues

**1. `is_valid_target` restated the target requirement, and got it narrower.** Removed.

- Code was: `state.face_data(obj.id, registry).is_some_and(|d| d.card_types.contains(&CardType::Land))`,
  plus `o.zone == Zone::Battlefield`.
- The requirement already says it: `PermanentWithFilter(HasCardType([Land]))`. `legal_actions` enumerates
  `all_objects_in_zone(Battlefield)` through `matches_target_filter`, whose `HasCardType` arm is
  `state.has_card_type(obj.id, *t, registry)`; and `stack::is_target_legal` re-runs both the zone check
  (`_ => obj.zone == Zone::Battlefield || obj.zone == Zone::Stack`) and the same filter on the way down
  (CR 608.2b).
- The two did not agree. `face_data` is printed types only; `has_card_type` is `card_types_of`, which is the
  object's granted types **union** the active face's. So a permanent that had been granted the land type would
  be offered by the filter and then refused by the card. No card in this set grants a land type, so there was
  no live bug — but two checks answering one question differently is the defect regardless of whether anything
  currently walks into it.

**2. The `on_resolve` zone guard was unreachable.** Removed.

- Code was: `// If the target is illegal (not on battlefield), the spell fizzles — no effects.` followed by an
  early `return`.
- The comment is right about the rule and wrong about who applies it. This spell has one target; if it is
  illegal at resolution, `any_legal` is false and `stack.rs` counters the spell by game rules — `on_resolve` is
  never called. Confirmed by deleting the guard and running the full suite: nothing failed.

Both were also confirmed dead by deleting them independently before writing the replacement tests, so the
removal rests on more than "the suite still passes".

### Rules check
- **The ruling** is the engine countering the spell, not the card returning early. Now tested, and mutation 1
  shows which code enforces it.
- **"Destroy"** goes through `destruction::try_destroy_by`, so indestructible (CR 701.7b) and regeneration
  apply.
- **The two sentences are sequential, not conditional.** The life is gained whether or not the land actually
  died — the spell resolved. Mutation 3 covers the misreading.
- **`controller_of`** (last known information, CR 608.2g) for "you", so the caster gains the life even if
  something removed the Maw from the stack after it began resolving.

### Changes made
- `mtg-engine/src/cards/isd/maw_of_the_mire.rs` — both checks removed; the doc comment now records what applies
  the rule instead, and that the deleted copy was the narrower of the two.
- `mtg-engine/tests/fizzle.rs` — three tests:
  - `a_countered_spells_untargeted_rider_does_not_happen_either` (the ruling). Placed here rather than with the
    card because the card returning early and the spell never resolving look identical from the life total;
    `fizzle.rs` exists for that and its `resolved()` helper watches `GameEvent::SpellResolved`. The table above
    it is all single-effect spells — this is the shape where CR 608.2b bites hardest, a second effect naming no
    target at all that still does not happen.
  - `maw_of_the_mire_gains_the_life_when_it_does_resolve` — the control, so an engine that never gained the
    life would not pass the ruling test.
  - `maw_of_the_mire_gains_the_life_even_when_the_land_survives` — indestructible.
- `mtg-engine/tests/cards_removal_and_bounce.rs` — a `("Maw of the Mire", Land, Creature, "'target land'")` row
  in the removal-target table, covering both halves of what it may point at.

### Mutation checks (all discriminating)
1. `is_target_legal`'s zone arm → `true` (the engine stops noticing the target left) →
   `a_countered_spells_untargeted_rider_does_not_happen_either` FAILED with three other fizzle tests. This is
   the one that matters: it shows the ruling is enforced where I claim it is, and that the guard I deleted was
   not what was holding it up.
2. `change_life(controller, 4)` → `0` → `maw_of_the_mire_destroys_land_and_gains_life` FAILED.
3. Life gain made conditional on `DestroyResult::Died` →
   `maw_of_the_mire_gains_the_life_even_when_the_land_survives` FAILED.
4. Filter widened to `TargetFilter::Any` → `targeted_removal_offers_the_targets_its_text_allows` FAILED.

### Tricky interactions checked
- Target land leaves in response → spell countered, no life: **pass** (new).
- Indestructible land → survives, life still gained: **pass** (new).
- Cannot target a creature: **pass** (new table row).
- Can target an opponent's land as well as your own — "target land" has no controller restriction, and the
  filter carries none: **pass** by construction; the existing test targets P1's Forest.

### Test coverage
- destroys a land and gains 4 life: `cards_lands_and_mana_sources.rs:433`
- the ruling — countered, no life gained: `fizzle.rs:83` (new)
- resolving control: `fizzle.rs:106` (new)
- indestructible land, life still gained: `fizzle.rs:127` (new)
- "target land" both ways: `cards_removal_and_bounce.rs:154` table (new row)

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1407 passing.

