## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.
**Type line**: Creature — Human Shaman
**Status**: ISSUE

### Code issues

- **Issue 1 — Simultaneous death: Rage Thrower's trigger does not fire for a creature dying at the same time** (`mtg-engine/src/triggers.rs`, lines 418–440)
  - Oracle text says (ruling 2011-09-22): `"If Rage Thrower dies at the same time as another creature, its ability will trigger."`
  - Code does: In `collect_triggers`, the death-watch watcher scan is `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)`. When multiple creatures die in the same SBA pass (`check_state_based_actions_with_registry` in `sba.rs`), each `destroy()` call immediately moves the object to `Zone::Graveyard` via `state.move_object(id, Zone::Graveyard)` before `collect_triggers` runs. So when `collect_triggers` processes `CreatureDied{other_creature}`, Rage Thrower is already in `Zone::Graveyard` and fails the `o.zone == Zone::Battlefield` filter. Its trigger is not collected and never fires. The ruling requires it to fire.

- **Issue 2 — DeathWatch trigger incorrectly fizzles if Rage Thrower leaves the battlefield after triggering but before resolution** (`mtg-engine/src/triggers.rs`, lines 906–912; `mtg-engine/src/cards/isd/rage_thrower.rs`, lines 39–42)
  - Oracle text says: `"Whenever another creature dies, this creature deals 2 damage to target player or planeswalker."` — no condition that Rage Thrower remains on the battlefield at resolution. Per CR 112.7a, a triggered ability on the stack is independent of its source.
  - Code does: `resolve_next_trigger` for `PendingTrigger::DeathWatch` guards with `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` — if Rage Thrower is bounced, destroyed, or exiled after the trigger is on the stack but before it resolves, the trigger silently fizzles and no damage is dealt. Additionally, `on_any_creature_dies` in `rage_thrower.rs` has an identical redundant guard: `Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return`. Both guards must be removed for this path; only the target-choice result matters, not the current zone of the source.

- **Issue 3 — Damage targeting a planeswalker does not reduce loyalty counters** (`mtg-engine/src/engine.rs`, lines 2179–2191; `mtg-engine/src/cards/isd/rage_thrower.rs`, line 56)
  - Oracle text says: `"this creature deals 2 damage to target player or planeswalker"` — damage dealt to a planeswalker reduces its loyalty counters by the damage amount (MTG rule 306.6).
  - Code does: Rage Thrower uses `PendingEffect::DealDamage { amount: 2, ... }` (rage_thrower.rs line 56). In `apply_pending_effect`, the `(Target::Object(id), PendingEffect::DealDamage { ... })` arm only does `obj.damage_marked += amount` (engine.rs line 2181) and does not reduce `CounterType::Loyalty`. The SBA for planeswalkers (`sba.rs` line 220) only checks `*o.counters.get(&crate::types::CounterType::Loyalty).unwrap_or(&0) == 0`; it never reads `damage_marked` for planeswalkers. So 2 damage dealt to a planeswalker via Rage Thrower has no effect on loyalty whatsoever. Contrast: `stensia_bloodhall.rs` lines 90–94 explicitly does `*loyalty = loyalty.saturating_sub(2)` when dealing damage to a planeswalker target.

- **Issue 4 — Trigger description and target-choice prompt omit "or planeswalker"** (`mtg-engine/src/cards/isd/rage_thrower.rs`, lines 33 and 57)
  - Oracle text says: `"deals 2 damage to target player or planeswalker"`
  - Code does: `TriggeredAbilityDef { description: "deal 2 damage to target player".into() }` (line 33) and `present_target_choice(... "Rage Thrower: deal 2 damage to target player" ...)` (line 57). Both omit "or planeswalker". The stack display and the player-facing target prompt will misrepresent what the ability can target.

### Tricky interactions checked

- **"another" creature — Rage Thrower's own death does not self-trigger**: PASS — Rage Thrower has no `SelfDies` trigger defined and `on_dies` is not implemented (default no-op). It correctly does not trigger on its own death.
- **Simultaneous death (Rage Thrower + another creature die in same SBA pass)**: FAIL — Rage Thrower is already in the graveyard when collect_triggers scans the battlefield for watchers; its AnyCreatureDies trigger is not collected. Violates the official ruling.
- **DeathWatch trigger resolves after Rage Thrower leaves battlefield**: FAIL — both `resolve_next_trigger` (triggers.rs line 908) and `on_any_creature_dies` (rage_thrower.rs line 39) guard on `Zone::Battlefield`; trigger fizzles silently if Rage Thrower is removed after triggering.
- **"target player or planeswalker" targeting — player target**: PASS — `state.players.iter().filter(|p| !p.lost).map(|p| Target::Player(p.id))` correctly includes all living players (not just opponent).
- **"target player or planeswalker" targeting — planeswalker target listed**: PASS — `state.objects.values().filter(|obj| obj.zone == Zone::Battlefield && obj.card_types.contains(&CardType::Planeswalker))` correctly adds planeswalkers as valid targets.
- **"target player or planeswalker" — planeswalker damage functional**: FAIL — `PendingEffect::DealDamage` through `apply_pending_effect` only marks `damage_marked`; no loyalty counter reduction occurs; planeswalker is unaffected.
- **Mandatory target choice (no "may")**: PASS — `present_target_choice(..., optional: false)` is passed; the choice is not optional. When exactly one target exists, the engine auto-applies. Otherwise, an `AwaitingAction::ResolutionChoice` is generated for the player to choose.
- **Rage Thrower controller check at trigger resolution**: PASS — `on_any_creature_dies` correctly re-reads `o.controller` from the live object at resolution time rather than using captured data.
- **Mana cost {5}{R}**: PASS — `ManaCost::new(vec![ManaSymbol::Generic(5), ManaSymbol::Colored(Color::Red)])`.
- **P/T 4/2**: PASS — `power: Some(4), toughness: Some(2)`.
- **Subtypes Human Shaman**: PASS — `subtypes: vec!["Human".into(), "Shaman".into()]`.
- **Keywords — none**: PASS — `keywords: vec![]`.
- **Missing `!desc.is_empty()` check in death-watch collection**: NOTED (engine-level) — unlike the ETB-watch section (triggers.rs line 375) which guards `if !desc.is_empty()`, the death-watch section (lines 422–440) pushes a `DeathWatch` trigger for every registered permanent on the battlefield regardless of whether it has `AnyCreatureDies` in its `triggered_abilities`. This creates spurious no-op triggers on the stack for every creature death. Does not affect Rage Thrower's own correctness (its trigger still fires), but produces misleading stack entries for cards without the ability. Not flagged as a Rage Thrower-specific ISSUE.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- **Normal case: Rage Thrower trigger fires when another creature dies while Rage Thrower is alive**: `mtg-engine/tests/tier3_cards.rs:310` (`rage_thrower_deals_2_on_death`) — TESTED
- **Simultaneous death ruling: Rage Thrower dies at same time as another creature, both trigger**: NOT TESTED
- **DeathWatch trigger resolves after Rage Thrower leaves battlefield**: NOT TESTED
- **Targeting a planeswalker reduces loyalty**: NOT TESTED
- **"Target player or planeswalker" — controller can choose to target self (not auto-opponent)**: NOT TESTED
- **Trigger fires for opponent's creature death (not just own creatures)**: NOT TESTED (the existing test uses `P1` as victim and `P0` as Rage Thrower controller, which does cover opponent-controlled creature dying, but does not verify the player choice is correctly presented vs. auto-applied)
- **Trigger description on stack includes "or planeswalker"**: NOT TESTED
