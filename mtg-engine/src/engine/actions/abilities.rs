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
    activator: crate::ids::PlayerId,
    target_requirement: Option<crate::cards::TargetRequirement>,
    registry: &CardRegistry,
) {
    // `target_requirement` is what the ability asked of its target, read by
    // the caller *before* any cost was paid: a sacrifice cost may have
    // removed the source — "sacrifice this", or "sacrifice a creature" paid
    // with the source itself — and a card's `activated_abilities` is gone
    // with it. Re-deriving it here answered `None` for Skirsdag Cultist
    // sacrificing itself, and the CR 608.2b re-check at resolution had
    // nothing to check against (found by fuzzing). It rides on the stack
    // entry (CR 601.2c).
    if let Some(behavior) = registry.get(behavior_card_id) {
        behavior.pay_activation_cost(state, object_id, ability_index, targets, registry);
    }
    crate::cards::push_ability(state, object_id, ability_index, behavior_card_id, targets, target_requirement, activator);
}

/// CR 601.2a-b via 602.2b: the activation and its announced X, said out loud
/// before any cost is paid.
///
/// `x` is `Some` for an X-cost ability. It used to be absent from this line
/// because the line was written before X was chosen, which made the ability
/// public in a state CR 601.2b says cannot exist (issue #290).
pub(crate) fn announce_activation(
    state: &mut GameState,
    player: crate::ids::PlayerId,
    object_id: ObjectId,
    description: &str,
    targets: &[Target],
    x: Option<u32>,
    registry: &CardRegistry,
) {
    let name = card_name(&*state, registry, object_id);
    // The ability's targets are logged the way a spell's are — they were
    // announced with the activation (CR 602.2b) and the log recorded none of
    // them (issue #135).
    let target_suffix = if targets.is_empty() {
        String::new()
    } else {
        let names: Vec<String> = targets.iter().map(|t| match t {
            crate::actions::Target::Object(id) => state.obj_name(*id),
            crate::actions::Target::Player(p) => format!("p{}", p.0),
            crate::actions::Target::Illegal => "an illegal target".into(),
        }).collect();
        format!(" targeting {}", names.join(", "))
    };
    let x_suffix = x.map_or_else(String::new, |n| format!(" (X={n})"));
    state.log(LogLevel::Event, format!(
        "p{} activated ability on {name}: {description}{target_suffix}{x_suffix}", player.0));
}

/// Pay a deferred activation cost, in the order CR 601.2h fixes: mana (the
/// tap plan first), then the `{T}`, then counters, then the sacrifice.
///
/// Split out because the funding handler now runs it — the payment happens
/// after X is announced, not before it (issue #290).
pub(crate) fn pay_activation_costs(
    state: &mut GameState,
    player: crate::ids::PlayerId,
    object_id: ObjectId,
    ability_index: usize,
    cost: &crate::state::DeferredActivationCost,
    registry: &CardRegistry,
) {
    for &(source_id, ma_idx) in &cost.tap_plan {
        activate_mana_source(&mut *state, source_id, ma_idx, registry);
    }
    let _ = mana::auto_pay(&mut state.get_player_mut(player).mana_pool, &cost.non_x_mana_cost);
    if cost.requires_tap {
        state.tap(object_id);
    }
    // Before the sacrifice below, which moves the permanent to the graveyard
    // and clears every counter it has at once — "remove three" has to remove
    // three, leaving any surplus to be lost to the zone change rather than
    // swallowed by it.
    if let Some((counter_type, amount)) = cost.counter_cost {
        state.remove_counters(object_id, counter_type, amount);
    }
    // Which creature paid the cost is part of what the ability resolves with
    // — Disciple of Griselbrand's "the sacrificed creature's toughness" is
    // about this one and not about whatever died most recently.
    state.last_activated_sacrifice = match &cost.sacrifice_cost {
        SacrificeCost::None => None,
        SacrificeCost::SacrificeThis => Some(object_id),
        SacrificeCost::SacrificeCreature | SacrificeCost::SacrificeAnotherCreature => cost.sacrifice,
    };
    // Captured NOW, while the creature is still on the battlefield: its
    // toughness as it last existed there is what the ability reads at
    // resolution (CR 608.2h; issue #141).
    state.last_activated_sacrifice_toughness = state.last_activated_sacrifice
        .and_then(|id| state.effective_toughness(id, registry));
    match &cost.sacrifice_cost {
        SacrificeCost::None => {}
        SacrificeCost::SacrificeThis => {
            crate::destruction::sacrifice_by(
                &mut *state, object_id, "to pay for its own ability", registry);
        }
        SacrificeCost::SacrificeCreature | SacrificeCost::SacrificeAnotherCreature => {
            let Some(sac_id) = cost.sacrifice else { return };
            // "Sacrifice a creature" with two eligible creatures: the menu
            // said which one would pay, and the log did not.
            let source_name = state.obj_name(object_id);
            let reason = if sac_id == object_id {
                "to pay for its own ability".to_string()
            } else {
                format!("to pay for {source_name}'s ability")
            };
            crate::destruction::sacrifice_by(&mut *state, sac_id, &reason, registry);
        }
    }
    if cost.once_per_turn {
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.abilities_activated_this_turn.insert(ability_index);
        }
    }
}

pub(crate) fn activate_ability(state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], tap_plan: &[(ObjectId, usize)], sacrifice: Option<ObjectId>, source_card_id: Option<crate::ids::CardId>, registry: &CardRegistry) -> Applied {
        let player = state.priority_player.expect("ActivateAbility requires priority");

        let obj = state.get_object(object_id).expect("activated ability object must exist");
        let card_id = obj.card_id;
        let copy_grantor = state.get_object(object_id).and_then(|o| o.copy_grantor);

        // Resolve which card's behavior contributed this ability:
        // - Some(cid): caller explicitly disambiguated the source — used by
        //   legal_actions to mark aura-granted abilities. Look up in cid only.
        // - None: backward-compat chained lookup (native → copy-grantor
        //   override → attached auras). Used by tests and code paths that
        //   don't need to disambiguate (only one contributes the ability).
        // The ability as one card contributes it: look it up by index among
        // the abilities that card grants THIS object.
        let contributed = |cid: crate::ids::CardId, state: &GameState| {
            registry.get(cid)
                .and_then(|b| b.activated_abilities(state, object_id, registry)
                    .into_iter().find(|a| a.ability_index == ability_index))
        };
        // The first attached aura or Equipment that contributes it. Written
        // out twice before — once as the fallback after a copy grantor missed
        // and once as the plain case — and the copies could disagree.
        let from_attached = |state: &GameState| {
            state.objects_in_id_order().into_iter()
                .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(object_id))
                .find_map(|a| contributed(a.card_id, state).map(|ab| (a.card_id, Some(ab))))
                .unwrap_or((card_id, None))
        };
        let (behavior_card_id, ability) = if let Some(cid) = source_card_id {
            // The caller disambiguated the source — legal_actions marks
            // aura-granted abilities this way. Look up in `cid` only.
            (cid, contributed(cid, &state))
        } else if let Some(native) = contributed(card_id, &state) {
            // Backward-compat chained lookup, for tests and paths where only
            // one card can contribute the ability: native first.
            (card_id, Some(native))
        } else {
            // CR 706.2: an ability the copy effect added — dispatch to the
            // card whose copy effect granted it, and only when that card
            // grants abilities to copies at all (issue #93: for a plain
            // enters-as-copy the grantor is just the printed card remembered
            // for the zone-change revert).
            let granted = copy_grantor
                .filter(|&g| g != card_id)
                .filter(|&g| registry.get(g).is_some_and(|b| b.grants_abilities_to_copies()))
                .and_then(|g| contributed(g, &state).map(|ab| (g, Some(ab))));
            granted.unwrap_or_else(|| from_attached(&state))
        };

        if let Some(ab) = ability {
            // Stony Silence: "activated abilities of artifacts can't be
            // activated" — equip included, since equip is an activated ability
            // of the Equipment. `legal_actions` never offers these, but the
            // submit path must speak for itself (neither client picks a whole
            // offered action).
            if crate::engine::mana_sources::prevents_artifact_abilities(state, registry)
                && state.has_card_type(object_id, CardType::Artifact, registry) {
                state.log(crate::state::LogLevel::Debug, format!(
                    "activation refused, activated abilities of artifacts can't be activated"));
                return Applied::ReturnNow;
            }
            // CR 601.2c via 602.2b: an activated ability chooses its targets as
            // it is activated, and they must be legal ones. Same reason as the
            // cast path — `legal_actions` enumerates only legal sets, and the
            // clients build their own action from per-slot choices — and same
            // placement: before any cost is paid, so a refusal leaves the state
            // untouched rather than charging for an activation that did not
            // happen.
            let req = ab.target_requirement.clone().unwrap_or(crate::cards::TargetRequirement::None);
            let legal = registry.get(behavior_card_id).is_some_and(|b|
                crate::engine::targeting::targets_are_legal(
                    state, &req, targets, player, object_id, b, registry));
            if !legal {
                state.log(crate::state::LogLevel::Debug, format!(
                    "activation refused, illegal targets {targets:?} (CR 601.2c)"));
                return Applied::ReturnNow;
            }

            // Pay mana cost (with X-cost support). For X-cost abilities
            // we pay only the non-X portion here; the X generic is paid
            // later via the ChooseXFunding flow (CR 602.1: the cost is
            // announced & paid before the ability resolves).
            //
            // Same offer/submit rule as the cast path: the pool has to prove
            // it can pay before anything is deducted — `auto_pay` drains as
            // it goes, so a failed payment cannot simply be unwound. An
            // unfunded activation is refused, not a panic (CR 601.2h via
            // 602.2b).
            let has_x_cost = ab.cost.has_x();
            let pay = if has_x_cost { ab.cost.without_x() } else { ab.cost.clone() };
            // Rehearse the tap plan on a scratch copy before anything is
            // tapped or paid: `auto_pay` drains the pool as it goes, so a
            // failed payment cannot simply be unwound, and an unfunded
            // activation is refused rather than half-charged (CR 601.2h via
            // 602.2b). Same rule as the cast path.
            let mut probe = state.clone();
            for &(source_id, ma_idx) in tap_plan {
                activate_mana_source(&mut probe, source_id, ma_idx, registry);
            }
            if !mana::can_pay(&probe.get_player(player).mana_pool, &pay) {
                state.log(crate::state::LogLevel::Debug, format!(
                    "activation refused, submitted funding cannot pay {pay:?} (CR 601.2h)"));
                return Applied::ReturnNow;
            }

            // CR 601.2b precedes 601.2h: X is announced BEFORE the total cost
            // is paid. So an X-cost activation with a real choice to make
            // stashes its whole cost and asks first — the permanent is not
            // tapped, no mana is spent, no counter is removed and nothing is
            // sacrificed until the player has answered, which is also what
            // makes that prompt cancellable (issue #290).
            let cost = crate::state::DeferredActivationCost {
                tap_plan: tap_plan.to_vec(),
                non_x_mana_cost: pay,
                requires_tap: ab.requires_tap,
                counter_cost: ab.counter_cost,
                sacrifice,
                sacrifice_cost: ab.sacrifice_cost.clone(),
                once_per_turn: ab.once_per_turn,
            };
            if has_x_cost {
                // What is left to announce X with is what remains once the
                // WHOLE non-X cost is paid — including the `{T}`, which for
                // Kessig Wolf Run is the source's own mana ability. Probing
                // before tapping it counted that mana twice.
                pay_activation_costs(&mut probe, player, object_id, ability_index, &cost, registry);
                let options = crate::funding::build_options(&probe, player, registry);
                if options.max_announceable_x() > 0 {
                    let name = card_name(&state, registry, object_id);
                    state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                        player,
                        source: object_id,
                        choice: crate::state::ResolutionChoiceKind::ChooseXFunding {
                            description: format!("{name}: choose X funding (0-{})",
                                options.max_announceable_x()),
                            options,
                            source_id: object_id,
                            is_ability: true,
                        },
                    });
                    state.pending_ability_effect = Some(crate::state::PendingAbilityEffect {
                        source_id: object_id,
                        ability_index,
                        behavior_card_id,
                        targets: targets.to_vec(),
                        description: ab.description.clone(),
                        activator: player,
                        target_requirement: ab.target_requirement.clone(),
                        unpaid: Some(cost),
                    });
                    // Nothing else happens until the player answers.
                    return Applied::ReturnNow;
                }
                // No mana to announce X with: X is forced to 0, there is no
                // choice, and the activation proceeds below as any other.
            }

            // CR 601.2a via 602.2b: the activation is announced before its
            // costs are paid — so a sacrifice cost reads "activated, then
            // died", not a creature dying on its own and then somehow
            // activating from the graveyard.
            announce_activation(&mut *state, player, object_id, &ab.description, targets,
                if has_x_cost { Some(0) } else { None }, registry);
            if !has_x_cost {
                state.last_activated_x_value = None;
            }
            // The player chose which creature to sacrifice when picking the
            // action — `legal_actions` enumerates one `ActivateAbility` per
            // (target, sacrifice) combo, so the choice is already encoded.
            pay_activation_costs(&mut *state, player, object_id, ability_index, &cost, registry);

            if has_x_cost {
                // Only reachable when no mana could fund X at all, so there
                // was no announcement to make: X is 0 (the prompt path
                // returned above).
                state.last_activated_x_value = Some(0);
            }
            put_ability_on_stack(&mut *state, object_id, ability_index, behavior_card_id, targets, player,
                ab.target_requirement.clone(), registry);
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
                let name = card_name(&state, registry, object_id);
                state.log(LogLevel::Event, format!("p{} activated loyalty ability on {}: {}", player.0, name, ab.description));
                // CR 606.5: a loyalty ability is an activated ability and uses
                // the stack. Resolving it on the spot meant its effect read
                // the battlefield before state-based actions had seen the
                // loyalty payment — Liliana's -6 put herself, at 0 loyalty, in
                // her own pile prompt — and the opponent never got to respond.
                let behavior_card_id = state.get_object(object_id)
                    .map_or(crate::ids::CardId(0), |o| o.card_id);
                crate::cards::push_loyalty_ability(&mut *state, object_id, ability_index,
                    behavior_card_id, targets, ab.target_requirement.clone(), player);
                // CR 117.3b: taking an action resets the pass count, as for
                // regular activated abilities above.
                state.consecutive_passes = 0;
            }
        }
    Applied::Continue
}
