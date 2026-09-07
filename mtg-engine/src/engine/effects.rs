use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, LogLevel};
use crate::types::Zone;
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
    payment: &CastPayment,
    targets: &[crate::actions::Target],
    registry: &CardRegistry,
) {
    state.events.push(GameEvent::SpellCast {
        player,
        object: object_id,
    });

    *state.num_spells_cast_this_turn.entry(player).or_insert(0) += 1;

    let name = card_name(state, registry, object_id);
    let suffix = payment.annotation();
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

/// Which cost a spell was actually cast for, for the one line the log writes
/// about the cast.
///
/// Flashback was annotated and every other alternative cost was not, so a
/// Rooftop Storm free cast read exactly like a paid one — the only difference
/// in the log was the *absence* of `p0 tapped <land> for mana` lines, which is
/// no signal at all when the caster had floating mana (issue #264). A cost
/// reduction had the same weakness in a milder form: the reader had to count
/// the tap lines to notice the {2} came off.
#[derive(Debug, Default)]
pub(crate) struct CastPayment<'a> {
    /// Cast from the graveyard for its flashback cost (CR 702.34a), itself an
    /// alternative cost — kept separate because it has a name players use.
    pub is_flashback: bool,
    /// The alternative cost that replaced the mana cost (CR 601.2b, 118.9).
    pub alternative: Option<&'a crate::types::ManaCost>,
    /// The printed mana cost, and what was actually paid, when a cost
    /// reduction (CR 601.2f) made the two differ.
    pub printed: Option<&'a crate::types::ManaCost>,
    pub paid: Option<&'a crate::types::ManaCost>,
    /// The announced value of X (CR 601.2b), when the spell had one.
    pub x: Option<u32>,
    /// Cast from the owner's graveyard under the card's own permission
    /// (CR 601.3a) — Skaab Ruinator. Recorded because it is otherwise
    /// indistinguishable from a hand cast of the same card, and the two are
    /// materially different: they exile different cards (issue #300).
    pub from_graveyard: bool,
}

impl CastPayment<'_> {
    /// The parenthetical the cast line carries: `" (flashback)"`,
    /// `" (alternative cost {0})"`, `" (paid {3}{U}, reduced from {5}{U})"`,
    /// `" (X=3)"` — or nothing, for a plain cast at the printed cost.
    fn annotation(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.is_flashback {
            parts.push("flashback".to_string());
        } else if let Some(alt) = self.alternative {
            // CR 118.9 distinguishes paying {0} from casting without paying
            // the mana cost, so print the cost rather than a phrase: Rooftop
            // Storm's is literally "you may pay {0}".
            let rendered = alt.to_string();
            parts.push(if rendered.is_empty() {
                "alternative cost {0}".to_string()
            } else {
                format!("alternative cost {rendered}")
            });
        } else if let (Some(printed), Some(paid)) = (self.printed, self.paid) {
            if printed != paid {
                parts.push(format!("paid {paid}, reduced from {printed}"));
            }
        }
        if self.from_graveyard {
            parts.push("from graveyard".to_string());
        }
        if let Some(x) = self.x {
            parts.push(format!("X={x}"));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" ({})", parts.join(", "))
        }
    }
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
            crate::destruction::sacrifice_by(state, *id, &format!("to {source_name}"), registry);
        }
        (Target::Object(target_id), PendingEffect::EnterAsCopy { object }) => {
            // CR 614.12b: the answer to "you may have this enter as a copy
            // of any creature on the battlefield". The permanent is not on
            // the battlefield yet — recording the answer is what lets it
            // finish entering, and its own replacement effect turns the
            // answer into `copy_of` on the way in. Everything the copy is
            // (its copiable values, the ETB triggers it enters with, whether
            // it enters tapped) therefore happens as part of entering, with
            // no window in between.
            //
            // A chosen creature that is gone by now (a token that ceased) is
            // no longer a legal answer, so the permanent enters as itself.
            let choice = if state.get_object(*target_id)
                .is_some_and(|o| o.zone == crate::types::Zone::Battlefield)
            {
                crate::state::EnterAsCopyChoice::Copy(*target_id)
            } else {
                crate::state::EnterAsCopyChoice::Declined
            };
            crate::replacement::record_entry_choice(state, *object, choice, registry);
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
                    .map(|o| (o.id, o.controller, state.name_of(o.id, registry)))
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
                        // A declared attacker gets its blocker list at
                        // declaration; a token that enters attacking needs
                        // one too, or every block against it is silently
                        // dropped (found by fuzzing: 14 Cagebreakers wolves,
                        // three blocks, none recorded).
                        combat.blocker_assignments.entry(*token_id).or_default();
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
                        // A declared attacker gets its blocker list at
                        // declaration; a token that enters attacking needs
                        // one too, or every block against it is silently
                        // dropped (found by fuzzing: 14 Cagebreakers wolves,
                        // three blocks, none recorded).
                        combat.blocker_assignments.entry(*token_id).or_default();
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
