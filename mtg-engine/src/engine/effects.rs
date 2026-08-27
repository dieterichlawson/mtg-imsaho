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
        (Target::Object(id), PendingEffect::DealDamage { amount, source_id, source_name: _ }) => {
            crate::damage::deal_damage(state, *source_id,
                crate::events::DamageTarget::Object(*id), *amount,
                crate::damage::DamageKind::NonCombat, registry);
        }
        (Target::Player(pid), PendingEffect::DealDamage { amount, source_id, source_name: _ }) => {
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
            let owner = state.get_object(*id).map_or(crate::ids::PlayerId(0), |o| o.owner);
            state.move_object(*id, Zone::Library, registry);
            // Insert at position 0 (top of library).
            state.get_player_mut(owner).library_order.insert(0, *id);
            state.log(LogLevel::Event, format!("{source_name}: put {name} on top of library"));
        }
        (Target::Object(id), PendingEffect::SacrificeCreature { source_name }) => {
            let name = state.obj_name(*id);
            crate::destruction::sacrifice(state, *id, registry);
            state.log(LogLevel::Event, format!("{source_name}: sacrificed {name}"));
        }
        (Target::Object(target_id), PendingEffect::CopyCreature { source_id }) => {
            // Copy the target creature's copiable characteristics onto the
            // source permanent (CR 707.2), including the legendary supertype.
            let (name, power, toughness, card_id, card_types, subtypes, keywords, colors, is_legendary) =
                match state.get_object(*target_id) {
                    Some(o) => {
                        // A generic token has no registry face, so its
                        // printed keywords live on the object. Reading only the
                        // face dropped them — a copy of a 1/1 Spirit token lost
                        // its flying.
                        let kw = state.printed_keywords_of(o.id, registry);
                        // Legendary is copiable (CR 707.2); read the object flag
                        // or fall back to the printed supertype.
                        let legendary = o.is_legendary
                            || state.face_data(o.id, registry)
                                .is_some_and(|d| d.supertypes.contains(&Supertype::Legendary));
                        (o.name.clone(), o.power, o.toughness, o.card_id,
                         o.card_types.clone(), o.subtypes.clone(), kw, o.colors.clone(), legendary)
                    }
                    None => return,
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
            let to_remove: Vec<ObjectId> = state.objects.values()
                .filter(|o| o.zone == crate::types::Zone::Battlefield
                    && o.controller == *player
                    && o.is_legendary
                    && o.name == *legend_name
                    && o.id != *keep_id)
                .map(|o| o.id)
                .collect();
            for id in to_remove {
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
            }
            // Continue processing the remaining pending triggers (may set up
            // another awaiting_action prompt for the next target choice).
            crate::triggers::process_pending_trigger_pushes(state, registry);
        }
        _ => {}
    }
}
