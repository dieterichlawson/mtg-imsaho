## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/197/naturalize?utm_source=api
**Type line**: `Instant` — {1}{G}
**Oracle text**:
```
Destroy target artifact or enchantment.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "artifact **or** enchantment" — both, and an artifact creature qualifies: PASS
- `try_destroy`, so indestructible survives: PASS
- The target is re-checked on resolution, so a permanent that stopped being an
  artifact or enchantment makes it fizzle (CR 608.2b): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Destroying each type, and indestructible: `cards_removal.rs`, `fizzle.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/197/naturalize?utm_source=api
**Type line**: `Instant` — {1}{G}
**Oracle text**:
```
Destroy target artifact or enchantment.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/197/naturalize
**Oracle text**:
```
Destroy target artifact or enchantment.
```
**Type line**: `Instant` · **Mana cost**: `{1}{G}`
**Rulings**: none published for this card.
**Status**: ISSUE (fixed) — a restated target requirement, and half the type line untested.

### Card data
| field | oracle | `naturalize.rs` | |
|---|---|---|---|
| cost | `{1}{G}` | `Generic(1) + Green` | ok |
| types | Instant | `vec![CardType::Instant]` | ok |
| oracle_text | as above | byte-identical | ok |
| targeting | "artifact **or** enchantment" | `PermanentWithFilter(HasCardType([Artifact, Enchantment]))` | ok |
| effect | destroy | `helpers::resolve_destroy` → `destruction::try_destroy` | ok |

The "or" is carried by `HasCardType`'s filter arm being `types.iter().any(..)`. Mutation 3 below is the reading
that would turn it into "artifact enchantment".

### Code issues

**1. `is_valid_target` restated the target requirement.** Removed — the third such restatement in three cards
(Bramblecrush, Ancient Grudge, Naturalize). Confirmed dead by deleting it and running the whole suite.

**2. Only the enchantment half was tested.** Naturalize had exactly one row in
`targeted_removal_offers_the_targets_its_text_allows`, with `Candidate::Enchantment`. Dropping
`CardType::Artifact` from the card's type list passes every test that existed.

### Rules check
- **"Artifact or enchantment"** says nothing about creatures, so an artifact creature is a legal target. That is
  the same reading as Ancient Grudge's "target artifact", and the opposite of Bramblecrush's "noncreature
  permanent". With this card audited, the three filters now hold one another apart against the same permanent
  (Manor Gargoyle).
- **"Destroy"** through `destruction::try_destroy`, so indestructible and regeneration apply.
- **Instant** — no timing restriction to add.

### Changes made
- `mtg-engine/src/cards/isd/naturalize.rs` — `is_valid_target` removed, with a comment naming what applies the
  rule and the creature reading.
- `mtg-engine/tests/cards_removal_and_bounce.rs`:
  - `naturalize_destroys_either_half_of_its_type_line` — an artifact and an enchantment, each offered and then
    actually destroyed. Claustrophobia is an Aura, so the row gives it a creature to enchant.
  - `naturalize_can_target_an_artifact_creature`, with a plain creature refused from the same board.

### Mutation checks (all discriminating)
1. `CardType::Artifact` dropped from the type list → `naturalize_destroys_either_half_of_its_type_line` and
   `naturalize_can_target_an_artifact_creature` FAILED. This is the gap: **nothing** failed before these tests.
2. `CardType::Enchantment` dropped → `naturalize_destroys_either_half_of_its_type_line` and the table test
   FAILED.
3. `HasCardType` changed from `any` to `all` → three tests FAILED. This is the engine-level reading that would
   silently turn every "X or Y" filter in the set into "X and Y"; Naturalize is the card that catches it,
   because it is the only one in this file with two types in its list.

### Tricky interactions checked
- Destroys an enchantment; refuses a plain creature: **pass** (`cards_removal_and_bounce.rs:159` table row).
- Destroys an artifact: **pass** (new — previously untested).
- An artifact creature is a legal target: **pass** (new).
- Indestructible: covered at the pipeline level (`bramblecrush_respects_indestructible`); the same
  `resolve_destroy` call, not duplicated here.
- Aura destroyed while attached — Claustrophobia goes to the graveyard and stops holding its creature tapped:
  the destruction half is covered by the new test; the "stops applying" half belongs to Claustrophobia's own
  audit.

### Test coverage
- what it may and may not target: `cards_removal_and_bounce.rs:159`
- both halves of the type line: `cards_removal_and_bounce.rs:322` (new)
- artifact creature is targetable: `cards_removal_and_bounce.rs:346` (new)

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1438 passing.

