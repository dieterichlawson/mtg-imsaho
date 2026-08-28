## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/178/essence-of-the-wild?utm_source=api
**Type line**: `Creature — Avatar` — {3}{G}{G}{G}, 6/6
**Oracle text**:
```
Creatures you control enter as a copy of this creature.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- Excludes itself, non-creatures, and creatures an opponent controls — all
  three required by "**Creatures you control** enter as a copy of **this**
  creature".
- `e.copy_of.is_some()` guard stops the effect applying twice to one event
  (CR 614.5).
- Uses `state.is_creature`, so a token entering is covered.

### Test coverage
`copy_effects.rs`
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/178/essence-of-the-wild?utm_source=api
**Type line**: `Creature — Avatar` — {3}{G}{G}{G}, 6/6
**Oracle text**:
```
Creatures you control enter as a copy of this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Creatures you control **enter as a copy of** this creature" is a replacement
  effect applied as the creature enters (CR 614), not something done afterwards:
  PASS
- It excludes **itself** — the Essence is not on the battlefield when it is
  entering, so it cannot copy itself: PASS
- "Creatures **you control**", so an opponent's creatures are unaffected: PASS
- It applies to tokens too, since a token entering is a creature entering: PASS
- `copy_of.is_some()` guards against applying twice when another copy effect has
  already claimed the entry: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The entering copy: `cards_complex_creatures.rs`, `enters_tapped.rs`

## Audit — 2026-08-28 18:48

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Essence of the Wild"`, https://scryfall.com/card/isd/178/essence-of-the-wild
**Oracle text**:
```
Creatures you control enter as a copy of this creature.
```
**Type line**: Creature — Avatar
**Mana cost**: {3}{G}{G}{G}   **P/T**: 6/6
**Rulings**: 8, several of them about the exact order replacement effects apply in.
**Status**: ISSUE (two engine bugs, both reachable in this pool, both fixed)

### Code issues
The card itself is correct: `{3}{G}{G}{G}`, `Creature`, `subtypes: ["Avatar"]`, 6/6, oracle text
verbatim, and a `replace_event` implementing CR 614.1d — not itself, creatures only, yours only.

The two bugs are in the engine, and each corresponds to a ruling on this card.

**1. The copy was applied after the other entering effects.**
- Ruling: "Replacement effects that modify how a creature enters are applied in the following
  order: first control-changing effects, then copy effects, then all other effects", and
  "a creature that normally enters tapped will enter as an untapped Essence of the Wild".
- Code did: `replacement::apply` ran every candidate in one pass, object-id order, with the
  entering permanent's own effects first. So Grimgrin, Corpse-Born set `tapped: true` from its
  own "enters tapped" and *then* Essence set `copy_of` — and it entered as a **tapped** Essence.

Fixed by splitting `for_entering` into two passes: one that keeps only the copy decision, then
one for everything else, in which the entering permanent's own abilities are read from the card
it is copying. Grimgrin-as-an-Essence has no "enters tapped" because it is not a Grimgrin.

**2. A copy kept its own abilities.**
- Ruling: "Because creatures you control enter as copies of Essence of the Wild, any 'enters'
  triggered abilities printed on such creatures won't trigger."
- Code did: `become_copy_of` wrote the copied card's characteristics onto the object but left
  `card_id` alone — and `card_id` is what every ability, trigger and replacement lookup reads.
  A Village Bell-Ringer entering as an Essence still untapped its controller's creatures.

Fixed by following the convention the engine already had. `engine/effects.rs` (Evil Twin's copy
path) sets `card_id` to the copied card and remembers the original in `copy_grantor`, and
`move_object` restores it on the way out (CR 400.7). `become_copy_of` was the one copy path not
doing that.

**A wrong turn, recorded because it matters.** I first fixed (2) by adding a *new* field
(`copied_from`) and teaching `face_data` and the ETB collector to read it — a second mechanism
for a question the codebase already had an answer to, which is exactly what this audit pass has
been removing from cards. Reverted before committing once I found `copy_grantor`.

### Tricky interactions checked
- **Creatures you control**: PASS; an opponent's is untouched.
- **However they arrive** — cast, or a token: PASS, and the token case is why the effect is a
  replacement rather than an `on_resolve`.
- **The Essence itself entering**: excluded by `e.object == self_id`. A *second* Essence entering
  would be copied by the first, which is the same card, so nothing is observable.
- **Two Essences**: "a copy of the one whose copy effect you apply last" — same card either way.
- **A creature that would enter tapped**: PASS after fix 1.
- **An "enters" trigger printed on the entering creature**: PASS after fix 2.
- **Evil Twin entering under an Essence**: PASS, and it was right before these fixes — it enters
  as an Essence and is not offered its own copy choice, which is what the ruling requires. I
  expected this to be broken by the `copy_of.is_some()` guard and it is not; that guard only
  keeps two Essences from applying twice.
- **A copy's *activated* abilities**: not reachable through this card — an Essence has none. The
  general question is handled by the `card_id`/`copy_grantor` convention this fix now joins.

### Test coverage
- a cast creature enters as a 6/6 Essence: `cards_complex_creatures.rs:2158 essence_overrides_entering_creatures`
- an opponent's does not: `cards_complex_creatures.rs:2183`
- a token does too: `replacement_effects.rs:53 essence_of_the_wild_applies_to_a_token_it_did_not_resolve`
- one that would enter tapped enters untapped:
  `cards_complex_creatures.rs:~2205 a_creature_that_would_enter_tapped_enters_as_an_untapped_essence` (NEW)
- its "enters" trigger does not fire:
  `cards_complex_creatures.rs:~2230 an_enters_trigger_does_not_fire_for_a_creature_that_arrived_as_an_essence` (NEW)

Mutation-checked, each half by its own test: running the passes in the other order fails the
tapped test; leaving `card_id` alone in `become_copy_of` fails the trigger test.

### Changes made
- `replacement.rs`: `for_entering` split into copy-first and everything-else passes.
- `state.rs`: `become_copy_of` takes the copied card's identity, recording the printed one.
- `cards_complex_creatures.rs`: the two ruling tests.
