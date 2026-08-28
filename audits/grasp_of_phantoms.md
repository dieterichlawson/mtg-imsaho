## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/58/grasp-of-phantoms?utm_source=api
**Type line**: `Sorcery` — {3}{U}
**Oracle text**:
```
Put target creature on top of its owner's library.
Flashback {7}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Put target creature **on top of** its owner's library" — inserted at position
  0 of the owner's `library_order`, not appended to the bottom: PASS
- "its **owner's** library", so a stolen creature goes to its owner's: PASS
- A token put on top of a library ceases to exist (CR 704.5e): PASS
- Flashback {7}{U}, and a sorcery's flashback keeps sorcery timing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Top-of-library placement and the flashback: `cards_flashback.rs`, `cards_bounce.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/58/grasp-of-phantoms?utm_source=api
**Type line**: `Sorcery` — {3}{U}
**Oracle text**:
```
Put target creature on top of its owner's library.
Flashback {7}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
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

Two, both engine-level; both fixed.

**1. A library's order is a hand-maintained index, and nothing owned it.**

Every zone but one is derived from `state.objects`, so it cannot disagree with
itself. A library is the exception: its order lives in
`PlayerState::library_order`, a `Vec<ObjectId>` beside the objects, and
`move_object` maintained the zone but never the order. Putting a card into a
library was therefore two half-steps a caller had to remember to take in step,
and taking one out was two more.

- Oracle text says: `Put target creature on top of its owner's library.`
- Code did: `state.move_object(*target_id, Zone::Library, registry);` followed
  by `state.get_player_mut(owner).library_order.insert(0, *target_id);` —
  correct, and copied verbatim from `PendingEffect::PutOnTopOfLibrary`, which
  is the general hook for exactly this.

The concrete bug the split allows: **a token put on top of a library leaves a
phantom card there.** CR 111.7 makes a token that changes zones cease to exist,
and SBA 704.5d does the removing — `state.objects.remove(&id)`, which empties
every derived zone and leaves `library_order` alone. The id stays listed. Its
owner's library is one card deeper than it is; `draw_top_card` returns an id
with no object behind it, `move_object` silently does nothing with it, and the
draw comes up empty: the hand does not grow, and a player who should have
decked out does not. Grasp of Phantoms can target any creature, so a token is
an ordinary target for it.

Fixed by giving both halves an owner in `state.rs`:

- `GameState::put_into_library(id, LibraryPosition, registry)` does the zone
  change and the position together, and makes the position an argument the
  caller has to name rather than whichever of `insert(0, ..)` or `push(..)`
  came to hand. CR 701.15a and 701.17a are separate keyword actions precisely
  because a library has an order and an effect has to pick one. Six call sites
  converted: this card, `PendingEffect::PutOnTopOfLibrary`, Memory's Journey,
  Mirror-Mad Phantasm, and both mulligan paths.
- `move_object` takes a card out of the order as it leaves the library, where
  it cannot be forgotten. Four call sites had been doing it by hand
  (`mill_one`, `finish_library_search`, Caravan Vigil, Bitterheart Witch) and
  no longer need to. `mill_one` loses its now-unused `player` parameter.
- `GameState::cease_to_exist(id)` replaces the bare `objects.remove(&id)` in
  SBA 704.5d and purges the id from every library's order — an object that
  stops existing is the one way a listed id can go stale without a zone change.
- `test_suite_guards.rs::only_the_library_helper_puts_a_card_into_a_library`
  fails the build on any `library_order` `insert`/`push`/`extend`/`retain`
  outside `state.rs`. `drain` stays allowed: Forbidden Alchemy and Mulch take
  the revealed cards out of the order while they are still in the zone, which
  is how "look at the top four" is modelled.

**2. Neither word in "on top of its **owner's** library" was under test.**

`grasp_of_phantoms_puts_creature_on_top_of_library` bounced a creature owned
and controlled by the same player into an *empty* library, so `insert(0, ..)`
and `push(..)` were the same operation and owner and controller were the same
player. Both mutations passed. Rewritten as
`grasp_of_phantoms_puts_creature_on_top_of_its_owners_library`: two cards
already in the library, and a creature owned by P1 while P0 controls and
bounces it. A stolen creature goes home, not to the thief's library.

### Card data

`{3}{U}` Sorcery, no subtypes, flashback `{7}{U}` — all four pinned pool-wide
against the Scryfall cache by `card_data_invariants.rs`
(`type_lines_...`, `mana_costs_and_printed_pt_...`, `flashback_costs_...`).
`TargetRequirement::Creature` matches "target creature" with no restriction.
No `is_valid_target` override, correctly: the card restricts nothing.

The redundant `zone == Battlefield` preamble in `on_resolve` is left alone, as
for the other cards carrying one: CR 608.2b fizzles the spell before
`on_resolve` is reached, so the branch is unreachable-false rather than wrong.
Removing all ~30 of them is one sweep, not thirty audits.

### Tricky interactions checked

- Token target — vanishes without leaving a phantom on the library: **was
  broken, fixed**, `cards_graveyard_interaction.rs` (see below).
- Owner vs. controller (a stolen creature) — goes to its owner's library:
  pass. `put_into_library` reads the owner before the move; `move_object` would
  also have reset `controller` to `owner` on leaving the battlefield (CR
  108.4), so this was right for two reasons.
- Top vs. bottom of a non-empty library: pass.
- Target illegal on resolution (creature removed in response) — the spell
  fizzles at the engine level (CR 608.2b) and `on_resolve` never runs: pass.
- Flashback from the graveyard, and exile afterwards (CR 702.33a): pass, and
  now sweep-covered.

### Test coverage

- top of library, and of the **owner's** library:
  `cards_graveyard_interaction.rs::grasp_of_phantoms_puts_creature_on_top_of_its_owners_library`
  (rewritten — the previous version passed both mutations)
- a token target leaves nothing behind:
  `cards_graveyard_interaction.rs::grasp_of_phantoms_puts_no_phantom_on_the_library_when_it_bounces_a_token`
  (new)
- library order has one owner:
  `test_suite_guards.rs::only_the_library_helper_puts_a_card_into_a_library` (new)
- flashback is reachable from the graveyard:
  `flashback.rs::every_flashback_card_is_offered_from_the_graveyard` (new,
  pool-wide). The module doc claimed "all 15 flashback cards"; there are 27,
  and seventeen of them — Grasp among them — had no test that cast them from a
  graveyard at all. The sweep deliberately names no cards, so a new flashback
  card is covered the day it is added.
- flashback exiles the card after it resolves: `flashback.rs::flashback_spell_is_exiled_after_resolve`
  (engine-level, via Geistflame)

### Mutations run

- `LibraryPosition::Top` → `Bottom` in this card: **fails** the rewritten test
  (`[bear, a, b]` vs `[a, b, bear]`). Passed before the rewrite.
- `put_into_library` reads `o.controller` instead of `o.owner`: **fails** the
  rewritten test (the creature lands in the thief's library). Passed before.
- SBA 704.5d purge of `library_order` removed: **fails** the token test.
- `move_object`'s removal-on-leaving-the-library deleted: **fails**
  `cards_death_triggers_and_tokens.rs`, and hangs Trepanation Blade's three
  tests — it reads `library_order[0]` and mills it in a loop, and without the
  removal the top card never changes. (This mutation was vacuous until the four
  hand-written `retain`s at the call sites were deleted; re-run afterwards.)
- Flashback withheld from sorceries in `legal/casting.rs`: **fails** the sweep,
  naming 15 cards including this one.

Suite: 1506 passing, exit 0, `cargo check --workspace --all-targets` clean.
