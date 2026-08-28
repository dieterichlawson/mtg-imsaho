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
## Full audit — 2026-08-27

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

**Rulings fetched**:
- [2016-04-08] If multiple creatures you control die simultaneously, Thraben Sentry's ability will trigger that many times. Only the first one to resolve will cause it to transform.

**Status**: ISSUE (fixed)

### Code issues

One found.

1. **The back face had no colour indicator, so Thraben Militia was colourless.** `thraben_sentry.rs:41`
   - Type line (back face) is `Creature — Human Soldier` with no mana cost, so CR 204.2 puts the colour in an indicator beside it.
   - Code declared `power`, `toughness`, `subtypes`, `keywords` and `oracle_text` for the back face and no colour, so `colors_of` returned an empty vector for a transformed Sentry.
   - Now `color_indicator: vec![Color::White]`. A web search this session, over the Gatherer and retailer results for the card, returned that Thraben Militia "is a white card from the Innistrad set" — external source obtained during this audit, not memory. Six of the set's twenty back faces now have theirs.

### Checked against the ruling

- `If multiple creatures you control die simultaneously, Thraben Sentry's ability will trigger that many times. Only the first one to resolve will cause it to transform.` — PASS, both halves, and they are separate claims:
  - **Triggers that many times.** Verified directly: two creatures destroyed together put two copies of the ability on the stack, one per death. This is a leaves-the-battlefield trigger looking back at each death (CR 603.10a), not one trigger for the event.
  - **Only the first transforms.** The second resolves against a permanent that is already Thraben Militia. `on_any_creature_dies` returns early on `is_transformed`, so it neither asks again nor flips back to the front face. Mutation-checked: removing that guard makes the second trigger prompt, and answering it would return the card to Thraben Sentry.

### Checked and correct

- Front: `{3}{W}`, `Creature — Human Soldier`, 2/2, `keywords: [Vigilance]`.
- Back: `Thraben Militia`, `Creature — Human Soldier`, 5/4, `keywords: [Trample]` — and **not** vigilance, which belongs to the front face only. Scryfall's aggregate keyword list for the card is `Vigilance, Transform, Trample`, which is the two faces' keywords together.
- Both faces keep the Human Soldier subtypes; this is one of the few double-faced cards in the set whose back face is still a Human.
- `you may transform this creature` is a real `YesNo` choice, asked of the Sentry's controller, and declining leaves it on its front face.
- "another creature **you control**" — the handler returns unless `dead_controller == controller`.
- "**another** creature" — enforced at collection: the dies-trigger collector excludes the dying creature from the watcher list, so a Sentry dying alone does not trigger itself. The handler's battlefield check covers the case where it died alongside another creature, since transforming a permanent in the graveyard would do nothing anyway (CR 712.8a).
- The back face declares no triggered abilities: the transform ability is the front face's, so a Militia does not keep watching for deaths.
- `should_transform` returns false — this card never transforms on its own, only through the may-choice.
- The transform goes through `apply_transform`, which logs it (the card's own duplicate log line was removed earlier in this audit).

### Tricky interactions checked

- Two creatures dying at once: two triggers, one transform. PASS.
- An opponent's creature dying: no trigger. PASS.
- Declining the may: stays on the front face. PASS.
- The choice is the Sentry's controller's, and the prompt names the card. PASS.
- Back face has trample, not vigilance. PASS.
- Colour kept across the transform: PASS (after fix).

### Test coverage

- transforms when another creature you control dies, and the back face is a 5/4: `cards_transforming_permanents.rs:478`
- back face is white, has trample, and lost vigilance: same test (NEW assertions, mutation-checked)
- an opponent's creature dying does not transform it: `cards_transforming_permanents.rs:511`
- the may-choice is presented, and both answers are honoured: `auto_pick.rs:717`
- two simultaneous deaths trigger twice and transform once: `cards_transforming_permanents.rs` `two_simultaneous_deaths_trigger_twice_and_transform_once` (NEW, mutation-checked)

