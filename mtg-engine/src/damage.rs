//! Unified damage pipeline.
//!
//! All damage — combat, fight, and card effects — flows through
//! [`deal_damage`], which applies prevention, replacement, protection,
//! multipliers, planeswalker loyalty removal, deathtouch, `damaged_by`
//! tracking, lifelink, and events in one place. Engine and card code must
//! never write `damage_marked` directly: every hand-rolled copy of this
//! logic has historically missed at least one check (fight damage skipped
//! Unbreathing Horde's prevention; noncombat damage skipped deathtouch
//! and player-lifelink).

use crate::cards::CardRegistry;
use crate::events::{DamageTarget, GameEvent};
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::{Keyword, Zone, ContinuousEffect};

/// Whether damage is combat damage. Combat-only modifiers (Ghostly
/// Possession, Moonmist, Inquisitor's Flail, Undead Alchemist's
/// replacement) apply only to `Combat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageKind {
    Combat,
    NonCombat,
}

/// Deal damage from a source object to a creature, planeswalker, or player.
/// The single entry point for all damage in the engine.
pub fn deal_damage(
    state: &mut GameState,
    source: ObjectId,
    target: DamageTarget,
    amount: u32,
    kind: DamageKind,
    registry: &CardRegistry,
) {
    if amount == 0 {
        return;
    }
    match target {
        DamageTarget::Object(id) => deal_damage_to_object(state, source, id, amount, kind, registry),
        DamageTarget::Player(pid) => deal_damage_to_player(state, source, pid, amount, kind, registry),
    }
}

fn deal_damage_to_object(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: u32,
    kind: DamageKind,
    registry: &CardRegistry,
) {
    if state.get_object(target).is_none_or(|o| o.zone != Zone::Battlefield) {
        return;
    }

    // Combat-only prevention (Ghostly Possession, Moonmist).
    if kind == DamageKind::Combat {
        if has_combat_damage_prevention(state, source, registry)
            || has_combat_damage_prevention(state, target, registry)
        {
            return;
        }
        if is_combat_damage_prevented_by_exception(state, source, registry) {
            return;
        }
    }

    // Protection from the source prevents the damage (CR 702.16e).
    if state.has_protection_from(target, source, registry) {
        let name = state.obj_name(target);
        let source_name = state.obj_name(source);
        state.log(LogLevel::Event,
            format!("{name}: damage from {source_name} prevented by protection"));
        return;
    }

    // "Prevent damage, remove a +1/+1 counter" replacement (Unbreathing Horde).
    if apply_prevent_damage_remove_counter(state, target, registry) {
        return;
    }

    // Combat damage multipliers (Inquisitor's Flail): the source's Flails
    // multiply damage dealt, the target's multiply damage received.
    let mut amount = amount;
    if kind == DamageKind::Combat {
        amount *= combat_damage_multiplier(state, source, registry);
        amount *= combat_damage_multiplier(state, target, registry);
    }

    let has_deathtouch = state.has_keyword(source, Keyword::Deathtouch, registry);
    let is_planeswalker = state.has_card_type(target, crate::types::CardType::Planeswalker, registry);

    if let Some(obj) = state.get_object_mut(target) {
        if is_planeswalker {
            // Damage to a planeswalker removes that many loyalty counters (CR 120.3c).
            let loyalty = obj.counters.entry(crate::types::CounterType::Loyalty).or_insert(0);
            *loyalty = loyalty.saturating_sub(amount);
            if *loyalty == 0 {
                obj.counters.remove(&crate::types::CounterType::Loyalty);
            }
        } else {
            obj.damage_marked += amount;
            if has_deathtouch {
                obj.dealt_deathtouch_damage = true;
            }
        }
        // Track which objects damaged this one (Abattoir Ghoul, Into the Maw of Hell).
        if !obj.damaged_by.contains(&source) {
            obj.damaged_by.push(source);
        }
    }

    let event_target = DamageTarget::Object(target);
    match kind {
        DamageKind::Combat => state.events.push(GameEvent::CombatDamageDealt {
            source, target: event_target, amount,
        }),
        DamageKind::NonCombat => {
            state.events.push(GameEvent::NonCombatDamageDealt {
                source, target: event_target, amount,
            });
            let source_name = state.obj_name(source);
            state.log(LogLevel::Event,
                format!("{} dealt {} damage to {}", source_name, amount, state.obj_name(target)));
        }
    }

    apply_lifelink(state, source, amount, registry);
}

fn deal_damage_to_player(
    state: &mut GameState,
    source: ObjectId,
    player: crate::ids::PlayerId,
    amount: u32,
    kind: DamageKind,
    registry: &CardRegistry,
) {
    // Combat-only prevention (Ghostly Possession, Moonmist).
    if kind == DamageKind::Combat {
        if has_combat_damage_prevention(state, source, registry) {
            return;
        }
        if is_combat_damage_prevented_by_exception(state, source, registry) {
            return;
        }
    }

    let mut amount = amount;
    if kind == DamageKind::Combat {
        amount *= combat_damage_multiplier(state, source, registry);

        // CR 614: damage to a player can be replaced (Undead Alchemist turns
        // a Zombie's combat damage into a mill).
        let replaced = crate::replacement::apply(
            state,
            crate::replacement::ReplaceableEvent::DealsDamage {
                source,
                target: crate::events::DamageTarget::Player(player),
                amount,
                combat: true,
            },
            registry,
        );
        if replaced.is_none() {
            return;
        }
    }

    let old_life = state.get_player(player).life;
    let new_life = old_life - i32::try_from(amount).unwrap_or(i32::MAX);
    state.get_player_mut(player).life = new_life;

    match kind {
        DamageKind::Combat => {
            state.events.push(GameEvent::CombatDamageDealt {
                source, target: DamageTarget::Player(player), amount,
            });
            state.events.push(GameEvent::LifeChanged { player, old: old_life, new_life });
            state.log(LogLevel::Event,
                format!("p{} took {} combat damage ({}) from {}", player.0, amount, new_life, state.obj_name(source)));
        }
        DamageKind::NonCombat => {
            state.events.push(GameEvent::NonCombatDamageDealt {
                source, target: DamageTarget::Player(player), amount,
            });
            state.events.push(GameEvent::LifeChanged { player, old: old_life, new_life });
            state.log(LogLevel::Event,
                format!("{} dealt {} damage to p{}", state.obj_name(source), amount, player.0));
        }
    }

    apply_lifelink(state, source, amount, registry);
}

/// Lifelink: the source's controller gains life equal to the damage dealt
/// (CR 702.15) — combat and noncombat alike.
fn apply_lifelink(state: &mut GameState, source: ObjectId, amount: u32, registry: &CardRegistry) {
    if !state.has_keyword(source, Keyword::Lifelink, registry) {
        return;
    }
    let Some(controller) = state.get_object(source).map(|o| o.controller) else { return };
    let old_life = state.get_player(controller).life;
    let new_life = old_life + i32::try_from(amount).unwrap_or(i32::MAX);
    state.get_player_mut(controller).life = new_life;
    state.events.push(GameEvent::LifeChanged {
        player: controller,
        old: old_life,
        new_life,
    });
}

/// Check if a creature has combat damage prevented (e.g., Ghostly Possession).
fn has_combat_damage_prevention(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> bool {
    state.has_effect(creature_id, &|e| matches!(e, ContinuousEffect::PreventCombatDamage { .. }), registry)
}

/// Whether a blanket "prevent all combat damage by creatures other than X"
/// effect in play this turn prevents this source's damage. The filter names
/// the exceptions; a source that doesn't match one is prevented.
fn is_combat_damage_prevented_by_exception(state: &GameState, source: ObjectId, registry: &CardRegistry) -> bool {
    let controller = state.get_object(source).map_or(crate::ids::PlayerId(0), |o| o.controller);
    state.until_end_of_turn.iter().any(|e| match e {
        crate::state::TemporaryEffect::PreventCombatDamageExcept { filter } =>
            !state.matches_filter(source, filter, controller, registry),
        _ => false,
    })
}

/// Compute the combat damage multiplier from `DoubleCombatDamage` effects
/// (e.g., Inquisitor's Flail). Each source doubles independently.
fn combat_damage_multiplier(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> u32 {
    let count = state.count_effect(creature_id,
        &|e| matches!(e, ContinuousEffect::DoubleCombatDamage { .. }), registry);
    1u32 << count // 2^count
}

/// "Prevent damage, remove a +1/+1 counter" replacement (Unbreathing Horde,
/// CR 614.1a). Returns true if the damage was prevented (always, when the
/// effect is present — even with no counters left).
fn apply_prevent_damage_remove_counter(state: &mut GameState, target: ObjectId, registry: &CardRegistry) -> bool {
    let has_effect = state.has_effect(target, &|e| matches!(e, ContinuousEffect::PreventDamageRemoveCounter { .. }), registry);
    if !has_effect {
        return false;
    }
    let counter_count = state.get_object(target)
        .and_then(|o| o.counters.get(&crate::types::CounterType::PlusOnePlusOne).copied())
        .unwrap_or(0);
    if counter_count > 0 {
        if let Some(obj) = state.get_object_mut(target) {
            let entry = obj.counters.entry(crate::types::CounterType::PlusOnePlusOne).or_insert(0);
            *entry = entry.saturating_sub(1);
            if *entry == 0 {
                obj.counters.remove(&crate::types::CounterType::PlusOnePlusOne);
            }
        }
        let name = state.obj_name(target);
        state.log(LogLevel::Event,
            format!("{name}: damage prevented, removed a +1/+1 counter"));
    }
    true
}
