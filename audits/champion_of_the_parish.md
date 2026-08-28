## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/6/champion-of-the-parish?utm_source=api
**Type line**: `Creature — Human Soldier` — {W}, 1/1
**Oracle text**:
```
Whenever another Human you control enters, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**another** Human you control enters" — the engine's `AnyCreatureEnters`
  collector excludes the entering permanent from the watcher list, so the
  Champion never sees its own arrival; the entering permanent gets `SelfEntered`
  instead: PASS
- "you control": PASS
- `has_subtype` reads the ACTIVE face, so a transformed Werewolf no longer counts
  as a Human — a hand-rolled `registry.card_data` check would always have read
  the front face: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The subtype and controller filters: `subtype.rs`, `cards_complex_creatures.rs`

## Audit — 2026-08-28 17:33

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Champion of the Parish"`, https://scryfall.com/card/isd/6/champion-of-the-parish
**Oracle text**:
```
Whenever another Human you control enters, put a +1/+1 counter on this creature.
```
**Type line**: Creature — Human Soldier
**Mana cost**: {W}   **P/T**: 1/1
**Rulings**: none on Scryfall for this card.
**Status**: ISSUE (fixed)

### Code issues

**The condition was read on resolution instead of as the creature entered.**

- Oracle text says: `Whenever another Human you control enters, put a +1/+1 counter on this
  creature.`
- Code did: `on_any_creature_enters` — the *resolution* hook — opened with
  `if entered_controller != controller { return; }` and
  `if state.has_subtype(entered_id, "Human", registry) { ... }`, and the card left
  `should_trigger_on_creature_enters` at its default of `true`.

CR 603.2: a condition on the event is part of whether the ability triggers at all. Reading it
late was wrong in both directions:

- Every creature entering the battlefield under any player's control put a Champion trigger on
  the stack, which then did nothing. A trigger that should not have triggered is not free —
  it is a stack object with a priority window around it. (This is the same objection the
  damage collector's own comment makes for Rakish Heir.)
- A Human that stopped being one before the trigger resolved lost the Champion its counter.
  Moonmist is an instant and reads "transform all Human creatures", so casting it in response
  to this trigger — with a Village Ironsmith, a Human Werewolf, as the creature that entered —
  is a real line of play.

The engine already had the hook and the collector already calls it; Mentor of the Meek, the
set's only other `AnyCreatureEnters` watcher, was already using it. The condition moved there,
and the resolution hook is now the counter alone — `add_counters` being where CR 121.1 says a
Champion that has left is not there to take one, so the card's own `still_on_battlefield`
preamble went with it.

Card data is otherwise correct: `{W}`, `Creature`, `subtypes: ["Human", "Soldier"]`, 1/1,
oracle text verbatim, `TriggerKind::AnyCreatureEnters` matching the implemented hook.

### Tricky interactions checked
- **"Another"**: PASS, and it is the collector's — the ETB-watch scan filters `o.id != *object`,
  so a permanent never sees its own arrival. The card correctly does not repeat it.
- **A Human entering under an opponent's control**: PASS.
- **A transformed werewolf is not a Human**: PASS. `has_subtype` reads the active face;
  a check through `registry.card_data` would always read the front one.
- **A Human that transforms in response**: PASS after the fix.
- **A token Human**: PASS — `has_subtype` reads `obj.subtypes` before the registry.
- **Several Humans entering at once**: each is its own event and its own trigger. Not tested.
- **The Champion leaving before the trigger resolves**: no counter, via `add_counters`
  (CR 121.1). Nothing observable distinguishes it from the counter landing on a dead object,
  so not tested at this card.

### Test coverage
- a Human you control enters: `cards_combat_damage_triggers.rs:234 champion_of_the_parish_counter_on_human_etb`
- a non-Human does not: `cards_combat_damage_triggers.rs:253`
- an opponent's Human does not: `cards_combat_damage_triggers.rs:273`
- its own arrival does not: `cards_combat_damage_triggers.rs:294 champion_of_the_parish_does_not_count_its_own_arrival` (NEW)
- nothing reaches the stack for a creature it does not care about:
  `cards_combat_damage_triggers.rs:311 champion_of_the_parish_puts_nothing_on_the_stack_for_a_creature_it_does_not_care_about` (NEW)
- a Human transformed in response still earns the counter:
  `cards_combat_damage_triggers.rs:335 champion_of_the_parish_keeps_its_counter_when_the_human_stops_being_one` (NEW)

Mutation-checked, each killing exactly its own test: putting the condition back at resolution
kills the stack test and the transform test; dropping "you control" kills the opponent test;
dropping the Human check kills the non-Human test; removing the collector's `o.id != *object`
kills the own-arrival test. That last one is honest about where the rule lives — the
own-arrival test is falsifiable only through the collector, because that is the only place
"another" is implemented.

The three original tests count counters, and counting counters cannot tell a trigger that never
happened from one that resolved and did nothing — which is why all three passed under the old
implementation and still do.

### Changes made
- `champion_of_the_parish.rs`: condition moved to `should_trigger_on_creature_enters`;
  resolution hook reduced to the counter; redundant battlefield preamble dropped.
- `cards_combat_damage_triggers.rs`: three new tests.
