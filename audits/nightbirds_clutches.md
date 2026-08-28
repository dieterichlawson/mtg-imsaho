## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/154/nightbirds-clutches?utm_source=api
**Type line**: `Sorcery` — {1}{R}
**Oracle text**:
```
Up to two target creatures can't block this turn.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Up to two** target creatures can't block this turn" — a blocking
  restriction until end of turn, not a tap: PASS
- It applies whether or not the creature is untapped, unlike tapping it: PASS
- Flashback {3}{R}, and a sorcery's flashback keeps sorcery timing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The block restriction and the flashback: `cards_flashback.rs`, `combat_rules.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/154/nightbirds-clutches?utm_source=api
**Type line**: `Sorcery` — {1}{R}
**Oracle text**:
```
Up to two target creatures can't block this turn.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.

**Status**: ISSUE (fixed)

### Code issues

**One, in the engine: `can_block` answered half its own question.**

Two things can stop a creature blocking, and they lived in different places:

```rust
// state.rs
pub fn can_block(&self, creature_id, registry) -> bool {
    !self.has_effect(creature_id, &|e| matches!(e, ContinuousEffect::PreventBlock { .. }), registry)
}

// combat.rs, can_block_at_all
if !state.can_block(blocker_id, registry) {
    return false;
}
// "Can't block this turn" (e.g., Nightbird's Clutches).
!state.until_end_of_turn.iter().any(|e| matches!(e,
    crate::state::TemporaryEffect::CantBlock { target } if *target == blocker_id
))
```

- Oracle text says: `Up to two target creatures can't block this turn.`
- `state.can_block` says: only whether a *static* ability stops it.

Gameplay was correct — `can_block_at_all` joined the two halves, and both
`eligible_blockers` and `can_block_attacker` go through it — but the query
named for the question gave the wrong answer to anyone who asked it directly,
and five tests already use it as an assertion helper. `can_block` now covers
both, and `can_block_at_all` delegates to it rather than carrying its own
scan, so there is one place that answers "can this creature block".

### Card data

`{1}{R}` Sorcery, flashback `{3}{R}`,
`TargetRequirement::UpToTargets(2, Creature)` for "up to two target
creatures", and `TemporaryEffect::CantBlock` per target — all matching, with
cost, type line and flashback cost pinned pool-wide by
`card_data_invariants.rs` and the graveyard cast covered by the flashback
sweep. No `is_valid_target` override, correctly: the card restricts nothing.
The redundant `zone == Battlefield` gate is kept, as elsewhere.

CR 601.2c is honoured on every path the engine offers: `UpToTargets`
enumerates `target_combinations`, so the two targets are always distinct.

### Tricky interactions checked

- Both targets stopped, and nobody else: pass — and untested until now; every
  existing test used one target, which a card that stopped after the first
  would pass.
- "this turn" — the restriction goes with the turn: pass.
- Asking `state.can_block` directly: **was half an answer, fixed**.
- A target that leaves the battlefield and returns: the until-end-of-turn
  effect is dropped by `move_object` (CR 400.7 — a new object), covered by
  `until_eot_object_identity.rs`.
- Declaring the creature as a blocker anyway: refused at
  `declare_blockers_with_registry`, not merely absent from the offered list.
- Zero targets: "up to two" permits it and the spell resolves doing nothing.
- The six rulings are all the generic flashback ones; nothing here is specific
  to the effect.

### Test coverage

- can't block, offered list and submitted declaration:
  `combat.rs::a_creature_that_cant_block_this_turn_is_rejected_as_a_blocker`
  (extended to assert `can_block` itself)
- both targets, and only them:
  `combat.rs::nightbirds_clutches_stops_both_of_its_targets_and_no_one_else` (new)
- the restriction ends with the turn:
  `combat.rs::nightbirds_clutches_wears_off_at_end_of_turn` (new)
- the effect is recorded, and the creature drops out of `eligible_blockers`:
  `cards_morbid_and_ltb.rs::nightbirds_clutches_prevents_blocking`
- flashback reachable from the graveyard:
  `flashback.rs::every_flashback_card_is_offered_from_the_graveyard`

### Mutations run

- The card stops after its first target (`targets.iter().take(1)`): **fails**
  the two-target test, passes every pre-existing one.
- `can_block` drops the until-end-of-turn half again: **fails** three tests in
  `combat.rs` — which is the point, since `can_block_at_all` now delegates
  entirely rather than keeping its own copy.
- `until_end_of_turn.clear()` removed from the cleanup step: **fails** the
  wears-off test, passes the two-target one.

Suite: 1521 passing, exit 0, `cargo check --workspace --all-targets` clean.
