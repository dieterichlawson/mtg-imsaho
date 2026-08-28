## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/54/dream-twist?utm_source=api
**Type line**: `Instant` — {U}
**Oracle text**:
```
Target player mills three cards.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Target player mills three cards" — through `mill_cards`, so creature cards
  among them emit `CreatureCardMilled`: PASS
- A library with fewer than three cards mills what it has rather than making the
  player lose: PASS
- Flashback {1}{U}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mill and the flashback: `cards_flashback.rs`, `multi_target_and_mill.rs`

## Audit — 2026-08-28 17:02

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Dream Twist"`, https://scryfall.com/card/isd/54/dream-twist
**Oracle text**:
```
Target player mills three cards.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Instant
**Mana cost**: {U}
**Keywords**: Flashback, Mill
**Status**: PASS (test coverage strengthened)

### Code issues
No issues found in `mtg-engine/src/cards/isd/dream_twist.rs`.

Card data matches line for line: cost `{U}`, `card_types: vec![CardType::Instant]`, no
subtypes, `flashback_cost: Some({1}{U})`, and the `oracle_text` field is the fetched text
verbatim including the reminder text.

Behaviour:
- `target_requirement()` is `TargetRequirement::PlayerOnly` — "target **player**", not
  "target opponent". `targeting.rs:408` offers every player who is not lost and not
  hexproof-from-this-caster, so the caster is among them (CR 702.11b: your own spells still
  reach you).
- `on_resolve` is one call to `crate::engine::mill_cards(state, *player_id, 3, "Dream Twist",
  registry)` — the shared mill helper, which takes `library_order[0]` (the top card) three
  times, stops early when the library runs out, and logs one accurate line naming the source.
- No self-cleanup: the card does not move itself off the stack. The engine owns that, and the
  flashback exile with it.

### Tricky interactions checked
- "Target player" includes yourself: PASS. Offered (`can_target_player` exempts the caster from
  hexproof) and honoured on resolve.
- Milling fewer than three from a short library (CR 701.13b — mill all, do not lose): PASS.
  `mill_cards` breaks out of the loop on an empty `library_order` and logs "(of 3 — library ran
  out)". Milling an empty library is not drawing from one, so no loss flag is set.
- Milling is not drawing: PASS. `mill_one` is a plain `move_object(Library -> Graveyard)`; the
  empty-library loss flag is only set by the draw path.
- A creature card milled is visible to watchers (CR 701.13a): PASS — the `CreatureCardMilled`
  event comes from `move_object` for *any* library-to-graveyard move, so Dream Twist gets it
  without knowing about it.
- Flashback (all six rulings are the generic flashback ones): PASS. Offering from the graveyard,
  the alternative cost, the timing restriction (instant, so any time), and the exile-instead-of-
  graveyard replacement are all engine-side.
- Instant timing: PASS — `CardType::Instant`, no card-side timing restriction.

### Test coverage
- mills three from the targeted player: `flashback.rs:246 dream_twist_mills_three`
- the target is *honoured*, not assumed — casting at yourself mills you and leaves the
  opponent alone: `flashback.rs:270 dream_twist_mills_the_caster_when_it_targets_them` (NEW).
  Both directions are needed: a resolve hard-coded to the opponent passes the first test, one
  hard-coded to the caster passes the second. Mutation-checked both ways.
- the caster is offered as a target through their own hexproof:
  `cards_lands_and_mana_sources.rs:913 can_target_self_with_hexproof`
- a hexproofed opponent is not offered: `cards_lands_and_mana_sources.rs:~890`
- short library mills all and does not lose (CR 701.13b): `cards_upkeep_triggers_and_curses.rs:77`
  (Curse of the Bloody Tome, same `mill_cards` mechanism)
- flashback offered from the graveyard for every flashback card in the pool, Dream Twist
  included: `flashback.rs:32 every_flashback_card_is_offered_from_the_graveyard`
- a flashback spell is exiled after it resolves, and after it is countered:
  `flashback.rs:141`, `flashback.rs:165`
- library-to-graveyard emits the mill event: `multi_target_and_mill.rs:229`

### Changes made
- `flashback.rs`: `dream_twist_mills_three` now stocks both libraries and asserts the caster's
  is untouched; added `dream_twist_mills_the_caster_when_it_targets_them`.
