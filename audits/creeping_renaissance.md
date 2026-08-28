## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/174/creeping-renaissance?utm_source=api
**Type line**: `Sorcery` — {3}{G}{G}
**Oracle text**:
```
Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: ISSUE

### Code issues
See below.

- Oracle text says: `Return all cards of the chosen type from your graveyard to your hand.`
- The resolution handler lives in the engine (`choices.rs`, the `ChooseCardType`
  arm) and filtered the graveyard by `has_card_type` alone.
- CR 109.1: a token is not a card, and a token sits in a graveyard until the next
  state-based action check. Same defect as the eight card-side ones fixed earlier
  in this audit, one layer down — the first sweep only looked at `src/cards/isd/`.
- Fixed: the filter now goes through `state.is_card`.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/174/creeping-renaissance?utm_source=api
**Type line**: `Sorcery` — {3}{G}{G}
**Oracle text**:
```
Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

Ruling: "The permanent types are artifact, creature, enchantment, land, and planeswalker."
`on_resolve` offers exactly those five and nothing else. The chosen type is
applied in `engine/actions/choices.rs:129` over
`objects_in_zone(Zone::Graveyard, controller)` — which filters by *owner*, so
"your graveyard" is right — and filters `state.is_card(*id)` so a token in the
graveyard is not returned (CR 109.1). "Return **all** cards", not a choice: no
subset prompt, every match moves.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_graveyard_interaction.rs` — chosen-type return and the token exclusion.

## Audit — 2026-08-28 18:55

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Creeping Renaissance"`, https://scryfall.com/card/isd/174/creeping-renaissance
**Oracle text**:
```
Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Sorcery
**Mana cost**: {3}{G}{G}   **Keywords**: Flashback
**Rulings**: 7 — six generic flashback, and one specific: "The permanent types are artifact,
creature, enchantment, land, and planeswalker."
**Status**: ISSUE (the card's effect was living in the engine)

### Code issues

**The engine implemented this card's effect.**

- Code did: `on_resolve` raised a `ChooseCardType` prompt and stopped there. The *effect* —
  "return all cards of the chosen type from your graveyard to your hand" — was the body of the
  engine's `ChooseCardType` arm in `engine/actions/choices.rs`.
- Creeping Renaissance is the only card in the set that raises that prompt, so the handler had
  quietly become this one card's `on_resolve`, sitting in the engine. A second card that named
  a permanent type for any other reason would have had to be special-cased beside it.

The codebase already had the shape for this: a `YesNo` prompt is generic and the engine
dispatches the answer back to the card through `on_yes_no_choice`. There was no equivalent for
a card-type choice, so I added `on_card_type_choice` and moved the effect onto the card. The
handler now validates the index (which it gained earlier in this pass) and calls the card.

Behaviour is unchanged — the full suite is green across the move, which is what a refactor
should look like.

Card data is correct: `{3}{G}{G}`, `CardType::Sorcery`, `flashback_cost: Some({5}{G}{G})`,
oracle text verbatim, no target requirement ("choose" is not "target"). The five options are
exactly the five permanent types the ruling lists.

### Tricky interactions checked
- **All five permanent types are offered, and each returns its own kind**: PASS.
- **"Your graveyard"**: PASS — keyed by owner (CR 404.3).
- **"Cards", so not tokens**: `is_card` excludes them (CR 109.1). Not tested — a token is swept
  out of a graveyard by SBA 704.5e, so staging one there takes a state the game does not reach.
- **"Choose", not "target"**: PASS — nothing is chosen until resolution, and it cannot be
  responded to after the choice.
- **An index past the end of the options**: refused, and the prompt stands (fixed earlier in
  this pass; it used to fall through to "Creature").
- **A card that is two permanent types**: `has_card_type` is a membership test, so a land
  creature comes back under either. Nothing in this pool is both.
- **Flashback**: PASS, tested.

### Test coverage
- creatures come back: `cards_complex_creatures.rs:725 creeping_renaissance_returns_creatures_from_graveyard`
- only the chosen type: `cards_complex_creatures.rs:759 creeping_renaissance_only_returns_chosen_type`
- all five types, and only your own graveyard:
  `cards_complex_creatures.rs:~800 creeping_renaissance_returns_the_chosen_type_from_your_graveyard_only` (NEW)
- cast by flashback, then exiled: `cards_complex_creatures.rs:~810 creeping_renaissance_flashback_exiles`

Mutation-checked: reading every graveyard fails the new test; mapping "Land" to `CardType::Creature`
fails it too. The two existing tests pass both mutations — they use Creature and Enchantment
only, with nothing of the opponent's on the board, so neither the other three options nor the
word "your" was visible to them.

### Changes made
- `cards/mod.rs`: new `on_card_type_choice` hook.
- `engine/actions/choices.rs`: the `ChooseCardType` arm dispatches to the card instead of
  implementing an effect.
- `creeping_renaissance.rs`: the effect, where it belongs.
- `cards_complex_creatures.rs`: the five-type / your-graveyard test.
