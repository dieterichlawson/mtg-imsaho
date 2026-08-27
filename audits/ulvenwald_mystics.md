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
## Full audit — 2026-08-27

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

**Rulings fetched**:
- [2011-09-22] You can regenerate Ulvenwald Primordials in response to the triggered ability that would transform it. If you do, the regeneration shield will apply to Ulvenwald Mystics that turn.

**Status**: PASS (ruling untested — now covered)

### Code issues

No behavioural issues found. The card's only published ruling was untested, and
the case it describes is one that a later refactor could silently break, so it
is covered now.

Cleaned up a dead binding in `activated_abilities` — it bound `obj` and
immediately discarded it with `let _ = obj;`. Replaced with the boolean check it
was actually performing, plus a line saying *why* the ability is back-face only.

### Rulings checked

- **"You can regenerate Ulvenwald Primordials in response to the triggered
  ability that would transform it. If you do, the regeneration shield will apply
  to Ulvenwald Mystics that turn."**

  This holds, and for the right reason: transforming is not a zone change, so
  none of the leave-the-battlefield cleanup in `move_object` runs, and
  `apply_transform` touches only `is_transformed` and the name cache. The shield
  sits on the object and comes with it (CR 712.8 — same object).

  It is worth being explicit about what makes this fragile: the front face has
  no regenerate ability of its own, so a shield on Ulvenwald Mystics only ever
  exists by having survived a transform. Anything that made `apply_transform`
  reset object state — which would look like reasonable hygiene — would break
  the ruling with no test failing. Mutation-checked exactly that way, by having
  `apply_transform` clear `regeneration_shields`. PASS.

### Tricky interactions checked

- **A shield lasts until end of turn (CR 701.15), not forever.** The cleanup
  step zeroes `regeneration_shields` for every battlefield permanent
  (`engine.rs:840`). Without that, the Primordials could bank a shield every
  turn for the rest of the game. Now tested. PASS.
- **The regenerate ability is back-face only.** Offered only while
  `is_transformed`, and the front-face test asserts zero activated abilities.
  PASS.
- **`{G}: Regenerate` has no tap cost and no timing restriction** —
  `requires_tap: false`, not sorcery-speed — which is what makes "in response to
  the transform trigger" possible at all. PASS.
- **Regeneration removes the creature from combat** (CR 701.15) — that is the
  engine's `destruction::regenerate`, and it is the same path that produced the
  combat-damage-step hang fixed earlier in this audit. Not this card's code, but
  this card is the set's only way to reach it. PASS.
- **Subtypes.** Front `Human Shaman Werewolf`, back `Werewolf` alone. Losing the
  Human type on transform matters for Moonmist and the Human-matters cards.
  PASS.
- **Transform conditions** go through the shared werewolf helpers, so this card
  inherits the CR 603.4 fix from Mayor of Avabruck — the condition re-checked on
  resolution belongs to the face that triggered. PASS.
- **`on_upkeep`'s battlefield guard is correct here**: unlike the end-step
  handlers fixed in Cloistered Youth and Bloodgift Demon, "transform this
  creature" genuinely needs the permanent. PASS.

### Test coverage

- the ruling — shield survives the transform and applies to the front face: `werewolf_cards.rs::a_regeneration_shield_survives_transforming_back_into_ulvenwald_mystics` (new, mutation-checked).
- shield expires at end of turn: `werewolf_cards.rs::an_unused_regeneration_shield_does_not_survive_the_turn` (new).
- transforms and gains the regenerate ability, front face has none: `werewolf_cards.rs::ulvenwald_mystics_transforms_and_gains_regenerate`.
- intervening-if gate on the transform trigger: `intervening_if.rs:132`.
- trigger-time snapshot behaviour: `trigger_snapshots.rs:131`.

