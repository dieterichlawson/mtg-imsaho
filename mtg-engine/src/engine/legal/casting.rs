//! Spells the player could cast now, from hand or by flashback.

use super::Ctx;
use super::super::*;
use crate::actions::Action;
use crate::cards::AdditionalCost;
use crate::ids::{CardId, ObjectId};
use crate::types::{Zone, CardType, ManaCost};

/// Spells the player can cast from hand.
pub(crate) fn from_hand(
    ctx: &Ctx,
    actions: &mut Vec<Action>,
    castable_spells: &mut Vec<crate::actions::CastableSpell>,
) {
    let Ctx { state, registry, player, is_sorcery_speed, .. } = *ctx;
    let mana_sources = &ctx.mana_sources;
    let hand_costs = &ctx.hand_costs;
    let casting_banned = &ctx.casting_banned;
    let player_state = state.get_player(player);
    // Cast spells from hand.
    // Deduplicate untargeted spells — only show one "Cast Kalonian Tusker" even if you have 3.
    // Targeted spells still get one entry per valid target.
    let mut seen_untargeted_casts: Vec<CardId> = Vec::new();
    for obj in state.objects_in_zone(Zone::Hand, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            let data = behavior.card_data();

            // Check PreventCastingNamed: spells with the banned name can't be cast.
            if casting_banned.contains(&data.name) {
                continue;
            }

            // Determine if this spell can be cast right now.
            let is_instant = data.card_types.contains(&CardType::Instant);
            let is_sorcery_type = data.card_types.contains(&CardType::Sorcery)
                || data.card_types.contains(&CardType::Creature)
                || data.card_types.contains(&CardType::Enchantment)
                || data.card_types.contains(&CardType::Artifact)
                || data.card_types.contains(&CardType::Planeswalker);

            let has_flash = data.keywords.contains(&Keyword::Flash);
            let can_cast_timing = if is_instant || has_flash {
                true // Instants and cards with flash can be cast anytime you have priority
            } else if is_sorcery_type {
                is_sorcery_speed
            } else {
                false
            };

            if !can_cast_timing {
                continue;
            }

            // Check mana via autotap (applying cost reduction effects).
            // Also check if any continuous effects provide an alternative cost.
            let alt_costs = alternative_costs_from_effects(state, registry, obj.card_id, player);
            let has_alt_cost = !alt_costs.is_empty();

            // Build hand_costs for other spells (exclude this spell's cost).
            let other_hand_costs: Vec<ManaCost> = hand_costs.iter()
                .enumerate()
                .filter(|&(i, _)| {
                    // Exclude this spell's cost from hand demand.
                    // Find the index in hand_costs that corresponds to this object.
                    let hand_objs: Vec<_> = state.objects_in_zone(Zone::Hand, player);
                    i < hand_objs.len() && hand_objs[i].id != obj.id
                })
                .map(|(_, c)| c.clone())
                .collect();

            // Compute autotap plan for the normal cost (if not X-cost).
            let has_x = data.cost.as_ref()
                .is_some_and(|c| c.has_x());
            let (can_pay_normal, normal_tap_plan) = if has_x {
                // X-cost spells: only autotap for the non-X portion. After the
                // agent chooses X, a second autotap pass covers the X generic.
                if let Some(cost) = &data.cost {
                    let effective_cost = effective_spell_cost(state, registry, obj.card_id, cost, player);
                    let non_x_cost = effective_cost.without_x();
                    if non_x_cost.symbols.is_empty() {
                        // No non-X cost (e.g., Mikaeus {X}{W} with W already floating).
                        if mana::can_pay(&player_state.mana_pool, &non_x_cost) {
                            (true, vec![])
                        } else {
                            // Need to tap for the colored portion.
                            match mana::compute_autotap(&non_x_cost, &player_state.mana_pool, &mana_sources, &other_hand_costs) {
                                Some(plan) => (true, plan),
                                None => (false, vec![]),
                            }
                        }
                    } else {
                        match mana::compute_autotap(&non_x_cost, &player_state.mana_pool, &mana_sources, &other_hand_costs) {
                            Some(plan) => (true, plan),
                            None => (false, vec![]),
                        }
                    }
                } else {
                    (true, vec![])
                }
            } else if let Some(cost) = &data.cost {
                let effective_cost = effective_spell_cost(state, registry, obj.card_id, cost, player);
                match mana::compute_autotap(&effective_cost, &player_state.mana_pool, &mana_sources, &other_hand_costs) {
                    Some(plan) => (true, plan),
                    None => (false, vec![]),
                }
            } else {
                // No cost (e.g. lands that somehow got here) — free.
                (true, vec![])
            };
            if !can_pay_normal && !has_alt_cost {
                continue;
            }

            // Check additional costs.
            let eligible_sacrifices: Vec<ObjectId> = match &data.additional_cost {
                Some(AdditionalCost::SacrificeCreature) => {
                    let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, player)
                        .iter()
                        .filter(|o| state.is_creature(o.id, registry))
                        .map(|o| o.id)
                        .collect();
                    if creatures.is_empty() { continue; }
                    creatures
                }
                Some(AdditionalCost::ExileXFromGraveyard) => {
                    // Player chooses X (0 to graveyard size). Actions are expanded below.
                    vec![]
                }
                Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) => {
                    // Check that there are enough creature cards in graveyard.
                    let creature_count = state.objects.values()
                        .filter(|o| {
                            o.zone == Zone::Graveyard && o.owner == player && o.id != obj.id
                                && state.is_creature(o.id, registry)
                        })
                        .count();
                    if creature_count < *n { continue; } // Not enough creatures to exile
                    vec![] // No sacrifice needed — exile handled at cast time in submit_action
                }
                _ => vec![],
            };

            // Generate cast actions with valid targets.
            let target_req = behavior.target_requirement();

            // For untargeted spells, deduplicate by card_id.
            if matches!(target_req, crate::cards::TargetRequirement::None) {
                if seen_untargeted_casts.contains(&obj.card_id) {
                    continue;
                }
                seen_untargeted_casts.push(obj.card_id);
            }

            let mut cast_actions = generate_cast_actions_with_targets(
                state, player, obj.id, &target_req, behavior,
            );

            // Set the autotap plan on all generated cast actions.
            if !normal_tap_plan.is_empty() {
                for action in &mut cast_actions {
                    if let Action::CastSpell { tap_plan, .. } = action {
                        tap_plan.clone_from(&normal_tap_plan);
                    }
                }
            }

            // If the spell requires a creature sacrifice, expand each action
            // into one per eligible creature.
            if !eligible_sacrifices.is_empty() {
                let base_actions = std::mem::take(&mut cast_actions);
                for action in base_actions {
                    if let Action::CastSpell { object_id, targets, tap_plan, .. } = action {
                        for &sac_id in &eligible_sacrifices {
                            cast_actions.push(Action::CastSpell {
                                object_id,
                                targets: targets.clone(),
                                sacrifice: Some(sac_id),
                                exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: tap_plan.clone(),
                            });
                        }
                    }
                }
            }

            // Exile-from-graveyard additional costs: emit exactly ONE
            // CastSpell per (target, sacrifice) with exile_ids=[] and
            // exile_count=None. The engine sets up a
            // `ChooseExileFromGraveyard` prompt when the action is
            // submitted, and the player picks which cards to exile
            // via `ResolvedChoice::ChosenExileSet`. Subset enumeration
            // here would flood the action list with C(gy,k) entries per
            // target — 2^N for Harvest Pyre with an N-card graveyard.
            // See `ResolutionChoiceKind::ChooseExileFromGraveyard`.

            // Generate alternative cost actions from continuous effects (e.g. Rooftop Storm).
            // The player chooses between the normal cost and the alternative cost.
            if has_alt_cost {
                // Use the first (cheapest) alternative cost. Multiple alternative costs
                // would need a chooser, but for now there's only one source at a time.
                let alt_mana = alt_costs[0].clone();
                // Compute autotap for the alternative cost.
                let alt_tap_plan = mana::compute_autotap(&alt_mana, &player_state.mana_pool, &mana_sources, &other_hand_costs)
                    .unwrap_or_default();
                if can_pay_normal {
                    // Player can pay normally — add alternative cost copies alongside normal ones.
                    let alt_actions: Vec<Action> = cast_actions.iter().filter_map(|a| {
                        if let Action::CastSpell { object_id, targets, sacrifice, exile_count, exile_ids, .. } = a {
                            Some(Action::CastSpell {
                                object_id: *object_id,
                                targets: targets.clone(),
                                sacrifice: *sacrifice,
                                exile_count: *exile_count,
                                exile_ids: exile_ids.clone(),
                                alternative_cost: Some(alt_mana.clone()), tap_plan: alt_tap_plan.clone(),
                            })
                        } else {
                            None
                        }
                    }).collect();
                    cast_actions.extend(alt_actions);
                } else {
                    // Player can't pay normally — replace all actions with alternative cost versions.
                    for action in &mut cast_actions {
                        if let Action::CastSpell { alternative_cost, tap_plan, .. } = action {
                            *alternative_cost = Some(alt_mana.clone());
                            tap_plan.clone_from(&alt_tap_plan);
                        }
                    }
                }
            }

            if !cast_actions.is_empty() {
                // Use the tap_plan from the first cast action for the CastableSpell.
                let spell_tap_plan = cast_actions.iter().find_map(|a| {
                    if let Action::CastSpell { tap_plan, .. } = a { Some(tap_plan.clone()) } else { None }
                }).unwrap_or_default();
                // Expose max X for ExileXFromGraveyard spells so the player
                // UI can show the effective damage in the label.
                let exile_x_from_gy_max = if matches!(&data.additional_cost,
                    Some(AdditionalCost::ExileXFromGraveyard)
                ) {
                    let n = state.objects.values()
                        .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != obj.id)
                        .count();
                    Some(u32::try_from(n).unwrap_or(u32::MAX))
                } else {
                    None
                };
                actions.extend(cast_actions);
                let spec = build_cast_target_spec(state, player, obj.id, &target_req, behavior);
                let additional_cost_label = match &data.additional_cost {
                    Some(AdditionalCost::SacrificeCreature) => Some("sacrifice a creature".into()),
                    Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) => {
                        Some(format!("exile {} creature{} from GY", n, if *n == 1 { "" } else { "s" }))
                    }
                    Some(AdditionalCost::ExileXFromGraveyard) => Some("exile cards from GY".into()),
                    None => None,
                };
                castable_spells.push(crate::actions::CastableSpell {
                    object_id: obj.id,
                    name: data.name.clone(),
                    is_flashback: false,
                    target_spec: spec,
                    tap_plan: spell_tap_plan,
                    exile_x_from_gy_max,
                    sacrifice_options: eligible_sacrifices.clone(),
                    additional_cost_label,
                });
            }
        }
    }
}

/// Spells the player can cast from the graveyard via flashback.
pub(crate) fn flashback(
    ctx: &Ctx,
    actions: &mut Vec<Action>,
    castable_spells: &mut Vec<crate::actions::CastableSpell>,
) {
    let Ctx { state, registry, player, is_sorcery_speed, .. } = *ctx;
    let mana_sources = &ctx.mana_sources;
    let hand_costs = &ctx.hand_costs;
    let casting_banned = &ctx.casting_banned;
    let player_state = state.get_player(player);
    // Cast spells via flashback from graveyard.
    let mut seen_untargeted_flashbacks: Vec<(CardId, ManaCost)> = Vec::new();
    for obj in state.objects_in_zone(Zone::Graveyard, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            let data = behavior.card_data();

            // Check PreventCastingNamed: banned spells can't be cast, even via flashback.
            if casting_banned.contains(&data.name) {
                continue;
            }

            // CR 702.33: a card can have several instances of flashback at
            // once — a granted one (Snapcaster Mage, Past in Flames) alongside
            // its printed one — and the player may pay ANY of them. This used
            // to pick a single winner, granted-before-printed, and silently
            // discard the rest. That is not merely a missing choice: with
            // Bump in the Night ({B} printed cost, {5}{R} printed flashback)
            // in the graveyard and only red mana available, the granted {B}
            // cost was found unaffordable and the payable {5}{R} was never
            // offered at all.
            let cast_from_gy = behavior.can_cast_from_graveyard();
            let mut fb_costs: Vec<ManaCost> = state.until_end_of_turn.iter()
                .filter_map(|e| if let crate::state::TemporaryEffect::GrantFlashback { target, cost } = e {
                    if *target == obj.id { Some(cost.clone()) } else { None }
                } else { None })
                .collect();
            if let Some(c) = &data.flashback_cost {
                fb_costs.push(c.clone());
            }
            if cast_from_gy {
                // Cast from graveyard uses the normal mana cost.
                if let Some(c) = &data.cost {
                    fb_costs.push(c.clone());
                }
            }
            // Two identical costs are one option, not two.
            let mut unique: Vec<ManaCost> = Vec::new();
            for c in fb_costs {
                if !unique.contains(&c) {
                    unique.push(c);
                }
            }
            if unique.is_empty() { continue; }

            let is_instant = data.card_types.contains(&CardType::Instant);
            let is_sorcery_type = data.card_types.contains(&CardType::Sorcery)
                || data.card_types.contains(&CardType::Creature)
                || data.card_types.contains(&CardType::Enchantment);

            let has_flash = data.keywords.contains(&Keyword::Flash);
            let can_cast_timing = if is_instant || has_flash {
                true
            } else if is_sorcery_type {
                is_sorcery_speed
            } else {
                false
            };

            if !can_cast_timing { continue; }

            // One castable option per distinct flashback cost.
            for fb_cost in &unique {
            // Compute autotap for the non-X portion of the flashback cost.
            // X-cost flashback spells (Devil's Play's {X}{R}{R}{R} flashback)
            // are funded via a ChooseXFunding prompt after the spell is
            // cast, exactly like non-flashback X casts — so here we only
            // need to verify the non-X portion is payable.
            let fb_has_x = fb_cost.has_x();
            let fb_non_x_cost;
            let fb_cost_for_autotap: &ManaCost = if fb_has_x {
                fb_non_x_cost = fb_cost.without_x();
                &fb_non_x_cost
            } else {
                fb_cost
            };
            let Some(fb_tap_plan) = mana::compute_autotap(fb_cost_for_autotap, &player_state.mana_pool, &mana_sources, &hand_costs) else {
                // This particular cost is unaffordable; another may not be.
                continue;
            };

            // Check additional cost eligibility for graveyard casts.
            {
                use crate::cards::AdditionalCost;
                if let Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) = &data.additional_cost {
                    // Count creature cards in graveyard (excluding the spell itself).
                    let creature_count = state.objects.values()
                        .filter(|o| {
                            o.zone == Zone::Graveyard && o.owner == player && o.id != obj.id
                                && state.is_creature(o.id, registry)
                        })
                        .count();
                    if creature_count < *n { continue; }
                }
            }

            let target_req = behavior.target_requirement();

            // Collapse identical untargeted flashbacks across duplicate copies
            // of the same card — but keyed on the COST as well, or a card's
            // second flashback option would be swallowed as a duplicate of its
            // first.
            if matches!(target_req, crate::cards::TargetRequirement::None) {
                let key = (obj.card_id, fb_cost.clone());
                if seen_untargeted_flashbacks.contains(&key) { continue; }
                seen_untargeted_flashbacks.push(key);
            }

            let mut cast_actions = generate_cast_actions_with_targets(
                state, player, obj.id, &target_req, behavior,
            );
            // Each action carries the cost it was offered for, so the cast
            // handler charges the one the player picked rather than
            // re-deriving a winner.
            for action in &mut cast_actions {
                if let Action::CastSpell { tap_plan, alternative_cost, .. } = action {
                    tap_plan.clone_from(&fb_tap_plan);
                    *alternative_cost = Some(fb_cost.clone());
                }
            }
            if !cast_actions.is_empty() {
                actions.extend(cast_actions);
                let spec = build_cast_target_spec(state, player, obj.id, &target_req, behavior);
                castable_spells.push(crate::actions::CastableSpell {
                    object_id: obj.id,
                    name: data.name.clone(),
                    is_flashback: !cast_from_gy,
                    target_spec: spec,
                    tap_plan: fb_tap_plan,
                    exile_x_from_gy_max: None,
                    sacrifice_options: vec![], // Flashback spells don't have sacrifice additional costs
                    additional_cost_label: None,
                });
            }
            }
        }
    }
}
