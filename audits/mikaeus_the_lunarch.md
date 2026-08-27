## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/moc/197/mikaeus-the-lunarch?utm_source=api
**Type line**: `Legendary Creature — Human Cleric` — {X}{W}, 0/0
**Oracle text**:
```
Mikaeus enters with X +1/+1 counters on it.
{T}: Put a +1/+1 counter on Mikaeus.
{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.
```

**Status**: ISSUE

### Code issues
See below.


- It arrived as a 0/0 and got its counters afterwards.
  - Oracle text says: `Mikaeus enters with X +1/+1 counters on it.`
  - Code did: `state.move_object(object_id, Zone::Battlefield, registry);` then
    `state.add_counters(object_id, CounterType::PlusOnePlusOne, x);`
  - CR 614.1c makes that a replacement effect, applied as the permanent enters.
    `move_object` emits `EnteredBattlefield` as part of the move, so every ETB
    watcher saw a 0/0 Mikaeus — and a 0/0 creature is one state-based-action
    check from the graveyard (CR 704.5f). Converted to `replace_event` with the
    `enters_with_counters` helper the set's other "enters with counters" cards
    already use. The same override also re-did the trait default's "move a
    permanent to the battlefield" and its `is_legendary` stamping; a guard now
    fails the build on a card that moves itself.

### Tricky interactions checked
- "{X}{W}" with X read from the object's `x_value`, so X = 0 makes a 0/0 that
  dies immediately — correct, not a special case: PASS
- "{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on **each
  other** creature you control" — the removal is a cost, and Mikaeus excludes
  itself from the beneficiaries: PASS
- Legendary, so the legend rule applies: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Entering with X counters, and both activated abilities: `cards_complex_creatures.rs`, `counter_costs.rs`
