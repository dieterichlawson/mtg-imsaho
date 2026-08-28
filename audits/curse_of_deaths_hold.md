## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/94/curse-of-deaths-hold?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {3}{B}{B}
**Oracle text**:
```
Enchant player
Creatures enchanted player controls get -1/-1.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Creatures enchanted player controls get -1/-1" — a *static* ability, so it
  applies to creatures that arrive later too, unlike a spell's anthem
  (CR 611.2c). `EffectScope::Global(ControlledByAttachedPlayer)` is re-evaluated
  rather than snapshotted: PASS
- A 1/1 the cursed player controls dies to state-based actions: PASS
- It follows control changes — the filter is "controlled by", read live: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The debuff and its scope: `curse_and_equip_scope.rs`, `snapshot_anthems.rs:a_static_anthem_stops_when_its_source_leaves`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/94/curse-of-deaths-hold?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {3}{B}{B}
**Oracle text**:
```
Enchant player
Creatures enchanted player controls get -1/-1.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

### Code issues

**One, in the engine, and it was a landmine rather than a live bug.**

`CreatureFilter::ControlledByAttachedPlayer` — the filter this card's whole
text rests on — had two implementations that disagreed, and the wrong one was
labelled as a guess:

```rust
CreatureFilter::ControlledByAttachedPlayer => {
    // This filter requires knowing the source object's attached_to_player.
    // It's resolved in effect_applies_to which has source_id.
    // If called directly from matches_filter without source context,
    // fall back to Opponents (the common case for curses).
    creature.controller != source_controller
}
```

- Oracle text says: `Creatures enchanted player controls get -1/-1.`
- The fallback says: creatures controlled by anyone who is not the Curse's
  controller.

Those are different sets whenever a Curse is on the player who controls it,
which "Enchant player" allows — it names a player, not an opponent — and in a
multiplayer game they differ for every player who is neither.

The card is not broken today: `EffectScope::Global` is special-cased in
`effect_applies_to`, which has the source object and reads
`attached_to_player` correctly, and both Curses in the set use `Global`. What
made it worth fixing rather than noting is that the correct answer was
reachable only through that special case. Changing a Curse to `GlobalOther`,
or wrapping the filter in `And([..])`, routes it to `matches_filter` and
silently swaps in the guess — with no test able to see it, because the filter
had no honest implementation to test.

Fixed by giving `matches_filter` the `source_id` it said it lacked
(`matches_filter(creature_id, filter, source_id, source_controller, registry)`),
implementing the arm there, and deleting the special case from
`effect_applies_to` so `Global` and `GlobalOther` both go through the one
implementation. Five call sites updated. Two of them — Moonmist's
`PreventCombatDamageExcept` and Spare from Evil's `GrantProtection` — are
until-end-of-turn effects with no permanent behind them any more; each passes
the object being filtered as its own source, with a comment saying why that is
sound (only this arm would notice, and no grant in the set uses it: a Curse's
filter is a static ability, not a grant).

### Card data

`{3}{B}{B}`, `Enchantment — Aura Curse` with both subtypes,
`ContinuousEffect::ModifyPT { power: -1, toughness: -1 }` for "get -1/-1",
`TargetRequirement::PlayerOnly` for "Enchant player", and `resolve_curse` for
the attachment. `EffectScope::Global` rather than `GlobalOther` is right: the
Curse is not a creature, so there is nothing for "other" to exclude, and
"Creatures enchanted player controls" excludes nothing. Cost and type line are
pinned pool-wide by `card_data_invariants.rs`. The `Enchant` keyword Scryfall
lists is one of the keywords `keywords_say_what_scryfall_says` deliberately
does not model.

No triggered or activated abilities, so nothing to declare and nothing to
present to the player; a static ability has no stack presence to describe.

### Tricky interactions checked

- The Curse on its own controller — "enchanted player", not "your opponents":
  pass (through `effect_applies_to` today; now through the one filter).
- -1/-1 puts an X/1 into the graveyard as SBA 704.5f, with no damage and no
  destruction involved: pass.
- The effect ends when the Curse leaves the battlefield: pass, and defended
  twice (see below).
- Layers: the -1/-1 is 7c and applies on top of a characteristic-defining base
  (7a), not instead of it — Boneyard Wurm with three creature cards in the
  graveyard is a 2/2 under the Curse. Pass. Modifiers are summed rather than
  ordered by layer, which is indistinguishable here: 7c and 7d are both
  addition.
- Two Curses stack to -2/-2: falls out of the same summation.
- CR 303.4h, a Curse that cannot legally enchant its target: covered for this
  card by `player_protection.rs`, which uses it against protection from black.

### Test coverage

- the opposing-player case:
  `cards_upkeep_triggers_and_curses.rs::curse_of_deaths_hold_debuffs_opponent_creatures`
- the enchanted player *is* the controller — the board on which "cursed
  player" and "everyone who isn't me" give opposite answers:
  `…::curse_of_deaths_hold_debuffs_its_own_controller_when_it_enchants_them` (new)
- SBA death and the effect ending:
  `…::curse_of_deaths_hold_kills_one_toughness_creatures_and_stops_when_it_leaves` (new)
- subtracting from a defined power:
  `…::curse_of_deaths_hold_subtracts_from_a_defined_power` (new)
- can't enchant a player with protection from black: `player_protection.rs`

### Mutations run

- Restore the "opponents" guess as the single filter arm: **fails** the
  self-curse test (3 vs 2), passes the other two — which is exactly the
  distinction the fix is about.
- `toughness: -1` → `0`: **fails** all three.
- The effect ending when the Curse leaves is held up by two independent
  guards, and the test only sees it when both go: removing `walk_effects`'
  `source.zone != Zone::Battlefield` alone passes (because `move_object`
  clears `attached_to_player`), removing that clear alone passes (because of
  the zone gate), and removing both **fails** (1 vs 2). Recorded as measured
  rather than claimed: the assertion is real but does not isolate either
  mechanism. (An earlier run of the both-removed mutation reported a pass; it
  had edited the wrong one of the two identical zone gates in `state.rs` and
  proves nothing.)

Suite: 1513 passing, exit 0, `cargo check --workspace --all-targets` clean.
