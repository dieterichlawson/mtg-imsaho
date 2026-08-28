## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/194/moldgraf-monstrosity?utm_source=api
**Type line**: `Creature — Insect` — {4}{G}{G}{G}, 8/8
**Oracle text**:
```
Trample
When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
```

**Status**: PASS

### Code issues
No issues found.

- "When this creature dies, **exile it**, then return two creature **cards** at
  random from your graveyard to the battlefield" — the exile applies to the card
  in the graveyard and only there, and the code comments the ordering hazard:
  two Monstrosities dying together each put a trigger on the stack, and the first
  can return the second.
- "your graveyard" uses the last-known **controller**, not the owner (CR 603.10c),
  which matters for a stolen Monstrosity.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_source_independence.rs` (a dies trigger resolving after its source is gone).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/194/moldgraf-monstrosity?utm_source=api
**Type line**: `Creature — Insect` — {4}{G}{G}{G}, 8/8
**Oracle text**:
```
Trample
When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature dies, **exile it**, then return two creature cards **at
  random** from your graveyard to the battlefield" — the exile is of itself, and
  the return is random rather than chosen: PASS
- CR 608.2: an ability does as much as it can — the return still happens if the
  Monstrosity was already exiled from the graveyard in response: PASS
- Two Monstrosities dying together: the first trigger can return the second, and
  the second trigger must then leave it on the battlefield rather than exiling a
  live creature: PASS
- CR 109.1: "two creature **cards** at random", so a token in the graveyard is
  not a candidate: PASS
- Trample: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both orderings and the token exclusion: `resolution_time_checks.rs:moldgraf_simultaneous_death_second_trigger_does_not_exile_live_creature`, `:moldgraf_exile_skipped_when_already_exiled_still_returns_creatures`, `token_is_not_a_card.rs:moldgraf_does_not_return_a_token_from_the_graveyard`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/194/moldgraf-monstrosity?utm_source=api
**Type line**: `Creature — Insect` — {4}{G}{G}{G}, 8/8
**Oracle text**:
```
Trample
When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
```

**Rulings fetched**:
- [2011-09-22] If Moldgraf Monstrosity's ability can't exile it (perhaps because it's not still in the graveyard when the ability resolves), the two creature cards are still returned to the battlefield.
- [2011-09-22] If two Moldgraf Monstrosities die simultaneously, the first ability to resolve could return the other Moldgraf Monstrosity to the battlefield. If it does, the second Moldgraf Monstrosity's ability won't exile it but it will return two more creature cards to the battlefield.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/194/moldgraf-monstrosity
**Oracle text**:
```
Trample
When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
```
**Type line**: Creature — Insect
**Mana cost**: {4}{G}{G}{G} — **P/T**: 8/8 — **Keywords**: Trample
**Rulings** (2, both 2011-09-22):
1. "If Moldgraf Monstrosity's ability can't exile it (perhaps because it's not still in the graveyard when the ability resolves), the two creature cards are still returned to the battlefield."
2. "If two Moldgraf Monstrosities die simultaneously, the first ability to resolve could return the other Moldgraf Monstrosity to the battlefield. If it does, the second Moldgraf Monstrosity's ability won't exile it but it will return two more creature cards to the battlefield."

**Status**: ISSUE (fixed)

### Card data
Matches the fetched text: `{4}{G}{G}{G}`, `card_types: [Creature]`,
`subtypes: ["Insect"]`, 8/8, `keywords: [Trample]`, oracle text verbatim in the
current "When this creature dies" errata wording, and one `TriggeredAbilityDef`
of kind `SelfDies` matching the one implemented hook.

Both rulings are implemented, commented, and already tested — the exile is
guarded on the card still being in the graveyard, and the return happens either
way (CR 608.2, do as much as you can).

### Code issues

1. **Seven cards each had their own RNG** (`helpers.rs`, six card files, plus a
   new guard).
   - Oracle text says: `return two creature cards at random`
   - The card said:
     `let mut rng = rand::thread_rng(); creatures_in_gy.shuffle(&mut rng); ... .take(2)`
   - "At random" (CR 104.3) and "flip a coin" (CR 705.2) are rules, and this
     card was one of **seven** reaching for `rand::thread_rng()` directly:
     five in this exact "shuffle my candidates, then take N" shape (Charmbreaker
     Devils, Ghoulraiser, Make a Wish, Moldgraf Monstrosity, Woodland Sleuth),
     one as `.choose()` (Desperate Ravings), one as `gen_bool(0.5)`
     (Creepy Doll).
   - Two shapes, so two helpers: `helpers::choose_at_random(candidates, n)` and
     `helpers::flip_coin()`. All seven cards go through them, and
     `a_card_gets_its_randomness_from_the_helpers` keeps card files out of
     `rand`. Same move, and the same reason, as `shuffle_library` two cards ago:
     one call site can take a seed from the game state, seven cannot.
   - The guard's first draft matched `rand::` as a substring and flagged
     `disciple_of_griselb**rand::**DiscipleOfGriselbrand`; it now asks for
     `rand::thread_rng`, `rand::Rng`, `rand::seq` or `SliceRandom`.
   - `card_data_invariants.rs`'s existing
     `a_card_that_says_at_random_actually_randomizes` keyed on the RNG calls
     that have now moved into the helper, so it failed on the refactor —
     correctly, doing its job. `choose_at_random` and `flip_coin` joined its
     needle list, since calling them is reaching for an RNG just as much as
     `shuffle(` was.

2. **"two *creature* cards" was never a claim the test could fail**
   (`cards_complex_creatures.rs:1643`, test extended).
   - Oracle text says: `return two creature cards at random from your graveyard`
   - `moldgraf_monstrosity_returns_creatures_on_death` put two creatures in the
     graveyard and nothing else.
   - Verified: dropping `state.is_creature(o.id, registry)` from the filter —
     return **any** card — produced zero failures across the whole workspace.
   - A Doom Blade now sits in that graveyard and must stay there.

### Tricky interactions checked
- **Ruling 1** (can't exile it → the returns still happen): PASS —
  `resolution_time_checks.rs:64`
  (`moldgraf_exile_skipped_when_already_exiled_still_returns_creatures`).
- **Ruling 2** (two dying together; the second must not exile a live creature):
  PASS — `resolution_time_checks.rs:31`
  (`moldgraf_simultaneous_death_second_trigger_does_not_exile_live_creature`).
- Returns exactly **two**: PASS — returning one fails three tests.
- From **your** graveyard, and under your control (CR 603.10c, last known
  controller rather than owner): PASS — reading the opponent's graveyard fails
  four tests, including `bug_ca_moldgraf_monstrosity_uses_controller_not_owner`.
- **Creature** cards only: PASS — new assertion.
- Creature **cards**, not tokens (CR 109.1): PASS —
  `token_is_not_a_card.rs:350`.
- Does not return itself: the filter excludes `object_id`, and it has just been
  exiled anyway.
- "At random" is genuinely random rather than "the first eligible": guarded by
  `a_card_that_says_at_random_actually_randomizes`, which requires a card whose
  oracle text says "at random" to reach for an RNG. **Which** two come back is
  not observable with an unseeded RNG — the same limitation recorded in the
  Memory's Journey entry — so the tests stock exactly two candidates and check
  both arrive.
- Trample: a keyword read through `has_keyword`; covered by the combat tests.
- Self-cleanup: this is a permanent; "exile it" is the card's own printed
  effect on itself in the graveyard, not spell cleanup.

### UI presentation
Trigger description: "exile, return two random creatures from graveyard". Log
lines name the source for both halves.

### Test coverage
- Returns two creature cards, exiles itself: `cards_complex_creatures.rs:1643`
  (`moldgraf_monstrosity_returns_creatures_on_death`) — **the non-creature
  assertion added this audit**.
- Ruling 1: `resolution_time_checks.rs:64`.
- Ruling 2: `resolution_time_checks.rs:31`.
- Controller, not owner: `bug_ca_moldgraf_monstrosity_uses_controller_not_owner`.
- Tokens excluded: `token_is_not_a_card.rs:350`.
- "At random" reaches an RNG: `card_data_invariants.rs`
  (`a_card_that_says_at_random_actually_randomizes`).
- Randomness comes from the helpers: `test_suite_guards.rs`
  (`a_card_gets_its_randomness_from_the_helpers`) — **added this audit**.
- **Which** two come back: NOT TESTED — unobservable with an unseeded RNG.

### Mutations run
| mutation | result |
| --- | --- |
| return any card, not just creature cards | fails the extended test (before: **nothing at all**) |
| hand-roll the RNG again instead of calling the helper | fails the new guard |
| return one creature instead of two | fails three tests |
| read the opponent's graveyard | fails four tests |

Suite after: 1462 passing, exit 0, zero warnings.

