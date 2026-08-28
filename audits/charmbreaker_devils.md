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
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/134/charmbreaker-devils?utm_source=api
**Type line**: `Creature — Devil` — {5}{R}, 4/4
**Oracle text**:
```
At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
```

**Rulings fetched**:
- [2017-11-17] The instant or sorcery card returned to your hand is chosen at random as Charmbreaker Devils's first ability resolves. If any player responds to the ability, that player won't yet know what card will be returned.
- [2017-11-17] Because the first ability doesn't target the instant or sorcery card, any instants or sorceries put into your graveyard in response to that ability may be returned to your hand.
- [2017-11-17] All players get to see which card you chose at random as it's returned to your hand.

**Status**: ISSUE (fixed)

### Code issues

**The spell-cast trigger re-asked its own condition on resolution — twice, differently.**

- Oracle text says: `Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.`
- `should_trigger_on_spell_cast` already gated on both halves (CR 603.2), and
  `on_spell_cast` then did it again:
  ```rust
  if caster != controller { return; }
  let is_instant_or_sorcery = state.get_object(spell_id)
      .and_then(|o| state.face_data(o.id, registry))
      .is_some_and(|d| d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery));
  if !is_instant_or_sorcery { return; }
  ```

Three things wrong with that, in increasing order of seriousness:

1. It is a duplicate of a condition that already has one correct home.
2. The two copies had drifted — the gate asks `has_card_type`, the re-check
   asked `face_data(...).card_types`.
3. It is wrong by the rules. CR 603.4 re-checks only an intervening-if clause,
   and this ability has none: once it has triggered, the pump is unconditional.
   The re-check read the *current* controller, so an instant-speed control
   change between the cast and the resolution would have swallowed it — and CR
   113.7a says the ability resolves and the +4/+0 lands on this creature
   whoever controls it by then.

Not reachable in this pool (no instant-speed control change; Traitorous Blood is
a sorcery), but wrong, and it is the same shape as the re-check removed from
Wooden Stake earlier in this audit.

Three tests called `on_spell_cast` directly and asserted no pump for a creature
spell. With the condition living only in the gate, those were testing the wrong
layer — a direct call now always pumps, correctly. They go through the trigger
system now, which is what decides whether the ability fires at all.

### Rulings checked

- **"The instant or sorcery card returned to your hand is chosen at random as
  Charmbreaker Devils's first ability resolves. If any player responds to the
  ability, that player won't yet know what card will be returned."** The pick
  happens inside `on_upkeep` — the resolution — not when the trigger is
  collected. PASS.
- **"Because the first ability doesn't target the instant or sorcery card, any
  instants or sorceries put into your graveyard in response to that ability may
  be returned to your hand."** The trigger declares no target requirement and
  the graveyard is enumerated at resolution, so a card milled in response is a
  candidate. PASS.
- **"All players get to see which card you chose at random as it's returned to
  your hand."** The return is logged at `LogLevel::Event`, and
  `GameView::for_player` puts `Info` and above into `display_log`, which both
  players read. Checked the enum ordering rather than assuming — `Event = 2`,
  `Info = 1`. This is the mirror of Delver of Secrets, where the card that is
  looked at must *not* be visible and is logged at `Debug`. PASS.

### Tricky interactions checked

- **"an instant or sorcery **card**"** — `is_card` excludes a token still in the
  graveyard (CR 109.1), and the type test goes through `face_data`, which is the
  front face for anything outside the battlefield (CR 712.8a). PASS.
- **"At the beginning of **your** upkeep"** — `TriggerScope::Your`, so an
  opponent's Devils return nothing on your turn. Had no test; now covered and
  mutation-checked by flipping the scope to `Each`. PASS.
- **An empty graveyard, or one with no instants or sorceries**, returns nothing
  rather than erroring. PASS, now tested.
- **The pump is `until_end_of_turn`, keyed on the object**, so it survives the
  Devils changing controller and expires at cleanup. PASS.
- **Casting two instants gives +8/+0** — each cast pushes its own effect. PASS,
  covered by the existing pump test plus the stacking assertion at
  `cards_upkeep_triggers_and_curses.rs:256`.

### Test coverage

- returns an instant, ignores a creature card: `cards_upkeep_triggers_and_curses.rs::charmbreaker_devils_returns_an_instant_or_sorcery_at_upkeep` (new).
- nothing to return: `::charmbreaker_devils_does_nothing_with_no_instants_or_sorceries` (new).
- not on the opponent's upkeep: `::charmbreaker_devils_returns_nothing_on_the_opponents_upkeep` (new, mutation-checked).
- +4/+0 on an instant: `::charmbreaker_devils_plus4_on_spell_cast`, `cards_shortcuts_taken.rs::charmbreaker_devils_does_pump_on_instant_spell`.
- no pump on a creature spell: `cards_shortcuts_taken.rs::charmbreaker_devils_no_pump_on_creature_spell`, `trigger_dispatch.rs::bug_l_charmbreaker_devils_does_not_buff_on_creature_spell` — both now driven through the trigger system.

