## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/85/abattoir-ghoul?utm_source=api
**Type line**: `Creature — Zombie` — {3}{B}, 3/2
**Oracle text**:
```
First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever a creature **dealt damage by this creature this turn** dies, you gain
  life equal to that creature's **toughness**" — reads the captured
  `dead_damaged_by` and `dead_toughness`, which is last-known information
  (CR 603.6d, cited in the code). A dead creature's toughness cannot be read off
  the object afterwards.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/85/abattoir-ghoul?utm_source=api
**Type line**: `Creature — Zombie` — {3}{B}, 3/2
**Oracle text**:
```
First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "You'll gain life equal to the creature's **last known toughness before
  it died**. For example, if Abattoir Ghoul deals 3 first-strike damage to a 7/7
  creature and then you give the creature -5/-5 before the regular combat damage
  step, you'll gain 2 life." Both the toughness *and* the `damaged_by` list are
  captured before the zone change clears them (CR 608.2g): PASS
- "a creature **dealt damage by this creature this turn**" — the check is
  `dead_damaged_by.contains(&self_id)`, so a creature that died to something else
  gives nothing: PASS
- CR 603.6d: the trigger resolves even if the Ghoul died in the same combat
  damage step: PASS
- The life gain goes through `change_life`: PASS
- A negative last-known toughness gains 0, not negative life: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Last-known toughness and the damaged-by check: `cards_morbid_and_ltb.rs`, `combat_rules.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/85/abattoir-ghoul?utm_source=api
**Type line**: `Creature — Zombie` — {3}{B}, 3/2
**Oracle text**:
```
First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
```

**Rulings fetched**:
- [2011-09-22] You'll gain life equal to the creature's last known toughness before it died. For example, if Abattoir Ghoul deals 3 first-strike damage to a 7/7 creature and then you give the creature -5/-5 before the regular combat damage step, you'll gain 2 life.

**Status**: ISSUE (fixed)

### Code issues

**Two, both about what "dealt damage by this creature **this turn**" is a
record of, and both in the engine.**

`damaged_by` is the list of objects that dealt damage to a permanent this
turn. It is the whole of this card's condition.

**1. Regenerating erased the record.**

```rust
fn regenerate(state: &mut GameState, id: ObjectId) {
    state.tap(id);
    if let Some(obj) = state.get_object_mut(id) {
        obj.damage_marked = 0;
        obj.dealt_deathtouch_damage = false; obj.damaged_by.clear();
```

- Oracle text says: `Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.`
- CR 701.15a says regenerating "removes all damage marked on it" — it does not
  un-deal that damage.

A creature the Ghoul damaged, that regenerated and then died later the same
turn, gained nothing. Five cards in this set regenerate (Skeletal Grimace,
Full Moon's Rise, Manor Skeleton, Ulvenwald Mystics, Grimgrin), so the board
is ordinary. `damage_marked` and `dealt_deathtouch_damage` still go — the
second is a property of the marked damage, and SBA 704.5h must not destroy the
creature again for damage that is no longer there — but `damaged_by` stays.

**2. Cleanup only visited permanents with damage still marked.**

```rust
let damaged: Vec<ObjectId> = state.all_objects_in_zone(Zone::Battlefield)
    .iter()
    .filter(|o| o.damage_marked > 0)
```

Which is why (1) was there: a regenerated creature has no marked damage, so
cleanup never reached it, and without `regenerate` clearing the record it
would have carried "the Ghoul damaged me" into the next turn and paid out
there. Fixing one without the other trades a missed trigger for a wrong one.

CR 514.2 removes all damage; the turn's record of who dealt it ends with the
turn for the same reason. Cleanup now visits every permanent. That also covers
a planeswalker, whose damage removes loyalty rather than marking damage
(CR 120.3c) while still recording `damaged_by` — Into the Maw of Hell reads
that list too.

### Card data

`{3}{B}` Creature — Zombie, 3/2, First strike, one `AnyCreatureDies` trigger
matching the one implemented hook. All pinned pool-wide by
`card_data_invariants.rs`. The hook reads the trigger's captured
`dead_damaged_by` and `dead_toughness` rather than the dead object, which is
right: by the time it resolves the creature has changed zones and
`move_object` has cleared both.

`toughness.max(0)` and the `> 0` guard: a creature that died with 0 or less
toughness gains nothing, and `change_life` is the one life pipeline.

### Tricky interactions checked

- The ruling — "you'll gain life equal to the creature's last known toughness
  before it died", with its own example of a 7/7 shrunk to 2/2: pass, and the
  ruling's actual case is now tested. The existing test only *added* a +1/+1
  counter, so a reading that took printed toughness was wrong in the same
  direction as one that took the base.
- Regenerated in between: **was broken, fixed**.
- Damaged on an earlier turn: pass, and now tested in both forms — with damage
  still marked (which the old cleanup filter caught) and after a regeneration
  (which it did not).
- The Ghoul dying at the same time as its victim: the trigger resolves anyway,
  and `controller_of` answers with last known information (CR 608.2g). The
  handler reads nothing else off the Ghoul.
- "a creature", not "another": the Ghoul cannot be in its own `damaged_by`, so
  its own death gains nothing either way.
- First strike is what makes this reachable in combat at all — the Ghoul
  damages a blocker in the first-strike step and the blocker dies in the
  regular one.

### Test coverage

- gains life from a creature it damaged:
  `cards_combat_damage_triggers.rs::abattoir_ghoul_gains_life_from_damaged_creature_death`
- gains nothing from a creature it did not damage:
  `…::abattoir_ghoul_no_life_if_not_damaged_by_ghoul`
- last known toughness, with a counter: `…::abattoir_ghoul_uses_last_known_toughness_with_counters`
- the ruling's own example, a creature shrunk before it dies:
  `…::abattoir_ghoul_gains_the_toughness_the_creature_died_with` (new)
- damaged, regenerated, dead the same turn:
  `…::abattoir_ghoul_still_gains_life_after_the_victim_regenerated` (new)
- damaged on an earlier turn: `…::abattoir_ghoul_gains_nothing_from_a_creature_damaged_on_an_earlier_turn` (new)
- damaged, regenerated, dead the *next* turn:
  `…::abattoir_ghouls_record_of_a_regenerated_creature_still_ends_with_the_turn` (new)

The three new damage tests go through `damage::deal_damage` rather than
pushing `damaged_by` by hand, which the three existing ones do.

### Mutations run

- `regenerate` clears `damaged_by` again: **fails** the regenerated-same-turn
  test, and only that one.
- Cleanup filters on `damage_marked > 0` again: **fails** the
  regenerated-then-next-turn test, and only that one. The two halves are
  pinned separately, which is the point — either alone leaves a bug.
- The card drops its `dead_damaged_by.contains(&self_id)` check: **fails**
  three tests.
- `death_event` takes the printed toughness instead of the effective one:
  **fails** both toughness tests. (An earlier attempt at this mutation edited
  `sba.rs`'s zero-toughness capture and changed nothing — these deaths are
  lethal damage, CR 704.5g, which goes through `destruction.rs`. Redone
  there.)

Suite: 1531 passing, exit 0, `cargo check --workspace --all-targets` clean.
