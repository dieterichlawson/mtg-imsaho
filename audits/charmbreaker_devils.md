## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/134/charmbreaker-devils?utm_source=api
**Type line**: `Creature — Devil` — {5}{R}, 4/4
**Oracle text**:
```
At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
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


### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/134/charmbreaker-devils?utm_source=api
**Type line**: `Creature — Devil` — {5}{R}, 4/4
**Oracle text**:
```
At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The instant or sorcery card returned to your hand is chosen **at
  random as Charmbreaker Devils's first ability resolves**. If any player
  responds to the ability, that player won't yet know what card will be
  returned." The candidate list is built and shuffled inside the trigger
  handler — at resolution — not when the trigger is put on the stack: PASS
- Ruling: "Because the first ability **doesn't target** the instant or sorcery
  card, any instants or sorceries put into your graveyard **in response** to
  that ability may be returned to your hand." The trigger declares no
  `target_requirement`, and building the list at resolution is what makes those
  cards eligible: PASS
- CR 109.1: "an instant or sorcery **card**", so a token is not one: PASS
- An empty graveyard returns nothing rather than failing: PASS
- "Whenever **you** cast an **instant or sorcery** spell" — both halves are part
  of the trigger condition (CR 603.2) and gate dispatch, so the ability does not
  go on the stack for an opponent's spell: PASS
- CR 113.7a: destroying the Devils in response does not counter either trigger:
  PASS
- "At the beginning of **your** upkeep" — `TriggerScope::Your`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The random return and the cast trigger: `cards_complex_creatures.rs`, `trigger_dispatch.rs`
