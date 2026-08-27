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
