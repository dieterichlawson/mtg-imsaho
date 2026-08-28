## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/172/bramblecrush?utm_source=api
**Type line**: `Sorcery` — {2}{G}{G}
**Oracle text**:
```
Destroy target noncreature permanent.
```

**Status**: PASS

### Code issues
No issues found.

- "Destroy target **noncreature** permanent" — `is_valid_target` requires the
  permanent's face **not** to include Creature, so an animated permanent or a
  creature is excluded.
- Destroys through the pipeline, so indestructible applies; the oracle says
  destroy, not exile.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/172/bramblecrush?utm_source=api
**Type line**: `Sorcery` — {2}{G}{G}
**Oracle text**:
```
Destroy target noncreature permanent.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **noncreature** permanent" — lands, artifacts, enchantments and
  planeswalkers are all legal; a creature is not, and an artifact *creature* is
  not either: PASS
- The restriction is re-checked on resolution, so a permanent that became a
  creature in response makes it fizzle (CR 608.2b): PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The noncreature filter: `cards_removal.rs`, `fizzle.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/172/bramblecrush?utm_source=api
**Type line**: `Sorcery` — {2}{G}{G}
**Oracle text**:
```
Destroy target noncreature permanent.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/172/bramblecrush
**Oracle text**:
```
Destroy target noncreature permanent.
```
**Type line**: `Sorcery` · **Mana cost**: `{2}{G}{G}`
**Rulings**: none published for this card.
**Status**: ISSUE (fixed) — a restated target requirement, removed.

### Card data
| field | oracle | `bramblecrush.rs` | |
|---|---|---|---|
| cost | `{2}{G}{G}` | `Generic(2) + Green + Green` | ok |
| types | Sorcery | `vec![CardType::Sorcery]` | ok |
| oracle_text | as above | byte-identical | ok |
| targeting | "target noncreature permanent" | `PermanentWithFilter(Noncreature)` | ok |
| effect | destroy | `helpers::resolve_destroy` → `destruction::try_destroy` | ok |

### Code issues

**`is_valid_target` restated the target requirement.** Removed.

- Code was: `o.zone == Zone::Battlefield` plus
  `state.face_data(obj.id, registry).is_some_and(|d| !d.card_types.contains(&CardType::Creature))`.
- The requirement already says it. `legal_actions` enumerates `all_objects_in_zone(Battlefield)` through
  `matches_target_filter`, whose `Noncreature` arm is `!state.is_creature(obj.id, registry)`; and
  `stack::is_target_legal` re-runs both the zone check and the filter on the way down (CR 608.2b).
- Confirmed dead by deleting it and running the whole suite: nothing failed.

**A claim I had to walk back.** My first version of the replacement comment said the card's copy was the
*narrower* of the two, as Maw of the Mire's was — printed card types versus `state.is_creature`, which also
counts the object's own types and the P/T sentinel a token or animated permanent carries. That is true in
principle, but mutation 1 shows it is not demonstrable here: swapping the filter itself to the printed reading
fails no test, because Manor Gargoyle's printed types already include Creature and nothing in this set is a
creature only at the object level. The comment now says this is a restatement being removed rather than a
disagreement being resolved.

### Rules check
- **"Destroy"** goes through `destruction::try_destroy`, so indestructible (CR 701.7b) and regeneration apply.
  Regeneration is not reachable for this card — the only regenerators in the set are creatures, which it cannot
  target.
- **CR 608.2b** — a single-target spell whose target left the battlefield is countered by game rules before
  `on_resolve`; `resolve_destroy`'s own zone check is therefore belt-and-braces at the helper, not this card's
  concern.
- **No controller restriction** — "target noncreature permanent" may point at your own.

### Changes made
- `mtg-engine/src/cards/isd/bramblecrush.rs` — `is_valid_target` removed, with a comment naming what applies the
  rule instead and the limit above.
- `mtg-engine/tests/cards_removal_and_bounce.rs` — two tests:
  - `bramblecrush_cannot_target_an_artifact_that_is_also_a_creature`. The existing table row uses a plain
    creature, which an implementation asking "is it an artifact, enchantment or land?" also refuses — only a
    permanent that is a creature *and* something else separates that reading from "is it noncreature". Manor
    Gargoyle is that permanent, and a plain artifact is offered from the same board.
  - `bramblecrush_may_target_your_own_permanents`, and destroys one when pointed there. Every existing case
    aims at the opponent.

### Mutation checks
1. `TargetFilter::Noncreature` switched to the printed-types reading → **vacuous** (see above).
2. Filter widened to `TargetFilter::Any` → `bramblecrush_cannot_target_an_artifact_that_is_also_a_creature` and
   `targeted_removal_offers_the_targets_its_text_allows` both FAILED. **Discriminating.**
3. `resolve_destroy` moving the permanent directly instead of calling `try_destroy` →
   `bramblecrush_respects_indestructible` FAILED. **Discriminating.**

### Tricky interactions checked
- Destroys a land, refuses a creature: **pass** (`cards_removal_and_bounce.rs:161` table row).
- An artifact creature is refused; a plain artifact is not: **pass** (new).
- Your own permanents are legal targets: **pass** (new).
- Indestructible noncreature permanent survives: **pass** (`cards_removal_and_bounce.rs:379`).
- Regeneration: **unreachable** — every regenerator in the set is a creature.

### Test coverage
- what it may and may not target: `cards_removal_and_bounce.rs:161`
- artifact creature refused: `cards_removal_and_bounce.rs:322` (new)
- your own permanents: `cards_removal_and_bounce.rs:352` (new)
- respects indestructible: `cards_removal_and_bounce.rs:379`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1429 passing.

