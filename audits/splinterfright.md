## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/205/splinterfright?utm_source=api
**Type line**: `Creature — Elemental` — {2}{G}, */*
**Oracle text**:
```
Trample
Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.
At the beginning of your upkeep, mill two cards. (Put the top two cards of your library into your graveyard.)
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/205/splinterfright?utm_source=api
**Type line**: `Creature — Elemental` — {2}{G}, */*
**Oracle text**:
```
Trample
Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.
At the beginning of your upkeep, mill two cards. (Put the top two cards of your library into your graveyard.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The ability that defines Splinterfright's power and toughness works
  **in all zones**, not just the battlefield. **If Splinterfright is in your
  graveyard, it will count itself.**" `dynamic_pt` has no self-exclusion and no
  battlefield gate, and Splinterfright's own card data carries the `Some(0)`
  P/T sentinel that marks a characteristic-defining creature — so it is counted
  by `is_creature` when it is in the graveyard: PASS
- CR 112.8: a card in a graveyard is controlled by its **owner**, so the count
  reads `obj.owner` rather than a `controller` left stale by a steal effect:
  PASS
- CR 109.1: "creature **cards** in your graveyard", so tokens are excluded: PASS
- Ruling: "If Splinterfright's controller has only one card in their library
  when its triggered ability resolves, they put that card into their graveyard"
  — `mill_cards` stops at an empty library: PASS
- The self-mill grows it, since a milled creature card is another creature card
  in the graveyard: PASS
- "At the beginning of **your** upkeep": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The CDA and the token exclusion: `token_is_not_a_card.rs:a_token_in_a_graveyard_is_not_a_creature_card`, `:cda_does_not_count_tokens_in_graveyard`
