## Audit — 2026-08-27 — CR 614.1c: enters tapped is a replacement, not a resolution step

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/97/diregraf-ghoul?utm_source=api
**Type line**: `Creature — Zombie` — {B}, 2/2
**Oracle text**:
```
This creature enters tapped.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: `This creature enters tapped.`
- Code did:
  ```rust
  fn on_resolve(&self, state, object_id, _targets, registry) {
      state.move_object(object_id, Zone::Battlefield, registry);
      if let Some(obj) = state.get_object_mut(object_id) { obj.tapped = true; }
  }
  ```
- The file's own comment already said it: *"'enters tapped' is a
  static/replacement ability, NOT a triggered ability."* The code did not follow
  it. `on_resolve` runs only when the card is **cast**, so every other way onto
  the battlefield produced an untapped Ghoul — and Innistrad has several
  (Unburial Rites, Grimoire of the Dead, Back from the Brink, Moldgraf
  Monstrosity). CR 614.1c applies wherever a permanent enters from.
- It also moved itself to the battlefield, which is the engine's job.
- Fixed: `replace_event` → `helpers::enters_tapped_unless(self_id, event, || false)`,
  the same helper the five ISD check lands use, with no condition. `on_resolve`
  is gone entirely.

### Tricky interactions checked
- **Entering from a graveyard**: now tapped. This is the case the old code missed
  and the new test covers.
- **Watchers of the entry**: under the old code `EnteredBattlefield` fired with
  an untapped Ghoul and the tap followed; a replacement modifies the entering
  event itself, so nothing ever observes it untapped.
- **No trigger on the stack**: a replacement opens no priority window, which is
  the third failure mode `enters_tapped_replacement.rs` was written for.

### Test coverage
- `cards_vanilla_and_keywords.rs::diregraf_ghoul_enters_tapped` — the cast path
  (passed under the old code too).
- `enters_tapped_replacement.rs::a_creature_that_enters_tapped_does_so_however_it_arrives`
  — **added by this audit**, the reanimation path. Mutation-verified: flipping
  the helper's condition to `|| true` fails it.

## Audit — 2026-08-28 19:15

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Diregraf Ghoul"`, https://scryfall.com/card/isd/97/diregraf-ghoul
**Oracle text**:
```
This creature enters tapped.
```
**Type line**: Creature — Zombie
**Mana cost**: {B}   **P/T**: 2/2
**Rulings**: none on Scryfall for this card.
**Status**: PASS

### Code issues
No issues found in `mtg-engine/src/cards/isd/diregraf_ghoul.rs`.

`{B}`, `Creature`, `subtypes: ["Zombie"]`, 2/2, oracle text verbatim. The one line is a
CR 614.1c replacement through the shared `enters_tapped_unless(.., || false)` — the same helper
the ISD dual lands use, with a condition that never exempts it. The card's own doc records the
old bug: set-tapped-after-entry fired `EnteredBattlefield` with an untapped Ghoul for watchers
to see.

### Tricky interactions checked
- **Enters tapped, however it arrives**: a replacement, so reanimation gets it too — the
  mechanism the Festerhide Boar reanimation test pins for the shared helper family.
- **No `Tapped` event, no untapped window**: `arrives_tapped` sets the flag and emits nothing
  (CR 614.1c — it was never untapped and nothing tapped it); the tap-verb guard test holds the
  whole engine to that.
- **Under an Essence of the Wild**: it enters as an untapped Essence — its "enters tapped" is
  not applied because it is not a Ghoul as it enters. The exact shape tested with Grimgrin at
  the Essence audit.
- **A Zombie for everything that counts Zombies**: Ghoulraiser, Endless Ranks — it is a prop in
  a dozen tests.

### Test coverage
- enters tapped: `cards_vanilla_and_keywords.rs:33 diregraf_ghoul_enters_tapped`
- the copy-effect exception: `cards_complex_creatures.rs a_creature_that_would_enter_tapped_enters_as_an_untapped_essence`
  (Grimgrin as the subject; same replacement shape)
- the no-event mechanism: `test_suite_guards.rs only_the_tap_helpers_tap_a_permanent` (engine-wide)

Mutation-checked: making the condition always-exempt fails the test.

### Changes made
None — a one-line card done right, with its one test biting.
