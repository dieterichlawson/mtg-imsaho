## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/99/endless-ranks-of-the-dead?utm_source=api
**Type line**: `Enchantment` — {2}{B}{B}
**Oracle text**:
```
At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/99/endless-ranks-of-the-dead?utm_source=api
**Type line**: `Enchantment` — {2}{B}{B}
**Oracle text**:
```
At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If you control **fewer than two** Zombies, you won't get any tokens."
  Integer division — `zombie_count / 2` — rounds down, so one Zombie makes none:
  PASS
- "**rounded down**": PASS
- Ruling: "The number of Zombies you control is counted **when the ability
  resolves**. If you control multiple Endless Ranks of the Dead, the tokens you
  get when the first ability resolves will count for the subsequent abilities."
  The count is taken inside the trigger handler, so a second copy resolving
  afterwards sees the first copy's tokens: PASS
- Zombie *tokens* count toward the number — the text says "Zombies you control",
  not "Zombie cards": PASS
- The tokens carry colour and the Zombie subtype, so they feed the next upkeep:
  PASS
- "At the beginning of **your** upkeep": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The halving and the token subtype: `cards_complex_creatures.rs`, `subtype.rs`
