## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/114/screeching-bat-stalking-vampire?utm_source=api
**Type line**: `Creature — Bat` — {2}{B}, 2/2
**Oracle text**:
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Back face**: Stalking Vampire, `Creature — Vampire`
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```

**Status**: ISSUE (fixed) — duplication, not a rules defect

### Code issue
- Oracle text says the trigger happens at **your** upkeep / **your** end step.
- Code did: declared `step_trigger_scope` → `TriggerScope::Your`, which is
  correct and sufficient, and then re-derived the same thing inside the handler
  as `state.active_player != controller`.
- The engine's gate is not taken on trust: `your_upkeep_scope.rs` sweeps the
  registry for every card with a controller-scoped step trigger and checks both
  directions — fires on the controller's step, silent on the opponent's. The
  handler check was provably dead.
- Fixed: removed, with a comment saying where the scoping actually lives.
The trigger is on the back face (Stalking Vampire's upkeep "you may pay
{2}{B}{B}").

### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/114/screeching-bat-stalking-vampire?utm_source=api
**Type line**: `Creature — Bat` — {2}{B}, 2/2
**Oracle text**:
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Back face**: Stalking Vampire — `Creature — Vampire`, 5/5
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "you **may** pay {2}{B}{B}. If you do, transform" is on **both** faces, so the
  prompt is offered whichever way round it is and it can flip back and forth —
  the handler has no `is_transformed` gate: PASS
- CR 106.4: mana pools empty between steps, so the pool is normally empty at
  upkeep. The prompt is offered when the player has enough *untapped sources*,
  planned through the engine's autotap, rather than only when mana is already
  floating: PASS
- Declining costs nothing: PASS
- The tap plan is recomputed when the answer comes back, so it cannot pay with
  sources that have since been tapped: PASS
- Flying is on the front face only: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Paying and transforming from either face: `cards_transforming_permanents.rs:screeching_bat_transforms_at_upkeep_when_player_pays`, `transform_dfc.rs`
