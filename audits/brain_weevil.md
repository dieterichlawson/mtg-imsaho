## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/91/brain-weevil?utm_source=api
**Type line**: `Creature — Insect` — {3}{B}, 1/1
**Oracle text**:
```
Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Sacrifice this creature**: Target player discards two cards" — the
  sacrifice is a cost, paid on activation, so the Weevil is in the graveyard
  while the ability is on the stack: PASS
- "**Activate only as a sorcery**" — `sorcery_speed_only`: PASS
- "discards **two** cards" — both, chained, and the discards go through
  `discard_card` so watchers see them: PASS
- A player with one card discards one and is not made to discard twice: PASS
- Intimidate is printed, not menace: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both discards and the sorcery-speed restriction: `auto_pick.rs:bug_brain_weevil_incomplete_discard`, `simultaneous_events.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/91/brain-weevil?utm_source=api
**Type line**: `Creature — Insect` — {3}{B}, 1/1
**Oracle text**:
```
Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
```

**Rulings fetched**:
- [2013-04-15] If you cast this as normal during your main phase, it will enter the battlefield and you’ll receive priority. If no abilities trigger because of this, you can activate its ability immediately, before any other player has a chance to remove it from the battlefield.

**Status**: ISSUE (1, fixed by adding the mechanism to the engine)

### Code issues found and fixed

**One: the card hand-rolled a two-step discard, and smuggled a player id
through a map of object ids to do it.**

- Oracle text says: `Sacrifice this creature: Target player discards two cards.
  Activate only as a sorcery.`
- Code did: discard one card, then pick the second up in `on_discard_choice`,
  carrying the target player between the two in `obj.card_state` — a
  `String -> ObjectId` map — as

```rust
obj.card_state.insert("weevil_target_player".into(), ObjectId(u64::from(target_player.0)));
```

read back with

```rust
let target_player = PlayerId(u8::try_from(raw_id.0).unwrap_or(u8::MAX));
```

Three things wrong with that, in increasing order of seriousness.

The `unwrap_or(u8::MAX)` names player 255, who does not exist. It is
unreachable today because the value going in is a `u8` widened to `u64`, but it
is the kind of fallback that quietly picks a wrong answer rather than failing.
Liliana of the Veil's file already records a bug of exactly this shape in its
own `card_state` encoding: "the previous encoding parsed a comma-joined string
of player ids *as a u64*, which silently became 0 for any game with more than
one player left to ask."

The state was written onto a permanent the engine had **already sacrificed** —
`SacrificeCost::SacrificeThis` is paid at activation, so by the time
`resolve_activated_ability` runs the Weevil is in the graveyard. It works
because nothing moves it again before the second choice, which is not a
property the card was checking.

And most of all, "discards two cards" is not one card's problem. It is a rules
action with a shape — ask, discard, ask again against the hand as it now
stands, stop when the count or the hand runs out — and the card was
reimplementing it, three hand-size branches at a time.

**Fixed by giving the engine the mechanism.** `engine::discard_cards(state,
player, count, source_id, source, registry)` joins `draw_cards` and
`mill_cards` in `cards_flow.rs`, and `ResolutionChoiceKind::ChooseCardFromHand`
gains a `remaining` count so the choice carries its own continuation. The
engine re-presents against the refreshed hand when a choice is answered with
more still to go. Brain Weevil is now two lines and has no `on_discard_choice`
and no `card_state` at all.

Where there is nothing to decide the engine does not ask: a hand no larger than
the count is discarded outright, which is both what the rules amount to and
what the card used to do for its one- and two-card cases.

The other four `ChooseCardFromHand` sites (Murder of Crows, Grimoire of the
Dead, Civilized Scholar, Liliana of the Veil) take `remaining: 1`. Liliana's
CR 101.4 queue still uses `card_state`, legitimately — several players each
choosing one card, held until they can leave together — and is a different
mechanism from a count on one player. It will get its own audit.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{3}{B}` | `Generic(3), Colored(Black)` OK |
| type | `Creature - Insect` | `Creature`, `["Insect"]` OK |
| P/T | 1/1 | `Some(1)/Some(1)` OK |
| keywords | Intimidate | `vec![Keyword::Intimidate]` OK |
| oracle text | verbatim, reminder text included | OK |
| ability | sacrifice cost, targets a player, sorcery speed | `SacrificeCost::SacrificeThis`, `TargetRequirement::PlayerOnly`, `sorcery_speed_only: true` OK |

### Tricky interactions checked

- **Ruling 2013-04-15: "you can activate its ability immediately, before any
  other player has a chance to remove it from the battlefield."** **Pass** —
  the cost has no {T} in it, and summoning sickness only restricts a {T} symbol
  in a creature's own cost (CR 302.6). Was untested; now is, with the Weevil
  explicitly summoning-sick.
- **"Target player" includes you.** **Pass** — `TargetRequirement::PlayerOnly`
  with no `is_valid_target` narrowing it. Was untested; now is.
- **A hand smaller than two** — one card goes, and a player does not lose for
  the shortfall. **Pass**, now tested, along with the empty-hand case (no
  prompt, no panic, and the sacrifice still paid because it is a cost).
- **Both discards are the targeted player's choice**, and the second is asked
  against the hand as it stands after the first. **Pass**, now tested.
- **`activated_abilities` does not check the zone.** Correct, and deliberately
  so: `legal/abilities.rs` iterates `objects_in_zone(Zone::Battlefield,
  player)`, so the engine has already answered "does this ability function
  here" (CR 113.6). Many cards in the set re-ask it themselves; this one does
  not need to, and I have not swept the others, since the duplication is inert.
- **The sacrifice is a cost, not an effect** — paid on activation, so the
  ability resolves with the Weevil already in the graveyard and cannot be
  "countered" by killing it in response. **Pass** by construction.
- **"Discards two cards" as one event.** I could not establish from an external
  source that the two cards must leave the hand simultaneously, so I have not
  changed the sequential behaviour on the strength of a guess. Nothing in this
  card pool watches individual discards in a way that could tell the
  difference.

### Test coverage

- two cards in hand, both go:
  `cards_sacrifice_and_additional_costs.rs::brain_weevil_forces_discard`
- three cards, two go: `auto_pick.rs::bug_brain_weevil_incomplete_discard`
- the discards interleave correctly with other simultaneous events:
  `simultaneous_events.rs:210`
- **the ruling — activatable the turn it arrives**:
  `::brain_weevil_can_be_sacrificed_the_turn_it_arrives` (new)
- **"target player" includes yourself**:
  `::brain_weevil_can_target_its_own_controller` (new)
- **a one-card hand, and no prompt for a non-choice**:
  `::brain_weevil_takes_the_only_card_in_a_one_card_hand` (new)
- **an empty hand**: `::brain_weevil_against_an_empty_hand_does_nothing` (new)
- **both choices belong to the targeted player, second asked against the
  refreshed hand**:
  `::brain_weevils_two_discards_are_both_the_targeted_players_choice` (new)

Mutation-checked: dropping the count to 1 fails both multi-discard tests;
disabling the engine's re-present fails the choice-ownership test; and changing
the auto-discard boundary from `<=` to `<` fails the two-card test.
