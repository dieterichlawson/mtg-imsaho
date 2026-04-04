## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Status**: ISSUE

### Code issues

- Dealing damage to a planeswalker via the choice path does not remove loyalty counters
  - Oracle text says: `"this Aura deals 1 damage to that player or a planeswalker that player controls"`
  - Code does: when a planeswalker target is chosen, `apply_pending_effect` in `mtg-engine/src/engine.rs` line 2181 executes `obj.damage_marked += amount` on the planeswalker object. No loyalty counters are removed. At cleanup (engine.rs line 3016), `damage_marked` is reset to 0 with no effect. Nothing in SBAs (`sba.rs`) converts `damage_marked` to loyalty counter removal for planeswalkers. The planeswalker survives the "damage" unharmed. Contrast with `stensia_bloodhall.rs` line 93–94 which correctly does `*loyalty = loyalty.saturating_sub(2)` when dealing damage to a planeswalker.

- The upkeep trigger goes on the stack during every player's upkeep, not only the enchanted player's upkeep
  - Oracle text says: `"At the beginning of enchanted player's upkeep"`
  - Code does: `mtg-engine/src/triggers.rs` lines 597–643, `StepStarted::Upkeep` handling collects all battlefield permanents with a non-empty `TriggerKind::Upkeep` description and pushes them onto the stack unconditionally — there is no check for which player's upkeep it is at collection time. The Curse has description `"deal 1 damage to enchanted player"` (non-empty), so an `UpkeepTrigger` is created and pushed onto the stack on every upkeep. The early-return check (`if state.active_player != cursed_player { return; }`) in `curse_of_the_pierced_heart.rs` line 58 only runs after the trigger has already been placed on the stack and resolved, meaning the trigger incorrectly appears on the stack — requiring players to pass priority — during the Curse controller's upkeep.

### Tricky interactions checked

- **Correct upkeep check**: The trigger fires during the enchanted player's upkeep — the `on_upkeep` check at line 58 (`state.active_player != cursed_player`) prevents effect during the wrong upkeep. The damage IS correctly dealt during the enchanted player's upkeep. Pass (for this path).
- **No-planeswalker path (damage to player)**: When no planeswalkers are present, `on_upkeep` directly reduces cursed player's life by 1 and pushes `NonCombatDamageDealt` + `LifeChanged` events (lines 71–79). Behavior matches the player-damage path of `apply_pending_effect`. Pass.
- **Planeswalker-present path (choice prompt)**: When planeswalkers are present, a `ResolutionChoice::ChooseTarget` is presented to the Curse's `controller` (not the cursed player). This is correct — the Curse controller makes the non-targeted choice. The option list includes `Target::Player(cursed_player)` and all `Target::Object(pw_id)` entries. Pass for who chooses and what options are shown.
- **Damage to chosen planeswalker removes loyalty counters**: FAIL. As described above, `apply_pending_effect` with `Target::Object` marks `damage_marked` instead of subtracting loyalty counters. The planeswalker is unaffected.
- **Trigger fires only on enchanted player's upkeep**: FAIL. The trigger goes on the stack during every upkeep (engine level), though it fizzles during the wrong upkeep.
- **Curse controller vs. cursed player gets the choice**: Pass. The `player: controller` field in the `AwaitingAction::ResolutionChoice` correctly gives the choice to the Curse's controller (line 88).
- **No "target" keyword — non-targeted ability**: The ability uses no `target` keyword. The choice is presented as a `ChooseTarget` resolution choice but this is an internal construct, not a rules targeting. No targeting rules violations arise from this. Pass.
- **Curse leaves battlefield between trigger and resolution**: The `on_upkeep` handler checks `o.zone == Zone::Battlefield` at line 50 before proceeding. If the Curse is destroyed before the trigger resolves, `on_upkeep` returns early. Per MTG rules, the ability should still resolve if the Curse leaves (it's an ability that has already triggered), but this early return causes the ability to fizzle. This is a pre-existing engine pattern (same in Curse of the Bloody Tome). Not flagged here as a separate new issue for this card.
- **Multiple planeswalkers present**: All planeswalkers controlled by the cursed player are included in the options list (lines 63–67 collect all `CardType::Planeswalker` objects controlled by `cursed_player`). Pass for option enumeration.

### Test coverage

- Basic damage to player on correct upkeep: `tier7_cards.rs:176` (`curse_of_pierced_heart_deals_damage_on_upkeep`) — TESTED
- Trigger does NOT fire during curse controller's upkeep: NOT TESTED
- Choice between player and planeswalker when planeswalker is present: NOT TESTED
- Damage to planeswalker removes loyalty counters: NOT TESTED (and behavior is wrong)
- Curse attached to player resolves correctly via `resolve_curse` helper: NOT TESTED directly (coverage exists for Bitterheart Witch placing the curse in `tier15_cards.rs`)
- Trigger fizzles if Curse leaves battlefield before resolution: NOT TESTED
