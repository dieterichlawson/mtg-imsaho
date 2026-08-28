## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/139/desperate-ravings?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
Draw two cards, then discard a card at random.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "Draw two cards, **then** discard a card at random" — draws first, then picks
  from the whole hand, so a just-drawn card is eligible. Picking from only the
  drawn pair would be the tempting mistake.
- The discard is random rather than chosen, matching the wording.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/139/desperate-ravings?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
Draw two cards, then discard a card at random.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

Ruling: "You draw two cards and discard one randomly all while Desperate
Ravings is resolving. Nothing can happen between the two." `on_resolve` draws
then discards in one body — no `awaiting_action` between them, so no player
gets priority. The discard is genuinely random (`SliceRandom::choose`) over the
whole hand, which includes the two just drawn.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_graveyard_interaction.rs` — hand size after resolution; the randomness itself is not asserted on.

## Audit — 2026-08-28 19:19

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Desperate Ravings"`, https://scryfall.com/card/isd/139/desperate-ravings
**Oracle text**:
```
Draw two cards, then discard a card at random.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Instant
**Mana cost**: {1}{R}   **Keywords**: Flashback
**Rulings**: 7, all the generic flashback ones.
**Status**: PASS (the two words the net-count test could not see gained a test)

### Code issues
No issues found in `mtg-engine/src/cards/isd/desperate_ravings.rs`.

`{1}{R}`, `CardType::Instant`, `flashback_cost: Some({2}{U})` — off-colour, pinned by the
registry-wide cost sweep — oracle text verbatim, no target requirement.

`on_resolve`: draw two through `engine::draw_cards` (so an empty library sets the loss flag the
ordinary way), then take the hand **as it stands after the draw** and discard one chosen by
`state.choose_at_random` — the seeded stream. `discard_card` emits `Discarded`, so a discard
watcher sees it.

### Tricky interactions checked
- **"Then"**: the discard pool includes the two cards just drawn. PASS — with an otherwise
  empty hand, the discarded card is always one of the two drawn.
- **"At random"**: PASS — the choice varies with the seed.
- **Drawing from a short library**: the draws go through the shared helper; the loss is the
  draw rule's, not this card's.
- **Empty hand after the draws** (library empty, hand empty): `to_discard` is `None` and nothing
  is discarded; the loss lands at SBA. Degenerate, unasserted.
- **The Discarded event**: emitted by `discard_card`; nothing in this pool watches an
  instant-speed discard of the caster's own (Civilized Scholar watches its own draw-discard
  loop and is covered there).
- **Off-colour flashback, exile after**: engine-side, pinned generically.

### Test coverage
- net +1 hand (draw 2, discard 1): `flashback.rs:416 desperate_ravings_draws_two_discards_one`
- the pool is the post-draw hand, and the choice is random:
  `flashback.rs:~440 desperate_ravings_discards_at_random_from_the_hand_after_drawing` (NEW)
- flashback cost matches print: `card_data_invariants.rs:1907` (sweep)
- offered from the graveyard: `flashback.rs:32` (sweep)

Mutation-checked: discarding the first card instead of a random one fails the new test and only
it — the net count is identical, which is exactly why it existed alone for so long; drawing one
instead of two fails both.

### Changes made
- `flashback.rs`: the random-discard test. No code change.
