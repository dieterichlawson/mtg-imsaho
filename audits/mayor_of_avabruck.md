## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/193/mayor-of-avabruck-howlpack-alpha?utm_source=api
**Type line**: `Creature — Human Advisor Werewolf` — {1}{G}, 1/1
**Oracle text**:
```
Other Human creatures you control get +1/+1.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Howlpack Alpha, `Creature — Werewolf`
```
Each other creature you control that's a Werewolf or a Wolf gets +1/+1.
At the beginning of your end step, create a 2/2 green Wolf creature token.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
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
The check here sat on the back face's end-step trigger and was fused as
`if !is_transformed || state.active_player != controller`; the transform half
stays.

### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
