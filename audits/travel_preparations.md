## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/206/travel-preparations?utm_source=api
**Type line**: `Sorcery` — {1}{G}
**Oracle text**:
```
Put a +1/+1 counter on each of up to two target creatures.
Flashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**up to two** target creatures" — `UpToTargets(2, Creature)`, so it is
  castable with zero, one or two targets: PASS
- "each of" — one counter on each, not two on one: PASS
- Flashback {1}{W} is a different colour from the front cost {1}{G}, and the
  card is exiled after the flashback resolution (CR 702.33a): PASS
- One of two targets becoming illegal leaves the other still getting its counter
  (CR 608.2b): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Up-to targeting and the counters: `cards_flashback.rs`, `multi_target_and_mill.rs`
- Flashback exile: `cards_flashback.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/206/travel-preparations?utm_source=api
**Type line**: `Sorcery` — {1}{G}
**Oracle text**:
```
Put a +1/+1 counter on each of up to two target creatures.
Flashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2018-12-07] If Travel Preparations targets two creatures, and one of them is an illegal target by the time Travel Preparations resolves, you'll still put a +1/+1 counter on the other creature.
- [2018-12-07] You can't target the same creature twice to put two +1/+1 counters on it.

**Status**: ISSUE (fixed)

### Code issues

**One, in the engine, and it was live in the shipped clients: a spell could be
handed the same target twice.**

- Ruling (2018-12-07): `You can't target the same creature twice to put two +1/+1 counters on it.`
- CR 601.2c: the same target can't be chosen multiple times for one instance of
  the word "target", and "each of **up to two target creatures**" is one
  instance.

`generate_cast_actions_with_targets` honours this — it enumerates
`target_combinations`, which are combinations — so no *offered* cast names a
creature twice. But neither client picks a whole offered action. Both
`mtg-player`'s CLI and its LLM driver assemble their own `Action::CastSpell`
from a per-slot choice, and the LLM path is:

```rust
response["target_indices"].as_array().map(|arr| arr.iter()
    .filter_map(|v| v.as_u64().map(...))
    .filter(|&i| i < options.len())
    .take(*max)
    .map(|i| options[i].clone())
    .collect())
```

— no distinctness check, and the JSON schema sets `maxItems` but not
`uniqueItems`. A model answering `[0, 0]` produced `targets: [c, c]`, and
`cast_spell` stored the list verbatim. I confirmed the consequence: two +1/+1
counters on one creature, which is precisely what the ruling forbids.

Fixed in the engine rather than in the clients, because the engine is the
authority for a declaration it is handed — the stance `declare_attackers`
already takes ("The engine is the authority; it does not trust the submitted
list"). `targeting::distinct_within_each_target_instance` drops a repeat
within one instance of "target", and `cast_spell` runs the submitted list
through it before charging for or storing anything.

Only `UpToTargets` is one instance covering many slots. `TwoTargets` is two
separate instances, which CR 601.2c does not join, so each half is checked on
its own — see below.

### Card data

`{1}{G}` Sorcery, flashback `{1}{W}`,
`TargetRequirement::UpToTargets(2, Creature)` for "up to two target
creatures", one `+1/+1` counter per surviving target. All matching, with cost,
type line and the two-colour flashback cost pinned pool-wide by
`card_data_invariants.rs`, and the graveyard cast covered by the flashback
sweep. `add_counters` rather than a hand-written counter map, so the counter
goes through the one pipeline. The redundant `zone == Battlefield` gate is
kept, as elsewhere.

### Tricky interactions checked

- Ruling 1, the same creature twice: **was two counters, fixed**.
- Ruling 2, two targets and one illegal on resolution — the other still gets
  its counter (CR 608.2b): pass, and newly tested for this card. The engine
  substitutes `Target::Illegal` in place, so the loop skips it and keeps the
  order.
- Zero targets: "up to two" permits it and the spell resolves doing nothing.
- The counter is a real +1/+1 counter, so it survives anything that reads
  counters rather than a P/T modifier.
- The six generic flashback rulings are engine-level.

### Recorded, not fixed

**The `TwoTargets` half of the new helper is not falsifiable in this pool.**
Treating `TwoTargets` as a single instance — the over-broad reading — passes
the entire suite, because no card here has two slots that could accept the
same object: Prey Upon's are "creature you control" and "creature you don't
control", Into the Maw of Hell's are a land and a creature with nothing in the
set that is both, and Memory's Journey's first slot is a player. The recursion
is the correct reading of CR 601.2c and is written that way deliberately, but
only the `UpToTargets` arm is under test, and this entry says so rather than
letting the mutation result look like coverage.

### Test coverage

- one counter on one target: `flashback.rs::travel_preparations_adds_counter`
- ruling 1, the same creature twice — checked both as an offer and as a
  submitted list:
  `flashback.rs::travel_preparations_cannot_target_the_same_creature_twice` (new)
- ruling 2, one of two targets gone by resolution:
  `flashback.rs::travel_preparations_counters_the_creature_that_is_still_there` (new)
- flashback reachable from the graveyard:
  `flashback.rs::every_flashback_card_is_offered_from_the_graveyard`

### Mutations run

- Drop the dedup from `cast_spell`: **fails** the duplicate-target test on its
  submitted-list assertion, and nothing else — the offer half was already
  right.
- Treat `TwoTargets` as one instance: passes the whole suite. Recorded above
  rather than counted as coverage.
- The card gives a counter to only its first target: **fails** the
  partial-fizzle test, passes the other two.

Suite: 1527 passing, exit 0, `cargo check --workspace --all-targets` clean.
