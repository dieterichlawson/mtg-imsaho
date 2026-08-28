//! Answering a mid-resolution choice the engine asked for.

use super::super::Applied;
use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::mana;
use crate::state::{AwaitingAction, GameState, LogLevel};
use crate::types::{Zone, CardType, ContinuousEffect};
use super::super::*;

pub(crate) fn resolve_choice(state: &mut GameState, resolved: &crate::actions::ResolvedChoice, registry: &CardRegistry) -> Applied {
        use crate::state::ResolutionChoiceKind;
        use crate::actions::ResolvedChoice;
        let awaiting = state.awaiting_action.take();
        if let Some(AwaitingAction::ResolutionChoice { choice: kind, source: choice_source, .. }) = awaiting {
            match (&kind, resolved) {
                (ResolutionChoiceKind::PayOrNot { spell_id, source_spell_id, cost, .. },
                 ResolvedChoice::PayDecision(pay)) => {
                    // CR 608.2g: paying may involve tapping for the mana.
                    // If it can't actually be paid the cost is unpaid, and
                    // the "unless" clause takes effect as if declined —
                    // this used to ignore the payment's result, so saying
                    // "pay" with an empty pool saved the spell for free.
                    let payer = state.get_object(*spell_id).map_or(PlayerId(0), |o| o.controller);
                    let paid = *pay
                        && pay_cost_with_sources(&mut *state, payer, cost, registry);
                    if paid {
                        state.log(LogLevel::Event, "Paid the cost to prevent the counter".into());
                    } else {
                        crate::cards::helpers::counter_spell(state, *spell_id, registry);
                    }

                    // Whatever else the card says happens is the card's, not
                    // this handler's. `payer` is passed rather than re-read:
                    // the spell may now be in a graveyard, where CR 108.4
                    // leaves it with no controller.
                    let source_card_id = state.get_object(*source_spell_id).map(|o| o.card_id);
                    if let Some(behavior) = source_card_id.and_then(|cid| registry.get(cid)) {
                        behavior.on_pay_decision(&mut *state, *source_spell_id, payer, paid, registry);
                    }
                    if state.awaiting_action.is_some() {
                        return Applied::ReturnNow;
                    }
                }
                (ResolutionChoiceKind::YesNo { source_card, .. },
                 ResolvedChoice::YesNoDecision(yes)) => {
                    // Dispatch to the card's on_yes_no_choice hook.
                    let source_card_id = state.get_object(*source_card).map(|o| o.card_id);
                    if let Some(behavior) = source_card_id.and_then(|cid| registry.get(cid)) {
                        behavior.on_yes_no_choice(&mut *state, *source_card, *yes, registry);
                    }
                }
                (ResolutionChoiceKind::ChooseTarget { effect, .. },
                 ResolvedChoice::ChosenTarget(chosen)) => {
                    if let Some(t) = chosen {
                        apply_pending_effect(&mut *state, t, effect, registry);
                    } else {
                        // "None of them" is an answer, not an absence of one.
                        let source_card_id = state.get_object(choice_source).map(|o| o.card_id);
                        if let Some(behavior) = source_card_id.and_then(|cid| registry.get(cid)) {
                            behavior.on_declined_choice(&mut *state, choice_source, registry);
                        }
                    }
                    // If this was an "enters as a copy" choice (Evil Twin)
                    // and the player declined, the copy never resolves —
                    // disarm the SBA guard so the printed 0/0 can die.
                    // (On accept, the CopyCreature handler already did.)
                    if let crate::state::PendingEffect::CopyCreature { source_id } = effect {
                        if let Some(obj) = state.get_object_mut(*source_id) {
                            obj.entering_copy_source = false;
                        }
                    }
                }
                (ResolutionChoiceKind::ChooseCardFromHand { discard_immediately, remaining, player, description, .. },
                 ResolvedChoice::ChosenCard(discard_id)) => {
                    // CR 101.4: when several players are each choosing a
                    // card to discard, the source collects the choices and
                    // discards them together once the last one has chosen
                    // — see `discard_immediately`.
                    if *discard_immediately {
                        let name = state.obj_name(*discard_id);
                        state.discard_card(*discard_id, registry);
                        state.log(LogLevel::Event, format!("Discarded {name}"));
                    }
                    // Notify the source card about the discard (e.g., Civilized Scholar
                    // checks if the discarded card was a creature to trigger transform).
                    notify_discard(&mut *state, choice_source, *discard_id, registry);
                    // "Discards two cards" is one choice with a count on it,
                    // so come back for the rest — unless the card's own hook
                    // has already asked the player something else, in which
                    // case that question is mid-flight and this one waits for
                    // the chain to unwind.
                    if *discard_immediately && *remaining > 1 && state.awaiting_action.is_none() {
                        let source = description.split(':').next().unwrap_or("Discard").to_string();
                        crate::engine::discard_cards(
                            &mut *state, *player, remaining - 1, choice_source, &source, registry);
                    }
                }
                (ResolutionChoiceKind::ChooseFromRevealed { revealed, .. },
                 ResolvedChoice::ChosenCard(keep_id)) => {
                    let keep_name = state.obj_name(*keep_id);
                    state.move_object(*keep_id, Zone::Hand, registry);
                    // "…and the rest into your graveyard" — library to
                    // graveyard, so it goes through `mill_one` and emits
                    // CreatureCardMilled. Moving them directly hid them from
                    // an opponent's Undead Alchemist.
                    for &card_id in revealed {
                        if card_id != *keep_id {
                            let Some(owner) = state.get_object(card_id).map(|o| o.owner)
                                else { continue };
                            crate::engine::mill_one(state, owner, card_id, registry);
                        }
                    }
                    state.log(LogLevel::Event, format!("Kept {keep_name}"));
                }
                (ResolutionChoiceKind::ChooseFromLibrary { searcher, destination, tapped, .. },
                 ResolvedChoice::ChosenCard(chosen_id)) => {
                    crate::cards::helpers::finish_library_search(
                        &mut *state, *searcher, *chosen_id, *destination, *tapped, registry);
                }
                // CR 701.19b: the player searched and chose to find nothing.
                // CR 701.19a: they searched, so they still shuffle.
                (ResolutionChoiceKind::ChooseFromLibrary { searcher, .. },
                 ResolvedChoice::ChosenTarget(None)) => {
                    let searcher = *searcher;
                    state.log(LogLevel::Event, format!("p{}: found nothing", searcher.0));
                    crate::cards::helpers::shuffle_library(&mut *state, searcher);
                }
                (ResolutionChoiceKind::ChooseCardType { options, controller, .. },
                 ResolvedChoice::ChosenIndex(index, _)) => {
                    let chosen_type = options.get(*index).cloned().unwrap_or_default();
                    let card_type = match chosen_type.as_str() {
                        "Artifact" => CardType::Artifact,
                        "Enchantment" => CardType::Enchantment,
                        "Land" => CardType::Land,
                        "Planeswalker" => CardType::Planeswalker,
                        _ => CardType::Creature,
                    };
                    let to_return: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, *controller)
                        .iter()
                        .map(|o| o.id)
                        // "Return all **cards** of the chosen type" — CR 109.1,
                        // so a token in the graveyard is not one of them.
                        .filter(|id| state.is_card(*id) && state.has_card_type(*id, card_type, registry))
                        .collect();
                    let count = to_return.len();
                    for id in to_return {
                        state.move_object(id, Zone::Hand, registry);
                    }
                    state.log(LogLevel::Event,
                        format!("{}: chose {chosen_type}. Returned {count} cards from graveyard to hand",
                            state.obj_name(choice_source)));
                }
                (ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, target_player, source_id, .. },
                 ResolvedChoice::ChosenSubset(pile_1_ids)) => {
                    // Controller has divided permanents into two piles.
                    // pile_1 = the chosen subset, pile_2 = the rest.
                    let pile_1: Vec<ObjectId> = pile_1_ids.clone();
                    let pile_2: Vec<ObjectId> = permanents.iter()
                        .filter(|id| !pile_1_ids.contains(id))
                        .copied()
                        .collect();

                    // Log the division.
                    let pile_1_names: Vec<String> = pile_1.iter()
                        .map(|id| state.obj_name(*id))
                        .collect();
                    let pile_2_names: Vec<String> = pile_2.iter()
                        .map(|id| state.obj_name(*id))
                        .collect();
                    state.log(LogLevel::Event,
                        format!("{}: Pile 1: [{}], Pile 2: [{}]",
                            state.obj_name(*source_id),
                            if pile_1_names.is_empty() { "empty".into() } else { pile_1_names.join(", ") },
                            if pile_2_names.is_empty() { "empty".into() } else { pile_2_names.join(", ") }));

                    // Now the target player chooses which pile to sacrifice.
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: *target_player,
                        source: *source_id,
                        choice: ResolutionChoiceKind::ChoosePile {
                            description: format!(
                                "{}: Choose a pile to sacrifice.\nPile 1: [{}]\nPile 2: [{}]",
                                state.obj_name(*source_id),
                                if pile_1_names.is_empty() { "empty".into() } else { pile_1_names.join(", ") },
                                if pile_2_names.is_empty() { "empty".into() } else { pile_2_names.join(", ") }),
                            pile_1,
                            pile_2,
                            source_id: *source_id,
                        },
                    });
                }
                (ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. },
                 ResolvedChoice::ChosenIndex(index, _)) => {
                    // Target player chose which pile to sacrifice.
                    let chosen_pile = if *index == 0 { pile_1 } else { pile_2 };
                    let pile_label = if *index == 0 { "Pile 1" } else { "Pile 2" };
                    state.log(LogLevel::Event,
                        format!("{}: chose to sacrifice {pile_label}", state.obj_name(choice_source)));
                    for &perm_id in chosen_pile {
                        let name = state.obj_name(perm_id);
                        if state.get_object(perm_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                            crate::destruction::sacrifice(&mut *state, perm_id, registry);
                            state.log(LogLevel::Event,
                                format!("{}: sacrificed {name}", state.obj_name(choice_source)));
                        }
                    }
                }
                (ResolutionChoiceKind::ChooseCardName { options, source_id, .. },
                 ResolvedChoice::ChosenIndex(index, _)) => {
                    let chosen_name = options.get(*index).cloned().unwrap_or_default();
                    // Store the restriction as an instance continuous effect on the source.
                    if let Some(obj) = state.get_object_mut(*source_id) {
                        let effect = ContinuousEffect::PreventCastingNamed { name: chosen_name.clone() };
                        obj.instance_continuous_effects = Some(vec![effect]);
                    }
                    state.log(LogLevel::Event,
                        format!("{} names \"{chosen_name}\"", state.obj_name(*source_id)));
                }
                // X-cost funding resolution: rules-strict atomic cast.
                // For spells: the spell is still in its origin zone.
                // Execute the stashed tap_plan, pay the non-X mana cost,
                // apply the funding response (pays X), pay additional
                // costs (sacrifice / exile), move the spell to the
                // stack, and fire SpellCast — all in one step. This
                // implements CR 601.2b → 601.2i: announce X, pay total
                // cost, THEN the spell becomes cast.
                //
                // For abilities: the ability was already partially
                // committed (tap/sac paid eagerly in the activate
                // handler); apply funding and fire the deferred effect.
                //
                // NOTE: If you change this, also update the "X-cost
                // spells" bullet in GAME_RULES in mtg-player/src/llm.rs.
                (ResolutionChoiceKind::ChooseXFunding { options, source_id, is_ability, .. },
                 ResolvedChoice::XFunding(response)) => {
                    let player = state.priority_player
                        .unwrap_or(state.active_player);
                    // Validate the response shape. For X-funding this
                    // should be impossible-by-construction with the
                    // string-enum schema, but validate defensively.
                    crate::funding::validate(response, options)
                        .expect("ChooseXFunding response must be valid (player implementations should pre-validate)");

                    if *is_ability {
                        let x = crate::funding::apply(&mut *state, player, options, response, registry);
                        state.log(LogLevel::Event, format!("Funded X = {x}"));
                        state.last_activated_x_value = Some(x);
                        let pending = state.pending_ability_effect.take()
                            .expect("pending_ability_effect must be set for X-cost ability funding");
                        super::abilities::put_ability_on_stack(
                            &mut *state,
                            pending.source_id,
                            pending.ability_index,
                            pending.behavior_card_id,
                            &pending.targets,
                            registry,
                        );
                        let name = card_name(&state, registry, pending.source_id);
                        state.log(
                            LogLevel::Event,
                            format!(
                                "p{} activated ability on {}: {}",
                                pending.activator.0, name, pending.description
                            ),
                        );
                    } else {
                        // Spells: pull the stashed casting context and
                        // run the full casting sequence atomically.
                        let pending = state.pending_spell_cast.take()
                            .expect("pending_spell_cast must be set for X-cost spell funding");
                        debug_assert_eq!(pending.object_id, *source_id);
                        debug_assert_eq!(pending.player, player);

                        // Step 1: execute tap_plan for the non-X cost.
                        for (src_id, ability_index) in &pending.tap_plan {
                            activate_mana_source(&mut *state, *src_id, *ability_index, registry);
                        }

                        // Step 2: pay the non-X mana portion.
                        mana::auto_pay(
                            &mut state.get_player_mut(player).mana_pool,
                            &pending.non_x_mana_cost,
                        ).expect("non-X mana should be payable after tap_plan");

                        // Step 3: apply the funding response (pays X by
                        // tapping funding sources + draining pool).
                        let x = crate::funding::apply(&mut *state, player, options, response, registry);
                        state.log(LogLevel::Event, format!("Funded X = {x}"));

                        // Step 4: pay additional costs (CR 601.2b), through
                        // the same dispatch the eager cast path uses.
                        crate::engine::costs::pay_additional_cost(
                            state, registry, pending.card_id, pending.object_id, player,
                            pending.sacrifice, pending.exile_count, &pending.exile_ids);

                        // Step 5: move spell to stack, set metadata,
                        // push StackEntry.
                        state.move_object(pending.object_id, Zone::Stack, registry);
                        {
                            let obj = state.get_object_mut(pending.object_id)
                                .expect("spell must exist after moving to stack");
                            obj.targets.clone_from(&pending.targets);
                            if pending.is_flashback {
                                obj.cast_with_flashback = true;
                            }
                            obj.x_value = Some(x);
                        }
                        if let Some(behavior) = registry.get(pending.card_id) {
                            if let crate::cards::TargetRequirement::ModalChoice(ref modes) =
                                behavior.target_requirement()
                            {
                                let chosen = detect_modal_choice_mode(
                                    &state, player, pending.object_id, &pending.targets, modes, behavior, registry,
                                );
                                if let Some(obj) = state.get_object_mut(pending.object_id) {
                                    obj.chosen_mode = Some(chosen);
                                }
                            }
                        }
                        state.stack.push(crate::state::StackEntry::Spell(pending.object_id));

                        // Step 6: fire SpellCast + bookkeeping.
                        finalize_spell_cast(
                            &mut *state, player, pending.object_id,
                            pending.is_flashback, &pending.targets, registry,
                        );
                    }
                }
                // Exile-choice resolution: player picked a subset of
                // graveyard cards to exile as the additional cost.
                // Reconstruct the CastSpell action with exile_ids
                // populated and recurse through submit_action — this
                // re-enters the CastSpell handler, which now sees a
                // non-empty exile_ids and takes the eager path
                // (tap mana, pay mana, exile, move to stack, fire
                // SpellCast).
                (ResolutionChoiceKind::ChooseExileFromGraveyard { min, max, options, .. },
                 ResolvedChoice::ChosenExileSet(chosen)) => {
                    // Validate: every chosen id must be in options,
                    // no duplicates, count in [min, max].
                    let validation_error = {
                        let n = chosen.len();
                        if n < *min || n > *max {
                            Some(format!("chose {n}, required {min}..={max}"))
                        } else if chosen.iter().any(|id| !options.contains(id)) {
                            Some("chosen id not in options".to_string())
                        } else {
                            let mut dedup = chosen.clone();
                            dedup.sort();
                            dedup.dedup();
                            if dedup.len() == chosen.len() {
                                None
                            } else {
                                Some("duplicate ids in choice".to_string())
                            }
                        }
                    };
                    if let Some(err) = validation_error {
                        // Invalid response — cancel the cast (spell stays
                        // in hand, no mana paid). Matches the rules-strict
                        // "if you can't pay all costs the spell was never
                        // cast" semantics.
                        state.log(LogLevel::Event,
                            format!("Exile-choice rejected: {err}; cast cancelled"));
                        state.pending_spell_cast = None;
                        return Applied::ReturnNow;
                    }

                    let pending = state.pending_spell_cast.take()
                        .expect("pending_spell_cast must be set for ChosenExileSet");
                    // Reconstruct a CastSpell with exile_ids populated.
                    // exile_count mirrors chosen.len() for ExileXFromGraveyard;
                    // ignored for fixed-count ExileCreaturesFromGraveyard.
                    let cast = crate::actions::Action::CastSpell {
                        object_id: pending.object_id,
                        targets: pending.targets.clone(),
                        sacrifice: pending.sacrifice,
                        exile_count: Some(u32::try_from(chosen.len()).unwrap_or(u32::MAX)),
                        exile_ids: chosen.clone(),
                        alternative_cost: pending.alternative_cost.clone(),
                        tap_plan: pending.tap_plan.clone(),
                    };
                    // Clear awaiting_action so the recursive submit_action
                    // doesn't treat this as another resolution choice.
                    state.awaiting_action = None;
                    return Applied::Replace(submit_action(state, &cast, registry));
                }
                // Player cancelled a cast mid-prompt (rarely reached —
                // only when a fixed-count exile choice couldn't be
                // satisfied after validation retries).
                (_, ResolvedChoice::CancelCast) => {
                    state.log(LogLevel::Event, "Cast cancelled".into());
                    state.pending_spell_cast = None;
                }
                _ => {}
            }
        }
        state.consecutive_passes = 0;
    // The activated ability that raised this choice is finished once no
    // further choice is pending. Until then its "you" is still the activator,
    // which `helpers::ability_controller` reads (CR 602.2a).
    if state.awaiting_action.is_none() {
        state.resolving_ability_activator = None;
    }
    Applied::Continue
}
