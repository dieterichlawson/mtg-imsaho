## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/208/ulvenwald-mystics-ulvenwald-primordials?utm_source=api
**Type line**: `Creature — Human Shaman Werewolf` — {2}{G}{G}, 3/3
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Ulvenwald Primordials — `Creature — Werewolf`, 5/5
```
{G}: Regenerate this creature.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
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
- Ruling: "You can regenerate Ulvenwald Primordials in response to the triggered
  ability that would transform it. If you do, the regeneration shield will apply
  to Ulvenwald Mystics that turn." The shield is object state, not face state,
  so it survives the flip: PASS
- Back: "{G}: Regenerate this creature" — a regeneration shield, which taps,
  removes damage and removes it from combat when it applies (CR 701.15): PASS
- The regenerate ability is offered only on the back face: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The flip conditions and the back face's regenerate: `werewolf_cards.rs`, `cards_morbid_and_ltb.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
