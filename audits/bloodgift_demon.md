## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/89/bloodgift-demon?utm_source=api
**Type line**: `Creature — Demon` — {3}{B}{B}, 5/4
**Oracle text**:
```
Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "At the beginning of **your** upkeep" — `TriggerScope::Your`, so it does not
  fire on the opponent's turn: PASS
- "**target player** draws a card and loses 1 life" — targeted, so it can point
  at yourself for the draw or at an opponent for the life: PASS
- Life **loss**, not damage, through `lose_life`: PASS
- CR 113.7a: killing the Demon in response does not counter the trigger: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The upkeep scope, the target and the life loss: `cards_complex_creatures.rs`, `trigger_dispatch.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/89/bloodgift-demon?utm_source=api
**Type line**: `Creature — Demon` — {3}{B}{B}, 5/4
**Oracle text**:
```
Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
```

**Rulings fetched**: none published for this card.

**Status**: PASS (card correct; a stale note in the test suite claimed otherwise, and is now a test)

### Code issues

**The card is correct.** No rulings are published for it, and every field and
behaviour matches the fetched text. The finding is in the test suite.

**A test's doc comment asserted a defect this card no longer has — and said so
about three others too.** `hexproof_filter.rs` carried:

```
/// NOTE: the same missing `player_has_hexproof` filter exists in
/// Bloodgift Demon (`bloodgift_demon.rs`),
/// Selhoff Occultist (`selhoff_occultist.rs`), and
/// Rage Thrower (`rage_thrower.rs`). All four cards use the
/// identical `state.players.iter()` pattern with no hexproof check.
/// One test covers the shared defect; the other three cards need
/// the same one-line fix.
```

None of the four hand-rolls a target list any more. Each declares a
`TargetRequirement` and lets the engine choose, and `stack.rs::is_target_legal`
filters player hexproof there:

| card | trigger(s) | declares |
|---|---|---|
| Falkenrath Noble | SelfDies, AnyCreatureDies | `PlayerOnly` (both) |
| Bloodgift Demon | Upkeep | `PlayerOnly` |
| Selhoff Occultist | SelfDies, AnyCreatureDies | `PlayerOnly` (both) |
| Rage Thrower | AnyCreatureDies | `PlayerOrPlaneswalker` |

A comment cannot notice when it goes out of date, so the claim is now a test:
`every_player_targeting_trigger_leaves_targeting_to_the_engine` asserts each of
those six triggers declares a player requirement. That is the mechanism — it is
*declaring* the requirement that routes targeting through the engine's hexproof
filter — so a card that went back to enumerating `state.players` itself would
fail it. Plus a behavioural test for Bloodgift Demon specifically, whose
trigger is on an upkeep rather than a death and so was not covered by the
Falkenrath Noble test.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{3}{B}{B}` | `Generic(3), Black, Black` OK |
| type | `Creature - Demon` | `Creature`, `["Demon"]` OK |
| P/T | 5/4 | `Some(5)/Some(4)` OK |
| keywords | Flying | `vec![Keyword::Flying]` OK |
| oracle text | verbatim match | OK |
| trigger | "At the beginning of your upkeep, target player draws a card and loses 1 life" | `TriggerKind::Upkeep`, `TriggerScope::Your`, `TargetRequirement::PlayerOnly` OK |

### Tricky interactions checked

- **"target player" includes yourself**, which is how the card is normally
  used. **Pass** — `PlayerOnly` with no narrowing `is_valid_target`.
- **A player with Witchbane Orb cannot be targeted** (CR 702.11b, player
  hexproof). **Pass**, and now tested for this card: with the Orb on the
  opponent there is exactly one legal target, so the engine takes it without
  prompting and the Demon's controller draws and loses the life.
- **CR 603.3b, the target is chosen as the trigger goes on the stack**, not
  during resolution. **Pass**, tested — and the test records that it used to be
  the other way round.
- **CR 113.7a, the Demon killed in response to its own trigger.** **Pass** —
  nothing in "target player draws a card and loses 1 life" is about the Demon,
  and the handler ignores its source. Tested.
- **Life *loss*, not damage.** **Pass** — `state.lose_life`, so it cannot be
  prevented or redirected as damage and does not trigger damage watchers. It
  routes through `change_life`, which emits `LifeChanged`.
- **Drawing from an empty library does not stop the life loss.** **Pass** —
  `draw_cards`'s return is deliberately ignored; the player still loses 1 and
  loses the game at the next state-based-action check (CR 104.3c), not
  immediately.
- **Only on your upkeep.** **Pass** — `TriggerScope::Your`, cross-checked
  against the printed "your" by the text-derived invariant added earlier in
  this run.
- **The order of "draws a card and loses 1 life".** The code draws first, which
  is the printed order. I could not construct a case in this engine where the
  order is observable — there is no priority between the two and no card in the
  pool watches either half — so I have not written a test that would pass for
  the wrong reason. Swapping the two lines fails nothing, and I am recording
  that rather than counting it.

### Test coverage

- draws and loses life, with the target chosen before resolution:
  `cards_upkeep_triggers_and_curses.rs::bloodgift_demon_draws_and_loses_life`
- the Demon killed in response:
  `cards_upkeep_triggers_and_curses.rs::bloodgift_demons_trigger_resolves_even_if_the_demon_dies_in_response`
- an opponent's Demon does not offer its controller's targets:
  `trigger_targets_declared.rs:45`
- fires only on its controller's upkeep: `your_upkeep_scope.rs` (both sweeps)
- **Witchbane Orb makes the opponent an illegal target**:
  `hexproof_filter.rs::bloodgift_demon_cannot_target_a_player_with_witchbane_orb` (new)
- **all four player-targeting cards leave targeting to the engine**:
  `hexproof_filter.rs::every_player_targeting_trigger_leaves_targeting_to_the_engine` (new,
  replacing the stale prose note)

Mutation-checked: giving the trigger a non-player `TargetRequirement` fails
both new tests. Swapping the draw and the life loss fails nothing, as noted
above.
