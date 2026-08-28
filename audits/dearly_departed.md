## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/9/dearly-departed?utm_source=api
**Type line**: `Creature — Spirit` — {4}{W}{W}, 5/5
**Oracle text**:
```
Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- "As long as this creature is in **your graveyard**" — gated on
  `o.zone == Zone::Graveyard`, so it does nothing from anywhere else.
- The card in a graveyard has an owner, not a controller, and the code compares
  the entering creature's *controller* against the Departed's *owner*. That is
  the right pairing for "each Human creature you control".
- Human check goes through `state.has_subtype`, so a token or a creature granted
  the type is included.

### Test coverage
`enters_tapped_replacement.rs` / `enter_trigger_conditions.rs` cover the ETB-counter replacement path generally
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/9/dearly-departed?utm_source=api
**Type line**: `Creature — Spirit` — {4}{W}{W}, 5/5
**Oracle text**:
```
Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**As long as this creature is in your graveyard**" — a static ability that
  works from the graveyard, so the `replace_event` is gated on the Departed
  being there rather than on the battlefield: PASS
- Ruling: "The effect is **cumulative**. Human creatures you control will enter
  with a +1/+1 counter **for each** Dearly Departed in your graveyard." Each
  copy is its own object with its own replacement, so two in the graveyard add
  two counters without any special-casing: PASS
- "each **Human** creature **you control**" — both filters, and `has_subtype`
  covers a Human token: PASS
- "enters with an **additional** +1/+1 counter" is a replacement effect
  (CR 614.1c), applied as the creature enters — so an ETB watcher sees the
  counter already there: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The graveyard-static counter: `cards_morbid_and_ltb.rs`, `enters_tapped.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/9/dearly-departed?utm_source=api
**Type line**: `Creature — Spirit` — {4}{W}{W}, 5/5
**Oracle text**:
```
Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
```

**Rulings fetched**:
- [2011-09-22] The effect is cumulative. Human creatures you control will enter with a +1/+1 counter for each Dearly Departed in your graveyard.
- [2011-09-22] In most cases, when determining whether a creature entering under your control should get a +1/+1 counter, you'll simply look at what the creature will looklike on the battlefield. You'll consider any effects affecting a creature entering under your control.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
```
**Type line**: `Creature — Spirit` — {4}{W}{W}, 5/5, Flying
**Status**: ISSUE (fixed) — test gaps; the card is correct

### Rulings (both 2011-09-22)
1. "The effect is cumulative. Human creatures you control will enter with a +1/+1 counter for each Dearly Departed in your graveyard."
2. "In most cases, when determining whether a creature entering under your control should get a +1/+1 counter, you'll simply look at what the creature will look like on the battlefield. You'll consider any effects affecting a creature entering under your control."

### Code issues

No issues in the card. It is a CR 614.1c replacement effect rather than a triggered ability (a bug fixed earlier and pinned by `trigger_dispatch.rs:672`), declared through the general `replacement_zones` / `replace_event` hooks with `Zone::Graveyard` — the case `replacement_zones` exists for. `{4}{W}{W}`, Creature — Spirit, 5/5, Flying, oracle text verbatim; the printed characteristics are now also cross-checked against the oracle cache by the invariants added during the Selfless Cathar audit.

The gap was in the tests: `e.controller != owner` — the whole of "each Human creature **you control**" — was unasserted, and removing it passed the entire workspace.

### Tricky interactions checked

- "in **your** graveyard": PASS, and via `o.owner`. A graveyard belongs to the card's owner (CR 404.1), so owner is the right field here rather than a controller that a card in a graveyard does not have (CR 108.4).
- "each Human creature **you control**": PASS. Untested until this audit.
- Ruling 1, cumulative: PASS, and it falls out of the pipeline rather than being coded — `replacement::apply` iterates over *objects*, not card ids, so two Dearly Departeds are two candidates. Worth stating because the same shape has bitten this codebase before: mana abilities were once deduplicated by description, and four Swamps collapsed into one offer. Untested until this audit.
- CR 614.5 does not stop the second one: PASS. `Replacement::Modified` passes the modified event to the next candidate; CR 614.5 stops one effect applying twice to an event, not two effects applying once each. The contrast is with `a_replaced_event_is_not_replaced_again`, where two Undead Alchemists really do collapse to one.
- Ruling 2, an entering token that is a copy of a Human: PASS. The copy effect has applied by the time the replacement is consulted, so the token is a Human as it enters (CR 706.2). Untested until this audit.
- Human-ness on a token or a granted subtype: PASS, `has_subtype` reads object-level subtypes as well as the active face's, so a granted "Human" would count.
- Non-Humans: PASS, tested at `replacement_effects.rs:118`.
- From the battlefield it does nothing: PASS.

### Test coverage

- Counter for an entering Human: `phantom_triggers.rs:124` `dearly_departed_in_graveyard_adds_counter`, `cards_complex_creatures.rs:156` `dearly_departed_gives_counter_to_entering_humans`, `replacement_effects.rs:107`
- Non-Human gets nothing: `replacement_effects.rs:107`
- "you control" — the opponent's Human gets nothing: `replacement_effects.rs:107`, added this audit
- Ruling 1, cumulative: `replacement_effects.rs:133` `each_dearly_departed_in_the_graveyard_adds_its_own_counter`, added this audit
- Ruling 2, token entering as a copy of a Human: `replacement_effects.rs:436` `a_token_entering_as_a_copy_of_a_human_gets_the_counter`, added this audit
- Nothing happens from the battlefield: `phantom_triggers.rs:147` — strengthened this audit from "no trigger was collected" to "no counter was given"
- It is a replacement effect, not a trigger: `trigger_dispatch.rs:672` `bug_q_dearly_departed_is_not_a_trigger`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 drop the `e.controller != owner` check | passed whole workspace | `a_replacement_can_apply_from_the_graveyard_to_another_permanent` FAILED |
| M2 drop the Human check | 1 test FAILED | (unchanged) |
| M3 two counters instead of one | 3 tests FAILED | (unchanged) |
| M4 `replacement_zones` -> `[Battlefield]` | 4 tests FAILED | (unchanged) |
| M5 refuse to add a second counter (break cumulativity) | passed whole workspace | `each_dearly_departed_in_the_graveyard_adds_its_own_counter` FAILED |
| M6 `replacement_zones` -> `[Graveyard, Battlefield]` | passed | passed — **redundant guard**, see below |
| M7 drop the in-card `o.zone == Zone::Graveyard` check | passed | passed — **redundant guard**, see below |
| M8 both zone guards dropped at once | n/a | `dearly_departed_on_battlefield_does_not_trigger` FAILED |

M6 and M7 are individually unobservable and are recorded as such rather than as gaps. The zone is guarded twice — `replacement_zones` filters which objects become candidates, and the card re-checks `o.zone` inside `replace_event` — so either alone still keeps the effect out of the battlefield. Only the combined mutation M8 changes behaviour, and the strengthened test catches it. The same pattern was recorded for Nevermore earlier in this audit run.

The in-card check is not pure duplication, incidentally: `replacement::apply` inserts the *entering object itself* as a candidate regardless of its zone, so a Dearly Departed entering the battlefield reaches `replace_event` with a non-graveyard zone. It is a Spirit, so the Human test would reject it anyway — but the zone check is the one that states the reason.

Source restored from `/tmp/dd.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1482 passing (was 1480). `cargo check --workspace --all-targets` clean, zero warnings.
