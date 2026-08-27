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
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/193/mayor-of-avabruck-howlpack-alpha?utm_source=api
**Type line**: `Creature — Human Advisor Werewolf` — {1}{G}, 1/1
**Oracle text**:
```
Other Human creatures you control get +1/+1.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Howlpack Alpha — `Creature — Werewolf`, 3/3
```
Each other creature you control that's a Werewolf or a Wolf gets +1/+1.
At the beginning of your end step, create a 2/2 green Wolf creature token.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "A creature that is both a Werewolf **and** a Wolf will only get +1/+1
  from Howlpack Alpha's first ability." One `ModifyPT` whose filter is
  `Or([Werewolf, Wolf])`, so a creature matching both matches the filter once
  and is buffed once — not two stacking effects: PASS
- "**Other** Human creatures you control" and "Each **other** creature you
  control" — `EffectScope::GlobalOther` on both faces, so the Mayor never buffs
  itself: PASS
- The front face's Human buff stops and the back face's Werewolf/Wolf buff
  starts on transform, because `continuous_effects_of` reads the active face:
  PASS
- "At the beginning of **your** end step, create a 2/2 green Wolf" —
  `TriggerScope::Your` on the back face only, while the upkeep trigger stays
  `Each`: PASS
- The Wolf token it makes is a Wolf you control, so the Alpha's own buff makes
  it a 3/3: PASS
- The werewolf flip conditions are the shared `werewolf_should_trigger` /
  `werewolf_should_transform` helpers, so "if no spells were cast last turn" and
  "if a player cast two or more spells last turn" are one implementation rather
  than one per card: PASS
- CR 603.4: both are intervening-ifs, checked when the trigger would go on the
  stack *and* again on resolution: PASS
- "At the beginning of **each** upkeep" — `TriggerScope::Each`, so it fires on
  the opponent's turn too: PASS
- The active face's characteristics come from `back_face_data` when transformed
  (CR 712.8) — P/T, keywords, subtypes, continuous effects and triggered
  abilities all switch together: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both faces' buffs and the Wolf token: `werewolf_cards.rs`, `cards_transforming_permanents.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
