## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: ISSUE

### Code issues

- **Issue 1 — "target player" is auto-targeted instead of chosen** (`mtg-engine/src/cards/isd/falkenrath_noble.rs`, lines 59–68)
  - Oracle text says: `"target player loses 1 life and you gain 1 life"`
  - Code does: `fn drain(state: &mut GameState, controller: PlayerId) { let opponent = state.opponent(controller); ... }` — the `drain` helper hard-codes the opponent as the life-loss target, with a comment explicitly acknowledging this: `"In 2-player, auto-targets the opponent (per project convention)."` No `awaiting_action` / `present_target_choice` is ever created. The player is never given a choice of target. By contrast, Rage Thrower (same trigger pattern, also "target player") correctly calls `present_target_choice` and generates a `ResolutionChoiceKind::ChooseTarget` action (`mtg-engine/src/cards/isd/rage_thrower.rs`, line 54). Falkenrath Noble should do the same for both `on_dies` (line 44) and `on_any_creature_dies` (line 53) paths.

- **Issue 2 — simultaneous death triggers only once instead of twice** (`mtg-engine/src/triggers.rs`, lines 418–421)
  - Oracle ruling says: `"If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them."`
  - Code does: In `collect_triggers`, the death-watch scan is `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)`. When multiple creatures die in the same SBA pass (`check_state_based_actions_with_registry`), each `destroy()` call immediately moves the object to `Zone::Graveyard` via `state.move_object(id, Zone::Graveyard)` before `collect_triggers` runs. So when `collect_triggers` processes `CreatureDied{OtherCreature}`, Falkenrath Noble is already in `Zone::Graveyard` and fails the `o.zone == Zone::Battlefield` filter. Only the `SelfDies` trigger fires (1 trigger); the `DeathWatch` trigger for the other creature's simultaneous death never fires. Per the ruling, 2 triggers should fire.

- **Issue 3 — DeathWatch trigger incorrectly fizzles if Noble leaves the battlefield between trigger and resolution** (`mtg-engine/src/triggers.rs`, lines 906–912, and `mtg-engine/src/cards/isd/falkenrath_noble.rs`, lines 49–52)
  - Oracle text says: `"Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life."` — no requirement that Noble remain on the battlefield at resolution.
  - Code does: `resolve_next_trigger` for `PendingTrigger::DeathWatch` guards with `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` — if Noble is bounced, destroyed, or otherwise removed from the battlefield after the trigger is placed on the stack but before it resolves, the trigger silently fizzles. The card's own `on_any_creature_dies` has a redundant identical guard (`Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return`). Per CR 112.7a, once a triggered ability is on the stack it exists independently of its source; removing Noble from the battlefield should not prevent resolution. The life-loss and life-gain effect does not require Noble to be present.

### Tricky interactions checked

- **"target player" choice (both players valid targets)**: FAIL — code auto-selects opponent; no choice presented to controller.
- **Noble's own death fires its trigger (SelfDies)**: PASS — `PendingTrigger::SelfDies` is generated and resolved without a battlefield check; `get_object` finds Noble in graveyard with correct `controller` field intact after `move_object`.
- **Another creature dying while Noble is on battlefield**: PASS — death-watch scan finds Noble on battlefield and `DeathWatch` trigger fires correctly.
- **Opponent's creature dying while Noble is on battlefield**: PASS — `AnyCreatureDies` has no controller filter; opponent creature deaths are watched.
- **Simultaneous death (Noble + another creature die together)**: FAIL — per ruling, two triggers should fire; only one fires because Noble is already in graveyard when the death-watch scan runs for the other creature's `CreatureDied` event.
- **DeathWatch trigger resolution when Noble leaves battlefield**: FAIL — trigger fizzles due to `o.zone == Zone::Battlefield` check in both `resolve_next_trigger` (engine) and `on_any_creature_dies` (card); per CR 112.7a triggers on the stack are independent of their source.
- **Controller lookup in on_dies after Noble is in graveyard**: PASS — `state.get_object` searches `state.objects` (a HashMap keyed by ObjectId containing all zones), `move_object` changes `zone` field but does not remove the entry; `controller` field is not cleared on zone change, so the correct PlayerId is retrieved.
- **Flying keyword**: PASS — `keywords: vec![Keyword::Flying]` present in `card_data`.
- **Mana cost {3}{B}**: PASS — `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Black)])`.
- **P/T 2/2**: PASS — `power: Some(2), toughness: Some(2)`.
- **Subtypes Vampire Noble**: PASS — `subtypes: vec!["Vampire".into(), "Noble".into()]`.

### Test coverage

- **"target player" choice presented**: NOT TESTED — all existing tests (`falkenrath_noble_drains_on_any_death`, `falkenrath_noble_triggers_on_opponent_creature_death`, `falkenrath_noble_triggers_on_own_creature_death`, `falkenrath_noble_triggers_on_self_death`) assert life totals directly without checking `awaiting_action`, thereby enshrining the auto-targeting behavior.
- **Simultaneous death triggers twice (ruling 2017-03-14)**: NOT TESTED — no test exists where Noble and another creature die in the same SBA pass.
- **DeathWatch trigger resolves after Noble leaves battlefield**: NOT TESTED.
- **Noble's own death triggers the ability (SelfDies)**: `mtg-engine/tests/bug_fixes.rs:449` (`falkenrath_noble_triggers_on_self_death`).
- **Opponent's creature death triggers Noble**: `mtg-engine/tests/bug_fixes.rs:401` (`falkenrath_noble_triggers_on_opponent_creature_death`).
- **Controller's own creature death triggers Noble**: `mtg-engine/tests/bug_fixes.rs:426` (`falkenrath_noble_triggers_on_own_creature_death`).
- **Noble drains life on any creature death (basic)**: `mtg-engine/tests/tier3_cards.rs:283` (`falkenrath_noble_drains_on_any_death`).
- **APNAP ordering with Noble**: `mtg-engine/tests/apnap.rs:94` (`non_active_player_triggers_resolve_first`), `mtg-engine/tests/apnap.rs:195` (`apnap_lifo_order_with_life_totals`).
