## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/90/bloodline-keeper-lord-of-lineage?utm_source=api
**Type line**: `Creature — Vampire` — {2}{B}{B}, 3/3
**Oracle text**:
```
Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.
```
**Back face**: Lord of Lineage — `Creature — Vampire`, 5/5
```
Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.
```

**Status**: ISSUE

### Code issues
See below.


- The back face's printed P/T came from a `dynamic_pt` override that did nothing
  but restate this card's own `back_face_data` — one derived fact written twice,
  in two places free to disagree, and every test that covered a flip asserted the
  *hook* rather than `effective_power`. CR 712.8: a transformed permanent has its
  back face's characteristics. `effective_power`/`effective_toughness` now read
  the back face directly when `is_transformed`, the nineteen echoes are deleted,
  and a guard fails the build on a new one.

### Tricky interactions checked
- "{T}: Create a 2/2 black Vampire creature token **with flying**" — colour,
  P/T, subtype and keyword all set via `create_token_with_subtypes`: PASS
- "{B}: Transform this creature. **Activate only if you control five or more
  Vampires.**" — an activation restriction, so the ability is not offered below
  five, and the count includes the tokens it made: PASS
- The token ability is on the front face and the lord ability on the back: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Token creation and the transform gate: `cards_transforming_permanents.rs`, `subtype.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
