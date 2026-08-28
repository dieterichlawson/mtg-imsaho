## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/102/ghoulraiser?utm_source=api
**Type line**: `Creature — Zombie` — {1}{B}{B}, 2/2
**Oracle text**:
```
When this creature enters, return a Zombie card at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.

- "return a **Zombie card** at random from your graveyard" — reads subtypes from
  the card's face, which is the right source for a graveyard card: continuous
  effects that grant types apply to permanents, not to cards in a graveyard.
  A Zombie *token* in the graveyard has no registry face, so it is excluded and
  CR 109.1 is satisfied without a separate check.
- "Zombie card", not "Zombie creature card" — the filter is deliberately not
  restricted to creatures, and a comment says so.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/102/ghoulraiser?utm_source=api
**Type line**: `Creature — Zombie` — {1}{B}{B}, 2/2
**Oracle text**:
```
When this creature enters, return a Zombie card at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "return a Zombie card **at random**" — the choice is made at resolution, not
  by the player: PASS
- CR 109.1: "a Zombie **card**", so a Zombie token in the graveyard is not a
  candidate. The filter relied on `face_data` being None for a token; it now
  says `state.is_card` outright: PASS
- Ghoulraiser is itself a Zombie, so it can return itself if it is in the
  graveyard when the trigger resolves — no self-exclusion, and the card says
  "a Zombie card", not "another": PASS
- An empty graveyard returns nothing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The random return and the token exclusion: `cards_complex_creatures.rs`, `token_is_not_a_card.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/102/ghoulraiser?utm_source=api
**Type line**: `Creature — Zombie` — {1}{B}{B}, 2/2
**Oracle text**:
```
When this creature enters, return a Zombie card at random from your graveyard to your hand.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/102/ghoulraiser
**Oracle text**:
```
When this creature enters, return a Zombie card at random from your graveyard to your hand.
```
**Type line**: `Creature — Zombie` · **Mana cost**: `{1}{B}{B}` · **P/T**: 2/2
**Rulings**: none published for this card.
**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `ghoulraiser.rs` | |
|---|---|---|---|
| cost | `{1}{B}{B}` | `Generic(1) + Black + Black` | ok |
| types / subtypes | Creature — Zombie | matching | ok |
| P/T | 2/2 | `Some(2)`/`Some(2)` | ok |
| oracle_text | as above | byte-identical | ok |
| trigger | enters | `EntersBattlefield` + `on_enter_battlefield` | ok |

### Code issues
No issues found. Three things are right in ways worth naming:

- **"a Zombie card", not "Zombie creature card"** — the filter does not require `is_creature`, and the comment
  says so. Every Zombie in this set happens to be a creature, so the difference is inert here, but the code
  matches the text rather than the pool.
- **`face_data` for the subtype rather than `state.has_subtype`.** For a card in a graveyard this is the right
  read: an object-level subtype granted on the battlefield belongs to a permanent that no longer exists
  (CR 400.7), and `face_data` cannot pick up such a leftover. Same shape as Runic Repetition's printed
  `flashback_cost` — the narrow read is the correct one when the question is about a card rather than a
  permanent.
- **`helpers::controller_of`** (last known information, CR 608.2g) rather than the source's current controller,
  so removal in response does not eat the card advantage along with the body (CR 113.7a).

### Changes made
Nothing in the card. `mtg-engine/tests/cards_graveyard_interaction.rs` gained four tests and a small helper. The
existing coverage put exactly one Zombie in the graveyard, so an implementation returning *any* card passed it.

- `ghoulraiser_returns_only_a_zombie_card` — a Grizzly Bears stays put, and so does a Zombie **token**
  (CR 109.1).
- `a_dead_ghoulraiser_can_return_itself` — a Ghoulraiser that died with its own trigger on the stack is a Zombie
  card in that graveyard and so is one of the candidates. The card's comment states this outright; nothing
  checked it, because `ghoulraiser_returns_its_zombie_after_being_killed_in_response` leaves a second Zombie
  sitting there to be found instead.
- `ghoulraiser_picks_at_random_among_several_zombies` — three candidates, forty entries, more than one distinct
  card returned. The suite-wide `a_card_that_says_at_random_actually_randomizes` guard only checks that this
  card reaches an RNG at all, not that its selection varies.
- `ghoulraiser_with_nothing_to_return_does_nothing`.

### Mutation checks
1. Zombie restriction dropped → `ghoulraiser_returns_only_a_zombie_card` FAILED. **Discriminating.**
2. `is_card` check dropped → **vacuous.** The card's own comment predicted this: `face_data` is `None` for a
   token, so the token is filtered out by the subtype check regardless. `is_card` is belt-and-braces, said
   explicitly rather than left to that coincidence — which is the right way to write it, but it means no test
   can hold it up on its own. Recorded rather than presented as covered.
3. `shuffle` replaced by sorting on object id → `ghoulraiser_picks_at_random_among_several_zombies` FAILED.
   **Discriminating.**
4. `controller_of` replaced by the source's current controller, with a battlefield filter →
   `a_dead_ghoulraiser_can_return_itself` FAILED. **Discriminating.**

### Tricky interactions checked
- Returns a Zombie from the graveyard: **pass** (`cards_graveyard_interaction.rs:85`).
- Killed in response, the trigger still resolves: **pass** (`trigger_source_independence.rs:129`).
- The dead Ghoulraiser is itself a candidate: **pass** (new).
- A non-Zombie card and a Zombie token are both refused: **pass** (new).
- The choice is genuinely random: **pass** (new).
- Empty graveyard: **pass** (new).
- A token copy of Ghoulraiser brings its enters trigger: **pass**
  (`cards_complex_creatures.rs:1136`, via Back from the Brink).

### Test coverage
- returns a Zombie: `cards_graveyard_interaction.rs:85`
- only a Zombie card: `cards_graveyard_interaction.rs:114` (new)
- can return itself when dead: `cards_graveyard_interaction.rs:141` (new)
- at random: `cards_graveyard_interaction.rs:162` (new)
- nothing to return: `cards_graveyard_interaction.rs:190` (new)
- survives removal in response: `trigger_source_independence.rs:129`
- enters trigger on a token copy: `cards_complex_creatures.rs:1136`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1436 passing.

