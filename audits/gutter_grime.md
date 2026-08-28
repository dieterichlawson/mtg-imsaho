## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/186/gutter-grime?utm_source=api
**Type line**: `Enchantment` — {4}{G}
**Oracle text**:
```
Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever a **nontoken** creature **you control** dies" — both filters present.
  The token check reads the *captured* `dead_is_token` rather than the object,
  and the comment says why: SBA 704.5d has already removed the dead token from
  `state.objects` by the time the trigger resolves, so the object is not there
  to ask.
- The Ooze token's P/T is linked to the slime-counter count on this Gutter Grime
  rather than fixed at creation, so every Ooze grows as more creatures die.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/186/gutter-grime?utm_source=api
**Type line**: `Enchantment` — {4}{G}
**Oracle text**:
```
Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If you control **more than one** Gutter Grime, each Ooze token
  **remembers which one created it**. The power and toughness of that Ooze will
  be equal to the number of slime counters on **that** Gutter Grime only." Each
  token stores its creator's id in `card_state`, and `effective_power` reads the
  counters of *that* object: PASS
- Ruling: "The power and toughness of the Ooze tokens will **constantly
  update**": it is read live, not snapshotted: PASS
- Ruling: "If Gutter Grime leaves the battlefield, the power and toughness of
  each Ooze token it created will become 0 ... put into its owner's graveyard
  the next time state-based actions are checked." A permanent's counters are
  cleared on a zone change, so the lookup yields 0 and the tokens die to SBA:
  PASS
- "Whenever a **nontoken** creature you control dies" — so the Oozes it makes do
  not feed it: PASS
- The counter goes on *then* the token is created, so the first Ooze is a 1/1
  rather than a 0/0: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The per-source counter link: `cards_complex_creatures.rs`, `state_based_actions.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/186/gutter-grime?utm_source=api
**Type line**: `Enchantment` — {4}{G}
**Oracle text**:
```
Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
```

**Rulings fetched**:
- [2011-09-22] The power and toughness of the Ooze tokens will constantly update as Gutter Grime accumulates slime counters.
- [2011-09-22] If you control more than one Gutter Grime, each Ooze token remembers which one created it. The power and toughness of that Ooze will be equal to the number of slime counters on that Gutter Grime only.
- [2011-09-22] If Gutter Grime leaves the battlefield, the power and toughness of each Ooze token it created will become 0. Unless another effect is raising its toughness above 0, each of these Ooze tokens will be put into its owner's graveyard the next time state-based actions are checked.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."`
**Type line**: `Enchantment` — {4}{G}
**Status**: ISSUE (fixed)

### Rulings (all 2011-09-22)
1. "The power and toughness of the Ooze tokens will constantly update as Gutter Grime accumulates slime counters."
2. "If you control more than one Gutter Grime, each Ooze token remembers which one created it. The power and toughness of that Ooze will be equal to the number of slime counters on that Gutter Grime only."
3. "If Gutter Grime leaves the battlefield, the power and toughness of each Ooze token it created will become 0. Unless another effect is raising its toughness above 0, each of these Ooze tokens will be put into its owner's graveyard the next time state-based actions are checked."

### Code issues

- `mtg-engine/src/state.rs:1388` and `:1456` — this card's rule was written into the engine, twice.
  - Oracle text says: `create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."`
  - `effective_power` did: `let counter_type = match counter_type_val { 1 => crate::types::CounterType::Slime, _ => crate::types::CounterType::PlusOnePlusOne };` — and `effective_toughness` carried a byte-identical copy.
  - The token wrote two `card_state` entries: `pt_source_counter` (the Grime's object id) and `pt_source_counter_type` holding `ObjectId(1)` as a tag meaning "slime". `card_state` is `HashMap<String, ObjectId>`, so a counter type was being smuggled through a field typed for object ids, decoded by a magic number, with every value but `1` silently meaning +1/+1. `state.rs`'s own comment named the card.
  - Of the eight `card_state` keys in the pool, seven are written and read by the same card. This was the only one the engine itself interpreted.

Fixed with `CardBehavior::token_dynamic_pt`, the general shape of the thing: a token whose *own* ability defines its P/T (CR 604.3) has no card face to hang a `dynamic_pt` on, so it records its creator under the one well-known key `cards::PT_DEFINED_BY` and that object's card is asked for the numbers. `state.rs` now holds one shared `token_defined_pt` helper that follows the link and asks; it knows nothing about counters. Gutter Grime's own file states the rule, quoting the ability that prints it.

Nothing else was wrong. `{4}{G}`, Enchantment with no subtypes, oracle text verbatim, `TriggerKind::AnyCreatureDies` matching the implemented `on_any_creature_dies`, counter added *before* the token is created as the oracle's "then" requires, token green/Ooze/creature/0-0 under the Grime's controller via `create_token_with_subtypes`.

### Tricky interactions checked

- Ruling 1, P/T recomputed rather than stamped: PASS — read live in `token_dynamic_pt` every call.
- Ruling 2, each Ooze counts its own Grime: PASS — the link is to `self_id`, the object, not to the card. Untested until this audit.
- Ruling 3, Grime leaves and the Oozes become 0/0: PASS, and it falls out rather than being special-cased: `move_object` clears counters on a zone change (CR 400.7), so the same `get_counter_count` answers 0. If the Grime object is gone from the game entirely, the link dangles, `token_defined_pt` returns `None`, and the token's own base 0/0 is the answer — the same number by a different road.
- "a **nontoken** creature": PASS, and correctly via the captured `dead_is_token` rather than reading the object back — SBA 704.5d has already removed a dead token from `state.objects` by the time the trigger resolves.
- "a creature **you control**": PASS, `dead_controller` compared against the Grime's controller, both as of the death (last known information, CR 603.10).
- The Grime dying to the same event that triggers it: PASS. `trigger_independence.rs:127` covers the case where the Grime in the graveyard is a different object, so the slime counter does not land there.
- The trigger firing on the Grime's own death: N/A — it is an enchantment, so no `CreatureDied` (covered at `trigger_independence.rs:102`).

### Test coverage

- Slime counter and one Ooze per qualifying death: `cards_complex_creatures.rs:435` `gutter_grime_creates_ooze_on_creature_death`
- "a **green Ooze creature** token", under your control: same test, added this audit
- Ruling 1, every Ooze resized as the count rises: `cards_complex_creatures.rs:2645` `every_ooze_is_sized_by_the_current_slime_count` — the loop that made this claim was vacuous until this audit (see below)
- Ruling 2, two Grimes at different counts: `cards_complex_creatures.rs:2687` `each_ooze_counts_the_slime_on_the_gutter_grime_that_made_it`, added this audit
- Ruling 3, Oozes go to 0/0 and SBA buries them: `cards_complex_creatures.rs:2755` `the_oozes_die_when_gutter_grime_leaves`
- "nontoken" and "you control", all three cases: `cards_complex_creatures.rs:2678` `gutter_grime_counts_only_your_own_nontoken_creatures`
- Trigger source independence: `trigger_independence.rs:102`, `:127`, `trigger_source_independence.rs:568`

### The vacuous loop

`every_ooze_is_sized_by_the_current_slime_count` gathered its Oozes with `o.name == "Ooze"`. CR 111.4 names a token after its subtypes plus "Token", so the tokens are `"Ooze Token"`, the filter matched nothing, and the loop asserting "every Ooze is the current size" — the entire point of the test — never executed. The same shape as the Kessig Cagebreakers Wolf filter found earlier in this audit run. It now asserts the count before iterating, so an empty list fails.

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 `token_dynamic_pt` returns `None` | n/a (new hook) | 3 tests FAILED |
| M2 link the token to `ObjectId(0)` instead of this Grime | n/a | 3 tests FAILED |
| M3 add the slime counter *after* creating the token | passed whole workspace | passed — **vacuous**, see below |
| M4 Ooze black instead of green | passed whole workspace | `gutter_grime_creates_ooze_on_creature_death` FAILED |
| M5 Ooze under the opponent's control | passed whole workspace | `gutter_grime_creates_ooze_on_creature_death` FAILED |

M3 is a genuinely vacuous mutation and is recorded as such rather than as a gap: the oracle's "put a slime counter on this enchantment, **then** create a token" has no observable consequence here, because the token's P/T is a characteristic-defining ability read live rather than a size stamped at creation. Either order produces the same Ooze. Reported honestly instead of being dressed up as a caught bug.

Source restored from `/tmp/gg.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1474 passing (was 1473). `cargo check --workspace --all-targets` clean, zero warnings.
