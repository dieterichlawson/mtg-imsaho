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
## Full audit — 2026-08-27

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

**Rulings fetched**:
- [2025-01-24] A creature that is both a Werewolf and a Wolf will only get +1/+1 from Howlpack Alpha’s first ability.

**Status**: ISSUE (fixed) + one deviation recorded

### Code issues

**Howlpack Alpha's Wolf vanished if the Alpha died in response to its own
trigger.**

- Oracle text (back) says: `At the beginning of your end step, create a 2/2 green Wolf creature token.`
- Code did:
  ```rust
  let (controller, is_transformed) = match state.get_object(self_id) {
      Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
      _ => return,
  };
  if !is_transformed { return; }
  ```

CR 113.7a: a triggered ability on the stack exists independently of its source.
The effect names nothing about Howlpack Alpha, so removing the Alpha in response
does not stop the Wolf. The handler failed *both* its checks in that case — the
object is in the graveyard, and `move_object` clears `is_transformed` on the way
out — so the token silently never appeared. Killing a creature in response to
its end-step trigger is ordinary play, not a corner.

The engine is already right about this and says so at
`triggers.rs:493`: "the source's zone is not consulted. A handler that
genuinely needs its permanent present checks for itself." The gap was that a
handler needing *last known information* had nothing to read — `controller` is
reset to the owner when a permanent leaves (CR 108.4). Added
`GameObject::last_controller` and `GameState::last_known_controller`, the same
shape as the existing `last_attached_to_player` fallback for Curses.

The `is_transformed` re-check is not needed at all: step triggers are selected by
face when they are collected (`face_trigger_description` in
`triggers/collect/timing.rs:44`), and the front face declares no end-step
ability, so the hook is only ever reached for a permanent that was Howlpack
Alpha when the end step began.

### Rulings checked

- **"A creature that is both a Werewolf and a Wolf will only get +1/+1 from
  Howlpack Alpha's first ability."** The anthem is a single `ModifyPT` whose
  filter is `Or([HasSubtype(Werewolf), HasSubtype(Wolf)])`, not two effects — so
  a creature matching both is matched once and pumped once. PASS, and tested.

### Tricky interactions checked

- **Subtypes.** Front is `Human Advisor Werewolf` — all three present, and the
  Werewolf type is what lets Moonmist and other Werewolf-matters cards see it.
  Back is `Werewolf` alone, 3/3. PASS.
- **"Other" on both anthems.** The Mayor is itself a Human and the Alpha is
  itself a Werewolf; `EffectScope::GlobalOther` excludes the source on both
  faces. PASS.
- **Transform conditions.** `werewolf_should_transform` reads
  `num_spells_cast_last_turn`: front→back needs `sum == 0` ("no spells were cast
  last turn"), back→front needs `any(count >= 2)` ("**a player** cast two or
  more"), which is per-player and not a total. Both correct. PASS.
- **Trigger scope.** Upkeep is `Each` on both faces ("each upkeep"); the back
  face's end step is `Your`. PASS.
- **Intervening-if at trigger time.** `werewolf_should_trigger` gates the upkeep
  trigger so no stack entry appears when the condition is false (CR 603.4), and
  it also refuses to trigger for a token copy, which cannot transform (CR 111.7).
  PASS.
- **A token copy of the Mayor** gets the anthem but never flips — correct, and
  handled in the shared helper rather than here.
- **Moonmist** transforms all Humans, and the front face is a Human, so it flips
  the Mayor forward; Howlpack Alpha is not a Human, so Moonmist cannot flip it
  back. Consistent with the ruling that Moonmist transforms any double-faced
  Human, not just Werewolves. PASS.

### Second issue, also fixed

The upkeep trigger re-checked its intervening-if against the permanent's
*current* face rather than the face that triggered. Reachable by casting
Moonmist in response to a front-face Werewolf's upkeep trigger: by the rules the
front face's ability still resolves and transforms the permanent (its condition
is about last turn and is unaffected), but the code re-reads the current face
and tests the back face's condition instead.

The fix is a mechanism, since a trigger had to carry the face it fired from:
`TriggerSource` now snapshots `from_back_face`, the step-trigger collector
records it (it already had `is_transformed` in hand), and the dispatcher
publishes it as `GameState::resolving_trigger_from_back_face` around the card's
hook. `werewolf_should_transform` reads that when resolving and falls back to
the current face at trigger time, which is when the two agree by definition.

One mechanism, one shared helper, and it covers all twelve Werewolf DFCs at
once.

### Test coverage

- Wolf token survives its source dying: `werewolf_cards.rs::howlpack_alphas_wolf_arrives_even_if_the_alpha_dies_in_response` (new, mutation-checked) — collects the trigger, kills the Alpha, then resolves.
- both-types ruling: `werewolf_cards.rs:301-326`.
- front-face Human anthem: `werewolf_cards.rs:198-216`.
- back-face Werewolf/Wolf anthem: `werewolf_cards.rs:218-232`.
- Wolf token on the end step: `werewolf_cards.rs::howlpack_alpha_creates_wolf_token_on_end_step`, now transforming through `helpers::apply_transform` and asserting `name_of` rather than hand-setting the name.
- transform conditions both ways: `werewolf_cards.rs:560-600`.

