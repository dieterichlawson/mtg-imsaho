## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/75/silent-departure?utm_source=api
**Type line**: `Sorcery` — {U}
**Oracle text**:
```
Return target creature to its owner's hand.
Flashback {4}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "to its **owner's** hand" — the hand zone is keyed by owner, so a stolen
  creature goes back to its owner rather than its controller: PASS
- A token returned to hand ceases to exist (CR 704.5e): PASS
- Auras and Equipment attached to it fall off (CR 704.5m / detach): PASS
- Flashback {4}{U}, and a sorcery's flashback keeps sorcery timing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The bounce and the flashback: `cards_flashback.rs`, `cards_bounce.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/75/silent-departure?utm_source=api
**Type line**: `Sorcery` — {U}
**Oracle text**:
```
Return target creature to its owner's hand.
Flashback {4}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2024-11-08] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2024-11-08] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2024-11-08] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2024-11-08] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2024-11-08] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2024-11-08] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.

**Status**: PASS

### Code issues

No issues found. The card needed no change.

### Card data

`{U}` Sorcery, flashback `{4}{U}`, `TargetRequirement::Creature` for "target
creature", `move_object(target, Zone::Hand)` for the bounce. Cost, type line
and flashback cost pinned pool-wide by `card_data_invariants.rs`, and the
graveyard cast is covered by the flashback sweep. No `is_valid_target`
override, and none wanted: "target creature" restricts nothing further, and
CR 608.2b re-checks creature-ness at the engine level.

**"its owner's hand" needs no extra work here, unlike "its owner's library".**
A hand is derived from `objects` by owner (`Zone::Hand => obj.owner == player`
in `objects_in_zone`), so moving the object is the whole of it. Grasp of
Phantoms had to name the owner explicitly only because a library keeps a
separate order that has to be told where to put the card — the audit that
found that is why this one is worth stating rather than assuming.

The card writes `move_object` rather than going through
`PendingEffect::ReturnToHand`, which does the same move plus a log line. Only
Angel of Flight Alabaster uses that effect, and the difference is one log
line, so this is a two-line duplication rather than a second mechanism —
noted, not changed.

### Tricky interactions checked

- "its **owner's** hand" for a stolen creature: pass, and untested until now.
  The existing test bounces a creature its owner also controls, where the word
  cannot be seen.
- A bounced token: goes to a hand and then ceases to exist (CR 111.7 /
  SBA 704.5d), so nobody gets a card out of it. Pass, untested until now.
- The creature comes back as a new object (CR 400.7) with no damage, counters
  or auras: `move_object` handles it, and `zone_change_reset_object.rs` covers
  the rule.
- Fizzle when the target leaves in response: the general mechanism, covered by
  `fizzle.rs` for the single-target shape.
- Sorcery timing on the flashback cast: engine-level, `flashback.rs`.

### Test coverage

- bounces a creature: `cards_removal_and_bounce.rs::silent_departure_bounces_creature`
- to its **owner's** hand:
  `cards_removal_and_bounce.rs::silent_departure_returns_the_creature_to_its_owners_hand` (new)
- a token leaves nothing behind:
  `cards_removal_and_bounce.rs::silent_departure_on_a_token_leaves_nothing_behind` (new)
- flashback reachable from the graveyard:
  `flashback.rs::every_flashback_card_is_offered_from_the_graveyard`

### Mutations run

- The card reassigns the creature's owner to the caster before the move:
  **fails** the owner's-hand test, and only that one.
- SBA 704.5d removes a vanished token only from a graveyard, not from any zone
  off the battlefield: **fails** the token test, and only that one.
- The card moves the creature to the graveyard instead of a hand: **fails**
  both bounce tests.
- (A first attempt at the token mutation replaced `cease_to_exist(id)` with a
  no-op, which hung the suite — the SBA loop keeps setting `took_action` while
  the token is still there. It shows the call is load-bearing but is a poor
  mutation; redone as the zone-filter change above.)

Suite: 1542 passing, exit 0, `cargo check --workspace --all-targets` clean.
