## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/38/thraben-sentry-thraben-militia?utm_source=api
**Type line**: `Creature — Human Soldier` — {3}{W}, 2/2
**Oracle text**:
```
Vigilance
Whenever another creature you control dies, you may transform this creature.
```
**Back face**: Thraben Militia — `Creature — Human Soldier`, 5/4
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
- Ruling: "If multiple creatures you control die simultaneously, Thraben
  Sentry's ability will trigger that many times. **Only the first one to resolve
  will cause it to transform.**" The handler returns early when already
  transformed, so the later triggers resolve with no effect: PASS
- "Whenever **another** creature **you control** dies" — both the self-exclusion
  and the controller test: PASS
- "you **may** transform" — a YesNo choice, and declining is recorded: PASS
- Vigilance is on the front face only; the back face has trample: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Transforming on a creature's death, and only once: `cards_transforming_permanents.rs:thraben_sentry_transforms_when_creature_dies`, `:thraben_sentry_does_not_transform_when_opponent_creature_dies`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
