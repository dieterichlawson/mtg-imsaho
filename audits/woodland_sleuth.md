## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/210/woodland-sleuth?utm_source=api
**Type line**: `Creature — Human Scout` — {3}{G}, 2/3
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.

- Morbid is an intervening-if on an ETB trigger (CR 603.4): the card overrides
  `should_trigger`, so with nothing dead this turn nothing reaches the stack, and
  the handler re-checks on resolution.
- "return a creature **card** at random" — at random, not chosen.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/210/woodland-sleuth?utm_source=api
**Type line**: `Creature — Human Scout` — {3}{G}, 2/3
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The creature card **isn't chosen at random until the ability
  resolves**." The candidate list is built and shuffled inside the trigger
  handler: PASS
- Ruling: "Woodland Sleuth could die in response to its own morbid ability. If
  this happens, the ability **could return Woodland Sleuth** to its owner's
  hand." There is no self-exclusion, and the handler reads the controller from
  whatever zone the Sleuth is in rather than requiring it on the battlefield:
  PASS
- "Morbid — ... **if** a creature died this turn" is an intervening-if
  (CR 603.4): PASS
- CR 109.1: "a creature **card** at random", so a token is not a candidate —
  the filter's `map_or` fallback used to admit exactly that and is now fixed:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- A token is not returned: `token_is_not_a_card.rs:woodland_sleuth_does_not_return_a_token_from_the_graveyard`
- The morbid condition: `intervening_if.rs`, `cards_morbid_and_ltb.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/210/woodland-sleuth?utm_source=api
**Type line**: `Creature — Human Scout` — {3}{G}, 2/3
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.
```

**Rulings fetched**:
- [2011-09-22] The creature card isn’t chosen at random until the ability resolves.
- [2011-09-22] Woodland Sleuth could die in response to its own morbid ability. If this happens, the ability could return Woodland Sleuth to its owner’s hand.

**Status**: PASS


### Code issues
No issues found. The implementation was already correct on every point the
oracle text and both rulings turn on; the audit's output is test coverage and
one engine-wide guard.

- Morbid is an ability word (CR 207.2c), so it is correctly absent from
  `keywords` and correctly implemented as an intervening-if clause: checked in
  `should_trigger` via `helpers::morbid_should_trigger` **and** re-checked at
  the top of `on_enter_battlefield`, which is what CR 603.4 requires (the
  condition is tested when the ability would trigger and again as it resolves).
- `is_card && is_creature` is the right filter for "a creature card"
  (CR 109.1): a creature token in the graveyard is not a card.
- `objects_in_zone(Zone::Graveyard, controller)` filters graveyards by *owner*,
  which is exactly "your graveyard" under CR 404.3.
- `controller_of` is the right read for the "you" of a triggered ability
  (CR 109.5), and the ability is untargeted, so nothing is locked in at
  trigger time.

### Tricky interactions checked
- Intervening-if both arms (returns only when a creature died): pass
- Token in the graveyard is not "a creature card" (CR 109.1): pass
- The Sleuth returning itself after dying to its own trigger: pass
- Opponent's graveyard is not "your graveyard" (CR 404.3): pass
- The choice is genuinely random, not the first eligible card: pass
- Return goes to hand, and CR 404.3 owner-scoping means "your hand" and
  "its owner's hand" coincide here: pass

### Test coverage
- Ruling 1, "the creature card isn't chosen at random until the ability
  resolves": `intervening_if.rs:307` — the Sleuth is not in the graveyard when
  the trigger goes on the stack and is still a legal choice when it resolves,
  which is the strongest form of the ruling.
- Ruling 2, "Woodland Sleuth could die in response to its own morbid ability
  ... the ability could return Woodland Sleuth to its owner's hand":
  `intervening_if.rs:307`
- Morbid intervening-if (CR 603.4), trigger side: `intervening_if.rs:206`
- Morbid intervening-if, resolution side, both arms:
  `cards_graveyard_interaction.rs:311`
- "creature **card**" excludes tokens: `token_is_not_a_card.rs:310`
- **NEW** "at random" is really random — sixty draws from three candidates,
  which a fixed choice fails with certainty:
  `cards_graveyard_interaction.rs:328`
- **NEW** "from **your** graveyard" — an opponent's creature card is not a
  legal choice: `cards_graveyard_interaction.rs:363`

### Engine-wide change
`card_data_invariants.rs::a_card_that_says_at_random_actually_randomizes` — a
build-failing source scan asserting that every card whose oracle text contains
"at random" reaches for an RNG. A card that picked the first eligible object
would pass every ordinary per-card test, because those tests set up a single
candidate and cannot tell a fixed choice from a random one; it would only
surface as a game that always makes the same "random" pick. All six ISD cards
that say "at random" (Charmbreaker Devils, Desperate Ravings, Ghoulraiser,
Make a Wish, Moldgraf Monstrosity, Woodland Sleuth) pass today; the guard keeps
the seventh honest.

### Not changed, and why
The engine draws randomness from `rand::thread_rng()` everywhere — library
shuffles, the opening coin flip, mulligans, and all six "at random" cards —
with no seeded-RNG infrastructure. That is a reproducibility property, not a
rules one: CR 104.3c and the shuffle rules ask for genuine randomness, which
`thread_rng` supplies. Threading a seeded RNG through `GameState` would be a
real improvement for replaying games, but it is an engine-wide change with no
bearing on whether this card is correct, so it is recorded here rather than
undertaken inside a card audit.

