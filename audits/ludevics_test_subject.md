## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/64/ludevics-test-subject-ludevics-abomination?utm_source=api
**Type line**: `Creature — Lizard Egg` — {1}{U}, 0/3
**Oracle text**:
```
Defender
{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.
```
**Back face**: Ludevic's Abomination — `Creature — Lizard Horror`, 13/13
```
Trample
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
- "{1}{U}: Put a hatching counter on this creature. Then if there are five or
  more hatching counters on it, remove them and transform it." — the removal and
  the flip happen together, and only at five: PASS
- The front face has defender and the back face trample; the back face's
  keywords come from the active face rather than being granted: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The counter loop and the flip at five: `cards_transforming_permanents.rs:ludevics_test_subject_transforms_at_five_counters`
- Counter costs: `counter_costs.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
