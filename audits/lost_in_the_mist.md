## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/63/lost-in-the-mist?utm_source=api
**Type line**: `Instant` — {3}{U}{U}
**Oracle text**:
```
Counter target spell. Return target permanent to its owner's hand.
```

**Status**: PASS

### Code issues
No issues found.

**Ruling [2011-09-22]**: partial resolution, same as Into the Maw of Hell.

- Both halves guard independently — the counter half on the spell still being on
  the stack, the bounce half on the permanent still being on the battlefield —
  so one illegal target does not stop the other.
- Counters through `move_countered_spell` (CR 701.5a), not the resolving-spell
  cleanup path, which is the right entry point for disposing of a *different*
  spell.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/63/lost-in-the-mist?utm_source=api
**Type line**: `Instant` — {3}{U}{U}
**Oracle text**:
```
Counter target spell. Return target permanent to its owner's hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Lost in the Mist targets **both** the spell and the permanent. You can
  only cast it if you can choose legal targets for both": PASS
- Ruling: "If **one** of Lost in the Mist's targets is illegal by the time it
  resolves, Lost in the Mist will **still affect the remaining legal target**. If
  **both** targets are illegal at this time, Lost in the Mist won't resolve."
  The engine substitutes `Target::Illegal` rather than removing, so the
  positions hold and the surviving half still happens: PASS
- Countering uses `move_countered_spell` (CR 701.5a), not the resolving-spell
  cleanup path — so a countered flashback spell is still exiled: PASS
- "Return target **permanent**", not just a creature: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Partial fizzle across two targets: `fizzle.rs:a_multi_target_spell_is_countered_only_when_every_target_is_illegal`
- Countering: `cards_counterspells.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/63/lost-in-the-mist?utm_source=api
**Type line**: `Instant` — {3}{U}{U}
**Oracle text**:
```
Counter target spell. Return target permanent to its owner's hand.
```

**Rulings fetched**:
- [2011-09-22] Lost in the Mist targets both the spell and the permanent. You can only cast it if you can choose legal targets for both parts.
- [2011-09-22] If one of Lost in the Mist’s targets is illegal by the time it resolves, Lost in the Mist will still affect the remaining legal target. If both targets are illegal at this time, Lost in the Mist won’t resolve.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/63/lost-in-the-mist
**Oracle text**:
```
Counter target spell. Return target permanent to its owner's hand.
```
**Type line**: `Instant`
**Mana cost**: `{3}{U}{U}`
**Rulings** (2, both 2011-09-22, https://api.scryfall.com/cards/1e5fc39d-590a-436b-ab90-a1741d2ae3da/rulings):
1. "Lost in the Mist targets both the spell and the permanent. You can only cast it if you can choose legal
   targets for both parts."
2. "If one of Lost in the Mist's targets is illegal by the time it resolves, Lost in the Mist will still affect
   the remaining legal target. If both targets are illegal at this time, Lost in the Mist won't resolve."

**Status**: ISSUE (fixed) — the card's behaviour was correct; neither ruling was tested, and the counter
sequence it shares with three other sites had drifted in one of them.

### Card data
| field | oracle | `lost_in_the_mist.rs` | |
|---|---|---|---|
| cost | `{3}{U}{U}` | `Generic(3) + Blue + Blue` | ok |
| types | Instant | `vec![CardType::Instant]` | ok |
| oracle_text | as above | byte-identical | ok |
| targeting | two targets, different kinds | `TwoTargets(Spell, PermanentWithFilter(Any))` | ok |

### Rules check
- **Ruling 1** is satisfied by construction: `TwoTargets` enumeration is a Cartesian product, so an empty first
  slot yields no cast action at all. Verified, and now tested.
- **Ruling 2** is satisfied: `stack.rs` fizzles only when *no* target is legal (`any_legal`), and each half of
  `on_resolve` guards its own zone, so an illegal target is simply not affected.
- **"to its owner's hand"**: `move_object(perm_id, Zone::Hand, ..)` — `objects_in_zone` keys the hand on
  `obj.owner`, so a permanent an opponent had gained control of returns to its owner. Correct without the card
  having to say so.
- **CR 701.5a**: countering goes through `move_countered_spell`, which exiles a spell cast with flashback
  (CR 702.33a) instead of putting it in the graveyard.

### Code issues

**The counter sequence was written out four times, and one copy had drifted.** Fixed.

Countering is two steps that must not come apart — remove the entry from the stack, then dispose of the card.
Four places did both by hand: `counterspell.rs`, `dissipate.rs`, `lost_in_the_mist.rs`, and
`engine/actions/choices.rs` (which finishes Frightful Delusion's "unless its controller pays {1}").

- Three called `state.stack.retain(..)` then `state.move_countered_spell(..)`.
- Dissipate called `state.stack.retain(..)` then `state.move_object(*target_id, Zone::Exile, registry)`.

Exile *is* the right destination for Dissipate ("exile it instead of putting it into its owner's graveyard"), so
this was not a bug and I am not claiming it was. It is the shape of the problem: a card reached its destination
without going through the pipeline, and the pipeline is where the flashback rule lives.

Three of the four also carried a byte-identical `is_valid_target` testing `o.zone == Zone::Stack`.

### Changes made
- `mtg-engine/src/cards/helpers.rs` — `counter_spell`, `counter_spell_exiling` (CR 614.1a replacing CR 701.5a's
  default, which is what Dissipate does), and `spell_target_is_legal`. `counter_spell` returns whether anything
  was countered, since CR 608.2b makes "the target already left the stack" an ordinary outcome.
- The four sites converted. Net −60 lines against +65 in one place.
- `mtg-engine/tests/card_data_invariants.rs` — `no_card_removes_a_spell_from_the_stack_itself`.
- `mtg-engine/tests/cards_removal_and_bounce.rs` — `lost_in_the_mist_needs_a_target_for_both_halves` (ruling 1),
  with a control that adds a spell to the stack so "uncastable" is about the missing target and not about mana.
- `mtg-engine/tests/fizzle.rs` — `lost_in_the_mist_counters_or_bounces_whichever_target_survives` (ruling 2, all
  three combinations).

**A test that claimed more than it showed.** I first wrote the ruling-2 test in
`cards_removal_and_bounce.rs`, asserting on the battlefield: the spell in the graveyard, the creature not in
hand. Mutation 2 below showed it passed with fizzling disabled outright. Of course it did — this card does
nothing observable beyond what it does to its targets, so "countered by game rules" and "resolved and found
neither target" leave identical boards. `fizzle.rs` exists for exactly this and says so in its header; its
`resolved()` helper watches `GameEvent::SpellResolved`. The test moved there and now discriminates.

### Mutation checks
1. `any_legal` → `all(&legal)` (fizzle whenever any target is illegal, violating ruling 2's first half) →
   `lost_in_the_mist_counters_or_bounces_whichever_target_survives` FAILED, with three other fizzle tests.
   **Discriminating.**
2. `any_legal` → `|| true` (never fizzle, violating ruling 2's second half) → **vacuous against the first
   version of the test**; discriminating against the rewritten one, which FAILED along with four others.
3. `counter_spell` removing the stack entry but not disposing of the card → three tests FAILED across the two
   cards and Frightful Delusion, which is the point of putting both steps in one function.
4. Reintroducing `stack.retain` into Lost in the Mist → `no_card_removes_a_spell_from_the_stack_itself` FAILED.
5. `TwoTargets` enumeration made to offer a cast with an empty first slot →
   `lost_in_the_mist_needs_a_target_for_both_halves` FAILED. **Discriminating**, and nothing else caught it.
6. `is_target_legal`'s `TwoTargets` arm judging only the first slot → did **not** fail this card's tests; it
   failed Into the Maw of Hell's. Recorded as-is rather than claimed: Lost in the Mist does not depend on that
   branch, because both halves of its `on_resolve` guard their own zone.

### Tricky interactions checked
- Uncastable with nothing on the stack: **pass** (new).
- One target illegal, either side → the other half still happens: **pass** (new).
- Both targets illegal → countered by game rules, no `SpellResolved`: **pass** (new).
- Permanent returns to its *owner's* hand, not the controller's: **pass** by construction (`obj.owner` keys the
  hand); not separately tested, and no card in this set steals a permanent and then bounces it.
- Countering a flashback spell exiles it: covered generally by `fizzle.rs:202`, and now reached by this card
  through the shared pipeline where before it was reached through a copy.

### Test coverage
- counters and bounces (happy path): `cards_removal_and_bounce.rs:256`
- ruling 1, both halves need a target: `cards_removal_and_bounce.rs:282` (new)
- ruling 2, all three combinations: `fizzle.rs:194` (new)
- no card removes a spell from the stack itself: `card_data_invariants.rs` (new)

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1396 passing.

