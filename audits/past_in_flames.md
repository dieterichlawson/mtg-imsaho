## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/155/past-in-flames?utm_source=api
**Type line**: `Sorcery` — {3}{R}
**Oracle text**:
```
Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "Each instant and sorcery **card** in your graveyard" — filters `!o.is_token`
  and reads types from the card's face.
- CR 702.33a: the granted flashback cost equals the card's mana cost, so a card
  with no mana cost is skipped rather than handed a free one — covered by
  `flashback_multiple_instances.rs`.
- Excludes itself, which is on the stack while resolving.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/155/past-in-flames?utm_source=api
**Type line**: `Sorcery` — {3}{R}
**Oracle text**:
```
Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Past in Flames affects **only cards in your graveyard at the time it
  resolves**. Instant and sorcery cards put into your graveyard later in the turn
  won't gain flashback." The list is built at resolution: PASS
- CR 702.33a: "The flashback cost is equal to its **mana cost**" — a card with no
  mana cost is skipped rather than given a free flashback: PASS
- Past in Flames is still on the stack while it resolves, so it is not in its own
  list — and the engine moves it afterwards (CR 608.2m): PASS
- CR 109.1: "each instant and sorcery **card**", so tokens are excluded: PASS
- "in **your** graveyard": PASS
- Its own flashback {4}{R} is separate from the flashback it grants: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Granting flashback at resolution: `cards_flashback.rs`

## Audit — 2026-08-28 17:36

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Past in Flames"`, https://scryfall.com/card/isd/155/past-in-flames
**Oracle text**:
```
Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Sorcery
**Mana cost**: {3}{R}   **Keywords**: Flashback
**Rulings**: 12 — one specific to this card ("affects only cards in your graveyard at the time
it resolves"), the rest generic flashback plus three about costs a card in this pool cannot
have (X in mana cost, split cards, no mana cost).
**Status**: PASS (three rulings tested that were not)

### Code issues
No issues found in `mtg-engine/src/cards/isd/past_in_flames.rs`.

`{3}{R}`, `CardType::Sorcery`, `flashback_cost: Some({4}{R})`, oracle text verbatim, no target
requirement (the ability says "each", which does not target).

`on_resolve` reads the graveyard once, at resolution:
- `objects_in_zone(Zone::Graveyard, controller)` — a graveyard is keyed by *owner*
  (`state.rs:1237`), which is what CR 404.3 makes "your graveyard" mean.
- `state.is_card(o.id)` excludes tokens (CR 109.1). Vacuous here — no token is an instant — but
  correct.
- `face_data(...).card_types` for instant-or-sorcery, so a creature card in the graveyard gains
  nothing.
- `d.cost.clone()` for the flashback cost: CR 702.33a, "equal to its mana cost". A card with no
  mana cost is skipped rather than given a free one, which matches the ruling ("it has no
  flashback cost. It can't be cast this way") in effect. No card in this pool lacks a mana cost.

### Tricky interactions checked
- **"at the time it resolves"**: PASS. The grant is a list of object ids fixed at resolution, so
  a card put into the graveyard later in the turn is not in it.
- **"your graveyard"**: PASS — the owner filter above.
- **"until end of turn"**: PASS. The grant is a `TemporaryEffect::GrantFlashback` in
  `until_end_of_turn`, which cleanup clears (CR 514.2).
- **A card that already has flashback**: gets a second instance, which is right — "If a card has
  multiple instances of flashback, you may choose any of its flashback costs to pay." Covered by
  `flashback_multiple_instances.rs`, which is about exactly this card's grant sitting beside
  Think Twice's printed one.
- **Past in Flames cast via its own flashback**: it is on the stack while resolving, so it is not
  in its own list, and the `o.id != object_id` filter says so a second time. It is then exiled
  rather than returning to the graveyard.
- **Casting Past in Flames twice in a turn**: the second grant is skipped for a card that already
  has one. Strictly the card should gain a second instance; both would carry the same cost, so
  nothing is observable. Recorded, not changed.
- **A granted card cast, exiled, then returned to hand by Runic Repetition**: the grant is keyed
  to the object id and outlives the zone change, but flashback only enables a cast from a
  graveyard, so it does nothing from hand. `runic_repetition_clears_the_flashback_flag_on_the_returned_card`
  covers the flag that did matter.
- **Sorcery timing**, for this spell and for the sorceries it enables: engine-side.

### Test coverage
- the right cards gain it, at their own mana cost, and the cast is really offered:
  `flashback.rs:685 past_in_flames_grants_flashback_at_each_cards_own_cost`
- a creature card in the graveyard gains nothing: same test
- a card that arrives after resolution gains nothing:
  `flashback.rs:719 past_in_flames_does_not_reach_a_card_that_arrives_after_it_resolves` (NEW)
- an opponent's graveyard is untouched: `flashback.rs:740 past_in_flames_leaves_an_opponents_graveyard_alone` (NEW)
- the grant expires at end of turn: `flashback.rs:755 past_in_flames_flashback_grant_expires_at_end_of_turn` (NEW)
- a granted cost sitting beside a printed one: `flashback_multiple_instances.rs`
- a card cast with the granted flashback is exiled: engine-side, `flashback.rs:141`

Mutation-checked: reading every graveyard instead of the caster's kills the opponent test;
granting to any card type kills the cost test; leaving `until_end_of_turn` uncleared at cleanup
kills the expiry test.

`past_in_flames_does_not_reach_a_card_that_arrives_after_it_resolves` is **not** mutation-checked
and cannot be by a local edit: the implementation fixes a list of object ids at resolution, so
there is no nearby way to make it wrong. It stands as a regression guard against the obvious
rewrite — a continuous "instants in your graveyard have flashback" effect — which would break
the card's first ruling.

### Changes made
- `flashback.rs`: three new tests, one per untested ruling. No code change.
