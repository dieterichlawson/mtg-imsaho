//! The rules engine: legal actions, applying a chosen action, and the loop
//! that drives a game from setup to a result.
//!
//! Split by what each part is *for*, since this file was once 4,576 lines and
//! over half of it was two functions:
//!
//! - [`actions`] — applying a chosen [`Action`], one module per variant.
//! - [`legal`] — enumerating what a player may do right now.
//! - [`mana_sources`] — producing mana and paying costs with it.
//! - [`targeting`] — what a spell or ability may point at.
//! - [`effects`] — applying a resolved effect.
//! - [`cards_flow`] — drawing, milling, discarding.

mod actions;
mod cards_flow;
mod effects;
mod mana_sources;
mod targeting;

pub use cards_flow::{draw_cards, mill_cards, mill_one};
pub use effects::apply_pending_effect;
pub use mana_sources::{
    activate_mana_source, available_mana_abilities, can_pay_with_sources,
    effective_spell_cost, pay_cost_with_sources,
};
pub use targeting::can_be_targeted_by;

pub(crate) use cards_flow::{card_name, has_castable_with_potential_mana, legal_discard_actions};
pub(crate) use effects::{finalize_spell_cast, finish_spell_resolution_if_idle};
pub(crate) use mana_sources::{
    activatable_mana_abilities, alternative_costs_from_effects, execute_tap_plan_and_pay,
    gather_mana_sources, plan_autotap_for_cost, prevents_artifact_abilities,
};
pub(crate) use targeting::{
    matches_target_filter,
    build_cast_target_spec, combinations, detect_modal_choice_mode, generate_ability_targets,
    generate_cast_actions_with_targets,
    valid_targets_for_req,
};

use rand::seq::SliceRandom;

use crate::actions::Action;
use crate::cards::CardRegistry;
use crate::combat;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::mana;
use crate::sba::check_state_based_actions;
use crate::stack;
use crate::state::{AwaitingAction, GameState, LogLevel};
use crate::triggers;
use crate::types::{Zone, CardType, ManaCost, ContinuousEffect, Keyword, CounterType, Step};

/// A decklist: card name -> count.
#[derive(Clone)]
pub struct Decklist {
    pub entries: Vec<(String, u32)>,
}

/// Configuration for setting up a game.
pub struct GameConfig {
    pub player_names: Vec<String>,
    pub decklists: Vec<Decklist>,
    pub starting_life: i32,
    /// Which player is on the play (goes first). `None` means `setup_game`
    /// will pick `PlayerId(0)` by default. Callers that want to honour
    /// proper MTG rules (randomised game 1, loser-chooses for games 2+)
    /// should set this explicitly.
    pub starting_player: Option<PlayerId>,
}

/// Result of `legal_actions`: a list of actions plus an optional combat prompt.
/// When a combat prompt is present, the player should construct a
/// DeclareAttackers/DeclareBlockers action from it (not pick from the actions list).
pub struct LegalActions {
    pub actions: Vec<Action>,
    pub combat_prompt: Option<crate::actions::CombatPrompt>,
    /// Castable spells with valid target options, for interactive target selection.
    /// Each entry is one castable spell (collapsed view). The `actions` list still
    /// contains the fully-expanded `CastSpell` entries for LLM/random players.
    pub castable_spells: Vec<crate::actions::CastableSpell>,
    /// Activated abilities with valid target options, for interactive target selection.
    /// Each entry is one ability (collapsed view). The `actions` list still
    /// contains the fully-expanded `ActivateAbility` entries.
    pub activatable_abilities: Vec<crate::actions::ActivatableAbility>,
    /// Human-readable description of why the player has priority or needs to act.
    pub context: Option<String>,
    /// Set when the engine is waiting on a structured mid-resolution choice
    /// (e.g. X-cost funding). Player implementations inspect this to produce
    /// a tailored prompt + response. When present, `actions` is empty —
    /// the only legal response is an `Action::ResolveChoice` constructed by
    /// the player based on this prompt's payload.
    pub resolution_prompt: Option<crate::state::ResolutionChoiceKind>,
}













/// Compute all legal actions for the player who currently needs to act.
///
/// # Panics
/// Panics if internal invariants are violated while enumerating actions — for
/// example, if a just-confirmed-affordable flashback cost suddenly fails its
/// autotap lookup, or if other object/registry lookups that were checked
/// earlier in the function disagree with themselves later on.
pub fn legal_actions(state: &GameState, registry: &CardRegistry) -> LegalActions {
    use crate::actions::ActivatableAbilityOption;
    use crate::cards::AdditionalCost;

    struct AbilityGroup {
        name: String,
        description: String,
        target_options: Vec<crate::actions::Target>,
        tap_plan: Vec<(ObjectId, usize)>,
        option_combos: Vec<ActivatableAbilityOption>,
    }

    if state.is_game_over() {
        return LegalActions { actions: vec![], combat_prompt: None, castable_spells: vec![], activatable_abilities: vec![], context: None, resolution_prompt: None };
    }

    // If we're waiting for a specific action (attackers, blockers, discard).
    if let Some(awaiting) = &state.awaiting_action {
        return match awaiting {
            AwaitingAction::DeclareAttackers => {
                let active = state.active_player;
                let eligible = combat::eligible_attackers(state, active, registry);
                let defending = state.opponent(active);
                // Find creatures that must attack (e.g., enchanted by Furor of the Bitten).
                let must_attack: Vec<ObjectId> = eligible.iter()
                    .filter(|&&id| {
                        state.has_continuous_effect(id, &|e| {
                            match e {
                                crate::types::ContinuousEffect::ForceAttack { scope } => Some(scope),
                                _ => None,
                            }
                        }, registry)
                    })
                    .copied()
                    .collect();
                LegalActions {
                    actions: vec![],
                    combat_prompt: Some(crate::actions::CombatPrompt::ChooseAttackers {
                        eligible,
                        must_attack,
                        defending_player: defending,
                    }),
                    castable_spells: vec![],
                    activatable_abilities: vec![], context: Some("DECLARE ATTACKERS".into()),
                    resolution_prompt: None,
                }
            }
            AwaitingAction::DeclareBlockers { defending_player } => {
                let eligible_blockers = combat::eligible_blockers(state, *defending_player, registry);
                let attacker_ids: Vec<_> = state.combat.as_ref()
                    .map(|c| c.attackers.keys().copied().collect())
                    .unwrap_or_default();
                let mut legal_blocks = std::collections::HashMap::new();
                for &blocker_id in &eligible_blockers {
                    let can_block: Vec<_> = attacker_ids.iter()
                        .filter(|&&att_id| combat::can_block_attacker(state, blocker_id, att_id, registry))
                        .copied()
                        .collect();
                    legal_blocks.insert(blocker_id, can_block);
                }
                LegalActions {
                    actions: vec![],
                    combat_prompt: Some(crate::actions::CombatPrompt::ChooseBlockers {
                        eligible_blockers,
                        attackers: attacker_ids,
                        legal_blocks,
                    }),
                    castable_spells: vec![],
                    activatable_abilities: vec![],
                    context: Some("DECLARE BLOCKERS".into()),
                    resolution_prompt: None,
                }
            }
            AwaitingAction::DiscardToHandSize { player, discard_count } => {
                LegalActions {
                    actions: legal_discard_actions(state, *player, *discard_count),
                    combat_prompt: None,
                    castable_spells: vec![],
                    activatable_abilities: vec![],
                    context: Some(format!("DISCARD {} CARD{}", discard_count,
                        if *discard_count == 1 { "" } else { "S" })),
                    resolution_prompt: None,
                }
            }
            AwaitingAction::MulliganDecision { player } => {
                let mull_count = state.get_player(*player).mulligan_count;
                let mut actions = vec![Action::MulliganKeep];
                if mull_count < crate::state::LONDON_MULLIGAN_CAP {
                    actions.push(Action::MulliganMull);
                }
                LegalActions {
                    actions,
                    combat_prompt: None,
                    castable_spells: vec![],
                    activatable_abilities: vec![],
                    context: Some(format!(
                        "MULLIGAN DECISION (mulligans taken: {}/{})",
                        mull_count, crate::state::LONDON_MULLIGAN_CAP)),
                    resolution_prompt: None,
                }
            }
            AwaitingAction::BottomAfterMulligan { player, count } => {
                // Enumerate combinations of `count` cards from hand so the
                // action list is self-contained for simple players. Rich
                // players (LLM/CLI) can bypass this and construct a
                // BottomCards action directly — submit_action validates that
                // the chosen cards are in hand and distinct.
                let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, *player)
                    .iter().map(|o| o.id).collect();
                let combos = combinations(&hand, *count);
                let actions: Vec<Action> = combos.into_iter()
                    .map(|cards| Action::BottomCards { cards })
                    .collect();
                LegalActions {
                    actions,
                    combat_prompt: None,
                    castable_spells: vec![],
                    activatable_abilities: vec![],
                    context: Some(format!("BOTTOM {} CARD{} AFTER MULLIGAN",
                        count, if *count == 1 { "" } else { "s" })),
                    resolution_prompt: None,
                }
            }
            AwaitingAction::ResolutionChoice { choice, source, player, .. } => {
                use crate::state::ResolutionChoiceKind;
                use crate::actions::ResolvedChoice;
                let source_name = card_name(state, registry, *source);
                let actions = match choice {
                    ResolutionChoiceKind::PayOrNot { cost, .. } => {
                        // Declining is always available; paying is offered only
                        // when the player can actually produce the mana, here
                        // or by tapping (CR 608.2g).
                        let mut acts = Vec::new();
                        if can_pay_with_sources(state, *player, cost, registry) {
                            acts.push(Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) });
                        }
                        acts.push(Action::ResolveChoice { choice: ResolvedChoice::PayDecision(false) });
                        acts
                    }
                    ResolutionChoiceKind::ChooseTarget { options, optional, .. } => {
                        let mut acts: Vec<Action> = options.iter()
                            .map(|t| Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(t.clone())) })
                            .collect();
                        if *optional {
                            acts.push(Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) });
                        }
                        acts
                    }
                    ResolutionChoiceKind::YesNo { .. } => {
                        vec![
                            Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
                            Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(false) },
                        ]
                    }
                    ResolutionChoiceKind::ChooseCardFromHand { cards, .. } => {
                        cards.iter()
                            .map(|&id| Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(id) })
                            .collect()
                    }
                    ResolutionChoiceKind::ChooseFromRevealed { revealed, .. } => {
                        revealed.iter()
                            .map(|&id| Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(id) })
                            .collect()
                    }
                    ResolutionChoiceKind::ChooseFromLibrary { options, .. } => {
                        options.iter()
                            .map(|&id| Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(id) })
                            .collect()
                    }
                    ResolutionChoiceKind::ChooseCardType { options, .. } => {
                        options.iter().enumerate()
                            .map(|(i, name)| Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(i, name.clone()) })
                            .collect()
                    }
                    ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, .. } => {
                        // Generate all possible subsets of permanents (each subset = pile 1).
                        // With N permanents there are 2^N subsets. This is fine for typical
                        // board states (up to ~15 permanents = 32768 actions).
                        let n = permanents.len();
                        (0..(1u64 << n))
                            .map(|mask| {
                                let subset: Vec<ObjectId> = (0..n)
                                    .filter(|&i| mask & (1u64 << i) != 0)
                                    .map(|i| permanents[i])
                                    .collect();
                                Action::ResolveChoice { choice: ResolvedChoice::ChosenSubset(subset) }
                            })
                            .collect()
                    }
                    ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. } => {
                        let fmt_pile = |ids: &[ObjectId]| -> String {
                            if ids.is_empty() { return "empty".to_string(); }
                            ids.iter().filter_map(|id| state.objects.get(id).map(|o| o.name.clone()))
                                .collect::<Vec<_>>().join(", ")
                        };
                        vec![
                            Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0, format!("Pile 1: [{}]", fmt_pile(pile_1))) },
                            Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(1, format!("Pile 2: [{}]", fmt_pile(pile_2))) },
                        ]
                    }
                    ResolutionChoiceKind::ChooseCardName { options, .. } => {
                        options.iter().enumerate()
                            .map(|(i, name)| Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(i, name.clone()) })
                            .collect()
                    }
                    ResolutionChoiceKind::ChooseXFunding { .. }
                    | ResolutionChoiceKind::ChooseExileFromGraveyard { .. } => {
                        // Structured prompt — can't be enumerated as a flat
                        // action list. Player implementations see the
                        // `resolution_prompt` field and construct the
                        // response directly (XFunding / ChosenExileSet).
                        vec![]
                    }
                };
                let context = match choice {
                    ResolutionChoiceKind::ChooseTarget { description, .. }
                    | ResolutionChoiceKind::PayOrNot { description, .. }
                    | ResolutionChoiceKind::ChooseCardFromHand { description, .. }
                    | ResolutionChoiceKind::ChooseCardName { description, .. }
                    | ResolutionChoiceKind::ChooseXFunding { description, .. }
                    | ResolutionChoiceKind::ChooseExileFromGraveyard { description, .. } => description.clone(),
                    ResolutionChoiceKind::YesNo { .. } => format!("{source_name}: choose yes or no"),
                    ResolutionChoiceKind::ChooseFromRevealed { .. } => format!("{source_name}: choose a card"),
                    ResolutionChoiceKind::ChooseFromLibrary { .. } => format!("{source_name}: search library"),
                    ResolutionChoiceKind::ChooseCardType { options, .. } => {
                        let opts = options.iter().enumerate()
                            .map(|(i, name)| format!("{i}: {name}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{source_name}: choose a card type ({opts})")
                    }
                    ResolutionChoiceKind::DividePermanentsIntoPiles { .. } => format!("{source_name}: divide into piles"),
                    ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. } => {
                        let fmt_pile = |ids: &[ObjectId]| -> String {
                            if ids.is_empty() { return "empty".to_string(); }
                            ids.iter().filter_map(|id| state.objects.get(id).map(|o| o.name.clone()))
                                .collect::<Vec<_>>().join(", ")
                        };
                        format!("{}: choose which pile to sacrifice (0: [{}], 1: [{}])",
                            source_name, fmt_pile(pile_1), fmt_pile(pile_2))
                    }
                };
                LegalActions {
                    actions,
                    combat_prompt: None,
                    castable_spells: vec![],
                    activatable_abilities: vec![],
                    context: Some(context),
                    resolution_prompt: Some(choice.clone()),
                }
            }
        };
    }

    let Some(player) = state.priority_player else {
        return LegalActions { actions: vec![], combat_prompt: None, castable_spells: vec![], activatable_abilities: vec![], context: None, resolution_prompt: None };
    };

    let mut actions = Vec::new();
    let mut castable_spells = Vec::new();

    // PassPriority is always available when you have priority.
    actions.push(Action::PassPriority);

    // Stony Silence: no abilities of artifacts can be activated, mana
    // abilities included.
    let prevent_artifact_abilities = prevents_artifact_abilities(state, registry);

    // Mana abilities: can activate anytime you have priority.
    // Deduplicate by card_id — if you have 5 untapped Forests, only show one "Tap Forest".
    let mut seen_mana_abilities: Vec<(CardId, usize)> = Vec::new();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        // Stony Silence: skip mana abilities from artifacts.
        if prevent_artifact_abilities {
            if state.has_card_type(obj.id, CardType::Artifact, registry) { continue; }
        }
        for ma in activatable_mana_abilities(state, obj.id, registry) {
            let key = (obj.card_id, ma.ability_index);
            if !seen_mana_abilities.contains(&key) {
                seen_mana_abilities.push(key);
                actions.push(Action::ActivateManaAbility {
                    object_id: obj.id,
                    ability_index: ma.ability_index,
                });
            }
        }
    }

    // Sorcery-speed window: your main phase, stack empty, your turn.
    let is_sorcery_speed = state.step.is_main_phase()
        && state.stack.is_empty()
        && state.active_player == player;

    // Gather mana sources up-front so both the activated-ability loop below and the
    // spell-casting loop further down can compute auto-tap plans against them.
    let early_mana_sources = gather_mana_sources(state, player, registry, prevent_artifact_abilities);

    // Non-mana activated abilities: can activate anytime you have priority (if you can pay).
    // Check attached permanents too (auras granting abilities to creatures).
    let mana_pool = &state.get_player(player).mana_pool;
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
        // be collected from `copy_grantor` — whichever card that happens to be.
        let grantor = state.get_object(obj_id).and_then(|o| o.copy_grantor);
        if let Some(grantor_id) = grantor.filter(|&g| g != obj_card_id) {
            if let Some(behavior) = registry.get(grantor_id) {
                for ab in behavior.activated_abilities(state, obj_id, registry) {
                    abilities.push((grantor_id, ab));
                }
            }
        }
        for attached in state.objects.values() {
            // Only offer abilities granted by attachments the acting player
            // controls. Every granted activated ability in the set includes
            // sacrificing the attached source as a cost (often paid manually
            // in on_activate_ability, e.g. Blazing Torch), and a player can
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
                // Targeted ability: generate one action per valid (target, sacrifice) combo.
                // We exclude pairs where the sacrifice IS the target — sacrificing the
                // target makes the ability fizzle, no rational player picks that.
                let behavior = registry.get(source_card_id);
                if let Some(behavior) = behavior {
                    let targets = generate_ability_targets(state, obj_id, &ab, player, registry, behavior);
                    for target in targets {
                        let target_obj_id = match &target {
                            crate::actions::Target::Object(id) => Some(*id),
                            crate::actions::Target::Player(_) => None,
                        };
                        let sacrifices_for_target: Vec<&Option<ObjectId>> = eligible_sacrifices.iter()
                            .filter(|sac| match (sac, target_obj_id) {
                                (Some(sac_id), Some(t_id)) => *sac_id != t_id,
                                _ => true,
                            })
                            .collect();
                        // If the sac filter eliminated every option (i.e. the only
                        // creature available is also the target), don't generate this
                        // target — the ability has no legal way to resolve.
                        if sacrifices_for_target.is_empty() { continue; }
                        for sac in sacrifices_for_target {
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

    // Collect names banned by PreventCastingNamed effects (e.g. Nevermore).
    let casting_banned: Vec<String> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield)
        .flat_map(|o| {
            o.instance_continuous_effects.as_deref()
                .unwrap_or(&[])
                .iter()
                .filter_map(|e| {
                    if let ContinuousEffect::PreventCastingNamed { name } = e {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // Gather available mana sources for autotap.
    // Reuse the set we computed earlier for activated abilities.
    let mana_sources = early_mana_sources;

    // Collect costs of all castable spells in hand for hand-demand heuristic.
    // This is a quick pre-pass: gather effective costs for spells that could be cast this turn.
    let hand_costs: Vec<ManaCost> = state.objects_in_zone(Zone::Hand, player).iter()
        .filter_map(|obj| {
            let behavior = registry.get(obj.card_id)?;
            let data = behavior.card_data();
            data.cost.as_ref().map(|c| effective_spell_cost(state, registry, obj.card_id, c, player))
        })
        .collect();

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

    // Concede is always last.
    actions.push(Action::Concede);

    // Build context string based on game state.
    let context = if state.stack.is_empty() {
        // Normal priority — show the phase, with context for opponent's turn.
        let is_your_turn = state.active_player == player;
        let step_name = match state.step {
            Step::PrecombatMain => "MAIN PHASE 1",
            Step::PostcombatMain => "MAIN PHASE 2",
            Step::BeginCombat => "BEGIN COMBAT",
            Step::EndCombat => "END COMBAT",
            Step::Upkeep => "UPKEEP",
            Step::EndStep => "END STEP",
            Step::Draw => "DRAW",
            Step::DeclareAttackers => "AFTER ATTACKERS DECLARED",
            Step::DeclareBlockers => "AFTER BLOCKERS DECLARED",
            Step::CombatDamage => "COMBAT DAMAGE",
            Step::Untap => "UNTAP",
            Step::Cleanup => "CLEANUP",
        };
        if is_your_turn {
            step_name.into()
        } else {
            format!("OPPONENT'S TURN: {step_name}")
        }
    } else {
        // Responding to something on the stack.
        let top_name = match state.stack.last() {
            Some(crate::state::StackEntry::Spell(id)) => card_name(state, registry, *id),
            Some(crate::state::StackEntry::Trigger(t)) => t.display_name_with_state(registry, Some(state)),
            Some(crate::state::StackEntry::Ability { source_id, .. }) => card_name(state, registry, *source_id),
            None => "?".into(),
        };
        let caster = match state.stack.last() {
            Some(crate::state::StackEntry::Spell(id)) =>
                state.get_object(*id).map(|o| o.controller),
            Some(crate::state::StackEntry::Trigger(t)) => Some(t.controller()),
            Some(crate::state::StackEntry::Ability { activator, .. }) => Some(*activator),
            None => None,
        };
        let who = match caster {
            Some(p) if p == player => "your".into(),
            Some(p) => format!("p{}'s", p.0),
            None => "?".into(),
        };
        format!("RESPOND TO {who} {top_name}")
    };

    // Build collapsed activatable abilities from the expanded ActivateAbility actions.
    // Group by (object_id, source_card_id, ability_index) — including source_card_id
    // ensures aura-granted abilities don't collapse into native ones with the same
    // ability_index. Collect every (target, sacrifice) combo as well as the
    // de-duplicated target list. We capture the tap_plan from the first action in
    // each group so player UIs can display "(tap Forest, Mountain)" alongside the
    // ability label, like Cast does.
    let mut ability_map: std::collections::HashMap<(ObjectId, Option<crate::ids::CardId>, usize), AbilityGroup> =
        std::collections::HashMap::new();
    for action in &actions {
        if let Action::ActivateAbility { object_id, ability_index, targets, tap_plan, sacrifice, source_card_id, .. } = action {
            let entry = ability_map.entry((*object_id, *source_card_id, *ability_index)).or_insert_with(|| {
                let name = state.obj_name(*object_id);
                // Look up the description from the source card's behavior. For
                // native (source_card_id = None), use the object's own card.
                let lookup_card_id = source_card_id.unwrap_or_else(|| {
                    state.get_object(*object_id).map_or(crate::ids::CardId(0), |o| o.card_id)
                });
                let desc = registry.get(lookup_card_id)
                    .and_then(|b| {
                        b.activated_abilities(state, *object_id, registry)
                            .into_iter()
                            .find(|a| a.ability_index == *ability_index)
                            .map(|a| a.description.clone())
                    }).unwrap_or_default();
                AbilityGroup {
                    name,
                    description: desc,
                    target_options: Vec::new(),
                    tap_plan: tap_plan.clone(),
                    option_combos: Vec::new(),
                }
            });
            for t in targets {
                if !entry.target_options.contains(t) {
                    entry.target_options.push(t.clone());
                }
            }
            entry.option_combos.push(ActivatableAbilityOption {
                targets: targets.clone(),
                sacrifice: *sacrifice,
            });
        }
    }
    let activatable_abilities: Vec<crate::actions::ActivatableAbility> = ability_map
        .into_iter()
        .map(|((object_id, source_card_id, ability_index), g)| {
            crate::actions::ActivatableAbility {
                object_id,
                ability_index,
                source_card_id,
                name: g.name,
                description: g.description,
                target_options: g.target_options,
                tap_plan: g.tap_plan,
                option_combos: g.option_combos,
            }
        })
        .collect();

    LegalActions { actions, combat_prompt: None, castable_spells, activatable_abilities, context: Some(context), resolution_prompt: None }
}











// Attacker/blocker enumeration removed — players now construct combat
// actions from CombatPrompt data. The engine validates on submission.




/// Apply a chosen action, returning the resulting game state.
///
/// A pure transition: the caller's state is untouched. Each `Action` variant
/// is handled by its own function in [`actions`]; this is the dispatch and the
/// bookkeeping either side of it.
pub fn submit_action(state: &GameState, action: &Action, registry: &CardRegistry) -> GameState {
    let mut new_state = state.clone();
    new_state.events.clear();
    new_state.trigger_event_index = 0;

    let applied = match action {
        Action::PassPriority => actions::simple::pass_priority(&mut new_state, registry),
        Action::PlayLand { object_id } =>
            actions::simple::play_land(&mut new_state, *object_id, registry),
        Action::DiscardCards { cards } =>
            actions::simple::discard_cards(&mut new_state, cards, registry),
        Action::Concede => actions::simple::concede(&mut new_state, registry),

        Action::CastSpell { object_id, targets, sacrifice, exile_count, exile_ids,
                            alternative_cost, tap_plan } =>
            actions::cast::cast_spell(&mut new_state, *object_id, targets, *sacrifice,
                *exile_count, exile_ids, alternative_cost.as_ref(), tap_plan, registry),

        Action::ActivateManaAbility { object_id, ability_index } =>
            actions::abilities::activate_mana_ability(&mut new_state, *object_id, *ability_index, registry),
        Action::ActivateAbility { object_id, ability_index, targets, tap_plan, sacrifice,
                                  source_card_id, .. } =>
            actions::abilities::activate_ability(&mut new_state, *object_id, *ability_index,
                targets, tap_plan, *sacrifice, *source_card_id, registry),
        Action::ActivateLoyaltyAbility { object_id, ability_index, targets } =>
            actions::abilities::activate_loyalty_ability(&mut new_state, *object_id, *ability_index,
                targets, registry),

        Action::DeclareAttackers { attackers } =>
            actions::combat::declare_attackers(&mut new_state, attackers, registry),
        Action::DeclareBlockers { assignments } =>
            actions::combat::declare_blockers(&mut new_state, assignments, registry),

        Action::MulliganKeep => actions::mulligan::mulligan_keep(&mut new_state, registry),
        Action::MulliganMull => actions::mulligan::mulligan_mull(&mut new_state, registry),
        Action::BottomCards { cards } =>
            actions::mulligan::bottom_cards(&mut new_state, cards, registry),

        Action::ResolveChoice { choice: resolved } =>
            actions::choices::resolve_choice(&mut new_state, resolved, registry),
    };

    match applied {
        // The handler asked to return this state verbatim, skipping the
        // end-of-action cleanup below — it is mid-way through a choice chain.
        Applied::ReturnNow => return new_state,
        // The handler re-entered submit_action (a cast resumed after its
        // prompts were answered); that call already did the cleanup.
        Applied::Replace(s) => return s,
        Applied::Continue => {}
    }

    finish_spell_resolution_if_idle(&mut new_state, registry);

    new_state
}

/// What an action handler wants `submit_action` to do once it returns.
///
/// The arms of the old single-function `submit_action` reached this by falling
/// off the end or by an early `return`; as separate functions they have to say
/// so explicitly.
pub(crate) enum Applied {
    /// Run the end-of-action cleanup and return.
    Continue,
    /// Return immediately, skipping the cleanup.
    ReturnNow,
    /// Return this state instead — the handler produced it by re-entering
    /// `submit_action`.
    Replace(GameState),
}



/// Pick a starting player for game 1 of a match by a fair coin flip
/// (uniform over `num_players`). Per MTG tournament rules, the player
/// chosen always elects to play first in this implementation — declining
/// to play is a legal choice but is almost never strategically correct.
///
/// # Panics
/// Panics if `num_players` is 0.
#[must_use]
pub fn random_starting_player(num_players: u8) -> PlayerId {
    use rand::Rng;
    assert!(num_players >= 1, "random_starting_player needs at least 1 player");
    PlayerId(rand::thread_rng().gen_range(0..num_players))
}

/// Pick the starting player for the next game of a match, baking in the
/// "loser always chooses to play first" strategic default.
///
/// Per MTG tournament rules (MTR §2.3) the loser of the previous game is
/// the one who *chooses* who takes the first turn of the next game. In
/// practice the loser effectively always elects to play first — going on
/// the draw is a legitimate but extremely rare strategic choice, and
/// never correct in Limited. This helper bakes in that default choice
/// and returns the loser directly; callers who want to expose play/draw
/// as a real decision should implement their own flow.
///
/// - `previous_starter`: who was on the play in the previous game
/// - `previous_winner`: the previous game's winner, or `None` for a draw
/// - `num_players`: currently only 2-player matches are supported
///
/// On a drawn game there is no loser, so the previous starter stays on
/// the play (per MTR §2.3 — the pre-game choice simply persists).
///
/// # Panics
/// Panics if `num_players` is not 2; only 2-player matches are supported.
#[must_use]
pub fn next_starter_loser_plays(
    previous_starter: PlayerId,
    previous_winner: Option<PlayerId>,
    num_players: u8,
) -> PlayerId {
    assert_eq!(num_players, 2,
        "next_starter_loser_plays only supports 2-player matches");
    match previous_winner {
        None => previous_starter,
        Some(winner) => PlayerId(1 - winner.0),
    }
}

/// Set up a new game: create objects, shuffle libraries, draw opening hands.
///
/// # Panics
/// Panics if `config.starting_player` is set to a `PlayerId` outside the
/// range of players in the config, or if any card name in a decklist is not
/// present in `registry`.
#[must_use]
pub fn setup_game(config: &GameConfig, registry: &CardRegistry) -> GameState {
    let num_players = u8::try_from(config.player_names.len()).unwrap_or(u8::MAX);
    let mut state = GameState::new(num_players);

    // Set starting life.
    for p in &mut state.players {
        p.life = config.starting_life;
    }

    // Honour the caller's choice of starting player if specified. Default
    // to PlayerId(0) (the legacy behaviour) otherwise. The caller is
    // expected to randomise game 1 and apply loser-chooses for games 2+
    // per MTG tournament rules.
    if let Some(starting) = config.starting_player {
        assert!(
            starting.0 < num_players,
            "starting_player {starting:?} out of range for {num_players}-player game",
        );
        state.active_player = starting;
    }

    // Create card objects for each player's deck.
    let mut rng = rand::thread_rng();
    for (player_idx, decklist) in config.decklists.iter().enumerate() {
        let player_id = PlayerId(u8::try_from(player_idx).unwrap_or(u8::MAX));
        let mut library_ids = Vec::new();

        for (card_name, count) in &decklist.entries {
            let card_id = registry.get_id_by_name(card_name)
                .unwrap_or_else(|| panic!("Unknown card: {card_name}"));

            let card_data = registry.card_data(card_id).expect("card must be in registry");

            for _ in 0..*count {
                let obj_id = state.create_object(
                    card_id,
                    player_id,
                    Zone::Library,
                    card_data.power,
                    card_data.toughness,
                );
                // Printed characteristics are NOT copied onto the object: they
                // live on the card's active face and are read through the
                // characteristics accessors. Copying them here was the source
                // of a test/production split — a real game's objects had
                // populated `card_types`/`subtypes`/`colors` while every object
                // built by `create_object` (tests, tokens, reanimation) had them
                // empty, so code reading the raw fields worked in play and
                // silently did nothing under test. `name` stays as a display
                // cache; `name_of` is the authoritative read.
                let obj = state.get_object_mut(obj_id).expect("object must exist for library draw");
                obj.name.clone_from(card_name);
                library_ids.push(obj_id);
            }
        }

        // Shuffle the library.
        library_ids.shuffle(&mut rng);
        state.get_player_mut(player_id).library_order = library_ids;
    }

    state.log(
        LogLevel::Milestone,
        format!("Game started (p{} on the play)", state.active_player.0),
    );

    // Draw opening hands (7 cards each).
    for player_idx in 0..num_players {
        let player_id = PlayerId(player_idx);
        let _ = draw_cards(&mut state, player_id, 7, registry);
    }

    state.events.push(GameEvent::GameStarted);

    // Enter the London mulligan phase. The first turn banner is logged once
    // the mulligan phase completes (see advance_mulligan_phase).
    state.log(LogLevel::Milestone, "Mulligan phase".into());
    state.awaiting_action = Some(AwaitingAction::MulliganDecision {
        player: state.active_player,
    });
    state
}

/// Drive the London mulligan phase forward after a decision or bottom is
/// resolved.
///
/// The keep/mull sub-phase runs in *rounds*. Within each round, every
/// not-yet-kept player makes one keep/mull decision in turn order
/// starting from the active player. After the round completes, if any
/// player mulled this round we start a new round (giving any still-
/// undecided player another chance). Once every player has kept, we
/// drain the bottoming queue.
///
/// This matches real London mulligan info flow: the non-active player
/// sees the active player's *current-round* decision before deciding
/// (because the active player decides first within a round), but
/// neither player ever sees the other's *future-round* decisions.
///
/// The caller is expected to have already advanced
/// `state.mulligan_round_position` past the player whose decision was
/// just processed (or to have set `state.awaiting_action = None` for
/// post-bottoming transitions).
fn advance_mulligan_phase(state: &mut GameState, _registry: &CardRegistry) {
    let num_players = u8::try_from(state.players.len()).unwrap_or(u8::MAX);

    loop {
        // Sub-phase 1: keep/mull. Find the next not-yet-kept player in this
        // round (skipping over players who have already kept). If no such
        // player exists in this round, check end-of-round.
        while state.mulligan_round_position < num_players {
            let pos = state.mulligan_round_position;
            let player = PlayerId((state.active_player.0 + pos) % num_players);
            if state.get_player(player).mulligan_kept {
                // Already kept (in a previous round). Skip and try the next
                // position.
                state.mulligan_round_position += 1;
                continue;
            }
            // Ask this player.
            state.awaiting_action = Some(AwaitingAction::MulliganDecision { player });
            return;
        }

        // End of round. If anyone mulled this round, start a new round and
        // reset the within-round flags. Otherwise (everyone in this round
        // chose to keep — or there was nobody left to ask) every player is
        // now kept and we proceed to bottoming.
        if state.mulligan_round_mulled {
            state.mulligan_round_position = 0;
            state.mulligan_round_mulled = false;
            continue;
        }
        break;
    }

    // Sub-phase 2: bottoming. Drain pending_mulligan_bottoms in turn
    // order. Players with count 0 are skipped. The pending list was
    // populated as each player chose to keep, so iterating it preserves
    // turn order.
    while let Some((player, count)) = state.pending_mulligan_bottoms.first().copied() {
        if count == 0 {
            state.pending_mulligan_bottoms.remove(0);
            continue;
        }
        state.pending_mulligan_bottoms.remove(0);
        state.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player, count });
        return;
    }

    // Mulligan phase fully complete.
    state.awaiting_action = None;
    state.log(LogLevel::Milestone, "── Turn 1 (p0) ──".into());
}

/// True if the state is in the opening-hand mulligan phase.
#[must_use]
pub fn in_mulligan_phase(state: &GameState) -> bool {
    matches!(state.awaiting_action,
        Some(AwaitingAction::MulliganDecision { .. } |
AwaitingAction::BottomAfterMulligan { .. }))
        || !state.pending_mulligan_bottoms.is_empty()
}





/// Advance the game by one step. Performs turn-based actions for the new step.
/// Returns the updated state.
pub fn advance_step(state: &mut GameState, registry: &CardRegistry) {
    // Empty mana pools between steps.
    for player in &mut state.players {
        if !player.mana_pool.is_empty() {
            state.events.push(GameEvent::ManaPoolEmptied { player: player.id });
            player.mana_pool.empty();
        }
    }

    // CR 510.5: with first/double strikers in combat there are TWO combat
    // damage steps. The first instance set combat_damage_step_pending; repeat
    // Step::CombatDamage (regular damage) instead of moving to EndCombat.
    let next = if state.step == Step::CombatDamage && state.combat_damage_step_pending {
        Some(Step::CombatDamage)
    } else {
        state.step.next()
    };
    if let Some(next_step) = next {
        state.step = next_step;
    } else {
        // End of turn: advance to next player's turn.
        let next_player = state.next_player(state.active_player);
        state.active_player = next_player;
        state.turn_number += 1;
        state.step = Step::Untap;
        state.is_first_turn = false;
        state.creature_died_this_turn = false;
        // Copy this turn's spell counts to last_turn, then clear for next turn.
        state.num_spells_cast_last_turn = state.num_spells_cast_this_turn.clone();
        state.num_spells_cast_this_turn.clear();
        // Clear once-per-turn ability tracking for all permanents.
        for obj in state.objects.values_mut() {
            obj.abilities_activated_this_turn.clear();
        }

        state.events.push(GameEvent::TurnStarted {
            player: next_player,
            turn: state.turn_number,
        });
        state.log(LogLevel::Milestone, format!("── Turn {} (p{}) ──", state.turn_number, next_player.0));
    }

    state.events.push(GameEvent::StepStarted { step: state.step });
    state.log(LogLevel::Debug, format!("Step: {:?}", state.step));
    state.consecutive_passes = 0;

    // Perform turn-based actions for this step.
    perform_turn_based_actions(state, registry);
}

/// Perform automatic actions when entering a step.
fn perform_turn_based_actions(state: &mut GameState, registry: &CardRegistry) {
    let active = state.active_player;

    match state.step {
        Step::Untap => {
            // Check which creatures are prevented from untapping (e.g., by Claustrophobia).
            let locked_ids: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, active)
                .iter()
                .filter(|o| {
                    state.has_continuous_effect(o.id, &|e| {
                        match e {
                            crate::types::ContinuousEffect::PreventUntap { scope } => Some(scope),
                            _ => None,
                        }
                    }, registry)
                })
                .map(|o| o.id)
                .collect();

            // Untap all permanents the active player controls, except locked ones.
            let to_untap: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, active)
                .iter()
                .filter(|o| o.tapped && !locked_ids.contains(&o.id))
                .map(|o| o.id)
                .collect();

            for id in to_untap {
                state.get_object_mut(id).expect("object must exist for untap").tapped = false;
                state.events.push(GameEvent::Untapped { object: id });
            }

            // Clear summoning sickness for creatures the active player controls.
            let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, active)
                .iter()
                .filter(|o| o.summoning_sick)
                .map(|o| o.id)
                .collect();

            for id in creatures {
                state.get_object_mut(id).expect("object must exist for summoning sickness clear").summoning_sick = false;
            }

            // Reset land plays.
            state.get_player_mut(active).land_plays_remaining = 1;

            // No priority during untap step.
            state.priority_player = None;
        }

        Step::Draw => {
            // Active player draws a card (skip on the very first turn).
            if !state.is_first_turn {
                let _ = draw_cards(state, active, 1, registry);
            }
            state.priority_player = Some(active);
        }

        Step::DeclareAttackers => {
            // Set up the awaiting action for attacker declaration.
            state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
            state.priority_player = Some(active);
        }

        Step::DeclareBlockers => {
            // Check if there are any attackers.
            let has_attackers = state.combat.as_ref()
                .is_some_and(|c| !c.attackers.is_empty());

            if has_attackers {
                let defending = state.opponent(active);
                state.awaiting_action = Some(AwaitingAction::DeclareBlockers {
                    defending_player: defending,
                });
                state.priority_player = Some(defending);
            } else {
                // No attackers, skip.
                state.priority_player = Some(active);
            }
        }

        Step::CombatDamage => {
            let has_attackers = state.combat.as_ref()
                .is_some_and(|c| !c.attackers.is_empty());

            if has_attackers {
                if state.combat_damage_step_pending {
                    // Second combat damage step (CR 510.5): regular damage
                    // from creatures that didn't deal first-strike damage,
                    // plus double strikers.
                    combat::deal_regular_damage_pass(state, registry);
                    state.combat_damage_step_pending = false;
                } else if combat::any_first_strike_in_combat(state, registry) {
                    // First of two combat damage steps (CR 510.5): only
                    // first/double strikers deal damage now. advance_step
                    // repeats Step::CombatDamage after this step's SBA /
                    // trigger / priority round.
                    combat::deal_first_strike_damage_pass(state, registry);
                    state.combat_damage_step_pending = true;
                } else {
                    combat::deal_combat_damage(state, registry);
                }
            }
            // Players get priority after combat damage is dealt (CR 510.4).
            state.priority_player = Some(active);
        }

        Step::EndCombat => {
            combat::end_combat(state, registry);
            state.priority_player = Some(active);
        }

        Step::Cleanup => {
            // Remove damage from all creatures.
            let damaged: Vec<ObjectId> = state.all_objects_in_zone(Zone::Battlefield)
                .iter()
                .filter(|o| o.damage_marked > 0)
                .map(|o| o.id)
                .collect();

            for id in damaged {
                let obj = state.get_object_mut(id).expect("object must exist for damage clear");
                obj.damage_marked = 0;
                obj.dealt_deathtouch_damage = false; obj.damaged_by.clear();
            }

            // Remove "until end of turn" effects.
            // First, revert control changes before clearing.
            for effect in &state.until_end_of_turn {
                if let crate::state::TemporaryEffect::ChangeControl { target, original_controller } = effect {
                    if let Some(obj) = state.objects.get_mut(target) {
                        if obj.zone == Zone::Battlefield {
                            obj.controller = *original_controller;
                        }
                    }
                }
            }
            state.until_end_of_turn.clear();

            // Clear unused regeneration shields.
            for obj in state.objects.values_mut() {
                if obj.zone == Zone::Battlefield {
                    obj.regeneration_shields = 0;
                }
            }

            // Empty mana pools.
            for player in &mut state.players {
                player.mana_pool.empty();
            }

            // CR 514.3a: Check SBAs after clearing effects. If any SBA
            // fires, players get priority (the cleanup step essentially restarts).
            let registry_ref = registry;
            let sba_fired = crate::sba::check_state_based_actions(state, registry_ref);
            if sba_fired {
                // SBA occurred — give active player priority. The game loop
                // will process actions and eventually advance past cleanup.
                state.priority_player = Some(active);
            } else {
                // Check hand size: max 7 cards.
                let hand_size = state.objects_in_zone(Zone::Hand, active).len();
                if hand_size > 7 {
                    state.awaiting_action = Some(AwaitingAction::DiscardToHandSize {
                        player: active,
                        discard_count: hand_size - 7,
                    });
                    state.priority_player = Some(active);
                } else {
                    state.priority_player = None; // No priority in cleanup normally.
                }
            }
        }

        // All other steps: just give priority to active player.
        _ => {
            state.priority_player = Some(active);
        }
    }
}

/// The main game loop. Takes a mutable game state and player callbacks.
/// `choose_action` receives the game state, acting player, legal actions,
/// and an optional combat prompt. It returns the chosen Action.
pub fn run_game_loop<F>(
    state: &mut GameState,
    registry: &CardRegistry,
    mut choose_action: F,
) where
    F: FnMut(&GameState, PlayerId, &LegalActions) -> Action,
{
    // If we're still in the opening-hand mulligan phase, run it first.
    // The phase clears itself when done and we fall through to turn 1.
    run_game_loop_inner(state, registry, &mut choose_action);
}

/// Drive the opening-hand London mulligan phase to completion, asking each
/// player for keep/mull and bottom decisions via `choose_action`. Returns
/// when `in_mulligan_phase` becomes false. Useful for tests that want to
/// exercise the mulligan phase in isolation without driving into turn 1
/// (which would invoke turn-based actions, draws, and auto-pass logic that
/// can mask the post-mulligan state).
pub fn run_mulligan_phase<F>(
    state: &mut GameState,
    registry: &CardRegistry,
    mut choose_action: F,
) where
    F: FnMut(&GameState, PlayerId, &LegalActions) -> Action,
{
    run_mulligan_phase_inner(state, registry, &mut choose_action);
}

fn run_mulligan_phase_inner<F>(
    state: &mut GameState,
    registry: &CardRegistry,
    choose_action: &mut F,
) where
    F: FnMut(&GameState, PlayerId, &LegalActions) -> Action,
{
    loop {
        if !in_mulligan_phase(state) {
            break;
        }
        // If awaiting_action is None but there are queued bottoms, advance.
        if state.awaiting_action.is_none() {
            advance_mulligan_phase(state, registry);
            continue;
        }

        let acting_player = match &state.awaiting_action {
            Some(AwaitingAction::MulliganDecision { player } | AwaitingAction::BottomAfterMulligan { player, .. }) => *player,
            _ => unreachable!("in_mulligan_phase guaranteed a mulligan awaiting_action"),
        };

        let legal = legal_actions(state, registry);
        if legal.actions.is_empty() {
            // Safety: if somehow no action is legal (e.g. zero cards to
            // bottom), just clear and continue.
            state.awaiting_action = None;
            advance_mulligan_phase(state, registry);
            continue;
        }

        let action = choose_action(state, acting_player, &legal);
        *state = submit_action(state, &action, registry);
    }
}

/// Resume a game loop from a previously saved state. Unlike `run_game_loop`,
/// this skips the initial turn setup since the state already has it applied.
pub fn resume_game_loop<F>(
    state: &mut GameState,
    registry: &CardRegistry,
    mut choose_action: F,
) where
    F: FnMut(&GameState, PlayerId, &LegalActions) -> Action,
{
    run_game_loop_inner(state, registry, &mut choose_action);
}

fn run_game_loop_inner<F>(
    state: &mut GameState,
    registry: &CardRegistry,
    choose_action: &mut F,
) where
    F: FnMut(&GameState, PlayerId, &LegalActions) -> Action,
{
    const MAX_AUTO_PASSES: u32 = 100;

    let num_players = u32::try_from(state.players.len()).unwrap_or(u32::MAX);
    let mut auto_pass_count = 0u32;

    // Opening-hand mulligan phase. When present, drive it first; it will
    // clear itself by setting awaiting_action = None and draining the
    // pending bottom queue.
    if in_mulligan_phase(state) {
        run_mulligan_phase_inner(state, registry, choose_action);
        if state.is_game_over() {
            return;
        }
    }

    // Start of turn 1 (or continuation from a resumed game). If we're at
    // the very start of the game (no prior turn-based actions applied), do
    // them now. We detect "fresh first turn" by checking that step is Untap
    // and there are no priority/awaiting markers yet — a resumed game will
    // already have priority_player set from a saved state.
    if state.is_first_turn
        && state.turn_number == 1
        && state.step == Step::Untap
        && state.priority_player.is_none()
        && state.awaiting_action.is_none()
    {
        state.events.push(GameEvent::TurnStarted {
            player: state.active_player,
            turn: state.turn_number,
        });
        perform_turn_based_actions(state, registry);
    }

    loop {
        if state.is_game_over() {
            break;
        }

        // CR 117.5: Before a player gets priority, check SBAs and collect
        // triggered abilities onto the stack. Repeat until neither produces
        // new work. Triggers resolve through the normal priority cycle,
        // giving players a chance to respond between each resolution.
        loop {
            let mut any_work = false;
            loop {
                let sba = check_state_based_actions(state, registry);
                if !sba { break; }
                any_work = true;
            }
            if triggers::collect_triggers(state, registry) {
                any_work = true;
            }
            if !any_work { break; }
        }
        if state.is_game_over() {
            break;
        }

        // If no one has priority and there's no awaiting action, advance step.
        if state.priority_player.is_none() && state.awaiting_action.is_none() {
            advance_step(state, registry);
            continue;
        }

        // Determine who needs to act.
        let acting_player = if let Some(AwaitingAction::DeclareBlockers { defending_player }) = &state.awaiting_action {
            *defending_player
        } else if let Some(AwaitingAction::DiscardToHandSize { player, .. }) = &state.awaiting_action {
            *player
        } else if let Some(AwaitingAction::ResolutionChoice { player, .. }) = &state.awaiting_action {
            *player
        } else if let Some(p) = state.priority_player { p } else {
            advance_step(state, registry);
            continue;
        };

        let legal = legal_actions(state, registry);
        if legal.actions.is_empty() && legal.combat_prompt.is_none() {
            advance_step(state, registry);
            continue;
        }

        // Auto-declare zero attackers when there are no eligible creatures.
        if let Some(crate::actions::CombatPrompt::ChooseAttackers {
            ref eligible, ref must_attack, ..
        }) = legal.combat_prompt {
            if eligible.is_empty() && must_attack.is_empty() {
                *state = submit_action(state, &Action::DeclareAttackers { attackers: vec![] }, registry);
                state.priority_player = Some(state.active_player);
                continue;
            }
        }

        // Auto-pass: if the player has no meaningful actions, auto-pass.
        // Mana abilities alone aren't meaningful UNLESS the player could cast
        // something after tapping — compute potential mana to check.
        let has_meaningful_action = legal.combat_prompt.is_some()
            || state.awaiting_action.is_some()
            || legal.actions.iter().any(|a| !matches!(a,
                Action::PassPriority | Action::Concede | Action::ActivateManaAbility { .. }
            ))
            || has_castable_with_potential_mana(state, acting_player, registry);

        let action = if has_meaningful_action {
            auto_pass_count = 0;
            choose_action(state, acting_player, &legal)
        } else if state.priority_player.is_some() {
            auto_pass_count += 1;
            if auto_pass_count > MAX_AUTO_PASSES {
                // Safety: break infinite auto-pass loops.
                advance_step(state, registry);
                auto_pass_count = 0;
                continue;
            }
            Action::PassPriority
        } else {
            advance_step(state, registry);
            continue;
        };

        *state = submit_action(state, &action, registry);

        // After submitting, handle priority flow.
        match &action {
            Action::PassPriority => {
                if state.consecutive_passes >= num_players {
                    // All players passed in succession.
                    if state.stack.is_empty() {
                        // Stack empty, advance step.
                        state.priority_player = None;
                        advance_step(state, registry);
                    } else {
                        let mut new_state = state.clone();
                        stack::resolve_top_of_stack(&mut new_state, registry);
                        *state = new_state;
                        state.consecutive_passes = 0;
                        state.priority_player = Some(state.active_player);
                    }
                } else if let Some(current) = state.priority_player {
                    // Pass to next player.
                    state.priority_player = Some(state.next_player(current));
                }
            }

            Action::DeclareAttackers { .. } => {
                // After declaring attackers (even zero), give priority to active player.
                // The step will advance naturally through DeclareBlockers, CombatDamage,
                // and EndCombat — no skipping. advance_step handles empty combat gracefully.
                state.priority_player = Some(state.active_player);
            }

            Action::DeclareBlockers { .. } => {
                // After declaring blockers, give priority to active player.
                state.priority_player = Some(state.active_player);
            }

            Action::DiscardCards { .. } => {
                // After discarding, cleanup continues (no priority).
                state.priority_player = None;
            }

            Action::MulliganKeep
            | Action::MulliganMull
            | Action::BottomCards { .. }
            | Action::ActivateManaAbility { .. }
            | Action::ActivateAbility { .. }
            | Action::ActivateLoyaltyAbility { .. }
            | Action::Concede
            | Action::PlayLand { .. }
            | Action::CastSpell { .. } => {
                // Mulligan-phase actions don't touch priority (mulligan advances via
                // awaiting_action), ability activations and spell casts leave priority
                // with the current player, and Concede relies on SBAs to end the game.
            }

            Action::ResolveChoice { .. } => {
                // After resolving a choice, return priority to active player.
                // Triggers may continue processing in the next loop iteration.
                state.priority_player = Some(state.active_player);
            }
        }
    }
}
