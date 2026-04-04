## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
**Type line**: Creature — Homunculus
**Status**: ISSUE

### Code issues

- Engine bug: `trigger_event_index` desync causes `CreatureDied` events from the sacrifice to be skipped when ETB-watch permanents (Champion of the Parish, Mentor of the Meek, Dearly Departed) are present on the battlefield and the sacrifice choice is presented (i.e., 2+ creatures exist so `present_target_choice` uses `awaiting_action` rather than auto-applying).
  - Oracle text says: `{1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.`
  - Mechanism: After `ActivateAbility` resolves, `process_triggers` is called. `collect_triggers` processes the `EnteredBattlefield` event (index 0), sets `state.trigger_event_index = 1` (`triggers.rs:873`). An ETB-watch trigger (e.g., Champion of the Parish's `AnyCreatureEnters`) is pushed to the stack and resolved. After that resolution, `if state.awaiting_action.is_some()` is true (the sacrifice `ResolutionChoice` set by `on_activate_ability` is still active), so `process_triggers` returns early (`triggers.rs:1035-1037`), leaving `trigger_event_index = 1` without resetting it to 0.
  - When the player then submits `ResolveChoice` for the sacrifice, `submit_action` clones the state (copying `trigger_event_index = 1`) and calls `new_state.events.clear()` (`engine.rs:1450`) — clearing events without resetting `trigger_event_index`. The sacrifice then pushes new events: `CreatureDied` at index 0 and `LeftBattlefield` at index 1. When `process_triggers` is called after `ResolveChoice`, `collect_triggers` starts scanning from `start = trigger_event_index = 1` (`triggers.rs:336`), skipping `CreatureDied` at index 0. As a result, `SelfDies` and `DeathWatch` triggers from the sacrificed creature never fire.
  - Code does: `new_state.events.clear()` in `engine.rs:1450` without resetting `new_state.trigger_event_index`, combined with `state.trigger_event_index = events.len()` in `triggers.rs:873` persisting across action boundaries when `process_triggers` returns early at `triggers.rs:1035-1037`.

### Tricky interactions checked

- Sacrifice is part of the effect, not the cost: pass — `sacrifice_cost: SacrificeCost::None` is correct; the sacrifice is implemented as a `PendingEffect::SacrificeCreature` via `present_target_choice` during `on_activate_ability`.
- Sacrifice is mandatory (no "you may"): pass — `present_target_choice` called with `optional: false` (line 79), and when `optional = false` the `available_actions` generator does not produce a `ChosenTarget(None)` option (`engine.rs:199-201`).
- Newly created token is a valid sacrifice target: pass — `creatures_controlled_by` is called AFTER `create_token_with_subtypes`, so the token is already in `state.objects` on the battlefield and is included in the list.
- Stitcher's Apprentice itself is a valid sacrifice target: pass — `creatures_controlled_by` includes all `power.is_some()` objects controlled by the player, including the source.
- Auto-apply when exactly one creature remains: pass — `present_target_choice` auto-applies when `targets.len() == 1 && !optional` (helpers.rs:129-133); in practice after token creation there are always ≥ 2 creatures, so this branch fires only in unusual edge cases (e.g., token creation replacement).
- Sacrifice choice: player gets to choose among all controlled creatures: pass — `ResolutionChoiceKind::ChooseTarget` with `optional = false` presents all creatures as selectable options.
- Token stats (2/2 blue Homunculus creature token): pass — `create_token_with_subtypes("Homunculus", controller, 2, 2, vec![Color::Blue], vec![CardType::Creature], vec![], vec!["Homunculus".into()])` matches oracle exactly.
- Parallel Lives doubles token production: pass — `create_token_with_subtypes` checks for Parallel Lives and creates extra copies; the single sacrifice still applies.
- ETB triggers on token resolve after sacrifice (per ruling): partial concern — `process_triggers` is called before the sacrifice `ResolutionChoice` is presented; ETB-watch triggers from other permanents (e.g., Champion of the Parish) are resolved before the player makes the sacrifice choice, which is technically incorrect per the ruling. However, since the token itself (CardId(0)) has no self-ETB behavior, this ordering only matters for watchers on other permanents.
- Death triggers fire after sacrifice: FAIL in edge case — as described in the code issues section, when ETB-watch triggers cause `process_triggers` to return early, the subsequent `CreatureDied` events from the sacrifice are skipped and death triggers do not fire.
- Ability usable at instant speed: pass — `sorcery_speed_only: false`.
- Once-per-turn restriction: pass — `once_per_turn: false` (no such restriction per oracle text).

### Test coverage

- Basic token creation and player chooses which creature to sacrifice: `mtg-engine/tests/tier8_cards.rs:413` (`stitchers_apprentice_creates_token_then_sacrifices`) — TESTED
- Token is 2/2 named "Homunculus": `mtg-engine/tests/tier8_cards.rs:466` (`stitchers_apprentice_token_is_2_2_homunculus`) — TESTED
- Sacrifice is mandatory (player cannot skip): NOT TESTED
- Token can be sacrificed (player sacrifices the token): TESTED (tier8_cards.rs:450 sacrifices the token by choice)
- Apprentice can sacrifice itself: NOT TESTED
- Death triggers fire after sacrifice: NOT TESTED
- ETB-watch triggers with sacrifice choice interaction (`trigger_event_index` desync): NOT TESTED
- Parallel Lives doubles token: NOT TESTED
- Ruling: "nothing can happen between token creation and sacrifice": NOT TESTED
