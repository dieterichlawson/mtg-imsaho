---
id: curiosity-01
status: new
card: Curiosity
audit_run_id: 2026-04-19-curiosity-audit
audit_model: sonnet
audit_tokens: 23098
audit_duration: 429
---

## Audit Finding

**Oracle text:**
> Whenever enchanted creature deals damage to an opponent, you may draw a card.

**Code:**
> // AnyDamageToPlayer watchers (combat damage is also damage).
let desc2 = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::AnyDamageToPlayer, false);
if !desc2.is_empty() {
    let trigger = PendingTrigger::DamageToPlayerWatch {
        watcher_id,
        watcher_card_id,
        controller: watcher_controller,
        source_id,
        damaged_player: *damaged_player,
        amount: *amount,
        description: desc2,
    };
    if watcher_controller == active_player {
        ap_triggers.push(trigger);
    } else {
        nap_triggers.push(trigger);
    }
}

**Description:**
The `AnyDamageToPlayer` dispatch in `triggers.rs` (lines 794–811 for combat damage, lines 818–848 for non-combat damage) creates a `DamageToPlayerWatch` trigger for every battlefield permanent with that trigger kind whenever ANY permanent deals damage to ANY player, with no pre-filtering on source identity or target player identity. Curiosity's oracle condition is 'enchanted creature deals damage to an opponent' — two constraints the dispatch ignores entirely. As a result, Curiosity is placed on the stack whenever (a) a creature other than the enchanted one deals damage to any player, or (b) the enchanted creature deals damage to the Curiosity controller. The trigger resolves to do nothing in those cases (the in-card checks in `on_any_damage_to_player` return early), but the trigger's presence on the stack is a rules violation: per CR 603.2, a triggered ability goes on the stack only when the specified event occurs. A spurious stack entry grants players an incorrect priority window and reveals observable intermediate game state that should not exist under rules-strict execution. This is the same pattern documented for `SpellCast` dispatch (see the 'SpellCast trigger dispatch fires unconditionally' insight) but applied to the damage-to-player event path.

**Engine path:** mtg-engine/src/triggers.rs:794

**Required check:** 8b

## Tests

### non_enchanted_creature_deals_damage_curiosity_no_spurious_trigger
Scenario: When a creature other than the Curiosity-enchanted creature deals combat damage to a player, Curiosity should produce no stack entry at all — the trigger should not be created.

### enchanted_creature_damages_curiosity_controller_no_spurious_trigger
Scenario: When the Curiosity-enchanted creature (controlled by the opponent) deals combat damage back to the Curiosity controller, Curiosity should produce no stack entry — the damaged player is not an opponent of Curiosity's controller.

