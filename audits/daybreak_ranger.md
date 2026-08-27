## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator?utm_source=api
**Type line**: `Creature — Human Archer Ranger Werewolf` — {2}{G}, 2/2
**Oracle text**:
```
{T}: This creature deals 2 damage to target creature with flying.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Nightfall Predator — `Creature — Werewolf`, 4/4
```
{R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)
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
- Front: "{T}: This creature deals 2 damage to target creature **with flying**"
  — `TargetFilter::HasKeyword(Flying)`, and a creature that loses flying in
  response is no longer a legal target (CR 608.2b), now re-checked for abilities:
  PASS
- Back: "{R}, {T}: This creature **fights** target creature" — the fight
  pipeline, so both deal damage equal to power simultaneously: PASS
- The damage source is the Ranger, so protection from green stops it: PASS
- "At the beginning of **each** upkeep, **if** no spells were cast last turn" —
  an intervening-if checked both when the trigger would go on the stack and
  again on resolution (CR 603.4), via the shared werewolf helpers: PASS
- Both faces' upkeep triggers are declared, and the transform is the effect: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both faces' abilities and the flip conditions: `werewolf_cards.rs`, `transform_dfc.rs`
- Intervening-if on both directions: `intervening_if.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
