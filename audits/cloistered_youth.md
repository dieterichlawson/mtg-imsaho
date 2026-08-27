## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/8/cloistered-youth-unholy-fiend?utm_source=api
**Type line**: `Creature — Human` — {1}{W}, 1/1
**Oracle text**:
```
At the beginning of your upkeep, you may transform this creature.
```
**Back face**: Unholy Fiend, `Creature — Horror`
```
At the beginning of your end step, you lose 1 life.
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
Both faces carried the duplicate check — the front's upkeep "you may transform"
and the back's end-step "you lose 1 life".

### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/8/cloistered-youth-unholy-fiend?utm_source=api
**Type line**: `Creature — Human` — {1}{W}, 1/1
**Oracle text**:
```
At the beginning of your upkeep, you may transform this creature.
```
**Back face**: Unholy Fiend — `Creature — Horror`, 3/3
```
At the beginning of your end step, you lose 1 life.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "At the beginning of **your** upkeep, you **may** transform" —
  `TriggerScope::Your`, and a YesNo choice rather than automatic: PASS
- One-way: Unholy Fiend has no transform ability, and the handler only offers
  the prompt on the front face, so it cannot flip back: PASS
- "At the beginning of your end step, you lose 1 life" is the *back* face's
  ability and only fires while transformed: PASS
- Life **loss**, not damage, through `change_life`: PASS
- Declining is recorded and costs nothing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The optional transform and the back face's upkeep cost: `cards_transforming_permanents.rs:cloistered_youth_presents_transform_choice_at_upkeep`
