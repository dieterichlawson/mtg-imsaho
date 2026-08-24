//! Casting a spell: paying its costs and putting it on the stack (CR 601).

use super::super::Applied;
use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::ids::ObjectId;
use crate::mana;
use crate::state::{GameState, LogLevel};
use crate::types::Zone;
use super::super::*;

pub(crate) fn cast_spell(state: &mut GameState, object_id: ObjectId, targets: &[Target], sacrifice: Option<ObjectId>, exile_count: Option<u32>, exile_ids: &[ObjectId], alternative_cost: Option<&crate::types::ManaCost>, tap_plan: &[(ObjectId, usize)], registry: &CardRegistry) -> Applied {
        let player = state.priority_player.expect("CastSpell requires priority");

        // Detect flashback vs cast-from-graveyard.
        // Flashback: card has flashback_cost or dynamically granted flashback.
        // Cast-from-graveyard: card has can_cast_from_graveyard() (Skaab Ruinator) — uses normal mana cost.
        let card_id = state.get_object(object_id).expect("CastSpell object must exist").card_id;
        let data = registry.get(card_id).expect("card must be in registry").card_data();
        let behavior = registry.get(card_id).expect("card must be in registry");
        let in_graveyard = state.get_object(object_id)
            .is_some_and(|o| o.zone == Zone::Graveyard);
        let is_cast_from_graveyard = in_graveyard && behavior.can_cast_from_graveyard();
        let is_flashback = in_graveyard && !is_cast_from_graveyard;

        // Resolve the appropriate mana cost (applying cost reduction for
        // non-flashback). If an alternative_cost is provided (e.g.
        // Rooftop Storm's {0}), use it directly.
        let cost = if let Some(alt) = alternative_cost {
            alt.clone()
        } else if is_flashback {
            // Check until_end_of_turn for dynamically granted flashback.
            let dynamic_fb = state.until_end_of_turn.iter()
                .find_map(|e| if let crate::state::TemporaryEffect::GrantFlashback { target, cost } = e {
                    if *target == object_id { Some(cost.clone()) } else { None }
                } else { None });
            dynamic_fb.unwrap_or_else(|| {
                data.flashback_cost.expect("flashback cast on card without flashback_cost")
            })
        } else {
            let base_cost = data.cost.expect("non-flashback spell must have a mana cost");
            effective_spell_cost(&state, registry, card_id, &base_cost, player)
        };

        // Rules-strict X-cost casting (CR 601.2h → 601.2i): costs are
        // paid BEFORE the spell becomes cast. For X-cost spells with a
        // non-zero max_x, we can't pay the cost yet — we don't know X.
        // So we stash the full casting context on `pending_spell_cast`,
        // set up `ChooseXFunding`, and leave the spell in its origin
        // zone (hand / graveyard). The resolution handler executes
        // everything atomically once funding lands: tap mana, pay mana,
        // pay additional costs, move to stack, fire SpellCast.
        //
        // The max_x == 0 fallthrough (no funding choice possible) and
        // the non-X path both run the eager flow below — no prompt
        // needed, so paying costs up front is fine.
        //
        // NOTE: If you change this flow, also update the "X-cost spells"
        // bullet in GAME_RULES in mtg-player/src/llm.rs so the agent's
        // system prompt stays accurate.
        let has_x = cost.has_x();

        if has_x {
            let non_x_cost = cost.without_x();
            // Probe max_x without touching state: simulate tap_plan's
            // mana output + existing pool + any remaining untapped
            // sources after the tap_plan runs.
            let probe_options = {
                // Apply tap_plan and non-X payment in a clone to see
                // what's left for X.
                let mut probe = state.clone();
                for &(source_id, ability_index) in tap_plan {
                    activate_mana_source(&mut probe, source_id, ability_index, registry);
                }
                let _ = mana::auto_pay(&mut probe.get_player_mut(player).mana_pool, &non_x_cost);
                crate::funding::build_options(&probe, player, registry)
            };

            if probe_options.max_x > 0 {
                // Stash context; leave spell in hand. Set up the prompt.
                let spell_name = card_name(&state, registry, object_id);
                state.pending_spell_cast = Some(crate::state::PendingSpellCast {
                    object_id: object_id,
                    player,
                    card_id,
                    targets: targets.to_vec(),
                    sacrifice: sacrifice,
                    exile_ids: exile_ids.to_vec(),
                    exile_count: exile_count,
                    tap_plan: tap_plan.to_vec(),
                    alternative_cost: alternative_cost.cloned(),
                    non_x_mana_cost: non_x_cost,
                    is_flashback,
                });
                state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                    player,
                    source: object_id,
                    choice: crate::state::ResolutionChoiceKind::ChooseXFunding {
                        description: format!("{spell_name}: choose X funding (0-{})", probe_options.max_x),
                        options: probe_options,
                        source_id: object_id,
                        is_ability: false,
                    },
                });
                // Nothing else happens until the player submits the
                // funding response — spell stays in hand, no mana
                // paid, no taps executed.
                return Applied::ReturnNow;
            }
            // max_x == 0: no legal funding choice, X is forced to 0.
            // Fall through to the eager path and pay as X=0.
        }

        // Rules-strict exile-cost casting: for spells with an
        // `ExileXFromGraveyard` or `ExileCreaturesFromGraveyard(n)`
        // additional cost, set up a `ChooseExileFromGraveyard` prompt
        // and leave the spell in hand until the player submits
        // `ChosenExileSet`. Mirrors the ChooseXFunding flow above.
        //
        // The prompt is only set up if the caller left `exile_ids`
        // empty AND hasn't specified an exile_count (for variable-X
        // exile cost). That lets tests and other code paths submit
        // an already-resolved `CastSpell` with specific exile_ids
        // (or an explicit X count) and bypass the prompt.
        {
            use crate::cards::AdditionalCost;
            let additional = data.additional_cost.clone();
            let needs_exile_prompt = exile_ids.is_empty() && match &additional {
                Some(AdditionalCost::ExileXFromGraveyard) => exile_count.is_none(),
                Some(AdditionalCost::ExileCreaturesFromGraveyard(_)) => true,
                _ => false,
            };
            if needs_exile_prompt {
                let (gy_options, min, max) = match &additional {
                    Some(AdditionalCost::ExileXFromGraveyard) => {
                        // Any card in the caster's graveyard is eligible,
                        // except the spell itself (if cast from GY).
                        let opts: Vec<ObjectId> = state.objects.values()
                            .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != object_id)
                            .map(|o| o.id)
                            .collect();
                        let n = opts.len();
                        (opts, 0usize, n)
                    }
                    Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) => {
                        // Only creature cards in GY.
                        let opts: Vec<ObjectId> = state.objects.values()
                            .filter(|o| {
                                o.zone == Zone::Graveyard && o.owner == player && o.id != object_id
                                    && state.is_creature(o.id, registry)
                            })
                            .map(|o| o.id)
                            .collect();
                        (opts, *n, *n)
                    }
                    _ => unreachable!(),
                };

                let spell_name = card_name(&state, registry, object_id);
                let description = match &additional {
                    Some(AdditionalCost::ExileXFromGraveyard) => format!(
                        "{spell_name}: choose 0-{} cards to exile from your graveyard (each exiled card adds to the spell's X)",
                        gy_options.len()
                    ),
                    Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) => format!(
                        "{spell_name}: choose exactly {n} creature{} to exile from your graveyard",
                        if *n == 1 { "" } else { "s" }
                    ),
                    _ => unreachable!(),
                };

                // For X-cost spells, the non_x_mana_cost is the stripped
                // cost. For non-X spells, use the full cost.
                let non_x_mana_cost = if has_x {
                    cost.without_x()
                } else {
                    cost.clone()
                };

                state.pending_spell_cast = Some(crate::state::PendingSpellCast {
                    object_id: object_id,
                    player,
                    card_id,
                    targets: targets.to_vec(),
                    sacrifice: sacrifice,
                    exile_ids: exile_ids.to_vec(),
                    exile_count: exile_count,
                    tap_plan: tap_plan.to_vec(),
                    alternative_cost: alternative_cost.cloned(),
                    non_x_mana_cost,
                    is_flashback,
                });
                state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                    player,
                    source: object_id,
                    choice: crate::state::ResolutionChoiceKind::ChooseExileFromGraveyard {
                        description,
                        options: gy_options,
                        min,
                        max,
                        source_id: object_id,
                    },
                });
                // Spell stays in hand; no mana tapped or paid yet.
                return Applied::ReturnNow;
            }
        }

        // Eager path: non-X spells and X-cost spells with max_x == 0.
        // Execute tap_plan, pay mana, pay additional costs, move to stack.
        for &(source_id, ability_index) in tap_plan {
            activate_mana_source(&mut *state, source_id, ability_index, registry);
        }

        if has_x {
            // max_x == 0 case: pay only the non-X portion.
            let non_x_cost = cost.without_x();
            mana::auto_pay(&mut state.get_player_mut(player).mana_pool, &non_x_cost)
                .expect("legal_actions should have verified mana availability");
        } else {
            mana::auto_pay(&mut state.get_player_mut(player).mana_pool, &cost)
                .expect("legal_actions should have verified mana availability");
        }

        // Pay additional costs (sacrifice) at cast time, before the spell goes on the stack.
        if let Some(sac_id) = sacrifice {
            let sac_name = card_name(&state, registry, sac_id);
            crate::destruction::sacrifice(&mut *state, sac_id, registry);
            state.log(LogLevel::Event,
                format!("Sacrificed {sac_name} as additional cost"));
        } else {
            // Backward compatibility: if sacrifice is None but the spell has
            // AdditionalCost::SacrificeCreature, auto-sacrifice the first creature.
            use crate::cards::AdditionalCost;
            let needs_sac = registry.get(card_id)
                .is_some_and(|b| matches!(b.card_data().additional_cost, Some(AdditionalCost::SacrificeCreature)));
            if needs_sac {
                let creature = state.objects_in_zone(Zone::Battlefield, player)
                    .iter()
                    .find(|o| state.is_creature(o.id, registry))
                    .map(|o| o.id);
                if let Some(cid) = creature {
                    let sac_name = card_name(&state, registry, cid);
                    crate::destruction::sacrifice(&mut *state, cid, registry);
                    state.log(LogLevel::Event,
                        format!("Sacrificed {sac_name} as additional cost"));
                }
            }
        }

        // Handle ExileCreaturesFromGraveyard additional cost (Skaab Ruinator, Corpse Lunge, etc.).
        {
            use crate::cards::AdditionalCost;
            if let Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) = registry.get(card_id)
                .and_then(|b| b.card_data().additional_cost)
            {
                // Use player-chosen exile_ids if provided, otherwise fall back to auto-pick.
                let to_exile: Vec<ObjectId> = if exile_ids.is_empty() {
                    let mut exile_candidates: Vec<(ObjectId, i32)> = state.objects.values()
                        .filter(|o| {
                            o.zone == Zone::Graveyard && o.owner == player && o.id != object_id
                                && state.is_creature(o.id, registry)
                        })
                        .map(|o| (o.id, o.power.unwrap_or(0)))
                        .collect();
                    exile_candidates.sort_by(|a, b| b.1.cmp(&a.1));
                    exile_candidates.into_iter().take(n).map(|(id, _)| id).collect()
                } else {
                    exile_ids.to_vec()
                };

                // Store the first exiled creature's power for cards that need it
                // (Corpse Lunge uses the power to determine damage).
                // Use `effective_power` so characteristic-defining-ability creatures
                // (Boneyard Wurm, Splinterfright, etc.) whose P/T is a function of
                // graveyard state — which CR 208.2 says "works in all zones" —
                // store their CDA-computed power instead of the base 0.
                if let Some(&first_exile) = to_exile.first() {
                    let power = state.effective_power(first_exile, registry).unwrap_or(0);
                    if let Some(obj) = state.get_object_mut(object_id) {
                        obj.card_state.insert("exiled_power".into(), ObjectId(u64::try_from(power).unwrap_or(0)));
                    }
                }

                for exile_id in &to_exile {
                    let name = card_name(&state, registry, *exile_id);
                    state.move_object(*exile_id, Zone::Exile, registry);
                    state.log(LogLevel::Event,
                        format!("Exiled {name} from graveyard as additional cost"));
                }
            }
        }

        // Handle ExileXFromGraveyard additional cost (Harvest Pyre).
        // The player chose X via exile_count in the action.
        {
            use crate::cards::AdditionalCost;
            let needs_exile_x = registry.get(card_id)
                .is_some_and(|b| matches!(b.card_data().additional_cost, Some(AdditionalCost::ExileXFromGraveyard)));
            if needs_exile_x {
                // If specific cards were chosen (via exile_ids), exile those exactly.
                // Otherwise fall back to auto-selecting the first exile_count cards (legacy behavior).
                let graveyard_cards: Vec<ObjectId> = if exile_ids.is_empty() {
                    let x = exile_count.unwrap_or(0) as usize;
                    state.objects.values()
                        .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != object_id)
                        .map(|o| o.id)
                        .take(x)
                        .collect()
                } else {
                    exile_ids.to_vec()
                };
                let count = u32::try_from(graveyard_cards.len()).unwrap_or(u32::MAX);
                for gid in &graveyard_cards {
                    state.move_object(*gid, Zone::Exile, registry);
                }
                // Store the count on the spell for resolution.
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.card_state.insert("exile_count".into(), ObjectId(u64::from(count)));
                }
                state.log(LogLevel::Event,
                    format!("Exiled {count} cards from graveyard as additional cost"));
            }
        }

        // Move to stack and store targets.
        state.move_object(object_id, Zone::Stack, registry);
        {
            let obj = state.get_object_mut(object_id).expect("spell must exist after moving to stack");
            obj.targets = targets.to_vec();
            if is_flashback {
                obj.cast_with_flashback = true;
            }
        }

        // For ModalChoice spells, determine and store which mode was chosen
        // by checking which mode's valid targets match the actual targets.
        if let Some(behavior) = registry.get(card_id) {
            if let crate::cards::TargetRequirement::ModalChoice(ref modes) = behavior.target_requirement() {
                let chosen = detect_modal_choice_mode(&state, player, object_id, targets, modes, behavior);
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.chosen_mode = Some(chosen);
                }
            }
        }

        state.stack.push(crate::state::StackEntry::Spell(object_id));

        if has_x {
            // Eager path reached here only when max_x == 0 (X forced to
            // 0). The funding-prompt path already returned earlier.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.x_value = Some(0);
            }
        }
        finalize_spell_cast(&mut *state, player, object_id, is_flashback, targets, registry);
    Applied::Continue
}
