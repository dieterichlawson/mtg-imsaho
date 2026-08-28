//! Casting a spell: paying its costs and putting it on the stack (CR 601).

use super::super::Applied;
use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::ids::ObjectId;
use crate::mana;
use crate::state::GameState;
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
        // CR 601.2c: the same target cannot be chosen twice for one instance of
        // the word "target". The offered action list already respects it; a
        // list built by hand — which is how both clients assemble a cast — did
        // not.
        let target_req = behavior.target_requirement();
        let deduped = crate::engine::targeting::distinct_within_each_target_instance(
            &target_req, targets);
        if deduped.len() != targets.len() {
            state.log(crate::state::LogLevel::Debug, format!(
                "{}: dropped a target named twice (CR 601.2c)", data.name));
        }
        let targets: &[Target] = &deduped;

        // CR 601.2c: the targets are chosen as the spell is cast, and they must
        // be legal ones. `legal_actions` only offers legal sets, but neither
        // client picks a whole offered action — both assemble their own from
        // per-slot choices — so nothing had read the list back. Refused here,
        // before any cost is paid and before the card moves, so the state is
        // untouched: an illegal choice means the cast did not happen, not that
        // it happened for nothing.
        if !crate::engine::targeting::targets_are_legal(
            state, &target_req, targets, player, object_id, behavior, registry)
        {
            state.log(crate::state::LogLevel::Debug, format!(
                "{}: cast refused, illegal targets {targets:?} (CR 601.2c)", data.name));
            return Applied::ReturnNow;
        }

        // CR 601.2h: the additional cost must be one the caster can pay, and
        // what they named to pay it with must really be theirs to pay. Refused
        // here for the same reason the target check is: before anything is
        // paid, so a refused cast is a cast that did not happen.
        if !crate::engine::costs::additional_cost_is_payable(
            state, registry, card_id, object_id, player, sacrifice, exile_ids)
        {
            state.log(crate::state::LogLevel::Debug, format!(
                "{}: cast refused, additional cost not payable as submitted (CR 601.2h)",
                data.name));
            return Applied::ReturnNow;
        }

        let in_graveyard = state.get_object(object_id)
            .is_some_and(|o| o.zone == Zone::Graveyard);
        let is_cast_from_graveyard = in_graveyard && behavior.can_cast_from_graveyard();
        let is_flashback = in_graveyard && !is_cast_from_graveyard;

        // Resolve the appropriate mana cost (applying cost reduction for
        // non-flashback). If an alternative_cost is provided (e.g.
        // Rooftop Storm's {0}), use it directly.
        // `legal_actions` puts the determined total on the action, so the
        // player is charged the cost they were offered. The fallback is for
        // callers that build a `CastSpell` directly (tests, replays): work the
        // cost out the same way, through the one determination.
        let method = match (alternative_cost, is_flashback) {
            (Some(alt), _) => CastMethod::Alternative(alt.clone()),
            (None, true) => {
                let dynamic_fb = state.until_end_of_turn.iter()
                    .find_map(|e| if let crate::state::TemporaryEffect::GrantFlashback { target, cost } = e {
                        if *target == object_id { Some(cost.clone()) } else { None }
                    } else { None });
                CastMethod::Alternative(dynamic_fb.unwrap_or_else(|| {
                    data.flashback_cost.clone().expect("flashback cast on card without flashback_cost")
                }))
            }
            (None, false) => CastMethod::Normal,
        };
        let cost = match &method {
            // Already the determined total — reducing it again would
            // double-count.
            CastMethod::Alternative(c) if alternative_cost.is_some() => c.clone(),
            _ => cost_to_cast(&state, registry, card_id, player, &method).mana,
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

        // Rules-strict exile-cost casting: a spell with an exile-from-graveyard
        // additional cost sets up a `ChooseExileFromGraveyard` prompt and stays
        // in its origin zone until the player submits `ChosenExileSet`, exactly
        // as the ChooseXFunding flow above does. A caller that already named
        // the cards (a test, a replay) skips the prompt.
        if let Some(prompt) = costs::exile_prompt(
            &state, registry, card_id, object_id, player, exile_count, exile_ids,
            &card_name(&state, registry, object_id),
        ) {
            // For X-cost spells the stashed cost is the stripped one.
            let non_x_mana_cost = if has_x { cost.without_x() } else { cost.clone() };
            state.pending_spell_cast = Some(crate::state::PendingSpellCast {
                object_id,
                player,
                card_id,
                targets: targets.to_vec(),
                sacrifice,
                exile_ids: exile_ids.to_vec(),
                exile_count,
                tap_plan: tap_plan.to_vec(),
                alternative_cost: alternative_cost.cloned(),
                non_x_mana_cost,
                is_flashback,
            });
            state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                player,
                source: object_id,
                choice: crate::state::ResolutionChoiceKind::ChooseExileFromGraveyard {
                    description: prompt.description,
                    options: prompt.options,
                    min: prompt.min,
                    max: prompt.max,
                    source_id: object_id,
                },
            });
            // Spell stays in hand; no mana tapped or paid yet.
            return Applied::ReturnNow;
        }

        // Eager path: non-X spells and X-cost spells with max_x == 0.
        // Execute tap_plan, pay mana, pay additional costs, move to stack.
        //
        // Neither client submits a whole offered action, so the funding has to
        // prove itself before anything is tapped or paid: an unfundable cast
        // is refused with the state untouched (CR 601.2h), not a panic
        // mid-payment. The tap plan is rehearsed on a scratch copy because
        // what it produces can depend on state it also changes (Deranged
        // Assistant milling its own cost away), and `auto_pay` drains the
        // pool as it goes, so a failed payment cannot simply be unwound.
        {
            let mut probe = state.clone();
            for &(source_id, ability_index) in tap_plan {
                activate_mana_source(&mut probe, source_id, ability_index, registry);
            }
            let pay = if has_x { cost.without_x() } else { cost.clone() };
            if !mana::can_pay(&probe.get_player(player).mana_pool, &pay) {
                state.log(crate::state::LogLevel::Debug, format!(
                    "{}: cast refused, submitted funding cannot pay the cost (CR 601.2h)",
                    card_name(&state, registry, object_id)));
                return Applied::ReturnNow;
            }
        }
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

        // CR 601.2b: additional costs are paid before the spell goes on the
        // stack. One dispatch on the kind, shared with the exile-choice handler.
        costs::pay_additional_cost(
            state, registry, card_id, object_id, player, sacrifice, exile_count, exile_ids);

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
                let chosen = detect_modal_choice_mode(&state, player, object_id, targets, modes, behavior, registry);
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
