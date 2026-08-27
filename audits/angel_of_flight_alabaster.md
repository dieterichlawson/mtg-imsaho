## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/2/angel-of-flight-alabaster?utm_source=api
**Type line**: `Creature — Angel` — {4}{W}, 4/4
**Oracle text**:
```
Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
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
Additionally, the handler opened with a `get_object(self_id)` match that
returned early when the source was gone. That never fired — a permanent in a
graveyard still resolves `get_object` — but CR 113.7a means the ability owes
nothing to its source and the effect only needs its target, so the lookup was
removed as well.

### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/2/angel-of-flight-alabaster?utm_source=api
**Type line**: `Creature — Angel` — {4}{W}, 4/4
**Oracle text**:
```
Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The Spirit card must already be in your graveyard **when the ability
  triggers** at the beginning of your upkeep. If there is no Spirit card in your
  graveyard when your upkeep begins, the ability will be **removed from the
  stack with no effect**." The trigger declares a `target_requirement`, so
  CR 603.3d chooses the target as it goes on the stack and no legal Spirit means
  it is not put on the stack at all: PASS
- "target **Spirit** card" — `is_valid_target` narrows the graveyard enumeration
  to Spirits, and the engine applies that filter when building the target list:
  PASS
- CR 109.1: a Spirit *card*, so a Spirit token in the graveyard is not offered —
  now enforced in the engine's graveyard enumeration: PASS
- "from **your** graveyard" — `GraveyardCardOwnedByCaster`: PASS
- "At the beginning of **your** upkeep": PASS
- CR 113.7a: killing the Angel in response does not counter the trigger, and the
  handler does not need the source to resolve: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Spirit filter and the no-target case: `cards_complex_creatures.rs`, `trigger_dispatch.rs`
