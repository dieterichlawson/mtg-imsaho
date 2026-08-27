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
## Full audit — 2026-08-27

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

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

**Unholy Fiend's life loss vanished if the Fiend was killed in response.**

- Oracle text (back) says: `At the beginning of your end step, you lose 1 life.`
- Code did:
  ```rust
  let (controller, is_transformed) = match state.get_object(self_id) {
      Some(o) if o.zone == Zone::Battlefield => (o.controller, o.is_transformed),
      _ => return,
  };
  if is_transformed { state.change_life(controller, -1); ... }
  ```

CR 113.7a: a triggered ability on the stack exists independently of its source.
"You lose 1 life" names nothing about the Unholy Fiend, so killing it in
response to its own end-step trigger does not save the life. The handler failed
*both* checks in that case — the object is in the graveyard, and `move_object`
clears `is_transformed` on the way out — so the life loss silently never
happened. Killing a creature in response to its trigger is ordinary play.

Fixed the same way as Howlpack Alpha's Wolf token: read
`state.last_known_controller(self_id)` (CR 608.2g) and drop both guards. The
`is_transformed` re-check is unnecessary anyway — step triggers are picked by
face when they are collected (`face_trigger_description`), and the front face
declares no end-step ability, so the hook is only reached for a permanent that
was an Unholy Fiend when the end step began.

**The same shape, found by sweeping for it, in Bloodgift Demon.**

Having hit this twice I swept every `on_upkeep` / `on_end_step` / `on_end_combat`
handler in the set for a battlefield-presence guard. Nineteen matched, but most
are Werewolves whose ability is "transform this creature" — those genuinely need
the permanent, and `apply_transform` guards it anyway. Two were real:

- Cloistered Youth, above.
- **Bloodgift Demon**: `At the beginning of your upkeep, target player draws a
  card and loses 1 life.` The effect is entirely about the target; the Demon is
  not mentioned. `if state.get_object(self_id).is_none_or(|o| o.zone !=
  Zone::Battlefield) { return; }` meant killing the Demon in response to its own
  trigger stopped the draw. Fixed and tested. (It keeps its own full audit entry
  when the list reaches it.)

Curse of the Pierced Heart matched the sweep too but is already correct — it
carries a comment citing CR 113.7a and uses `attached_player`'s last-known-
information fallback. Civilized Scholar's end-step handler also matched and is
correct: "tap this creature, then transform it" does need the permanent.

### Rulings checked

The only published ruling is a link to a mechanics article, with no rules
content.

### Tricky interactions checked

- **"You may transform"** is a real choice, presented as `YesNo` to the
  controller; declining leaves it a Youth. Not auto-applied. PASS.
- **Trigger scope** is `Your` for both faces ("your upkeep", "your end step"),
  so neither fires on the opponent's turn. PASS.
- **The front face has no end-step ability and the back face has no upkeep
  ability.** Each face declares exactly one trigger, and the collector picks by
  face — so a Youth never loses life and a Fiend is never offered the transform.
  PASS.
- **It only transforms one way.** `should_transform` returns `false` (this is
  not a Werewolf), and nothing in the set turns a Horror back into a Human —
  Moonmist transforms *Humans*, and the back face is `Creature — Horror`. So the
  one-way flip is correct. PASS.
- **The life loss goes through `state.change_life`**, the single writer that
  emits `LifeChanged`, rather than assigning the life total. PASS.
- **Back face characteristics**: 3/3 Horror, losing the Human type. Since the
  card declares `back_face_data`, `name_of`, subtypes and P/T all read the
  active face — which matters because Human-matters cards (Elder Cathar, Hamlet
  Captain, Spare from Evil) must stop seeing it once it flips. PASS.

### Test coverage

- life loss survives the Fiend dying in response: `cards_transforming_permanents.rs::unholy_fiends_life_loss_happens_even_if_it_dies_in_response` (new, mutation-checked).
- Bloodgift Demon's trigger survives the Demon dying in response: `cards_upkeep_triggers_and_curses.rs::bloodgift_demons_trigger_resolves_even_if_the_demon_dies_in_response` (new, mutation-checked).
- transform on upkeep and the decline branch: `cards_transforming_permanents.rs`.
- stops being a Human once flipped: `subtype.rs`, `continuous_effects.rs`.

