## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/200/prey-upon?utm_source=api
**Type line**: `Sorcery` — {G}
**Oracle text**:
```
Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)
```

**Status**: PASS

### Code issues
No issues found.

**Ruling [2018-12-07]**: "If either target is an illegal target as Prey Upon
resolves, no creature will deal or be dealt damage."

- The opposite of Into the Maw of Hell's partial resolution, and the code gets
  it: `if a_on_bf && b_on_bf` — both or neither.
- With the new `Target::Illegal` substitution this also holds for a target that
  gained hexproof: the `(Target::Object(a), Target::Object(b))` pattern stops
  matching, so nothing happens. The all-or-nothing ruling falls out of the
  representation rather than needing its own check.
- "Target creature **you control** fights target creature **you don't control**"
  — two filtered slots, and the handler sorts them by controller rather than
  trusting the order they arrive in.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/200/prey-upon?utm_source=api
**Type line**: `Sorcery` — {G}
**Oracle text**:
```
Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If **either** target is an illegal target as Prey Upon resolves, **no
  creature will deal or be dealt damage**." This is the opposite of Into the Maw
  of Hell's partial resolution — a fight needs both creatures, so losing one
  cancels the whole thing rather than half of it. The card checks both before
  fighting: PASS
- "target creature **you control** fights target creature **you don't control**"
  — two different filters on the two slots: PASS
- The fight is simultaneous damage from each creature's power, through the damage
  pipeline: PASS
- A 0-power creature fights and deals nothing rather than being an illegal
  choice: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both targets required: `fizzle.rs:prey_upon_resolves_without_fighting_when_one_of_its_two_targets_is_gone`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/200/prey-upon?utm_source=api
**Type line**: `Sorcery` — {G}
**Oracle text**:
```
Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)
```

**Rulings fetched**:
- [2018-12-07] If either target is an illegal target as Prey Upon resolves, no creature will deal or be dealt damage.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)`
**Type line**: `Sorcery` — {G}
**Status**: ISSUE (fixed) — a test gap and a card re-implementing the pipeline; behaviour was correct

### Ruling (2018-12-07)
"If either target is an illegal target as Prey Upon resolves, no creature will deal or be dealt damage."

### Code issues

- `mtg-engine/tests/hexproof_filter.rs` — only one of the two target filters was checked by anything.
  - Oracle text says: `Target creature **you control** fights target creature **you don't control**.`
  - `bug_aw_prey_upon_rejects_two_of_your_own_creatures` covers the pair being two of yours. Nothing covered the mirror: changing the *first* slot to `YouDontControl`, so the card read "target creature you don't control fights target creature you don't control", **passed the whole workspace**. Added the other direction — a board with only the opponent's creatures must offer no legal pair.

- `mtg-engine/src/cards/isd/prey_upon.rs:30` — the card re-implemented what `combat::fight` owns.
  - Code did: `let a_on_bf = ...; let b_on_bf = ...; if a_on_bf && b_on_bf { ... let (my_creature, their_creature) = if a_mine { (*a, *b) } else { (*b, *a) }; crate::combat::fight(state, my_creature, their_creature, registry); }`
  - `combat::fight` already implements CR 701.12b, and more of it than the card did — "no longer on the battlefield **or are no longer creatures**". And a fight is symmetric (CR 701.12a), so the sort into "mine" and "theirs" decided nothing: `fight(a, b)` and `fight(b, a)` are the same call. The card is now three lines, and the mutation swapping the arguments is vacuous by construction rather than by luck.

Everything else is right: `{G}`, Sorcery, oracle text verbatim, and `TwoTargets(CreatureWithFilter(YouControl), CreatureWithFilter(YouDontControl))` — the declared requirement, which is what CR 608.2b re-checks per slot.

### Tricky interactions checked

- The ruling, one target illegal: PASS, and it needs no code — an illegal target arrives as `Target::Illegal` and fails the pattern. Tested at `fizzle.rs:255`, which asserts *both* that the spell resolved (it is not countered — CR 608.2b needs **all** targets illegal) and that no damage happened.
- Fight damage is noncombat: PASS, `damage_pipeline.rs:129` — a fixed bug with its regression test.
- Both powers read before either damage is dealt: PASS, in `combat::fight`, so the second creature's damage is not reduced by the first's (CR 701.12a).
- "you control" on the first slot: PASS. Untested until this audit.
- "you don't control" on the second: PASS, `hexproof_filter.rs:468`.
- Damage source attribution: PASS — each creature is the source of its own damage, which is what makes lifelink and "damaged by" work.

### Test coverage

- The fight happens and both take damage, and SBA kills the loser: `cards_removal_and_bounce.rs:246` `prey_upon_fight`
- The ruling — one target gone means no damage, and the spell still resolves: `fizzle.rs:255` `prey_upon_resolves_without_fighting_when_one_of_its_two_targets_is_gone`
- Two of your own creatures are not a legal pair: `hexproof_filter.rs:468` `bug_aw_prey_upon_rejects_two_of_your_own_creatures`
- Two of the opponent's are not either: `hexproof_filter.rs:508` `prey_upon_rejects_two_creatures_the_caster_does_not_control`, added this audit
- Fight damage is noncombat: `damage_pipeline.rs:129`
- CR 701.12b in the pipeline: `a_fight_deals_no_damage_when_the_target_has_left_the_battlefield`, `nightfall_predators_fight_does_nothing_if_the_predator_dies_in_response`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 second slot also `YouControl` | `bug_aw_prey_upon_rejects_two_of_your_own_creatures` FAILED | (unchanged) |
| M2 first slot `YouDontControl` | passed whole workspace | `prey_upon_rejects_two_creatures_the_caster_does_not_control` FAILED |
| M3 the first creature fights itself | 2 tests FAILED | (unchanged) |
| M4 remove `combat::fight`'s CR 701.12b guard | 3 tests FAILED | (unchanged) |

M4 is worth noting for what it says about the simplification: the pipeline's guard is already held by three tests, one of which is Prey Upon's own. Removing the card's duplicate of it therefore lost no coverage — which is the check I wanted before deleting it, rather than assuming.

Also corrected a rule citation in `fizzle.rs`: fight is CR 701.12b, not 701.15 (regenerate).

Source restored from `/tmp/pu2.bak` and `/tmp/combat.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1500 passing (was 1499). `cargo check --workspace --all-targets` clean, zero warnings.
