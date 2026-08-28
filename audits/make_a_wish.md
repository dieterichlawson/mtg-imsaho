## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/192/make-a-wish?utm_source=api
**Type line**: `Sorcery` — {3}{G}
**Oracle text**:
```
Return two cards at random from your graveyard to your hand.
```
**Status**: PASS

### Code issues
No issues found.

- "Return two **cards** at random" — filters `!o.is_token` (CR 109.1) and
  excludes the spell itself, which is on the stack rather than in the graveyard
  while it resolves.
- Genuinely random via `shuffle`, and `take(2)` handles a graveyard with fewer
  than two cards without panicking.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/192/make-a-wish?utm_source=api
**Type line**: `Sorcery` — {3}{G}
**Oracle text**:
```
Return two cards at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Return **two cards at random** from your graveyard to your hand" — random, not
  chosen, and the pick is made at resolution: PASS
- **Any** cards, not just creature cards — lands and spells alike: PASS
- CR 109.1: "two **cards**", so a token in the graveyard is not a candidate: PASS
- Make a Wish itself is still on the stack while it resolves, so it excludes its
  own id and cannot return itself: PASS
- A graveyard with one card returns that one rather than failing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The random return and the token exclusion: `cards_graveyard_recursion.rs`, `token_is_not_a_card.rs`

## Audit — 2026-08-28 18:50

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Make a Wish"`, https://scryfall.com/card/isd/192/make-a-wish
**Oracle text**:
```
Return two cards at random from your graveyard to your hand.
```
**Type line**: Sorcery
**Mana cost**: {3}{G}
**Rulings** (3, all 2011-09-22):
- "Make a Wish isn't put into its owner's graveyard until it is finished resolving, so it can't
  be returned by its own effect."
- "The cards aren't randomly chosen until Make a Wish resolves."
- "If you only have one card in your graveyard when Make a Wish resolves, that card will be
  returned to your hand."
**Status**: PASS (all three rulings tested; none were)

### Code issues
No issues found in `mtg-engine/src/cards/isd/make_a_wish.rs`.

`{3}{G}`, `CardType::Sorcery`, oracle text verbatim, no target requirement — "at random" is not
targeting, and nothing is chosen when the spell is cast.

Every ruling is satisfied by construction:
- The choice happens in `on_resolve`, so nothing is picked until it resolves.
- `o.id != object_id` keeps the spell out of its own effect. Belt-and-braces: it is on the stack
  while resolving and so is not in the graveyard to be found — but the filter says so anyway.
- `state.choose_at_random(&gy_cards, 2)` returns all of them when there are fewer than two,
  which is what "at random" does with a short list, and is exactly ruling three.

Also right, and not from a ruling: `state.is_card` excludes tokens (CR 109.1 — the card says
"cards"), and `objects_in_zone(Graveyard, controller)` is keyed by owner, which is what "your
graveyard" means (CR 404.3).

The randomness comes from the game's seeded stream, which is what lets the last test below say
anything.

### Tricky interactions checked
- **Exactly two, from three**: PASS.
- **One card, and none**: PASS — ruling three, and the spell finishes either way.
- **"Your" graveyard**: PASS.
- **"At random"**: PASS, and the pair really varies with the seed.
- **The spell cannot return itself**: by construction, twice over.
- **A token in the graveyard**: excluded by `is_card`. Not tested — a token is swept out of a
  graveyard by SBA 704.5e, so staging one there takes a state the game does not reach.

### Test coverage
- exactly two of three come back: `cards_lands_and_mana_sources.rs:582 make_a_wish_returns_cards_from_graveyard`
- one card, and none: `cards_lands_and_mana_sources.rs:~607 make_a_wish_returns_what_is_there_when_the_graveyard_is_short` (NEW)
- an opponent's graveyard is untouched: `cards_lands_and_mana_sources.rs:~632 make_a_wish_does_not_reach_an_opponents_graveyard` (NEW)
- the two are chosen at random: `cards_lands_and_mana_sources.rs:~652 make_a_wish_picks_its_two_at_random` (NEW)

Mutation-checked: reading every graveyard fails the opponent test; taking the first two instead
of choosing at random fails the randomness test. The original test passed both — three cards of
your own, two returned, so "whose graveyard" and "which two" were both invisible to it.

### Changes made
- `cards_lands_and_mana_sources.rs`: three new tests, one per ruling plus the word "random". No
  code change.
