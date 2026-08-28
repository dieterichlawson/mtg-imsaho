//! Shared test helpers.
//!
//! Each integration test file compiles as a separate binary, so helpers
//! only used by *other* test files appear dead in each binary. This is a
//! known Rust issue (rust-lang/rust#46379) with no upstream fix.
#![allow(dead_code, unused_imports)]

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
// Re-exported: `use common::*` is already how tests reach P0/P1 and the
// helpers below, so the id types come with them.
pub use mtg_engine::ids::{CardId, ObjectId, PlayerId};
pub use mtg_engine::state::GameState;
use mtg_engine::types::*;

pub const P0: PlayerId = PlayerId(0);
pub const P1: PlayerId = PlayerId(1);

/// The full card registry.
///
/// This was defined identically in 89 test files. A test that needs a
/// registry with an extra card registered (see `player_protection.rs`) still
/// builds its own — that is a different thing, not a copy of this one.
pub fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Set up a game state at a specific step with the given player as active and having priority.
pub fn game_at_step(step: Step, active: PlayerId) -> GameState {
    let mut state = GameState::new(2);
    state.step = step;
    state.active_player = active;
    state.priority_player = Some(active);
    state.is_first_turn = false;
    state.players[0].life = 20;
    state.players[1].life = 20;
    state
}

/// Place a creature on the battlefield that is ready to act (no summoning sickness).
pub fn ready_creature(state: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    let id = state.create_object(CardId(9999), owner, Zone::Battlefield, Some(power), Some(toughness));
    state.get_object_mut(id).unwrap().summoning_sick = false;
    id
}

/// Grant `keyword` to `id` until end of turn.
///
/// The way to do this, and not obvious: `state.has_keyword` deliberately does
/// *not* consult `obj.keywords` for a card with a registry entry — keywords
/// have an effects layer (`ContinuousEffect::GrantKeyword` and
/// `TemporaryEffect`) and nothing grants one by writing the object vector, so
/// unioning it in would resurrect a stale front-face keyword on a transformed
/// DFC. Pushing to `obj.keywords` therefore works on an anonymous creature
/// from [`ready_creature`] (no registry entry, so the vector *is* its printed
/// keywords) and is silently ignored on a real card — a test that grants
/// indestructible to a Forest that way then passes for the wrong reason.
pub fn grant_keyword(state: &mut GameState, id: ObjectId, keyword: Keyword) {
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: id,
        keyword,
    });
}

/// Place a creature on the battlefield with summoning sickness.
pub fn sick_creature(state: &mut GameState, owner: PlayerId, power: i32, toughness: i32) -> ObjectId {
    state.create_object(CardId(9999), owner, Zone::Battlefield, Some(power), Some(toughness))
}

/// Put a named card into a player's hand. Returns the object ID.
pub fn spell_in_hand(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) -> ObjectId {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let data = registry.card_data(card_id);
    let power = data.as_ref().and_then(|d| d.power);
    let toughness = data.as_ref().and_then(|d| d.toughness);
    let id = state.create_object(card_id, player, Zone::Hand, power, toughness);
    state.get_object_mut(id).unwrap().name = name.into();
    id
}

/// Add exactly enough mana to a player's pool to pay for a card by name.
pub fn add_mana_for(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let data = registry.card_data(card_id)
        .unwrap_or_else(|| panic!("No card data for: {name}"));
    if let Some(ref cost) = data.cost {
        for sym in &cost.symbols {
            match sym {
                ManaSymbol::Colored(c) => {
                    let mana_type = match c {
                        Color::White => ManaType::White,
                        Color::Blue => ManaType::Blue,
                        Color::Black => ManaType::Black,
                        Color::Red => ManaType::Red,
                        Color::Green => ManaType::Green,
                    };
                    state.get_player_mut(player).mana_pool.add(mana_type, 1);
                }
                ManaSymbol::Generic(n) => {
                    state.get_player_mut(player).mana_pool.add(ManaType::Colorless, *n);
                }
                _ => {}
            }
        }
    }
}

/// Put a named card in hand and add mana to cast it. Returns the object ID.
pub fn castable_spell(state: &mut GameState, registry: &CardRegistry, name: &str, player: PlayerId) -> ObjectId {
    let id = spell_in_hand(state, registry, name, player);
    add_mana_for(state, registry, name, player);
    id
}

/// Cast a spell targeting something, then resolve the top of the stack.
/// Returns the new state after resolution. For X-cost spells, funds X to
/// the max (taps every offered source + drains pool). Tests wanting a
/// specific X value should construct the `FundingResponse` themselves
/// instead of calling this helper.
/// Set a planeswalker's loyalty to exactly `n`.
///
/// [`named_permanent`] gives it the starting loyalty its card prints, so a test
/// that wants a different number has to replace it rather than add to it —
/// 31 sites used to `add_counters(.., Loyalty, 3)` on top of a Garruk that was
/// already on 3, and the one test keying on the exact value was the only one
/// that noticed.
pub fn set_loyalty(state: &mut GameState, id: ObjectId, n: u32) {
    state.get_object_mut(id).expect("planeswalker exists")
        .counters.insert(CounterType::Loyalty, n);
}

/// Add mana of each listed kind to `player`'s pool.
pub fn add_mana(state: &mut GameState, player: PlayerId, mana: &[(ManaType, u32)]) {
    for &(kind, n) in mana {
        state.get_player_mut(player).mana_pool.add(kind, n);
    }
}

/// Activate the one activated ability the engine currently offers.
///
/// Asserts there is exactly one, which is the point: "find the first
/// `ActivateAbility` in `legal_actions`" — the hand-rolled form, in a dozen
/// tests — silently picks an arbitrary ability once a second one is offered,
/// and the assertion that follows then measures the wrong thing.
pub fn activate_only_offered_ability(state: &GameState, registry: &CardRegistry) -> GameState {
    let legal = mtg_engine::engine::legal_actions(state, registry);
    let offered: Vec<&Action> = legal.actions.iter()
        .filter(|a| matches!(a, Action::ActivateAbility { .. }))
        .collect();
    assert_eq!(offered.len(), 1,
        "expected exactly one activated ability on offer, got {offered:?}");
    let after = mtg_engine::engine::submit_action(state, offered[0], registry);
    resolve_activated(after, registry)
}

/// Activate the ability the engine offers for `object_id` and stop at the
/// stack, without resolving it (CR 602.2a).
///
/// [`activate_offered`] is the usual helper; this one is for tests that need
/// the window in between — respond to the ability, or make a target illegal
/// (CR 608.2b).
pub fn activate_onto_stack(
    state: &GameState,
    registry: &CardRegistry,
    object_id: ObjectId,
    target: Option<Target>,
) -> GameState {
    let legal = mtg_engine::engine::legal_actions(state, registry);
    let action = legal.actions.iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id: o, targets, .. }
            if *o == object_id && target.as_ref().is_none_or(|t| targets.contains(t))))
        .unwrap_or_else(|| panic!("no activated ability offered for {object_id:?} at {target:?}"));
    mtg_engine::engine::submit_action(state, action, registry)
}

/// Activate `object_id`'s `ability_index`-th ability without going through
/// `legal_actions` — pay whatever cost the card declares beyond its
/// `ActivatedAbilityDef`, then put the ability on the stack (CR 602.2a).
///
/// This is the pair of hooks the engine drives; use it where a test needs to
/// set up a board `legal_actions` would not offer the ability from, and
/// [`activate`] everywhere else. Stops at the stack: the caller resolves.
pub fn activate_via_hooks(
    state: &mut GameState,
    registry: &CardRegistry,
    object_id: ObjectId,
    ability_index: usize,
    targets: &[Target],
) {
    let Some(card_id) = state.get_object(object_id).map(|o| o.card_id) else { return };
    let mut target_requirement = None;
    if let Some(behavior) = registry.get(card_id) {
        // The costs the ability declares, paid the way `submit_action` pays
        // them — before the ability goes on the stack (CR 601.2h via 602.2b).
        // This helper used to pay only `pay_activation_cost`, the card-level
        // hook, so an ability whose cost is declared rather than hand-written
        // (a tap, `counter_cost`) went on the stack for free and every test
        // through this path measured the effect without the cost.
        //
        // Mana is the exception: it comes from a pool these tests do not fill.
        if let Some(ab) = behavior.activated_abilities(state, object_id, registry)
            .into_iter().find(|a| a.ability_index == ability_index)
        {
            // Read before the cost is paid, for the same reason the engine
            // does: a `SacrificeThis` takes the source's ability list with it.
            target_requirement = ab.target_requirement.clone();
            if ab.requires_tap {
                if let Some(obj) = state.get_object_mut(object_id) { obj.tapped = true; }
            }
            if let Some((counter_type, amount)) = ab.counter_cost {
                state.remove_counters(object_id, counter_type, amount);
            }
            // "Sacrifice this permanent:" is part of the cost too, and leaving
            // it unpaid is not a small difference: Full Moon's Rise kept
            // granting its Werewolves +1/+0 and trample after the ability that
            // is supposed to cost you the enchantment had resolved.
            //
            // The variants that sacrifice *some other* creature are the
            // player's choice, which `legal_actions` enumerates and this
            // helper has no way to make — those go through `activate`.
            match ab.sacrifice_cost {
                mtg_engine::cards::SacrificeCost::SacrificeThis => {
                    mtg_engine::destruction::sacrifice(state, object_id, registry);
                }
                mtg_engine::cards::SacrificeCost::None => {}
                mtg_engine::cards::SacrificeCost::SacrificeCreature
                | mtg_engine::cards::SacrificeCost::SacrificeAnotherCreature => panic!(
                    "activate_via_hooks cannot choose which creature to sacrifice; \
                     use `activate`, which picks from what legal_actions offers"),
            }
        }
        behavior.pay_activation_cost(state, object_id, ability_index, targets, registry);
    }
    mtg_engine::cards::push_ability(state, object_id, ability_index, card_id, targets, target_requirement);
}

/// Activating an ability only puts it on the stack (CR 602.2a); it resolves
/// when every player has passed. A test that wants the ability's *effect*
/// wants both halves, so the activate helpers do both — a test about the
/// response window in between activates through `submit_action` directly.
pub fn resolve_activated(mut state: GameState, registry: &CardRegistry) -> GameState {
    if matches!(state.stack.last(), Some(mtg_engine::state::StackEntry::Ability { .. })) {
        mtg_engine::stack::resolve_top_of_stack(&mut state, registry);
    }
    state
}

/// Activate the ability the engine offers for `object_id`, at `target` if given.
///
/// Asserts one is offered: the hand-rolled form — find it in `legal_actions`,
/// `assert!(x.is_some())`, `submit_action(x.unwrap())` — appeared a dozen times
/// and says the same thing in four lines.
pub fn activate_offered(
    state: &GameState,
    registry: &CardRegistry,
    object_id: ObjectId,
    target: Option<Target>,
) -> GameState {
    let legal = mtg_engine::engine::legal_actions(state, registry);
    let action = legal.actions.iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id: o, targets, .. }
            if *o == object_id && target.as_ref().is_none_or(|t| targets.contains(t))))
        .unwrap_or_else(|| panic!(
            "no activated ability offered for {object_id:?} at {target:?}; offered {:?}",
            legal.actions.iter().filter(|a| matches!(a, Action::ActivateAbility { .. })).collect::<Vec<_>>()));
    let after = mtg_engine::engine::submit_action(state, action, registry);
    resolve_activated(after, registry)
}

/// Whether the engine currently offers any activated ability of `object_id`.
pub fn offers_ability_of(state: &GameState, registry: &CardRegistry, object_id: ObjectId) -> bool {
    mtg_engine::engine::legal_actions(state, registry).actions.iter()
        .any(|a| matches!(a, Action::ActivateAbility { object_id: o, .. } if *o == object_id))
}

/// The `CastSpell` action for `object_id` at `targets`, unsubmitted.
///
/// For the `run_game_loop` tests, whose callbacks return an action rather than
/// applying one, so [`cast_onto_stack`] does not fit.
pub fn cast_action(object_id: ObjectId, targets: Vec<Target>) -> Action {
    Action::CastSpell {
        object_id, targets,
        sacrifice: None, exile_count: None, exile_ids: vec![],
        alternative_cost: None, tap_plan: vec![],
    }
}

/// The options of the resolution choice the game is currently waiting on.
///
/// Panics if it is not waiting on a `ChooseTarget`, which is the point: a test
/// that reaches for the options wants them to exist, and matching with a
/// fallback to `false`/`vec![]` turns "the prompt never appeared" into a quiet
/// pass.
pub fn pending_choice_options(state: &GameState) -> Vec<Target> {
    match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => options.clone(),
        other => panic!("expected a ChooseTarget prompt, got {other:?}"),
    }
}

/// Every *set* of targets the engine offers for casting `spell`, grouped as
/// each cast action names them.
///
/// [`offered_targets`] flattens these, which loses the shape — for a modal or
/// multi-target spell the grouping is the thing under test.
pub fn offered_target_sets(state: &GameState, registry: &CardRegistry, spell: ObjectId) -> Vec<Vec<Target>> {
    mtg_engine::engine::legal_actions(state, registry).actions.iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if *object_id == spell => Some(targets.clone()),
            _ => None,
        })
        .collect()
}

/// Every target the engine currently offers for casting `spell`.
///
/// Scoped to `spell`: the hand-rolled version of this asked whether *any*
/// castable spell named the target, which is only the same question when
/// exactly one spell is castable.
pub fn offered_targets(state: &GameState, registry: &CardRegistry, spell: ObjectId) -> Vec<Target> {
    mtg_engine::engine::legal_actions(state, registry).actions.iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if *object_id == spell => Some(targets.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Tap the first untapped permanent named `name` for mana, through
/// `legal_actions` so the engine's own offer is what gets used.
pub fn tap_for_mana(state: &GameState, registry: &CardRegistry, name: &str) -> GameState {
    let legal = mtg_engine::engine::legal_actions(state, registry);
    let action = legal.actions.iter()
        .find(|a| match a {
            Action::ActivateManaAbility { object_id, .. } => state
                .get_object(*object_id)
                .and_then(|o| registry.card_data(o.card_id))
                .is_some_and(|d| d.name == name),
            _ => false,
        })
        .unwrap_or_else(|| panic!("no mana ability offered for an untapped {name}"));
    mtg_engine::engine::submit_action(state, action, registry)
}

/// Activate `object_id`'s `ability_index`-th ability at `targets`.
///
/// The `ActivateAbility` literal has seven fields and appeared 116 times; at
/// almost every site the other four are `vec![]`/`None`. Use
/// [`activate_sacrificing`] when the cost includes sacrificing a permanent.
pub fn activate(
    state: &GameState,
    registry: &CardRegistry,
    object_id: ObjectId,
    ability_index: usize,
    targets: Vec<Target>,
) -> GameState {
    let after = mtg_engine::engine::submit_action(
        state,
        &Action::ActivateAbility {
            object_id, ability_index, targets,
            tap_plan: vec![], sacrifice: None, x_value: None, source_card_id: None,
        },
        registry,
    );
    resolve_activated(after, registry)
}

/// [`activate`], for an ability whose cost includes sacrificing `sacrifice`
/// (Demonmail Hauberk's equip, Brimstone Volley's morbid check, …).
pub fn activate_sacrificing(
    state: &GameState,
    registry: &CardRegistry,
    object_id: ObjectId,
    ability_index: usize,
    targets: Vec<Target>,
    sacrifice: ObjectId,
) -> GameState {
    let after = mtg_engine::engine::submit_action(
        state,
        &Action::ActivateAbility {
            object_id, ability_index, targets,
            tap_plan: vec![], sacrifice: Some(sacrifice), x_value: None, source_card_id: None,
        },
        registry,
    );
    resolve_activated(after, registry)
}

/// Put `spell_id` on the stack with `targets` chosen, and stop there.
///
/// [`cast_and_resolve`] is the usual helper; this one is for tests that need to
/// do something between the cast and the resolution — respond to it, or make a
/// target illegal (CR 608.2b). The `CastSpell` literal has seven fields, five
/// of which are `None`/`vec![]` at almost every call site.
pub fn cast_onto_stack(
    state: &GameState,
    registry: &CardRegistry,
    spell_id: ObjectId,
    targets: Vec<Target>,
) -> GameState {
    mtg_engine::engine::submit_action(
        state,
        &Action::CastSpell { object_id: spell_id, targets, sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        registry,
    )
}

pub fn cast_and_resolve(
    state: &GameState,
    registry: &CardRegistry,
    spell_id: ObjectId,
    targets: Vec<Target>,
) -> GameState {
    let new_state = mtg_engine::engine::submit_action(
        state,
        &Action::CastSpell { object_id: spell_id, targets, sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        registry,
    );
    // Resolve any follow-up casting prompts (exile-choice, then X-funding).
    // Order matters: exile-choice comes first since the engine sets up
    // ChooseExileFromGraveyard from the CastSpell handler, and only after
    // the cast recurses (via ChosenExileSet) might a ChooseXFunding
    // appear. No current card needs both chained, but the helper handles
    // the sequence defensively.
    let new_state = resolve_exile_choice_max_power(&new_state, registry);
    let mut new_state = resolve_funding_max(&new_state, registry);
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, registry);
    new_state
}

/// Resolve a pending `ChooseExileFromGraveyard` prompt (if any) by picking
/// the `max` highest-`effective_power` subset. For fixed-count costs this
/// is the count; for variable-count (Harvest Pyre) it's the maximum
/// possible X. Matches legacy "auto-pick max" behavior so tests that
/// previously relied on engine-side auto-pick keep working.
pub fn resolve_exile_choice_max_power(
    state: &GameState,
    registry: &CardRegistry,
) -> GameState {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseExileFromGraveyard { options, max, .. },
        ..
    }) = state.awaiting_action.clone() else {
        return state.clone();
    };
    // Sort by effective_power desc so Corpse Lunge et al. pick the big creature.
    let mut ranked: Vec<(ObjectId, i32)> = options.iter()
        .map(|&id| (id, state.effective_power(id, registry).unwrap_or(0)))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    let chosen: Vec<ObjectId> = ranked.into_iter().take(max).map(|(id, _)| id).collect();
    let action = Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(chosen) };
    mtg_engine::engine::submit_action(state, &action, registry)
}

/// Resolve a pending `ChooseXFunding` prompt (if any) by maxing out every
/// offered source and draining the pool. No-op when no such prompt is
/// pending. Used by `cast_and_resolve` and by tests that activate X-cost
/// abilities and don't care about a specific X value.
pub fn resolve_funding_max(state: &GameState, registry: &CardRegistry) -> GameState {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::funding::FundingResponse;
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { options, .. },
        ..
    }) = state.awaiting_action.clone() else {
        return state.clone();
    };
    let mut response = FundingResponse::default();
    for (mt, amt) in &options.pool {
        response.pool.insert(*mt, *amt);
    }
    for g in &options.groups {
        response.taps.insert(g.name.clone(), g.max_contribution());
    }
    let action = Action::ResolveChoice { choice: ResolvedChoice::XFunding(response) };
    // Funding an X-cost *ability* is the last step of activating it, so the
    // ability lands on the stack here (CR 602.2a). A spell's funding leaves a
    // spell there instead, which `resolve_activated` leaves alone.
    let after = mtg_engine::engine::submit_action(state, &action, registry);
    resolve_activated(after, registry)
}

/// Put a named card from the registry onto the battlefield, untapped and not
/// summoning sick.
///
/// Called `named_creature` until it turned out to be placing lands, artifacts,
/// enchantments and planeswalkers under 37 distinct card names. P/T and the
/// legendary flag come from the registry; card types and subtypes do not — the
/// characteristics accessors read those from the active face.
pub fn named_permanent(
    state: &mut GameState,
    registry: &CardRegistry,
    name: &str,
    owner: PlayerId,
) -> ObjectId {
    let card_id = registry.get_id_by_name(name)
        .unwrap_or_else(|| panic!("Unknown card: {name}"));
    let data = registry.card_data(card_id)
        .unwrap_or_else(|| panic!("No card data for: {name}"));
    let is_legendary = data.supertypes.contains(&Supertype::Legendary);
    let id = state.create_object(card_id, owner, Zone::Battlefield, data.power, data.toughness);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.summoning_sick = false;
    obj.is_legendary = is_legendary;
    // A planeswalker enters with its starting loyalty (CR 306.5b). The engine
    // does that as part of entering; placing one directly has to do it too, or
    // every planeswalker built this way is already dead to SBA 704.5i.
    if let Some(loyalty) = registry.get(card_id).and_then(|b| b.starting_loyalty()) {
        state.add_counters(id, CounterType::Loyalty, loyalty);
    }
    id
}

/// Process triggers, auto-resolving any "choose target player" choices by picking the opponent.
/// Repeats until all triggers and their choices are fully resolved.
pub fn process_triggers_auto_target_opponent(state: &mut GameState, registry: &CardRegistry) {
    for _ in 0..50 {
        mtg_engine::triggers::process_triggers(state, registry);
        if state.awaiting_action.is_none() {
            break;
        }
        // Auto-resolve: choose the opponent as target.
        let resolved = match &state.awaiting_action {
            Some(mtg_engine::state::AwaitingAction::ResolutionChoice { player, choice, .. }) => {
                let controller = *player;
                match choice {
                    mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. } => {
                        let opponent = state.opponent(controller);
                        options.iter()
                            .find(|t| matches!(t, Target::Player(p) if *p == opponent))
                            .or_else(|| options.first())
                            .cloned()
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(target) = resolved {
            let new_state = mtg_engine::engine::submit_action(
                state,
                &Action::ResolveChoice {
                    choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(target)),
                },
                registry,
            );
            *state = new_state;
        } else {
            break;
        }
    }
}

/// Place a named card directly into `owner`'s graveyard. Equivalent to
/// `named_permanent(...)` followed by `state.move_object(..., Graveyard)`,
/// which appeared in ~9 tests. Useful for setting up graveyard-matters
/// scenarios (Dearly Departed in the graveyard, Unbreathing Horde
/// counting Zombies, flashback targets, reanimation fodder).
pub fn named_card_in_graveyard(
    state: &mut GameState,
    registry: &CardRegistry,
    name: &str,
    owner: PlayerId,
) -> ObjectId {
    let id = named_permanent(state, registry, name, owner);
    state.move_object(id, Zone::Graveyard, registry);
    id
}

/// Push a `StepStarted` event and process any triggers it fires. Used
/// by tests that need to exercise an upkeep / end-step / etc. triggered
/// ability without running the full turn-advancement machinery. The
/// hand-rolled two-line version of this (push the event, then call
/// `triggers::process_triggers`) appeared in ~15 tests.
pub fn fire_step_trigger(state: &mut GameState, step: Step, registry: &CardRegistry) {
    state.events.push(mtg_engine::events::GameEvent::StepStarted { step });
    // Several distinguishable triggers for one player raise a CR 603.3b
    // ordering prompt before anything reaches the stack; tests driving a step
    // through this helper don't care about that order, so take them
    // front-first. A test that does care answers the prompt itself.
    mtg_engine::triggers::collect_triggers(state, registry);
    order_triggers_front_first(state, registry);
    mtg_engine::triggers::process_triggers(state, registry);
}

/// Count tokens on the battlefield with the given name. Useful in
/// assertions like "Gutter Grime should now have N Ooze tokens" — the
/// filter `state.objects.values().filter(|o| o.is_token && ...).count()`
/// was repeated in ~25 tests; this centralises it.
pub fn count_tokens_named(state: &GameState, name: &str) -> usize {
    state.objects.values()
        .filter(|o| o.is_token && o.zone == Zone::Battlefield && o.name == name)
        .count()
}

/// Count tokens on the battlefield with a given name under a specific
/// controller. Used where tests need to distinguish "P0's Zombies" from
/// "P1's Zombies" (e.g., Undead Alchemist).
pub fn count_tokens_named_by(state: &GameState, name: &str, controller: PlayerId) -> usize {
    state.objects.values()
        .filter(|o| o.is_token && o.zone == Zone::Battlefield
            && o.name == name && o.controller == controller)
        .count()
}

/// Find the first token on the battlefield with the given name. Returns
/// its object id so callers can inspect P/T, keywords, or counters.
pub fn find_token_named(state: &GameState, name: &str) -> Option<ObjectId> {
    state.objects.values()
        .find(|o| o.is_token && o.zone == Zone::Battlefield && o.name == name)
        .map(|o| o.id)
}

/// Put a curse (or any "enchant player" permanent) onto the battlefield
/// under `controller` and attach it to `target_player`. Returns the
/// curse's object id.
pub fn attach_curse_to_player(
    state: &mut GameState,
    registry: &CardRegistry,
    card_name: &str,
    controller: PlayerId,
    target_player: PlayerId,
) -> ObjectId {
    let card_id = registry.get_id_by_name(card_name)
        .unwrap_or_else(|| panic!("Unknown card: {card_name}"));
    let id = state.create_object(card_id, controller, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = card_name.into();
    obj.attached_to_player = Some(target_player);
    id
}

/// Count counters of a given type on `obj`. Returns 0 if the object has
/// no counters of that type (or doesn't exist). Prefer this over
/// reaching into `state.get_object(x).unwrap().counters.get(...)` — it
/// makes the intent clear and survives if the internal representation
/// of counters changes.
pub fn counters_of(state: &GameState, obj: ObjectId, kind: CounterType) -> u32 {
    state.get_object(obj)
        .and_then(|o| o.counters.get(&kind).copied())
        .unwrap_or(0)
}

/// Drive `Action::DeclareAttackers` through `engine::submit_action` (the
/// player-facing API). Preferred over `combat::declare_attackers` in tests
/// that exercise end-to-end gameplay, because it sets up the
/// `AwaitingAction`, fires events, and runs the forced-attacker pass.
pub fn submit_declare_attackers(
    state: &mut GameState,
    attackers: &[(ObjectId, PlayerId)],
    registry: &CardRegistry,
) {
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);
    *state = mtg_engine::engine::submit_action(
        state,
        &Action::DeclareAttackers { attackers: attackers.to_vec(), planeswalker_attacks: vec![] },
        registry,
    );
}

/// Drive `Action::DeclareBlockers` through `engine::submit_action` (the
/// player-facing API). `defender` is the defending player (typically
/// `state.opponent(state.active_player)`).
pub fn submit_declare_blockers(
    state: &mut GameState,
    defender: PlayerId,
    assignments: &[(ObjectId, ObjectId)],
    registry: &CardRegistry,
) {
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareBlockers {
        defending_player: defender,
    });
    *state = mtg_engine::engine::submit_action(
        state,
        &Action::DeclareBlockers { assignments: assignments.to_vec() },
        registry,
    );
}

/// How a permanent would enter the battlefield, after every applicable
/// replacement effect (CR 614).
///
/// Tests used to call the card's `entering_with_counters` / `enters_tapped`
/// hooks directly; those are one `replace_event` now, so this asks the same
/// question through the engine's own entry point.
pub fn plan_entering(
    state: &mut GameState,
    registry: &CardRegistry,
    id: ObjectId,
    from: Option<Zone>,
) -> mtg_engine::replacement::EnteringPermanent {
    let controller = state.get_object(id).map_or(P0, |o| o.controller);
    mtg_engine::replacement::for_entering(
        state,
        mtg_engine::replacement::EnteringPermanent {
            object: id,
            from,
            controller,
            tapped: false,
            counters: Vec::new(),
            copy_of: None,
        },
        registry,
    )
}

/// Advance to this turn's cleanup step, so "until end of turn" effects wear off
/// the way the game ends them (CR 514.2) rather than by the test clearing
/// `state.until_end_of_turn` itself.
///
/// A test that clears the list by hand — or, worse, replays the cleanup step's
/// body inline — asserts that its own copy of the rule works. Deleting the
/// engine's cleanup step entirely would leave such a test passing.
pub fn advance_to_cleanup(state: &mut GameState, registry: &CardRegistry) {
    advance_to_step(state, registry, Step::Cleanup);
}

/// Advance until the game reaches `step`, running every step in between and
/// answering the combat declarations with "nobody attacks, nobody blocks".
///
/// The declarations have to be answered rather than stepped past: the engine
/// parks an `AwaitingAction` at each of them, and a state still holding one has
/// no legal actions at all, so a later assertion about what a player may do
/// would be reading a stalled game.
pub fn advance_to_step(state: &mut GameState, registry: &CardRegistry, step: Step) {
    for _ in 0..80 {
        match state.awaiting_action {
            Some(mtg_engine::state::AwaitingAction::DeclareAttackers) => {
                *state = mtg_engine::engine::submit_action(
                    state, &Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] }, registry);
            }
            Some(mtg_engine::state::AwaitingAction::DeclareBlockers { .. }) => {
                *state = mtg_engine::engine::submit_action(
                    state, &Action::DeclareBlockers { assignments: vec![] }, registry);
            }
            _ => mtg_engine::engine::advance_step(state, registry),
        }
        if state.step == step && state.awaiting_action.is_none() {
            return;
        }
    }
    panic!("never reached {step:?} from {:?}", state.step);
}

/// Advance into the next player's turn, so once-per-turn state resets the way
/// the game resets it. Same reasoning as [`advance_to_cleanup`].
pub fn advance_to_next_turn(state: &mut GameState, registry: &CardRegistry) {
    let turn = state.turn_number;
    for _ in 0..80 {
        match state.awaiting_action {
            Some(mtg_engine::state::AwaitingAction::DeclareAttackers) => {
                *state = mtg_engine::engine::submit_action(
                    state, &Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] }, registry);
            }
            Some(mtg_engine::state::AwaitingAction::DeclareBlockers { .. }) => {
                *state = mtg_engine::engine::submit_action(
                    state, &Action::DeclareBlockers { assignments: vec![] }, registry);
            }
            _ => mtg_engine::engine::advance_step(state, registry),
        }
        if state.turn_number != turn {
            return;
        }
    }
    panic!("never reached the next turn from {:?}", state.step);
}

/// Put `n` filler cards into `player`'s library.
///
/// A game built with [`game_at_step`] starts with empty libraries, so any test
/// that takes a real draw step — anything using [`advance_to_next_turn`] — has
/// to stock one or the player decks out and the game ends underneath the test.
/// Use this for tests that just need cards to be there; when the identity of
/// the card matters, build it yourself.
pub fn stock_library(state: &mut GameState, registry: &CardRegistry, player: PlayerId, n: usize) -> Vec<ObjectId> {
    let card_id = registry.get_id_by_name("Forest").expect("Forest is in the registry");
    let ids: Vec<ObjectId> = (0..n)
        .map(|_| state.create_object(card_id, player, Zone::Library, None, None))
        .collect();
    state.get_player_mut(player).library_order.extend(&ids);
    ids
}

/// Kill `id` by marking lethal damage and running state-based actions, so it
/// dies the way the game kills it (CR 704.5g) and its death trigger fires off a
/// real death event.
///
/// Queued events are cleared first, so a `process_triggers` after this sees
/// this death and nothing the setup happened to leave behind.
pub fn kill_by_damage(state: &mut GameState, registry: &CardRegistry, id: ObjectId) {
    let lethal = state.effective_toughness(id, registry).unwrap_or(1).max(1) as u32;
    state.get_object_mut(id).expect("creature to kill").damage_marked = lethal;
    state.events.clear();
    mtg_engine::sba::check_state_based_actions(state, registry);
    // A token that dies ceases to exist in the same SBA pass (CR 111.7), so
    // "it is gone" and "it is in the graveyard" are both success.
    assert!(state.get_object(id).is_none_or(|o| o.zone == Zone::Graveyard),
        "kill_by_damage: {lethal} damage should have been lethal, but it is in {:?}",
        state.get_object(id).map(|o| o.zone));
}

/// Whether the engine currently offers `id` as a spell its controller may cast.
///
/// The `CastSpell` action names only the object, so this answers "is a cast
/// offered at all", not which cost it would be paid at — see
/// `flashback.rs::a_flashback_card_in_hand_is_cast_normally` for why that
/// distinction matters when a card can be cast two ways.
pub fn can_cast(state: &GameState, registry: &CardRegistry, id: ObjectId) -> bool {
    mtg_engine::engine::legal_actions(state, registry).actions.iter()
        .any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == id))
}

/// Put the game into the state `combat::declare_blockers` leaves behind: each
/// attacker attacking the named player, with its blockers assigned and its
/// blocked-ness recorded.
///
/// Thirty-one sites across sixteen files used to build `CombatState` by hand,
/// in three different shapes, and every one of them left `blocked_attackers`
/// empty. The engine never does: CR 509.2 makes blocked-ness permanent for
/// the combat, so an attacker whose blockers all leave is still blocked and
/// still deals no damage to the player. A test standing on a state the engine
/// cannot produce can pass while the path it claims to cover is broken.
pub fn declare_combat(state: &mut GameState, attacks: &[(ObjectId, PlayerId, &[ObjectId])]) {
    let mut combat = mtg_engine::state::CombatState::new();
    for &(attacker, defender, blockers) in attacks {
        combat.attackers.insert(attacker, defender);
        combat.blocker_assignments.insert(attacker, blockers.to_vec());
        if !blockers.is_empty() {
            combat.blocked_attackers.insert(attacker);
        }
    }
    state.combat = Some(combat);
}

/// One attacker, unblocked.
pub fn attacks_unblocked(state: &mut GameState, attacker: ObjectId, defender: PlayerId) {
    declare_combat(state, &[(attacker, defender, &[])]);
}

/// One attacker, blocked by `blockers`.
pub fn attacks_blocked_by(
    state: &mut GameState,
    attacker: ObjectId,
    defender: PlayerId,
    blockers: &[ObjectId],
) {
    declare_combat(state, &[(attacker, defender, blockers)]);
}

/// Add an attacker to a combat already under way — a creature put onto the
/// battlefield attacking (CR 508.4), which was never *declared* as an attacker
/// and so fired no `AttackersDeclared` for anything to watch.
///
/// Distinct from [`declare_combat`], which replaces the whole combat state.
pub fn joins_the_attack(state: &mut GameState, attacker: ObjectId, defender: PlayerId) {
    let combat = state.combat.get_or_insert_with(mtg_engine::state::CombatState::new);
    combat.attackers.insert(attacker, defender);
    combat.blocker_assignments.insert(attacker, Vec::new());
}

/// Answer a pending library-search prompt: take `found`, or fail to find.
///
/// CR 701.19b lets a player searching a hidden zone come back with nothing
/// even when a match is there, so every search stops and asks — including a
/// mandatory one with exactly one candidate, which used to be taken for the
/// player. Tests that drive a search have to answer it.
pub fn answer_library_search(
    state: &GameState,
    registry: &CardRegistry,
    found: Option<ObjectId>,
) -> GameState {
    assert!(state.awaiting_action.is_some(),
        "expected a pending library-search choice; there is none");
    let choice = match found {
        Some(id) => mtg_engine::actions::ResolvedChoice::ChosenCard(id),
        None => mtg_engine::actions::ResolvedChoice::ChosenTarget(None),
    };
    mtg_engine::engine::submit_action(state, &Action::ResolveChoice { choice }, registry)
}

/// Set the game's random stream so the next coin flip comes out `win`.
///
/// Randomness lives on `GameState` precisely so a test can say which way it
/// went. Searching for a seed rather than hard-coding one keeps this honest if
/// the generator is ever replaced: the test asks for an outcome, not for a
/// magic number.
///
/// # Panics
/// Panics if no seed in the first thousand produces the requested outcome,
/// which would mean `flip_coin` is not a coin.
pub fn rig_next_coin_flip(state: &mut GameState, win: bool) {
    for seed in 0..1000u64 {
        let mut probe = state.clone();
        probe.rng_state = seed;
        if probe.flip_coin() == win {
            state.rng_state = seed;
            return;
        }
    }
    panic!("no seed in the first thousand flips {win}");
}

/// Answer pending CR 603.3b trigger-order prompts by taking the first offered
/// trigger each time, until something else (or nothing) is awaiting. For a
/// test that does not care about the order, this reproduces the pre-choice
/// front-first ordering; a test that *does* care answers the prompt itself.
pub fn order_triggers_front_first(state: &mut GameState, registry: &CardRegistry) {
    while let Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        choice: mtg_engine::state::ResolutionChoiceKind::ChooseTriggerOrder { options, .. },
        ..
    }) = &state.awaiting_action
    {
        let label = options[0].clone();
        *state = mtg_engine::engine::submit_action(
            state,
            &Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::ChosenIndex(0, label),
            },
            registry,
        );
    }
}
