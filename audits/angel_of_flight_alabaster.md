## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/2/angel-of-flight-alabaster?utm_source=api
**Type line**: `Creature — Angel` — {4}{W}, 4/4
**Oracle text**:
```
Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
```

**Status**: ISSUE (fixed) — duplication, not a rules defect

### Code issue
- Oracle text says the trigger happens at **your** upkeep / **your** end step.
- Code did: declared `step_trigger_scope` → `TriggerScope::Your`, which is
  correct and sufficient, and then re-derived the same thing inside the handler
  as `state.active_player != controller`.
- The engine's gate is not taken on trust: `your_upkeep_scope.rs` sweeps the
  registry for every card with a controller-scoped step trigger and checks both
  directions — fires on the controller's step, silent on the opponent's. The
  handler check was provably dead.
- Fixed: removed, with a comment saying where the scoping actually lives.
Additionally, the handler opened with a `get_object(self_id)` match that
returned early when the source was gone. That never fired — a permanent in a
graveyard still resolves `get_object` — but CR 113.7a means the ability owes
nothing to its source and the effect only needs its target, so the lookup was
removed as well.

### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/2/angel-of-flight-alabaster?utm_source=api
**Type line**: `Creature — Angel` — {4}{W}, 4/4
**Oracle text**:
```
Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The Spirit card must already be in your graveyard **when the ability
  triggers** at the beginning of your upkeep. If there is no Spirit card in your
  graveyard when your upkeep begins, the ability will be **removed from the
  stack with no effect**." The trigger declares a `target_requirement`, so
  CR 603.3d chooses the target as it goes on the stack and no legal Spirit means
  it is not put on the stack at all: PASS
- "target **Spirit** card" — `is_valid_target` narrows the graveyard enumeration
  to Spirits, and the engine applies that filter when building the target list:
  PASS
- CR 109.1: a Spirit *card*, so a Spirit token in the graveyard is not offered —
  now enforced in the engine's graveyard enumeration: PASS
- "from **your** graveyard" — `GraveyardCardOwnedByCaster`: PASS
- "At the beginning of **your** upkeep": PASS
- CR 113.7a: killing the Angel in response does not counter the trigger, and the
  handler does not need the source to resolve: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Spirit filter and the no-target case: `cards_complex_creatures.rs`, `trigger_dispatch.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/2/angel-of-flight-alabaster?utm_source=api
**Type line**: `Creature — Angel` — {4}{W}, 4/4
**Oracle text**:
```
Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
```

**Rulings fetched**:
- [2011-09-22] The Spirit card must already be in your graveyard when the ability triggers at the beginning of your upkeep. If there is no Spirit card in your graveyard when your upkeep begins, the ability will be removed from the stack with no effect.

**Status**: PASS (behaviour correct; the ruling and two targeting restrictions were untested)

### Code issues

**The card's behaviour is correct.** The one code change is cosmetic; the rest
of this is coverage.

**Cosmetic:** `on_upkeep` opened with `let _ = self_id;`. The parameter really
is unused — deliberately, because the ability resolves whether or not the Angel
is still there (CR 113.7a) and its "your" comes from the trigger's scope rather
than from the source — so it is now `_self_id` in the signature, with the
reason as a doc comment rather than a statement that does nothing.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{4}{W}` | `Generic(4), Colored(White)` OK |
| type | `Creature - Angel` | `Creature`, `["Angel"]` OK |
| P/T | 4/4 | `Some(4)/Some(4)` OK |
| keywords | Flying | `vec![Keyword::Flying]` OK |
| oracle text | verbatim match | OK |
| trigger | "At the beginning of your upkeep, return target Spirit card from your graveyard to your hand" | `TriggerKind::Upkeep`, `TriggerScope::Your`, `TargetRequirement::GraveyardCardOwnedByCaster` narrowed to Spirits by `is_valid_target` OK |

### Tricky interactions checked

- **Ruling 2011-09-22: "The Spirit card must already be in your graveyard when
  the ability triggers... If there is no Spirit card in your graveyard when
  your upkeep begins, the ability will be removed from the stack with no
  effect."** **Pass**, and the distinction is worth recording: this is CR 603.3c
  — the ability *does* trigger, goes on the stack, and is then removed for want
  of a legal target. It is deliberately **not** CR 603.4. Morkrut Banshee,
  audited earlier in this run, is the other case: its ruling says the ability
  "won't trigger at all", and I moved its condition out of `is_valid_target`
  into `should_trigger` for exactly that reason. The two cards are implemented
  differently because their rulings say different things. Was untested; the new
  test asserts both that the stack ends empty *and* that the removal is logged,
  which is what tells 603.3c from 603.4.
- **"target Spirit card"** — a Spirit token in the graveyard is not a card
  (CR 109.1) and sits there until the next state-based-action check, so it can
  be seen. Doomed Traveler makes one. **Pass**, and enforced twice over:
  `targeting.rs` filters `is_card` when enumerating, and `stack.rs` checks it
  again in the CR 608.2b re-check. The new test asserts the outcome; removing
  the guard from *either* layer alone still passes, and removing it from both
  fails — which is the honest description of what it covers.
- **"from your graveyard"** — an opponent's Spirit card is not a candidate.
  **Pass**, now tested.
- **CR 608.2b, the target stopped being a Spirit card**, in both halves — the
  generic one (exiled out of the graveyard) and the card's own restriction (not
  a Spirit). **Pass**, tested.
- **The Angel destroyed in response to its own trigger** — the Spirit still
  comes back (CR 113.7a). **Pass**, tested.
- **Only on your upkeep.** **Pass** — `TriggerScope::Your`, checked against the
  printed "your" by the text-derived invariant added earlier in this run, and
  scoping it to `Each` fails that sweep.
- **One legal target is taken without asking.** Correct: there is nothing to
  decide, and the ability is not optional.

### Test coverage

- returns a Spirit on upkeep:
  `cards_upkeep_triggers_and_curses.rs::angel_of_flight_alabaster_returns_spirit`
- the target stopped satisfying "Spirit card", and the happy path:
  `trigger_target_recheck.rs::a_trigger_fizzles_when_its_target_stops_satisfying_the_cards_restriction`,
  `::a_trigger_with_a_still_legal_target_resolves`
- the target was exiled out of the graveyard in response:
  `trigger_target_recheck.rs::a_trigger_whose_target_became_illegal_changes_nothing`
- the Angel destroyed in response:
  `trigger_source_independence.rs::angel_of_flight_alabaster_returns_its_spirit_after_dying`
- fires only on its controller's upkeep: `your_upkeep_scope.rs` (both sweeps)
- **the ruling — no Spirit card means the trigger is removed from the stack,
  not skipped**:
  `::angel_of_flight_alabaster_is_removed_from_the_stack_with_no_spirit` (new)
- **a Spirit token is not a Spirit card**:
  `::angel_of_flight_alabaster_cannot_target_a_spirit_token` (new)
- **an opponent's graveyard is out of reach**:
  `::angel_of_flight_alabaster_cannot_reach_an_opponents_graveyard` (new)

Mutation-checked: dropping the Spirit filter fails the removal test, and
scoping the trigger to `Each` fails the step-scope sweep. The token test needs
both `is_card` guards removed to fail, as described above.
