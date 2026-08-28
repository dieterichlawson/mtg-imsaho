//! Activating an ability — mana, non-mana, or loyalty.

use super::super::Applied;
use crate::cards::SacrificeCost;
use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::ids::ObjectId;
use crate::mana;
use crate::state::{GameState, LogLevel};
use crate::types::{Zone, CounterType};
use super::super::*;

pub(crate) fn activate_mana_ability(state: &mut GameState, object_id: ObjectId, ability_index: usize, registry: &CardRegistry) -> Applied {
        activate_mana_source(&mut *state, object_id, ability_index, registry);
    Applied::Continue
}

/// Pay whatever cost the `ActivatedAbilityDef` could not express, then put the
/// ability on the stack (CR 602.2a).
///
/// The push is the engine's, not the card's. When cards owned it — as the
/// default body of an `on_activate_ability` hook they were free to override —
/// 46 of the set's 53 activated abilities overrode it to do their effect
/// instead, so the effect happened at announcement and no opponent ever got
/// the priority CR 117.3b owes them. A card also cannot know
/// `behavior_card_id`: an ability granted by an attached Aura or Equipment is
/// activated on the *creature*, so the object's own card id is the wrong
/// behavior to dispatch to on resolution.
pub(crate) fn put_ability_on_stack(
    state: &mut GameState,
    object_id: ObjectId,
    ability_index: usize,
    behavior_card_id: crate::ids::CardId,
    targets: &[Target],
    registry: &CardRegistry,
) {
    // Read what the ability asks of its target *before* the cost is paid:
    // `SacrificeThis` removes the source, and a card's `activated_abilities`
    // is then gone with it. CR 608.2b re-checks the target against this on
    // resolution, so it rides on the stack entry (CR 601.2c).
    let target_requirement = registry.get(behavior_card_id)
        .and_then(|b| b.activated_abilities(state, object_id, registry)
            .into_iter()
            .find(|a| a.ability_index == ability_index)
            .and_then(|a| a.target_requirement));

    if let Some(behavior) = registry.get(behavior_card_id) {
        behavior.pay_activation_cost(state, object_id, ability_index, targets, registry);
    }
    crate::cards::push_ability(state, object_id, ability_index, behavior_card_id, targets, target_requirement);
}

pub(crate) fn activate_ability(state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], tap_plan: &[(ObjectId, usize)], sacrifice: Option<ObjectId>, source_card_id: Option<crate::ids::CardId>, registry: &CardRegistry) -> Applied {
        let player = state.priority_player.expect("ActivateAbility requires priority");

        // Execute autotap plan: tap mana sources to fill the mana pool before
        // we attempt to pay the ability's mana cost. This mirrors CastSpell.
        for &(source_id, ma_idx) in tap_plan {
            activate_mana_source(&mut *state, source_id, ma_idx, registry);
        }

        let obj = state.get_object(object_id).expect("activated ability object must exist");
        let card_id = obj.card_id;
        let copy_grantor = state.get_object(object_id).and_then(|o| o.copy_grantor);

        // Resolve which card's behavior contributed this ability:
        // - Some(cid): caller explicitly disambiguated the source — used by
        //   legal_actions to mark aura-granted abilities. Look up in cid only.
        // - None: backward-compat chained lookup (native → copy-grantor
        //   override → attached auras). Used by tests and code paths that
        //   don't need to disambiguate (only one contributes the ability).
        let (behavior_card_id, ability) = if let Some(cid) = source_card_id {
            let ab = registry.get(cid)
                .and_then(|b| b.activated_abilities(&state, object_id, registry)
                    .into_iter().find(|a| a.ability_index == ability_index));
            (cid, ab)
        } else {
            let native = registry.get(card_id)
                .and_then(|b| b.activated_abilities(&state, object_id, registry)
                    .into_iter().find(|a| a.ability_index == ability_index));
            if native.is_some() {
                (card_id, native)
            } else if copy_grantor.is_some() {
                // CR 706.2: an ability the copy effect added — dispatch to
                // the card whose copy effect granted it.
                let g_id = copy_grantor.filter(|&g| g != card_id);
                let ab = g_id.and_then(|cid| registry.get(cid))
                    .and_then(|b| b.activated_abilities(&state, object_id, registry)
                        .into_iter().find(|a| a.ability_index == ability_index));
                if let Some(ab) = ab {
                    (g_id.unwrap_or(card_id), Some(ab))
                } else {
                    // Fall through to attached lookup below.
                    let mut found = (card_id, None);
                    for attached in state.objects_in_id_order().into_iter()
                        .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(object_id))
                    {
                        if let Some(ab) = registry.get(attached.card_id)
                            .and_then(|b| b.activated_abilities(&state, object_id, registry)
                                .into_iter().find(|a| a.ability_index == ability_index))
                        {
                            found = (attached.card_id, Some(ab));
                            break;
                        }
                    }
                    found
                }
            } else {
                // Walk attached auras/equipment.
                let mut found = (card_id, None);
                for attached in state.objects_in_id_order().into_iter()
                    .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(object_id))
                {
                    if let Some(ab) = registry.get(attached.card_id)
                        .and_then(|b| b.activated_abilities(&state, object_id, registry)
                            .into_iter().find(|a| a.ability_index == ability_index))
                    {
                        found = (attached.card_id, Some(ab));
                        break;
                    }
                }
                found
            }
        };

        if let Some(ab) = ability {
            // Pay mana cost (with X-cost support). For X-cost abilities
            // we pay only the non-X portion here; the X generic is paid
            // later via the ChooseXFunding flow (CR 602.1: the cost is
            // announced & paid before the ability resolves).
            let has_x_cost = ab.cost.has_x();
            if has_x_cost {
                let non_x_cost = ab.cost.without_x();
                mana::auto_pay(&mut state.get_player_mut(player).mana_pool, &non_x_cost)
                    .expect("legal_actions should have verified mana availability");
            } else {
                mana::auto_pay(&mut state.get_player_mut(player).mana_pool, &ab.cost)
                    .expect("legal_actions should have verified mana availability");
                state.last_activated_x_value = None;
            }

            // Pay tap cost.
            if ab.requires_tap {
                state.get_object_mut(object_id).expect("object must exist for tapping").tapped = true;
            }

            // Pay the counter cost. Before the sacrifice below, which moves
            // the permanent to the graveyard and clears every counter it
            // has at once — "remove three" has to remove three, leaving any
            // surplus on the permanent to be lost to the zone change rather
            // than swallowed by it.
            if let Some((counter_type, amount)) = ab.counter_cost {
                state.remove_counters(object_id, counter_type, amount);
            }

            // Pay sacrifice cost. The player chose which creature to sacrifice
            // when picking the action — legal_actions enumerated one
            // ActivateAbility per (target, sacrifice) combo, so the choice is
            // already encoded in `sacrifice`. We just sacrifice it here.
            // Which creature paid the cost is part of what the ability
            // resolves with — Disciple of Griselbrand's "the sacrificed
            // creature's toughness" is about this one and not about whatever
            // died most recently. Carried to the stack entry alongside
            // `x_value`, so the priority window between paying and resolving
            // cannot change the answer.
            state.last_activated_sacrifice = match &ab.sacrifice_cost {
                SacrificeCost::None => None,
                SacrificeCost::SacrificeThis => Some(object_id),
                SacrificeCost::SacrificeCreature | SacrificeCost::SacrificeAnotherCreature => sacrifice,
            };
            match &ab.sacrifice_cost {
                SacrificeCost::None => {}
                SacrificeCost::SacrificeThis => {
                    crate::destruction::sacrifice(&mut *state, object_id, registry);
                }
                SacrificeCost::SacrificeCreature | SacrificeCost::SacrificeAnotherCreature => {
                    let sac_id = sacrifice
                        .expect("legal_actions must populate sacrifice for sacrifice-cost abilities");
                    crate::destruction::sacrifice(&mut *state, sac_id, registry);
                }
            }

            // Track once-per-turn.
            if ab.once_per_turn {
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.abilities_activated_this_turn.insert(ability_index);
                }
            }

            if has_x_cost {
                // Defer the stack push and the activation log until
                // funding completes — the ability's effect reads
                // `last_activated_x_value`, which isn't set until then.
                // See the ChooseXFunding handler for the continuation.
                let options = crate::funding::build_options(&state, player, registry);
                let name = card_name(&state, registry, object_id);
                if options.max_x > 0 {
                    state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                        player,
                        source: object_id,
                        choice: crate::state::ResolutionChoiceKind::ChooseXFunding {
                            description: format!("{name}: choose X funding (0-{})", options.max_x),
                            options,
                            source_id: object_id,
                            is_ability: true,
                        },
                    });
                    // Store context needed to fire the ability's effect
                    // once funding completes.
                    state.pending_ability_effect = Some(crate::state::PendingAbilityEffect {
                        source_id: object_id,
                        ability_index: ability_index,
                        behavior_card_id,
                        targets: targets.to_vec(),
                        description: ab.description.clone(),
                        activator: player,
                    });
                } else {
                    // No mana available; force X = 0.
                    state.last_activated_x_value = Some(0);
                    put_ability_on_stack(&mut *state, object_id, ability_index, behavior_card_id, targets, registry);
                    let name = card_name(&state, registry, object_id);
                    state.log(LogLevel::Event, format!("p{} activated ability on {}: {}", player.0, name, ab.description));
                }
            } else {
                put_ability_on_stack(&mut *state, object_id, ability_index, behavior_card_id, targets, registry);
                let name = card_name(&state, registry, object_id);
                state.log(LogLevel::Event, format!("p{} activated ability on {}: {}", player.0, name, ab.description));
            }
            // CR 117.3b: taking an action means every player gets priority
            // again before anything resolves. This used to be moot — the
            // ability was resolved on the spot — but now it waits on the
            // stack like any other object, and a stale pass count would
            // resolve it without the opponent ever seeing it.
            state.consecutive_passes = 0;
        }
    Applied::Continue
}

pub(crate) fn activate_loyalty_ability(state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) -> Applied {
        let player = state.priority_player.expect("ActivateLoyaltyAbility requires priority");
        if let Some(behavior) = registry.get(
            state.get_object(object_id).map_or(crate::ids::CardId(0), |o| o.card_id)
        ) {
            let abilities = behavior.loyalty_abilities(&state, object_id);
            if let Some(ab) = abilities.iter().find(|a| a.ability_index == ability_index) {
                // Pay loyalty cost: add or remove loyalty counters.
                let change = ab.loyalty_change;
                if change > 0 {
                    state.add_counters(object_id, CounterType::Loyalty, u32::try_from(change).unwrap_or(0));
                } else if change < 0 {
                    let remove = u32::try_from(-change).unwrap_or(0);
                    if let Some(obj) = state.get_object_mut(object_id) {
                        let current = obj.counters.entry(CounterType::Loyalty).or_insert(0);
                        *current = current.saturating_sub(remove);
                    }
                }
                // Mark that a loyalty ability was activated this turn on this permanent.
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.abilities_activated_this_turn.insert(999); // sentinel for "used loyalty this turn"
                }
                behavior.on_loyalty_ability(&mut *state, object_id, ability_index, targets, registry);
                let name = card_name(&state, registry, object_id);
                state.log(LogLevel::Event, format!("p{} activated loyalty ability on {}: {}", player.0, name, ab.description));
            }
        }
    Applied::Continue
}
