## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/14/feeling-of-dread?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Tap up to two target creatures.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Up to two** target creatures" — castable with zero, one or two: PASS
- Tapping an attacking creature does not remove it from combat (CR 506.4c), so
  this is a blocker-remover rather than a combat trick: PASS
- One of two targets becoming illegal leaves the other still tapped: PASS
- Flashback {1}{U} is a different colour from the {1}{W} front cost: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Up-to targeting and the flashback: `cards_flashback.rs`, `combat_rules.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/14/feeling-of-dread?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Tap up to two target creatures.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2011-09-22] If Feeling of Dread targets two creatures, and one of them is an illegal target by the time Feeling of Dread resolves, the other creature will still be tapped.

**Status**: ISSUE (fixed)

### Code issues

**The card tapped a permanent by writing the field, and so did fourteen other
places.**

```rust
if let Some(obj) = state.get_object_mut(*target_id) {
    if obj.zone == Zone::Battlefield {
        obj.tapped = true;
    }
}
```

`GameEvent::Tapped` exists and was pushed at exactly two of the sixteen sites
that tapped something — declaring an attacker and paying a `{T}` cost. The
other fourteen, this card among them, were silent. Nothing watches `Tapped`
today, so nothing is visibly broken; what is broken is that the event has no
owner, which is the state the untap side was in earlier in this audit before
`GameState::untap` and its guard.

The raw write also loses CR 701.21a — *"Only untapped permanents can be
tapped"* — twice over. Tapping an already-tapped permanent should do nothing
at all, and the field being already true hides that: the write is invisible,
but the event it should not have sent is not. Both sites that did emit sent
one unconditionally.

Fixed with two named operations in `state.rs`, because there are two game
concepts here and one field:

- `GameState::tap(id)` — a permanent *becomes* tapped. Applies CR 701.21a and
  emits `Tapped`. Eleven sites: this card, Avacynian Priest, Claustrophobia,
  Civilized Scholar's "tap Homicidal Brute, then transform it", Skirsdag High
  Priest (two), regeneration (CR 701.15), declaring attackers, the forced
  attackers added under a "must attack" requirement, paying a `{T}` activation
  cost, and tapping a mana source.
- `GameState::arrives_tapped(id)` — a permanent *arrives* tapped and nothing
  tapped it, so no event. Four sites: the enters-tapped replacement
  (CR 614.1c), a land put onto the battlefield tapped by a library search, and
  the tokens Army of the Damned and Kessig Cagebreakers create "tapped" /
  "tapped and attacking" — thirteen Zombies would otherwise be thirteen tap
  events.

`test_suite_guards.rs::only_the_tap_helpers_tap_a_permanent` fails the build on
any remaining `tapped = true`, with one documented exception: `e.tapped = true`
in `cards/helpers.rs` writes the *entering event* an enters-tapped replacement
returns, where nothing has entered yet and there is no object to tap.

(The helper was first called `enters_tapped`, which collided with
`replacement_effects.rs::replacement_has_exactly_one_mechanism` — that is the
name of a `CardBehavior` hook the single `replace_event` mechanism replaced,
and the guard is right to be blunt about it coming back. Renamed to
`arrives_tapped`.)

### Card data

`{1}{W}` Instant, flashback `{1}{U}`,
`TargetRequirement::UpToTargets(2, Creature)` for "up to two target
creatures". All pinned pool-wide by `card_data_invariants.rs`, and the
flashback is reachable from the graveyard per the sweep added during the Grasp
of Phantoms audit. The redundant `zone == Battlefield` gate is kept, as for
the other cards carrying one.

CR 601.2c — "the same target can't be chosen multiple times" — is honoured on
every path the engine offers: `UpToTargets` enumerates `target_combinations`,
which are combinations, so the two targets are always distinct.

### Tricky interactions checked

- The ruling — two targets, one illegal on resolution, the other still taps:
  pass. CR 608.2b counters a spell only when *every* target is illegal, and an
  illegal one arrives as `Target::Illegal`, which the loop skips.
- Tapping an already-tapped creature: **now no event**, and previously would
  have been one at the two sites that emitted.
- A creature that left the battlefield is untapped by CR 400.7 anyway, so the
  "one target left" case cannot be confused with "it was tapped and then
  untapped".
- The six generic flashback rulings are engine-level and covered by
  `flashback.rs`.

### Test coverage

- taps its target: `flashback.rs::feeling_of_dread_taps_creature`
- the ruling, one of two targets gone:
  `flashback.rs::feeling_of_dread_taps_the_target_that_is_still_there` (new)
- CR 701.21a, tapping what is already tapped:
  `tap_cost_legality.rs::tapping_an_already_tapped_permanent_is_not_an_event` (new)
- arriving tapped is not being tapped:
  `tap_cost_legality.rs::a_permanent_that_arrives_tapped_is_not_tapped_by_anything` (new)
- one verb per concept, enforced:
  `test_suite_guards.rs::only_the_tap_helpers_tap_a_permanent` (new)
- flashback reachable from the graveyard:
  `flashback.rs::every_flashback_card_is_offered_from_the_graveyard`

### Mutations run

- `tap` emits unconditionally instead of only when the permanent was untapped:
  **fails** the CR 701.21a test.
- `arrives_tapped` delegates to `tap`: **fails** the arrival test.
- The card taps only its first target (`targets.iter().take(1)`): **fails** the
  ruling test, passes the single-target one.
- Put a raw `t.tapped = true` back into Claustrophobia: the guard **fails** and
  names the file and line.

Suite: 1519 passing, exit 0, `cargo check --workspace --all-targets` clean.
