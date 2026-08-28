## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/125/village-cannibals?utm_source=api
**Type line**: `Creature — Human` — {2}{B}, 2/2
**Oracle text**:
```
Whenever another Human creature dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **another Human** creature dies" — self-exclusion matters here
  because Village Cannibals is itself `Creature — Human`, and it comes from the
  trigger kind rather than a hand-written id check. The Human test goes through
  `state.has_subtype`, so a token Human or a granted type counts.
- No controller filter, correctly: the wording says any Human, not yours.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/125/village-cannibals?utm_source=api
**Type line**: `Creature — Human` — {2}{B}, 2/2
**Oracle text**:
```
Whenever another Human creature dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever another **Human** creature dies" — the Human filter *and* the
  self-exclusion, and note there is **no** "you control": an opponent's Human
  dying feeds it too: PASS
- `has_subtype` reads the active face, so a transformed Werewolf that is no
  longer Human does not count: PASS
- It counts Human tokens: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Human death filter: `cards_morbid_and_ltb.rs`, `subtype.rs`

## Audit — 2026-08-28 18:09

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Village Cannibals"`, https://scryfall.com/card/isd/125/village-cannibals
**Oracle text**:
```
Whenever another Human creature dies, put a +1/+1 counter on this creature.
```
**Type line**: Creature — Human
**Mana cost**: {2}{B}   **P/T**: 2/2
**Rulings**: none on Scryfall for this card.
**Status**: ISSUE (fixed — a real, reachable wrong answer)

### Code issues

**"Human" was read off the object after it had stopped being able to say.**

- Oracle text says: `Whenever another Human creature dies, put a +1/+1 counter on this creature.`
- Code did: `state.has_subtype(dead_id, "Human", registry)`.

`has_subtype` reads the object's active face. By the time anything asks, the object has been
through `move_object`, which clears `is_transformed` (CR 400.7) — so a werewolf that died **as
a Werewolf** reads back as the Human on its front face. Verified before fixing: a transformed
Village Ironsmith killed with a Village Cannibals out produced a +1/+1 counter it had not
earned. Every werewolf in the set is a Human on its front, and Moonmist and Full Moon's Rise
transform them at instant speed, so this is a line of play rather than a curiosity.

A token would have been wrong in the other direction — SBA 704.5d removes it from
`state.objects` before the collector runs, so a Human token dying would have read as nothing —
but nothing in this pool makes a Human token, so that half is unreachable here.

Fixed by carrying the subtypes in the death event, beside the controller, damage record,
toughness and tokenness already there for exactly this reason (CR 608.2g). See the preceding
commit. `sba.rs`'s zero-toughness path had its own hand-rolled copy of that capture and now
calls `destruction::death_event`.

Everything else is correct: `{2}{B}`, `Creature`, `subtypes: ["Human"]`, 2/2, oracle text
verbatim, `TriggerKind::AnyCreatureDies` matching the hook. The condition moved to
`should_trigger_on_creature_dies` earlier in this pass (Unruly Mob's audit), and the card's own
copy of CR 121.1 went with it.

### Tricky interactions checked
- **"Another"**: PASS, and it is the collector's — a permanent never sees its own death.
- **A Zombie dying is not a Human dying**: PASS.
- **A werewolf dying on its back face**: PASS after the fix; it was the bug.
- **A werewolf dying on its front face**: PASS — still a Human, still counts. This is the half a
  fix that simply stopped counting werewolves would break, so it is tested too.
- **"Human creature", not "Human card"**: the event only fires for creatures, so the creature
  half is the collector's.
- **A Human an opponent controls**: counts. The card says "another Human creature", not "you
  control" — contrast Unruly Mob, and the two now differ where they should.
- **A Human token**: would count. Unreachable in this pool; the fix covers it anyway.
- **The Cannibals dying alongside**: triggers, and the counter lands nowhere (CR 121.1).

### Test coverage
- a Human dying gives a counter; a Zombie does not:
  `cards_death_triggers_and_tokens.rs:527-528 a_death_watcher_counts_the_deaths_its_text_names` (matched pair)
- neither reaches the stack unless it qualifies:
  `cards_death_triggers_and_tokens.rs:580-581 a_death_watcher_does_not_trigger_on_a_death_its_text_does_not_name`
- which face the werewolf died on, both ways:
  `cards_death_triggers_and_tokens.rs:~640 village_cannibals_reads_the_face_the_werewolf_died_on` (NEW)
- the live subtype layer follows a transform: `subtype.rs:661` — whose doc anticipated exactly
  this failure ("Village Cannibals' 'Human dies' trigger then fire on a creature whose live face
  is an Insect") and checked the layer, not the death path

Mutation-checked: putting `has_subtype(dead_id, ..)` back — the exact code that was there —
fails the new test and only it. The two table tests pass either way, because Doomed Traveler
and Walking Corpse have one face each.

### Changes made
- `events.rs`, `destruction.rs`, `sba.rs`, `triggers.rs`, `triggers/collect/zones.rs`,
  `cards/mod.rs`: subtypes carried in the death event and passed to the hook.
- `village_cannibals.rs`: reads them.
- `cards_death_triggers_and_tokens.rs`: the werewolf test.
