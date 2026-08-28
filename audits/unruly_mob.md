## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/39/unruly-mob?utm_source=api
**Type line**: `Creature — Human` — {1}{W}, 1/1
**Oracle text**:
```
Whenever another creature you control dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **another** creature **you control** dies" — self-excluded by the
  trigger kind, controller-filtered in the handler.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/39/unruly-mob?utm_source=api
**Type line**: `Creature — Human` — {1}{W}, 1/1
**Oracle text**:
```
Whenever another creature you control dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **another** creature **you control** dies" — both the self-exclusion
  and the controller check: PASS
- It counts tokens dying, since a token is a creature: PASS
- CR 603.6d: the Mob dying alongside another creature still gets its trigger,
  though the counter lands on a creature that is gone: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The death trigger: `cards_morbid_and_ltb.rs`

## Audit — 2026-08-28 17:56

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Unruly Mob"`, https://scryfall.com/card/isd/39/unruly-mob
**Oracle text**:
```
Whenever another creature you control dies, put a +1/+1 counter on this creature.
```
**Type line**: Creature — Human
**Mana cost**: {1}{W}   **P/T**: 1/1
**Rulings** (the same text dated 2016-04-08 and 2021-09-24): "If Unruly Mob and another creature
you control die simultaneously (perhaps because they were both attacking or blocking), Unruly
Mob won't be on the battlefield as its triggered ability resolves. It can't be saved by the
+1/+1 counter that would have been put on it."
**Status**: ISSUE (fixed; the fix turned out to span five cards)

### Code issues

**The condition was read on resolution instead of as the creature died.**

- Oracle text says: `Whenever another creature you control dies, put a +1/+1 counter on this
  creature.`
- Code did: `on_any_creature_dies` — the *resolution* hook — opened with
  `if !crate::cards::helpers::still_on_battlefield(state, self_id) { return; }` and
  `if dead_controller == controller { state.add_counters(...) }`, and the card left its
  trigger condition to the default.

CR 603.2 makes "another creature **you control** dies" part of whether the ability triggers at
all. Deciding late put a Mob trigger on the stack for every creature that died anywhere,
including an opponent's, which then did nothing — a stack object with a priority window around
it.

Unlike the ETB-watch scan (`should_trigger_on_creature_enters`) and the damage collector, the
death-watch scan had no hook to ask. **Five cards in the set were in the same position**, so
the fix is the hook plus its five callers — see the three preceding commits:

| card | condition that moved |
|---|---|
| Unruly Mob | "another creature **you control**" |
| Thraben Sentry | "another creature **you control**", and the front face |
| Village Cannibals | "another **Human**" |
| Gutter Grime | "a **nontoken** creature **you control**" |
| Abattoir Ghoul | "a creature **dealt damage by this creature this turn**" |

**A second copy of CR 121.1.** The `still_on_battlefield` preamble is what `add_counters`
already enforces for every card. Removed here, and from Village Cannibals and Lumberknot in the
same pass.

Card data is otherwise correct: `{1}{W}`, `Creature`, `subtypes: ["Human"]`, 1/1, oracle text
verbatim, `TriggerKind::AnyCreatureDies` matching the implemented hook.

### Tricky interactions checked
- **"Another"**: PASS, and it is the collector's — the death-watch scan filters `o.id != dead_id`,
  so a permanent never sees its own death.
- **"You control"**, read as the creature dies: PASS after the fix. `dead_controller` comes from
  the death event, so it is the controller at the moment of death (CR 608.2g) rather than the
  owner the field resets to.
- **The ruling — the Mob dying alongside the creature**: PASS, and both halves. The ability
  triggers (the Mob was on the battlefield when the other creature died, CR 603.10a) and
  resolves doing nothing (CR 121.1). It is not saved.
- **A token dying**: counts. "Creature", not "nontoken creature" — that is Gutter Grime's word,
  and the two now differ where they should.
- **The Mob changing controller between the death and the resolution**: the condition is settled
  at trigger time, so it does not matter. Not tested — nothing in this pool changes control at
  instant speed in that window.

### Test coverage
- another creature you control dying gives a counter:
  `cards_death_triggers_and_tokens.rs:484 a_death_watcher_counts_the_deaths_its_text_names` (row)
- an opponent's creature dying does not:
  same table, NEW row — the table's own comment says the rows come in matched pairs, and Unruly
  Mob was the one without a pair
- neither death reaches the stack unless it qualifies:
  `cards_death_triggers_and_tokens.rs:512 a_death_watcher_does_not_trigger_on_a_death_its_text_does_not_name` (NEW,
  thirteen rows across all five conditioned watchers plus two unconditioned ones)
- the ruling, both halves:
  `cards_death_triggers_and_tokens.rs:~600 unruly_mob_is_not_saved_by_its_own_counter` (NEW)
- it fires alongside another trigger under APNAP: `apnap.rs:155`

Mutation-checked: each of the five cards' conditions forced to `true` in turn fails the new
stack-counting table and only it; the collector ignoring the hook fails both tables; and
removing `add_counters`' CR 121.1 guard fails the ruling test.

The old table counts counters, and a trigger that never happened leaves the same board as one
that resolved and did nothing — which is why it passed against all five cards deciding late,
and still does.

### Changes made
- `unruly_mob.rs`: condition moved to `should_trigger_on_creature_dies`; CR 121.1 preamble dropped.
- `cards/mod.rs`, `triggers/collect/zones.rs`: the new hook and the collector calling it.
- `thraben_sentry.rs`, `village_cannibals.rs`, `gutter_grime.rs`, `abattoir_ghoul.rs`,
  `lumberknot.rs`: the same treatment.
- `cards_death_triggers_and_tokens.rs`: one new row, one new table, one new ruling test.
