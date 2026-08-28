use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::state::{GameState, LogLevel, StackEntry};
use crate::types::Zone;

/// Check if a target is still legal at resolution time.
pub(crate) fn is_target_legal(state: &GameState, target: &Target, target_req: &crate::cards::TargetRequirement, caster: crate::ids::PlayerId, source_id: Option<crate::ids::ObjectId>, registry: &crate::cards::CardRegistry) -> bool {
    use crate::cards::TargetRequirement;

    // ModalChoice: legal if legal under any mode.
    if let TargetRequirement::ModalChoice(ref modes) = target_req {
        return modes.iter().any(|mode_req| is_target_legal(state, target, mode_req, caster, source_id, registry));
    }

    // TwoTargets: the same shape. This is asked one target at a time, with no
    // way to know which slot the target came from, so it used to unwrap to the
    // FIRST slot and judge everything against that. For Into the Maw of Hell —
    // "Destroy target land. ... deals 13 damage to target creature" — that
    // meant the creature was tested against `PermanentWithFilter(Land)` and
    // could never count as legal. Whenever the land target became illegal,
    // `any_legal` was false and the whole spell was countered, against its own
    // ruling: "If one of Into the Maw of Hell's targets is illegal by the time
    // it resolves, Into the Maw of Hell will still affect the remaining legal
    // target."
    //
    // Which slot a target belongs to was settled when the spell was cast
    // (CR 601.2c); what CR 608.2b re-checks is whether the target is still
    // there and still targetable. So: legal under either slot.
    if let TargetRequirement::TwoTargets(ref first, ref second) = target_req {
        return is_target_legal(state, target, first, caster, source_id, registry)
            || is_target_legal(state, target, second, caster, source_id, registry);
    }

    // Unwrap nested requirements (UpToTargets — every target shares the inner
    // requirement, so unwrapping is exact there).
    let inner_req = match target_req {
        TargetRequirement::UpToTargets(_, inner) => inner.as_ref(),
        other => other,
    };
    match target {
        Target::Object(id) => {
            match state.get_object(*id) {
                Some(obj) => {
                    // Check zone legality.
                    // CR 109.1: every one of these says "card", and a token
                    // sits in a graveyard until the next state-based action
                    // pass — so it can be there when the target is chosen and
                    // still be there when the spell resolves.
                    let zone_ok = match inner_req {
                        TargetRequirement::GraveyardCard
                        | TargetRequirement::GraveyardCreature
                        | TargetRequirement::GraveyardCreatureOfSubtype(_)
                        | TargetRequirement::GraveyardCardOwnedByCaster
                        | TargetRequirement::GraveyardCardOwnedByOpponent
                        // Memory's Journey's card slot. Missing from this list,
                        // it fell to the battlefield-or-stack arm below and so
                        // called every legal graveyard card illegal. No
                        // observable difference today — the requirement is the
                        // second half of a `TwoTargets` whose first half is a
                        // player, so `any_legal` is satisfied by the player
                        // either way, and the card's own guard is what skips a
                        // card that has left the graveyard. Listed here because
                        // the table is meant to say which zone each requirement
                        // reads, not to be right by accident.
                        | TargetRequirement::GraveyardCardOwnedByTargetPlayer =>
                            obj.zone == Zone::Graveyard && state.is_card(*id),
                        TargetRequirement::ExileCard =>
                            obj.zone == Zone::Exile && state.is_card(*id),
                        _ => obj.zone == Zone::Battlefield || obj.zone == Zone::Stack,
                    };
                    if !zone_ok { return false; }

                    // Check hexproof: opponent's creature with hexproof can't be targeted.
                    if obj.zone == Zone::Battlefield && obj.controller != caster
                        && state.has_keyword(*id, crate::types::Keyword::Hexproof, registry) {
                        return false;
                    }

                    // "Target **creature**" has to still be one (CR 608.2b).
                    // The filter below carries the rest of the restriction —
                    // "you control", "power 4 or greater" — but never
                    // creature-ness, so this was the one part of
                    // `CreatureWithFilter` that the re-check did not re-check.
                    // Seven cards each put it back in their own
                    // `is_valid_target`; four of them are there for a further
                    // restriction anyway, but the preamble was doing this job.
                    //
                    // Bare `Creature` asks exactly the same question and was
                    // left out of it, so "target creature" with no further
                    // restriction was the one wording the re-check took on
                    // trust.
                    if matches!(inner_req, TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_))
                        && !state.is_creature(*id, registry)
                    {
                        return false;
                    }

                    // The graveyard requirements say more than a zone, and the
                    // table above says only the zone. "Return target creature
                    // card from **your** graveyard" (Unburial Rites) and
                    // "return target **Zombie** creature card from your
                    // graveyard" (Ghoulcaller's Chant) were re-checked as
                    // "some card in some graveyard" — so a target the engine
                    // would never have offered, an opponent's creature card,
                    // survived the re-check and was reanimated. Each clause is
                    // the one `targeting.rs` generates against.
                    let graveyard_ok = match inner_req {
                        TargetRequirement::GraveyardCard => true,
                        TargetRequirement::GraveyardCreature =>
                            obj.owner == caster && state.is_creature(*id, registry),
                        TargetRequirement::GraveyardCreatureOfSubtype(subtype) =>
                            obj.owner == caster
                                && state.is_creature(*id, registry)
                                && state.has_subtype(*id, subtype, registry),
                        TargetRequirement::GraveyardCardOwnedByCaster => obj.owner == caster,
                        TargetRequirement::GraveyardCardOwnedByOpponent => obj.owner != caster,
                        // Whose graveyard is named by this spell's *other*
                        // target (Memory's Journey), which this function is
                        // asked one target at a time and cannot see. The card
                        // checks it on resolution.
                        TargetRequirement::GraveyardCardOwnedByTargetPlayer => true,
                        _ => true,
                    };
                    if !graveyard_ok { return false; }

                    // Check TargetFilter for requirements that carry one.
                    let filter = match inner_req {
                        TargetRequirement::CreatureWithFilter(f) | TargetRequirement::PermanentWithFilter(f) => Some(f),
                        _ => None,
                    };
                    if let Some(filter) = filter {
                        // Re-run the full canonical filter check: a target
                        // whose characteristics changed in response (e.g. a
                        // creature that became black vs a Nonblack filter)
                        // is no longer legal (CR 608.2b).
                        if !crate::engine::matches_target_filter(state, obj, filter, caster, source_id, registry) {
                            return false;
                        }
                    }

                    true
                }
                None => false,
            }
        }
        // CR 608.2b: a target that stopped being legal is skipped.
        Target::Illegal => false,
        // CR 608.2b re-checks legality with the same rule that offered the
        // target in the first place, so this calls the one function rather
        // than restating it — plus the requirement's own restriction, which
        // `can_target_player` does not know about.
        Target::Player(pid) => {
            if matches!(inner_req, TargetRequirement::OpponentOnly) && *pid == caster {
                return false;
            }
            crate::engine::can_target_player(state, *pid, caster, registry)
        }
    }
}

/// Resolve the top item on the stack (spell or trigger).
///
/// For spells: checks target legality (CR 608.2b fizzle), calls `on_resolve`.
/// For triggers: delegates to `triggers::resolve_next_trigger`.
pub fn resolve_top_of_stack(state: &mut GameState, registry: &CardRegistry) {
    let entry = match state.stack.last() {
        Some(e) => e.clone(),
        None => return,
    };

    match entry {
        StackEntry::Trigger(_) => {
            // Trigger resolution is handled by the triggers module.
            crate::triggers::resolve_next_trigger(state, registry);
        }
        StackEntry::Spell(object_id) => {
            state.stack.pop(); // Remove the spell from the stack.
            resolve_spell(state, registry, object_id);
        }
        StackEntry::Ability { source_id, ability_index, behavior_card_id, targets, x_value, activator, target_requirement, sacrificed } => {
            state.stack.pop();
            state.last_activated_x_value = x_value;
            state.last_activated_sacrifice = sacrificed;
            let name = state.name_of(source_id, registry);

            // CR 608.2b applies to abilities as well as spells, and this path
            // used to skip the check entirely — an activated ability resolved
            // against whatever it had targeted however the board had changed.
            // Ghost Quarter's ruling is the plain statement of it: "If the
            // targeted land is an illegal target by the time Ghost Quarter's
            // ability resolves, it won't resolve and none of its effects will
            // happen. The land's controller won't get to search for a basic
            // land card."
            //
            // Two ways a target stops being legal: it can stop being targetable
            // at all (hexproof, protection), and it can stop satisfying what the
            // ability asks of it — Avacynian Priest's "target non-Human
            // creature" is not a legal target once it has become a Human. The
            // second is the card's own `is_valid_target`, and the behavior to
            // ask is the one that *granted* the ability, which is why
            // `behavior_card_id` rides on the stack entry.
            // CR 602.2a: the ability's controller is the player who activated
            // it, recorded on the stack entry. Re-reading the source's
            // `controller` here handed the ability to whoever had taken the
            // source in response.
            let controller = activator;
            let behavior = registry.get(behavior_card_id);
            // Three ways a target stops being legal, and the entry now
            // carries what it needs for all three: it can stop being
            // targetable at all (hexproof, protection); it can stop
            // satisfying the requirement the ability declared, which rides on
            // the stack entry because the source may be gone by now; and it
            // can stop satisfying whatever the card additionally restated in
            // `is_valid_target`. Only the first and third were checked, so an
            // ability whose card restated nothing resolved against a target
            // that no longer matched its own wording.
            let targets: Vec<Target> = targets.into_iter()
                .map(|t| match t {
                    Target::Object(id)
                        if !crate::engine::can_be_targeted_by(state, id, controller, Some(source_id), registry)
                            || !target_requirement.as_ref().is_none_or(|req|
                                    is_target_legal(state, &Target::Object(id), req, controller, Some(source_id), registry))
                            || !behavior.is_none_or(|b| b.is_valid_target(state, controller, &Target::Object(id), registry)) =>
                            Target::Illegal,
                    other => other,
                })
                .collect();
            if !targets.is_empty() && targets.iter().all(|t| matches!(t, Target::Illegal)) {
                state.log(LogLevel::Event,
                    format!("{name} ability fizzled (all targets illegal)"));
                return;
            }

            state.log(LogLevel::Event, format!("{name} ability resolved"));
            if let Some(behavior) = registry.get(behavior_card_id) {
                state.resolving_ability_activator = Some(activator);
                behavior.resolve_activated_ability(state, source_id, ability_index, &targets, registry);
                // Held across a choice the ability raised: the rest of the
                // effect happens when that choice is answered, and it is still
                // this ability's effect, so its "you" is still the activator.
                // `choices.rs` clears it when the chain runs out.
                if state.awaiting_action.is_none() {
                    state.resolving_ability_activator = None;
                }
            }
        }
    }
}

/// Resolve a spell from the stack.
fn resolve_spell(state: &mut GameState, registry: &CardRegistry, object_id: crate::ids::ObjectId) {
    let (card_id, mut targets, caster) = match state.get_object(object_id) {
        Some(obj) => (obj.card_id, obj.targets.clone(), obj.controller),
        None => return,
    };

    // CR 608.2b: Check target legality. If the spell has targets and ALL
    // are illegal, it's countered by game rules (fizzled).
    // This now checks hexproof at resolution time (not just at cast time).
    let target_req = registry.get(card_id)
        .map_or(crate::cards::TargetRequirement::None, super::cards::CardBehavior::target_requirement);
    if !targets.is_empty() {
        let behavior = registry.get(card_id);
        let legal = |t: &Target| {
            if !is_target_legal(state, t, &target_req, caster, Some(object_id), registry) {
                return false;
            }
            // Also re-check card-specific validity (e.g., "power 4 or greater").
            if let Some(b) = behavior {
                b.is_valid_target(state, caster, t, registry)
            } else {
                true
            }
        };
        let any_legal = targets.iter().any(&legal);
        // CR 608.2b: a target that is no longer legal is not affected, but the
        // spell still resolves and still affects its other targets. Substitute
        // rather than remove, so a multi-target card's positions hold — "the
        // land is targets[0]" stays true — and the illegal one simply fails to
        // match `Target::Object(..)`.
        //
        // Only checking `any_legal`, as this used to, meant a target that had
        // become illegal *without leaving the battlefield* was still affected:
        // Into the Maw of Hell dealt its 13 damage to a creature that had
        // gained hexproof in response (Ranger's Guile is in this very set).
        //
        // Scoped to targeting restrictions — hexproof (CR 702.11b) and
        // protection (CR 702.16b) — because those are properties of the target
        // alone. Whether a target still satisfies its *requirement* cannot be
        // asked per target here: `is_target_legal` unwraps only the first
        // branch of a composite requirement, so Memory's Journey's graveyard
        // cards would be judged against its `PlayerOnly` first slot. A target
        // that has changed zones is already skipped by each card's own guard.
        targets = targets.into_iter()
            .map(|t| match t {
                Target::Object(id)
                    if !crate::engine::can_be_targeted_by(state, id, caster, Some(object_id), registry) =>
                        Target::Illegal,
                other => other,
            })
            .collect();
        if !any_legal {
            state.log(LogLevel::Event, format!("{} fizzled (all targets illegal)", state.obj_name(object_id)));
            // Move to graveyard (or exile for flashback) without resolving.
            state.move_spell_after_resolve(object_id, registry);
            return;
        }
    }

    // Spell resolves normally.
    state.log(LogLevel::Event, format!("{} resolved", state.obj_name(object_id)));
    state.events.push(GameEvent::SpellResolved { object: object_id });

    // Track the spell so the ENGINE owns its post-resolution cleanup, even
    // when resolution is suspended on a player choice.
    state.resolving_spell = Some(object_id);

    // Call the card's on_resolve behavior with targets.
    if let Some(behavior) = registry.get(card_id) {
        behavior.on_resolve(state, object_id, &targets, registry);
    }

    // If the card set an awaiting_action, it's mid-resolution (e.g., Unburial
    // Rites waiting for player to choose a creature). Don't clean up yet —
    // `engine::finish_spell_resolution_if_idle` moves the spell once the
    // choice chain completes (CR 608.2m: graveyard as the final step).
    if state.awaiting_action.is_some() {
        return;
    }

    // If the card is still on the stack after resolution, move it to the
    // appropriate zone. Flashback spells go to exile; others to graveyard.
    state.resolving_spell = None;
    if let Some(obj) = state.get_object(object_id) {
        if obj.zone == Zone::Stack {
            state.move_spell_after_resolve(object_id, registry);
        }
    }
}
