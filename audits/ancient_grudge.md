## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/127/ancient-grudge?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

- "Destroy target artifact", with a flashback cost in a different colour
  ({G} against a {1}{R} face). The flashback cost was verified exact set-wide,
  and `flashback.rs` covers casting from the graveyard and the exile afterwards
  (CR 702.33a).

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/127/ancient-grudge?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Flashback {G} is a *different colour* from the {1}{R} front cost, which is the
  card's whole design — it is castable from the graveyard in a deck that could
  not cast it from hand: PASS
- Ruling: "A spell cast using flashback will **always** be exiled afterward,
  whether it resolves, is countered, or leaves the stack in some other way" —
  the engine's post-resolution move sends a flashback spell to exile, and
  `move_countered_spell` routes through the same place: PASS
- Ruling: "the mana value of the spell is determined only by its **mana cost**,
  no matter what the total cost to cast the spell was" — the flashback cost does
  not change the card's mana value: PASS
- `try_destroy`, so an indestructible artifact survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Flashback from the graveyard and the exile after: `cards_flashback.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/127/ancient-grudge?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/127/ancient-grudge
**Oracle text**:
```
Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: `Instant` · **Mana cost**: `{1}{R}` · **Keywords**: Flashback
**Rulings** (6, all 2021-03-19, https://api.scryfall.com/cards/e5e7b966-7c5b-44e6-a6df-4bd7af4edaa9/rulings) —
all six are the generic flashback rulings, none specific to this card. The load-bearing ones here: "A spell cast
using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some
other way"; "You must still follow any timing restrictions"; "You can cast a spell using flashback even if it
was somehow put into your graveyard without having been cast."

**Status**: ISSUE (fixed) — a restated target requirement, and no test of the card at all.

### Card data
| field | oracle | `ancient_grudge.rs` | |
|---|---|---|---|
| cost | `{1}{R}` | `Generic(1) + Red` | ok |
| types | Instant | `vec![CardType::Instant]` | ok |
| oracle_text | as above, reminder text included | byte-identical | ok |
| flashback | `{G}` | `flashback_cost: Some({G})` | ok |
| targeting | "target artifact" | `PermanentWithFilter(HasCardType([Artifact]))` | ok |
| effect | destroy | `helpers::resolve_destroy` → `destruction::try_destroy` | ok |

### Code issues

**1. `is_valid_target` restated the target requirement.** Removed.

- Code was `o.zone == Zone::Battlefield` plus
  `state.face_data(obj.id, registry).is_some_and(|d| d.card_types.contains(&CardType::Artifact))`.
- `PermanentWithFilter(HasCardType([Artifact]))` already says it, applied at enumeration and re-applied by
  `stack::is_target_legal` with the zone check (CR 608.2b). Confirmed dead by deleting it and running the whole
  suite.

**2. The card had no test of its own.** Fixed.

`grep` for "Ancient Grudge" in `mtg-engine/tests/` returned only `hexproof_filter.rs` and `trigger_dispatch.rs`,
where it is a *prop* — a convenient instant sitting in a graveyard for Snapcaster Mage to point at. Nothing cast
it from hand, nothing cast it with flashback, and nothing watched it destroy an artifact. Both halves of a
two-mode card were entirely unexercised.

### Rules check
- **"Target artifact"** says nothing about creatures, so an artifact creature is a legal target. That is the
  mirror of Bramblecrush, audited immediately before this one, whose "noncreature permanent" refuses the same
  permanent. Testing both against Manor Gargoyle makes each test about its own card's wording.
- **CR 702.33a** — a flashback spell is exiled instead of going anywhere else when it leaves the stack.
  Handled by `move_spell_after_resolve`, keyed on `cast_with_flashback`.
- **Timing** — an instant, so flashback at instant speed; no restriction to add.
- **"Destroy"** through `destruction::try_destroy`, so indestructible and regeneration apply.

### Changes made
- `mtg-engine/src/cards/isd/ancient_grudge.rs` — `is_valid_target` removed, with a comment naming what applies
  the rule and pointing at the Bramblecrush contrast.
- `mtg-engine/tests/cards_removal_and_bounce.rs` — three tests:
  - `ancient_grudge_destroys_an_artifact_from_hand` — the effect, plus that a hand cast lands in the graveyard
    rather than exile, which is where its flashback waits.
  - `ancient_grudge_can_target_an_artifact_that_is_also_a_creature`.
  - `ancient_grudge_can_be_flashed_back_and_is_then_exiled` — the whole second life, with exactly `{G}`
    available rather than the printed `{1}{R}`, so the cast is unambiguously the flashback one.

### Mutation checks (all discriminating)
1. Filter widened to `TargetFilter::Any` → `ancient_grudge_destroys_an_artifact_from_hand` FAILED.
2. Filter narrowed to `TargetFilter::Noncreature` →
   `ancient_grudge_can_target_an_artifact_that_is_also_a_creature` FAILED. This is the pair with Bramblecrush:
   the two cards' filters are now each held apart from the other's.
3. `flashback_cost` changed to `{9}` → `ancient_grudge_can_be_flashed_back_and_is_then_exiled` FAILED (the cast
   is no longer offered).
4. `move_spell_after_resolve` no longer exiling a flashback spell → same test FAILED.

### Tricky interactions checked
- Destroys an artifact, refuses a plain creature: **pass** (new).
- An artifact creature is a legal target: **pass** (new).
- Hand cast goes to the graveyard; flashback cast goes to exile: **pass** (new).
- Flashback offered from the graveyard for `{G}` only: **pass** (new).
- Indestructible artifact survives: covered at the pipeline level by `bramblecrush_respects_indestructible`;
  both cards call the same `resolve_destroy`. Not duplicated here.
- A flashback spell that is *countered* is still exiled: covered generally by
  `fizzle.rs:a_fizzled_flashback_spell_is_still_exiled`.

### Test coverage
- destroys an artifact from hand: `cards_removal_and_bounce.rs:322` (new)
- artifact creature is targetable: `cards_removal_and_bounce.rs:344` (new)
- flashback cast and exile: `cards_removal_and_bounce.rs:360` (new)
- previously: none for this card's own behaviour

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1432 passing.

