//! Activated and loyalty abilities the player could activate now.

use super::Ctx;
use super::super::*;
use crate::actions::Action;
use crate::ids::{CardId, ObjectId};
use crate::types::{Zone, CardType, CounterType};

/// Activated abilities of permanents the player controls, including ones
/// granted by attached auras and equipment.
pub(crate) fn activated(ctx: &Ctx, actions: &mut Vec<Action>) {
    let Ctx { state, registry, player, prevent_artifact_abilities, is_sorcery_speed, .. } = *ctx;
    let early_mana_sources = &ctx.mana_sources;
    let mana_pool = &state.get_player(player).mana_pool;
    // Non-mana activated abilities: can activate anytime you have priority (if you can pay).
    // Check attached permanents too (auras granting abilities to creatures).
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        let obj_id = obj.id;
        let obj_tapped = obj.tapped;
        let obj_card_id = obj.card_id;
        let activated_this_turn = obj.abilities_activated_this_turn.clone();

        // Stony Silence: skip artifact activated abilities.
        if prevent_artifact_abilities {
            if state.has_card_type(obj_id, CardType::Artifact, registry) { continue; }
        }

        // Collect abilities from this permanent's card and attached auras.
        let mut abilities: Vec<(crate::ids::CardId, crate::cards::ActivatedAbilityDef)> = Vec::new();
        if let Some(behavior) = registry.get(obj_card_id) {
            for ab in behavior.activated_abilities(state, obj_id, registry) {
                abilities.push((obj_card_id, ab));
            }
        }
        // CR 706.2: a copy effect may say "except it has <ability>". The copy's
        // `card_id` is the copied card, so the granting card's abilities have to
        // be collected from `copy_grantor` — but ONLY when that card's copy
        // effect actually grants abilities (Evil Twin). For a plain
        // enters-as-copy, `copy_grantor` is just the printed card remembered
        // for the zone-change revert, and consulting it handed an Essence of
        // the Wild copy its printed card's own abilities back (issue #93).
        let grantor = state.get_object(obj_id).and_then(|o| o.copy_grantor);
        if let Some(grantor_id) = grantor.filter(|&g| g != obj_card_id) {
            if let Some(behavior) = registry.get(grantor_id) {
                if behavior.grants_abilities_to_copies() {
                    for ab in behavior.activated_abilities(state, obj_id, registry) {
                        abilities.push((grantor_id, ab));
                    }
                }
            }
        }
        for attached in state.objects_in_id_order() {
            // Only offer abilities granted by attachments the acting player
            // controls. Every granted activated ability in the set includes
            // sacrificing the attached source as a cost (paid in
            // `pay_activation_cost`, e.g. Blazing Torch), and a player can
            // only sacrifice permanents they control (CR 601.2g/701.13) — so
            // an opponent-controlled attachment's granted ability is
            // unpayable.
            if attached.zone == Zone::Battlefield
                && attached.attached_to == Some(obj_id)
                && attached.controller == player {
                if let Some(behavior) = registry.get(attached.card_id) {
                    for ab in behavior.activated_abilities(state, obj_id, registry) {
                        abilities.push((attached.card_id, ab));
                    }
                }
            }
        }

        for (source_card_id, ab) in abilities {
            // Check mana cost. For X-cost abilities, check that non-X portion is affordable.
            // We use compute_autotap (mirroring spell casting) so abilities with mana costs
            // appear as legal actions in main phase even when no mana is currently floating —
            // the resulting tap plan is bundled into the action and executed at apply time.
            //
            // EXCEPTION: abilities with `SacrificeCreature` / `SacrificeAnotherCreature`
            // don't get auto-tap. The player has to manually tap their lands and float
            // the mana before activating. Otherwise the planner can pick a creature
            // mana source as part of the tap plan and then the player sacrifices that
            // same creature for the cost — the orderings get weird and lead to bugs.
            // Requiring manual mana for these abilities is rare enough (mostly
            // Demonmail Hauberk, Disciple of Griselbrand, Skirsdag Cultist) that the
            // tradeoff is acceptable.
            //
            // `SacrificeThis` DOES get auto-tap: the sacrifice target is fixed (the
            // source permanent itself), so there's no "which creature to sac" conflict.
            // We just exclude the source from the autotap plan's source pool so it
            // can't be used as a mana source for its own activation.
            //
            // NOTE: If you change this behavior, also update the "Sacrifice-cost activated
            // abilities" bullet in GAME_RULES in mtg-player/src/llm.rs.
            use crate::cards::SacrificeCost;
            let ability_has_free_sac_cost = matches!(
                ab.sacrifice_cost,
                SacrificeCost::SacrificeCreature | SacrificeCost::SacrificeAnotherCreature
            );
            let ability_has_sac_this = matches!(ab.sacrifice_cost, SacrificeCost::SacrificeThis);
            let has_x_cost = ab.cost.has_x();
            // Autotap sources to consider for this specific ability.
            //
            // The source pays for itself in two ways, and both have to be shut
            // off. Sacrificing it is one. Tapping it is the other: a permanent
            // that is part of the ability's own {T} cost cannot also be tapped
            // for mana to pay that ability's mana cost — one tap pays one cost
            // (CR 602.2h). The five ISD utility lands are all "{cost}, {T}:"
            // over a "{T}: Add {C}" mana ability, so without this the planner
            // credited Gavony Township's own {C} toward its {2}{G}{W} and
            // offered the ability with one land too few.
            let ability_sources: Vec<_> = if ability_has_sac_this || ab.requires_tap {
                early_mana_sources.iter()
                    .filter(|s| s.object_id != obj_id)
                    .cloned()
                    .collect()
            } else {
                early_mana_sources.clone()
            };
            let ability_tap_plan: Vec<(ObjectId, usize)> = if ability_has_free_sac_cost {
                // No auto-tap for "sacrifice a creature" abilities — require mana
                // already in the pool (see comment above).
                let cost_to_check = if has_x_cost {
                    ab.cost.without_x()
                } else {
                    ab.cost.clone()
                };
                if !mana::can_pay(mana_pool, &cost_to_check) { continue; }
                Vec::new()
            } else if has_x_cost {
                let non_x_cost = ab.cost.without_x();
                if mana::can_pay(mana_pool, &non_x_cost) {
                    Vec::new()
                } else {
                    match mana::compute_autotap(&non_x_cost, mana_pool, &ability_sources, &[]) {
                        Some(plan) => plan,
                        None => continue,
                    }
                }
            } else if mana::can_pay(mana_pool, &ab.cost) {
                Vec::new()
            } else {
                match mana::compute_autotap(&ab.cost, mana_pool, &ability_sources, &[]) {
                    Some(plan) => plan,
                    None => continue,
                }
            };
            // Check tap cost and summoning sickness.
            // Per MTG rules, creatures with summoning sickness cannot use
            // abilities with {T} in the cost (unless they have haste).
            // Non-creature permanents (lands, artifacts, enchantments) are not
            // affected by summoning sickness (CR 302.6).
            if ab.requires_tap {
                if obj_tapped { continue; }
                let is_creature = state.is_creature(obj.id, registry);
                if is_creature && obj.summoning_sick && !state.has_keyword(obj.id, Keyword::Haste, registry) {
                    continue;
                }
            }
            // Check the counter cost (CR 601.2h — the cost has to be payable).
            if let Some((counter_type, amount)) = ab.counter_cost {
                if state.get_counter_count(obj_id, counter_type) < amount { continue; }
            }
            // Check once-per-turn.
            if ab.once_per_turn && activated_this_turn.contains(&ab.ability_index) { continue; }
            // Check sorcery speed.
            if ab.sorcery_speed_only && !is_sorcery_speed { continue; }
            // Build the list of eligible sacrifices for this ability. We
            // enumerate one ActivateAbility per (target, sacrifice) combo so
            // the player chooses the sacrifice up front rather than having
            // the engine auto-pick (which could fizzle the ability by
            // sacrificing the very creature being targeted).
            let eligible_sacrifices: Vec<Option<ObjectId>> = match &ab.sacrifice_cost {
                SacrificeCost::None | SacrificeCost::SacrificeThis => {
                    // None: no choice. SacrificeThis: the source pays itself, no choice either.
                    vec![None]
                }
                SacrificeCost::SacrificeCreature => {
                    let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, player)
                        .iter()
                        .filter(|o| state.is_creature(o.id, registry))
                        .map(|o| o.id)
                        .collect();
                    if creatures.is_empty() { continue; }
                    creatures.into_iter().map(Some).collect()
                }
                SacrificeCost::SacrificeAnotherCreature => {
                    // "Another" excludes the source permanent (the equipment / card itself).
                    let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, player)
                        .iter()
                        .filter(|o| state.is_creature(o.id, registry) && o.id != obj_id)
                        .map(|o| o.id)
                        .collect();
                    if creatures.is_empty() { continue; }
                    creatures.into_iter().map(Some).collect()
                }
            };

            // Generate actions based on targeting.
            if let Some(ref _target_req) = ab.target_requirement {
                // Targeted ability: one action per (target, sacrifice) combo,
                // including the pair where the sacrifice IS the target.
                //
                // That pair used to be filtered out, on the reasoning that
                // sacrificing the target fizzles the ability and no rational
                // player would pick it. Both halves are true and the
                // conclusion still does not follow: CR 601.2b chooses targets
                // before CR 601.2h pays costs, so the activation is legal, the
                // sacrifice happens, and only the ability is countered on
                // resolution (CR 608.2b). The sacrifice can be the entire
                // point. Demonmail Hauberk's "Equip—Sacrifice a creature" is a
                // free sorcery-speed sac outlet: with one creature on the
                // battlefield, equipping it and sacrificing it is how you turn
                // a Doomed Traveler into a Spirit, and the equip fizzling
                // costs nothing. Filtering the pair removed the only way to do
                // it.
                let behavior = registry.get(source_card_id);
                if let Some(behavior) = behavior {
                    let targets = generate_ability_targets(state, obj_id, &ab, player, registry, behavior);
                    for target in targets {
                        for sac in &eligible_sacrifices {
                            actions.push(Action::ActivateAbility {
                                object_id: obj_id,
                                ability_index: ab.ability_index,
                                targets: vec![target.clone()],
                                tap_plan: ability_tap_plan.clone(),
                                sacrifice: *sac,
                                x_value: None,
                                source_card_id: if source_card_id == obj_card_id { None } else { Some(source_card_id) },
                            });
                        }
                    }
                }
            } else {
                // Untargeted ability — one action per eligible sacrifice (or one action
                // with sacrifice=None if there's no sacrifice cost).
                for sac in &eligible_sacrifices {
                    actions.push(Action::ActivateAbility {
                        object_id: obj_id,
                        ability_index: ab.ability_index,
                        targets: vec![],
                        tap_plan: ability_tap_plan.clone(),
                        sacrifice: *sac,
                        x_value: None,
                        source_card_id: if source_card_id == obj_card_id { None } else { Some(source_card_id) },
                    });
                }
            }

            // X-cost abilities are handled via a followup ChooseXValue prompt
            // after the ability is activated, not by enumerating multiple entries.
        }
    }
}

/// Loyalty abilities: sorcery speed, once per turn per planeswalker.
pub(crate) fn loyalty(ctx: &Ctx, actions: &mut Vec<Action>) {
    let Ctx { state, registry, player, is_sorcery_speed, .. } = *ctx;
    // Planeswalker loyalty abilities: sorcery speed, once per turn per planeswalker.
    if is_sorcery_speed {
        for obj in state.objects_in_zone(Zone::Battlefield, player) {
            let obj_id = obj.id;
            let obj_card_id = obj.card_id;
            let already_used = obj.abilities_activated_this_turn.contains(&999); // sentinel
            if already_used { continue; }

            if let Some(behavior) = registry.get(obj_card_id) {
                let loyalty_abs = behavior.loyalty_abilities(state, obj_id);
                if loyalty_abs.is_empty() { continue; }

                let current_loyalty = state.get_counter_count(obj_id, CounterType::Loyalty);
                for ab in &loyalty_abs {
                    // Check if we can pay the cost.
                    if ab.loyalty_change < 0 && u32::try_from(-ab.loyalty_change).unwrap_or(0) > current_loyalty {
                        continue; // Not enough loyalty
                    }
                    if let Some(ref target_req) = ab.target_requirement {
                        // Targeted loyalty ability: generate one action per valid target.
                        let targets = valid_targets_for_req(state, player, obj_id, target_req, behavior, registry);
                        for target in targets {
                            actions.push(Action::ActivateLoyaltyAbility {
                                object_id: obj_id,
                                ability_index: ab.ability_index,
                                targets: vec![target],
                            });
                        }
                    } else {
                        actions.push(Action::ActivateLoyaltyAbility {
                            object_id: obj_id,
                            ability_index: ab.ability_index,
                            targets: vec![],
                        });
                    }
                }
            }
        }
    }

    // Instant-speed window: anytime you have priority (which is already true here).
    let player_state = state.get_player(player);

    if is_sorcery_speed {
        // Play a land (if land plays remaining > 0).
        // Deduplicate — only show one "Play Forest" even if you have 3 in hand.
        if player_state.land_plays_remaining > 0 {
            let mut seen_lands: Vec<CardId> = Vec::new();
            for obj in state.objects_in_zone(Zone::Hand, player) {
                if let Some(behavior) = registry.get(obj.card_id) {
                    let data = behavior.card_data();
                    if data.card_types.contains(&CardType::Land) && !seen_lands.contains(&obj.card_id) {
                        seen_lands.push(obj.card_id);
                        actions.push(Action::PlayLand { object_id: obj.id });
                    }
                }
            }
        }
    }
}
