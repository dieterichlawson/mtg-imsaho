## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Non-combat damage also triggers** (ruling: "Any damage dealt by the enchanted creature to an opponent will cause Curiosity to trigger, not just combat damage"): PASS. `card_data()` declares `TriggerKind::AnyDamageToPlayer`. In `triggers.rs`, both the `CombatDamageDealt` branch (lines 542–559) and the `NonCombatDamageDealt` branch (lines 566–595) collect `DamageToPlayerWatch` triggers for any battlefield object with `AnyDamageToPlayer`. Both paths call `on_any_damage_to_player`, which correctly checks `source_id == attached_to`.

- **Damage to planeswalker/battle does NOT trigger** (ruling: "Curiosity doesn't trigger if the enchanted creature deals damage to a planeswalker or to a battle"): PASS. Both `CombatDamageDealt` and `NonCombatDamageDealt` in `triggers.rs` only enter the `DamageToPlayerWatch` collection path when `target` matches `DamageTarget::Player(...)`. Damage to permanents (planeswalkers, battles) is a `DamageTarget::Object(...)` and is never routed to Curiosity's callback.

- **Damage to Curiosity's controller does NOT trigger** (ruling: "If you control Curiosity and it's enchanting an opponent's creature, you won't draw a card when that creature deals damage to you"): PASS. `on_any_damage_to_player` (curiosity.rs:61–64) checks `if damaged_player == controller { return; }` using Curiosity's own controller, not the enchanted creature's controller.

- **"You" is Curiosity's controller, not the enchanted creature's controller** (ruling: "'You' refers to the controller of Curiosity"): PASS. The code reads `let controller = aura.controller;` from the Curiosity object (line 61), and all draws go to that player (lines 67, 82).

- **"You may" is genuinely optional**: PASS. `on_any_damage_to_player` sets `state.awaiting_action` to a `ResolutionChoiceKind::YesNo` choice presented to the controller. `on_yes_no_choice` returns early without drawing if `!yes` (line 77–79).

- **One card drawn per damage event, regardless of amount** (ruling: "You draw one card each time the enchanted creature deals damage to an opponent, no matter how much damage it deals"): PASS. The trigger fires once per `DamageDealt` event regardless of `amount`. `on_yes_no_choice` calls `draw_cards(state, controller, 1)` (line 82), always exactly 1.

- **Only the enchanted creature triggers Curiosity, not any creature**: PASS. `on_any_damage_to_player` checks `if source_id != attached_to { return; }` (lines 57–59), where `attached_to` is the specific creature Curiosity is currently attached to.

- **Curiosity correctly identifies the enchanted creature via `attached_to` field**: PASS. `resolve_aura` in `helpers.rs` sets `obj.attached_to = Some(*target_id)` when the aura resolves (line 23). The `on_any_damage_to_player` callback reads `aura.attached_to` at resolution time.

- **Aura resolves by attaching to target creature**: PASS. `on_resolve` calls `crate::cards::helpers::resolve_aura(state, object_id, targets)`, which attaches the aura to the target if it's still on the battlefield, or calls `move_spell_after_resolve` (sends to graveyard) if the target left.

- **Enchant keyword absent from `keywords` vec**: PASS (not an issue). "Enchant" is listed as a Scryfall keyword but is not in the engine's `Keyword` enum (which contains only keyword abilities like Flying, Lifelink, etc.). Its function is encoded via `TargetRequirement::Creature` and `attached_to` mechanics.

- **`DamageToPlayerWatch` trigger: watcher-still-on-battlefield guard in `resolve_next_trigger`**: PASS for normal gameplay. The battlefield check (`zone == Zone::Battlefield` on the watcher) in `resolve_next_trigger` (triggers.rs:941) could theoretically suppress a triggered ability that was already on the stack if Curiosity was destroyed in response — but the engine resolves all triggers synchronously before granting priority, so no opponent can destroy Curiosity "in response" in the current engine. The guard does not suppress any reachable scenario.

- **`TriggerKind::AnyDamageToPlayer` description is non-empty (required for trigger collection)**: PASS. The `TriggeredAbilityDef` has `description: "you may draw a card".into()` (curiosity.rs:33). In `triggers.rs`, the `AnyDamageToPlayer` watcher path only creates a trigger when `!desc.is_empty()` (line 543–544). Since the description is non-empty, the trigger is always collected.

### Test coverage
For each ruling and tricky interaction, whether it is tested and where:

- Combat damage to opponent triggers draw (yes): `tier6_cards.rs:357` (`curiosity_draw_on_enchanted_creature_combat_damage`)
- "You may" optionality — player declines, no draw: `tier6_cards.rs:405` (`curiosity_decline_draw`)
- Non-combat damage to opponent triggers draw: NOT TESTED
- Damage to Curiosity's controller does not trigger: NOT TESTED
- Damage to planeswalker/battle does not trigger: NOT TESTED
- One card drawn regardless of damage amount: NOT TESTED (only tested with a specific amount; no test asserts that a 5-damage hit also gives exactly 1 draw)
- Curiosity enchanting opponent's creature, draws for Curiosity's controller (not enchanted creature's controller): NOT TESTED
