use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
use crate::types::{Zone, Supertype};
use super::*;

/// Finalize a spell cast: fire `SpellCast`, bump the per-turn counter, and
/// emit the cast log message. Called either immediately (non-X spell) or
/// after X-funding completes (X-cost spell). This corresponds to CR 601.2i —
/// the point at which "the spell becomes cast" and triggers watching the
/// cast go on the stack.
pub(crate) fn finalize_spell_cast(
    state: &mut GameState,
    player: PlayerId,
    object_id: ObjectId,
    is_flashback: bool,
    targets: &[crate::actions::Target],
    registry: &CardRegistry,
) {
    state.events.push(GameEvent::SpellCast {
        player,
        object: object_id,
    });

    *state.num_spells_cast_this_turn.entry(player).or_insert(0) += 1;

    let name = card_name(state, registry, object_id);
    let suffix = if is_flashback { " (flashback)" } else { "" };
    let target_str = if targets.is_empty() {
        String::new()
    } else {
        let names: Vec<String> = targets.iter().map(|t| match t {
            crate::actions::Target::Object(id) => card_name(state, registry, *id),
            crate::actions::Target::Player(pid) => format!("p{}", pid.0),
            // CR 608.2b: a target that stopped being legal is skipped.
            crate::actions::Target::Illegal => "(no longer a legal target)".to_string(),
        }).collect();
        format!(" targeting {}", names.join(", "))
    };
    state.log(LogLevel::Event, format!("p{} cast {}{}{}", player.0, name, suffix, target_str));
    state.consecutive_passes = 0;
}
/// Complete a suspended spell resolution once its choice chain has finished.
///
/// When a spell's `on_resolve` presents a player choice, resolution pauses
/// with `awaiting_action` set and the spell tracked in
/// `state.resolving_spell`. Once no further choice is pending, the engine —
/// not the card — moves the spell off the stack (graveyard, or exile for
/// flashback), per CR 608.2m. Handlers that already moved the spell (or
/// permanents that entered the battlefield) cleared the tracker via
/// `move_object`, making this a no-op.
pub(crate) fn finish_spell_resolution_if_idle(state: &mut GameState, registry: &CardRegistry) {
    if state.awaiting_action.is_some() {
        return;
    }
    if let Some(spell_id) = state.resolving_spell.take() {
        if state.get_object(spell_id).is_some_and(|o| o.zone == Zone::Stack) {
            state.move_spell_after_resolve(spell_id, registry);
        }
    }
}
/// Apply a pending effect from a resolution choice to a target.
pub fn apply_pending_effect(state: &mut GameState, target: &crate::actions::Target, effect: &crate::state::PendingEffect, registry: &CardRegistry) {
    use crate::actions::Target;
    use crate::state::PendingEffect;

    match (target, effect) {
        (Target::Object(found), PendingEffect::FinishLibrarySearch { searcher, destination, tapped }) => {
            crate::cards::helpers::finish_library_search(
                state, *searcher, *found, *destination, *tapped, registry);
        }
        // Card-specific resolution: hand it straight back to the card. The
        // engine deliberately knows nothing about what happens next.
        (_, PendingEffect::CardEffect { source_id, key }) => {
            let card_id = state.get_object(*source_id).map(|o| o.card_id);
            if let Some(behavior) = card_id.and_then(|cid| registry.get(cid)) {
                behavior.resolve_card_effect(state, *source_id, key, target, registry);
            }
        }
        (Target::Object(id), PendingEffect::DealDamage { amount, source_id }) => {
            crate::damage::deal_damage(state, *source_id,
                crate::events::DamageTarget::Object(*id), *amount,
                crate::damage::DamageKind::NonCombat, registry);
        }
        (Target::Player(pid), PendingEffect::DealDamage { amount, source_id }) => {
            crate::damage::deal_damage(state, *source_id,
                crate::events::DamageTarget::Player(*pid), *amount,
                crate::damage::DamageKind::NonCombat, registry);
        }
        (Target::Object(id), PendingEffect::Destroy { source_name } | PendingEffect::DestroyCreature { source_name }) => {
            let name = state.obj_name(*id);
            crate::destruction::try_destroy(state, *id, registry);
            state.log(LogLevel::Event, format!("{source_name} destroyed {name}"));
        }
        (Target::Object(id), PendingEffect::ReturnToBattlefield { spell_id }) => {
            let name = state.obj_name(*id);
            state.move_object(*id, Zone::Battlefield, registry);
            state.log(LogLevel::Event, format!("{name} returned to the battlefield"));
            state.move_spell_after_resolve(*spell_id, registry);
        }
        (Target::Object(id), PendingEffect::AddCounters { count }) => {
            let name = state.obj_name(*id);
            state.add_counters(*id, crate::types::CounterType::PlusOnePlusOne, *count);
            state.log(LogLevel::Event,
                format!("Added {} +1/+1 counter{} to {}", count, if *count > 1 { "s" } else { "" }, name));
        }
        (Target::Object(id), PendingEffect::DebuffUntilEOT { power, toughness, source_name }) => {
            let name = state.obj_name(*id);
            state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
                target: *id,
                power_mod: *power,
                toughness_mod: *toughness,
            });
            state.log(LogLevel::Event, format!("{source_name} gave {name} {power}/{toughness} until end of turn"));
        }
        (Target::Object(id), PendingEffect::CantBlockThisTurn { source_name }) => {
            let name = state.obj_name(*id);
            state.until_end_of_turn.push(crate::state::TemporaryEffect::CantBlock { target: *id });
            state.log(LogLevel::Event, format!("{source_name} prevents {name} from blocking this turn"));
        }
        (Target::Object(id), PendingEffect::ReturnToHand { source_name }) => {
            let name = state.obj_name(*id);
            state.move_object(*id, Zone::Hand, registry);
            state.log(LogLevel::Event, format!("{source_name}: returned {name} to hand"));
        }
        (Target::Object(id), PendingEffect::PutOnTopOfLibrary { source_name }) => {
            let name = state.obj_name(*id);
            state.put_into_library(*id, crate::state::LibraryPosition::Top, registry);
            state.log(LogLevel::Event, format!("{source_name}: put {name} on top of library"));
        }
        (Target::Object(id), PendingEffect::SacrificeCreature { source_name }) => {
            let name = state.obj_name(*id);
            crate::destruction::sacrifice(state, *id, registry);
            state.log(LogLevel::Event, format!("{source_name}: sacrificed {name}"));
        }
        (Target::Object(target_id), PendingEffect::CopyCreature { source_id }) => {
            // The copy applies to the permanent that raised the choice. Killed
            // in response (a printed 0/0 until the copy lands), the card in
            // the graveyard is a new object the choice no longer concerns
            // (CR 400.7) — writing the copy onto it made a permanent copy in
            // the graveyard that a reanimation brought back as the copied
            // creature.
            if !state.get_object(*source_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                if let Some(obj) = state.get_object_mut(*source_id) {
                    obj.entering_copy_source = false;
                }
                return;
            }
            // CR 707.8: copying a transformed permanent copies the face that
            // is up, and the copy shows that face.
            let target_transformed = state.get_object(*target_id).is_some_and(|o| o.is_transformed);
            // Copy the target creature's copiable characteristics onto the
            // source permanent (CR 707.2), including the legendary supertype.
            let (name, power, toughness, card_id, card_types, subtypes, keywords, colors, is_legendary) =
                match state.get_object(*target_id) {
                    Some(o) => {
                        // CR 707.2: only *copiable* values are copied — what is
                        // printed on the card (as modified by other copy
                        // effects), not what non-copy effects have since done to
                        // it. The Evil Twin ruling spells this out: "It doesn't
                        // copy ... any non-copy effects that have changed its
                        // power, toughness, types, color, or so on."
                        //
                        // That distinction is exactly the object-vector /
                        // face-data split: `obj.subtypes` and `obj.colors` hold
                        // runtime grants for a real card — Olivia Voldaren's
                        // "Vampire", Grimoire of the Dead's "Zombie" and black —
                        // and only stand in for printed values on a token, which
                        // has no registry face. Reading the object vectors
                        // directly copied those grants; the `printed_*`
                        // accessors take the face when there is one and fall
                        // back to the object only for a faceless token.
                        let kw = state.printed_keywords_of(o.id, registry);
                        let (power, toughness) = state.printed_pt_of(o.id, registry);
                        // Legendary is copiable (CR 707.2); read the object flag
                        // or fall back to the printed supertype.
                        let legendary = o.is_legendary
                            || state.face_data(o.id, registry)
                                .is_some_and(|d| d.supertypes.contains(&Supertype::Legendary));
                        (state.name_of(o.id, registry), power, toughness, o.card_id,
                         state.printed_card_types_of(o.id, registry),
                         state.printed_subtypes_of(o.id, registry),
                         kw, state.printed_colors_of(o.id, registry), legendary)
                    }
                    None => {
                        // The chosen creature no longer exists (a token that
                        // ceased). The copy never applies — disarm the SBA
                        // copy-guard so the printed 0/0 can die rather than
                        // sitting exempt forever.
                        if let Some(obj) = state.get_object_mut(*source_id) {
                            obj.entering_copy_source = false;
                        }
                        return;
                    }
                };

            // CR 706.2: whatever card's copy effect this is, that card may have
            // added abilities of its own ("except it has ..."). Record it before
            // `card_id` is overwritten — this is the only place the granting
            // card's identity is still known, and the engine never needs to know
            // WHICH card it is.
            let grantor = state.get_object(*source_id).map(|o| o.card_id);

            if let Some(obj) = state.get_object_mut(*source_id) {
                // Setting `card_id` is what makes this object a copy: every
                // characteristics accessor now resolves through the copied
                // card's face. The object-level vectors are only carried over
                // when the *source* had runtime grants of its own (a token's
                // printed types, or a subtype some effect added to it) — they
                // are grants, not a duplicate of the copied face.
                obj.card_id = card_id;
                obj.name.clone_from(&name);
                obj.power = power;
                obj.toughness = toughness;
                obj.keywords = keywords;
                obj.card_types = card_types;
                obj.subtypes = subtypes;
                obj.colors = colors;
                obj.is_legendary = is_legendary;
                obj.copy_grantor = grantor;
                obj.is_transformed = target_transformed;
                // The copy has resolved — disarm the SBA copy-guard so the
                // permanent is once again subject to state-based actions.
                obj.entering_copy_source = false;
            }
            let copy_name = state.get_object(*source_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(LogLevel::Event,
                format!("{copy_name} enters as a copy of {}", state.obj_name(*target_id)));

            // CR 614.12: the permanent enters AS the copy, so the abilities
            // that trigger on it entering are the COPIED creature's. The copy
            // is modelled here as a choice resolving after entry, so those
            // triggers have to be raised now — otherwise copying a creature
            // with an enters-the-battlefield ability silently lost it.
            //
            // This queues the copied card's ETB trigger rather than re-emitting
            // `EnteredBattlefield`: the entering event already happened and
            // every watcher saw it, and firing it twice would double-count for
            // things like Champion of the Parish.
            if let Some(behavior) = registry.get(card_id) {
                if behavior.has_etb_handler() {
                    let etb_kind = crate::cards::TriggerKind::EntersBattlefield;
                    if behavior.should_trigger(state, *source_id, &etb_kind, registry) {
                        let controller = state.get_object(*source_id)
                            .map_or(PlayerId(0), |o| o.controller);
                        state.pending_triggers.push(crate::triggers::PendingTrigger::new(
                            crate::triggers::TriggerSource::new(
                                *source_id, card_id, controller,
                                format!("{copy_name} (copy) enters the battlefield"),
                            ),
                            crate::triggers::TriggerEvent::SelfEntered,
                        ));
                    }
                }
            }
        }
        (Target::Object(target_id), PendingEffect::GrantFlashback { source_name }) => {
            // Grant flashback to the chosen card until end of turn.
            // CR 702.33a: the flashback cost equals the card's mana cost, so
            // a card with none gains no usable flashback. Substituting a free
            // cost made it castable for {0}.
            let fb_info = state.face_data(*target_id, registry).and_then(|d| d.cost.clone());
            if let Some(cost) = fb_info {
                state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantFlashback { target: *target_id, cost });
                state.log(LogLevel::Event,
                    format!("{} grants flashback to {}", source_name, state.obj_name(*target_id)));
            }
        }
        (Target::Object(keep_id), PendingEffect::LegendRuleKeep { player, legend_name }) => {
            // Keep the chosen permanent, move all other legendaries with the same name to graveyard.
            // Through `state.is_legendary`, the same question the SBA asked to
            // raise this choice. Reading `o.is_legendary` here instead meant
            // the two halves could disagree: the choice was offered for a
            // reanimated legend and then removed nothing, because only the
            // ordinary "resolve a permanent spell" path fills that flag in.
            let candidates: Vec<(ObjectId, crate::ids::PlayerId, String)> =
                state.objects_in_id_order().into_iter()
                    .filter(|o| o.zone == crate::types::Zone::Battlefield)
                    .map(|o| (o.id, o.controller, o.name.clone()))
                    .collect();
            let to_remove: Vec<ObjectId> = candidates.into_iter()
                .filter(|(id, controller, name)| controller == player
                    && name == legend_name
                    && id != keep_id
                    && state.is_legendary(*id, registry))
                .map(|(id, _, _)| id)
                .collect();
            for id in to_remove {
                // CR 700.4: put into a graveyard from the battlefield is
                // "dies", legend rule included — morbid and every "whenever a
                // creature dies" watcher used to miss it because this was a
                // bare zone move with no death event. Same capture the
                // state-based zero-toughness death does.
                if let Some(event) = crate::destruction::death_event(state, id, Some(registry)) {
                    state.events.push(event);
                    state.creature_died_this_turn = true;
                }
                state.move_object(id, crate::types::Zone::Graveyard, registry);
            }
            state.log(LogLevel::Event, format!("Legend rule: kept {legend_name}"));
        }
        (chosen_target, PendingEffect::AttachTargetToPendingTrigger) => {
            // CR 603.3d: attach the chosen target to the next pending trigger
            // and push it onto the stack. The trigger was stashed at the front
            // of the AP/NAP queue when the prompt was set up — pop it now.
            let trigger = if !state.pending_trigger_pushes_ap.is_empty() {
                Some(state.pending_trigger_pushes_ap.remove(0))
            } else if !state.pending_trigger_pushes_nap.is_empty() {
                Some(state.pending_trigger_pushes_nap.remove(0))
            } else {
                None
            };
            if let Some(mut t) = trigger {
                t.source.chosen_targets = vec![chosen_target.clone()];
                state.stack.push(crate::state::StackEntry::Trigger(t));
                crate::triggers::log_trigger_pushed(state, registry);
            }
            // Continue processing the remaining pending triggers (may set up
            // another awaiting_action prompt for the next target choice).
            crate::triggers::process_pending_trigger_pushes(state, registry);
        }
        (chosen, PendingEffect::TokenAttacks { token_id, remaining, source_id }) => {
            // CR 508.4b: send a token that entered the battlefield attacking
            // at the player or planeswalker its controller chose.
            let token_name = state.obj_name(*token_id);
            match chosen {
                Target::Player(pid) => {
                    if let Some(combat) = &mut state.combat {
                        combat.attackers.insert(*token_id, *pid);
                    }
                    state.log(LogLevel::Event,
                        format!("{token_name} is attacking p{}", pid.0));
                }
                Target::Object(walker_id) => {
                    // Attacking a planeswalker: the attacker still defends
                    // against the walker's controller (CR 508.1a), and the
                    // walker is recorded in `planeswalker_defenders`, which is
                    // where the combat damage step looks.
                    let walker_controller = state.get_object(*walker_id).map(|o| o.controller);
                    let walker_name = state.obj_name(*walker_id);
                    if let (Some(wc), Some(combat)) = (walker_controller, state.combat.as_mut()) {
                        combat.attackers.insert(*token_id, wc);
                        combat.planeswalker_defenders.insert(*token_id, *walker_id);
                    }
                    state.log(LogLevel::Event,
                        format!("{token_name} is attacking {walker_name}"));
                }
                Target::Illegal => {}
            }
            // One choice per token (the rulings' "each token"): raise the
            // next. No player receives priority between these prompts, so the
            // option list cannot have changed, but it is recomputed rather
            // than carried. A single-option board auto-applies straight back
            // into this arm, so the whole chain runs silently there.
            if let Some((&next, rest)) = remaining.split_first() {
                if let Some(controller) = state.get_object(next).map(|o| o.controller) {
                    let options =
                        crate::cards::helpers::token_attack_options(state, controller, registry);
                    let source_name = state.obj_name(*source_id);
                    crate::cards::helpers::present_target_choice(
                        state,
                        *source_id,
                        controller,
                        options,
                        crate::state::PendingEffect::TokenAttacks {
                            token_id: next,
                            remaining: rest.to_vec(),
                            source_id: *source_id,
                        },
                        &format!("{source_name}: choose which player or planeswalker the token is attacking"),
                        false,
                        registry,
                    );
                }
            }
        }
        _ => {}
    }
}
