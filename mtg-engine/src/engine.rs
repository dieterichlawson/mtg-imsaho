use rand::seq::SliceRandom;

use crate::actions::Action;
use crate::cards::CardRegistry;
use crate::combat;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::mana;
use crate::sba::check_state_based_actions_with_registry;
use crate::stack;
use crate::state::{AwaitingAction, GameState, LogLevel};
use crate::triggers;
use crate::types::*;

/// A decklist: card name -> count.
pub struct Decklist {
    pub entries: Vec<(String, u32)>,
}

/// Configuration for setting up a game.
pub struct GameConfig {
    pub player_names: Vec<String>,
    pub decklists: Vec<Decklist>,
    pub starting_life: i32,
}

/// Result of legal_actions: a list of actions plus an optional combat prompt.
/// When a combat prompt is present, the player should construct a
/// DeclareAttackers/DeclareBlockers action from it (not pick from the actions list).
pub struct LegalActions {
    pub actions: Vec<Action>,
    pub combat_prompt: Option<crate::actions::CombatPrompt>,
    /// Castable spells with valid target options, for interactive target selection.
    /// Each entry is one castable spell (collapsed view). The `actions` list still
    /// contains the fully-expanded CastSpell entries for LLM/random players.
    pub castable_spells: Vec<crate::actions::CastableSpell>,
}

/// Compute the effective mana cost of a spell after applying cost reduction effects.
/// Returns a reduced ManaCost (generic portion lowered, colored requirements unchanged).
pub fn effective_spell_cost(state: &GameState, registry: &CardRegistry, card_id: CardId, base_cost: &ManaCost, caster: PlayerId) -> ManaCost {
    use crate::types::{ContinuousEffect, SpellFilter};

    // Check if the card has a custom cost modification (e.g., Blasphemous Act).
    if let Some(behavior) = registry.get(card_id) {
        if let Some(modified) = behavior.modified_cost(state, registry) {
            return modified;
        }
    }

    let card_data = registry.card_data(card_id);
    let is_creature = card_data.as_ref()
        .map(|d| d.card_types.contains(&CardType::Creature))
        .unwrap_or(false);
    let subtypes: Vec<String> = card_data.as_ref()
        .map(|d| d.subtypes.clone())
        .unwrap_or_default();

    // Gather all ReduceCost effects from permanents the caster controls.
    let mut total_reduction: u32 = 0;
    for obj in state.objects.values() {
        if obj.zone != Zone::Battlefield || obj.controller != caster {
            continue;
        }
        if let Some(behavior) = registry.get(obj.card_id) {
            for effect in &behavior.card_data().continuous_effects {
                if let ContinuousEffect::ReduceCost { reduction, filter } = effect {
                    let applies = match filter {
                        SpellFilter::CreatureSpells => is_creature,
                        SpellFilter::CreatureWithSubtype(sub) => {
                            is_creature && subtypes.iter().any(|s| s == sub)
                        }
                    };
                    if applies {
                        total_reduction += reduction;
                    }
                }
            }
        }
    }

    // Check for Rooftop Storm: Zombie creature spells cost {0}.
    if is_creature && subtypes.iter().any(|s| s == "Zombie") {
        let has_rooftop_storm = state.objects.values().any(|o| {
            o.zone == Zone::Battlefield && o.controller == caster && o.name == "Rooftop Storm"
        });
        if has_rooftop_storm {
            return ManaCost::free();
        }
    }

    if total_reduction == 0 {
        return base_cost.clone();
    }

    // Apply reduction to generic mana first, keeping colored requirements.
    let mut remaining_reduction = total_reduction;
    let mut new_symbols = Vec::new();
    for sym in &base_cost.symbols {
        match sym {
            ManaSymbol::Generic(n) => {
                if remaining_reduction >= *n {
                    remaining_reduction -= *n;
                    // Reduced to zero, omit this symbol.
                } else {
                    new_symbols.push(ManaSymbol::Generic(*n - remaining_reduction));
                    remaining_reduction = 0;
                }
            }
            other => new_symbols.push(other.clone()),
        }
    }
    ManaCost::new(new_symbols)
}

/// Compute all legal actions for the player who currently needs to act.
pub fn legal_actions(state: &GameState, registry: &CardRegistry) -> LegalActions {
    if state.is_game_over() {
        return LegalActions { actions: vec![], combat_prompt: None, castable_spells: vec![] };
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
                }
            }
            AwaitingAction::DeclareBlockers { defending_player } => {
                let eligible_blockers = combat::eligible_blockers(state, *defending_player, registry);
                let attacker_ids = state.combat.as_ref()
                    .map(|c| c.attackers.keys().copied().collect())
                    .unwrap_or_default();
                LegalActions {
                    actions: vec![],
                    combat_prompt: Some(crate::actions::CombatPrompt::ChooseBlockers {
                        eligible_blockers,
                        attackers: attacker_ids,
                    }),
                    castable_spells: vec![],
                }
            }
            AwaitingAction::DiscardToHandSize { player, discard_count } => {
                LegalActions {
                    actions: legal_discard_actions(state, *player, *discard_count),
                    combat_prompt: None,
                    castable_spells: vec![],
                }
            }
            AwaitingAction::ResolutionChoice { choice, .. } => {
                use crate::state::ResolutionChoiceKind;
                use crate::actions::ResolvedChoice;
                let actions = match choice {
                    ResolutionChoiceKind::PayOrNot { .. } => {
                        vec![
                            Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
                            Action::ResolveChoice { choice: ResolvedChoice::PayDecision(false) },
                        ]
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
                            Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
                            Action::ResolveChoice { choice: ResolvedChoice::PayDecision(false) },
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
                };
                LegalActions { actions, combat_prompt: None, castable_spells: vec![] }
            }
        };
    }

    let player = match state.priority_player {
        Some(p) => p,
        None => return LegalActions { actions: vec![], combat_prompt: None, castable_spells: vec![] },
    };

    let mut actions = Vec::new();
    let mut castable_spells = Vec::new();

    // PassPriority is always available when you have priority.
    actions.push(Action::PassPriority);

    // Mana abilities: can activate anytime you have priority.
    // Deduplicate by card_id — if you have 5 untapped Forests, only show one "Tap Forest".
    let mut seen_mana_abilities: Vec<(CardId, usize)> = Vec::new();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            let mana_abs = behavior.mana_abilities(state, obj.id);
            for ma in mana_abs {
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
    }

    // Sorcery-speed window: your main phase, stack empty, your turn.
    let is_sorcery_speed = state.step.is_main_phase()
        && state.stack.is_empty()
        && state.active_player == player;

    // Check for Stony Silence: artifact activated abilities can't be activated.
    let stony_silence_active = state.objects.values().any(|o| {
        o.zone == Zone::Battlefield && o.name == "Stony Silence"
    });

    // Non-mana activated abilities: can activate anytime you have priority (if you can pay).
    // Check attached permanents too (auras granting abilities to creatures).
    let mana_pool = &state.get_player(player).mana_pool;
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        let obj_id = obj.id;
        let obj_tapped = obj.tapped;
        let obj_card_id = obj.card_id;
        let activated_this_turn = obj.abilities_activated_this_turn.clone();

        // Stony Silence: skip artifact activated abilities.
        if stony_silence_active {
            let is_artifact = registry.card_data(obj_card_id)
                .map(|d| d.card_types.contains(&CardType::Artifact))
                .unwrap_or(false)
                || obj.card_types.contains(&CardType::Artifact);
            if is_artifact { continue; }
        }

        // Collect abilities from this permanent's card and attached auras.
        let mut abilities: Vec<(crate::ids::CardId, crate::cards::ActivatedAbilityDef)> = Vec::new();
        if let Some(behavior) = registry.get(obj_card_id) {
            for ab in behavior.activated_abilities(state, obj_id) {
                abilities.push((obj_card_id, ab));
            }
        }
        for attached in state.objects.values() {
            if attached.zone == Zone::Battlefield && attached.attached_to == Some(obj_id) {
                if let Some(behavior) = registry.get(attached.card_id) {
                    for ab in behavior.activated_abilities(state, obj_id) {
                        abilities.push((attached.card_id, ab));
                    }
                }
            }
        }

        for (source_card_id, ab) in abilities {
            // Check mana cost.
            if !mana::can_pay(mana_pool, &ab.cost) { continue; }
            // Check tap cost.
            if ab.requires_tap && obj_tapped { continue; }
            // Check once-per-turn.
            if ab.once_per_turn && activated_this_turn.contains(&ab.ability_index) { continue; }
            // Check sorcery speed.
            if ab.sorcery_speed_only && !is_sorcery_speed { continue; }
            // Check sacrifice cost.
            use crate::cards::SacrificeCost;
            match &ab.sacrifice_cost {
                SacrificeCost::None => {}
                SacrificeCost::SacrificeThis => {
                    // Object must be on the battlefield (it is, we're iterating battlefield).
                }
                SacrificeCost::SacrificeCreature => {
                    // Must control at least one creature to sacrifice.
                    let has_creature = state.objects_in_zone(Zone::Battlefield, player)
                        .iter()
                        .any(|o| o.power.is_some());
                    if !has_creature { continue; }
                }
            }

            // Generate actions based on targeting.
            if let Some(ref _target_req) = ab.target_requirement {
                // Targeted ability: generate one action per valid target.
                // Use the card behavior for is_valid_target filtering.
                let behavior = registry.get(source_card_id);
                if let Some(behavior) = behavior {
                    let targets = generate_ability_targets(state, obj_id, &ab, player, registry, behavior);
                    for target in targets {
                        actions.push(Action::ActivateAbility {
                            object_id: obj_id,
                            ability_index: ab.ability_index,
                            targets: vec![target],
                        });
                    }
                }
            } else {
                // Untargeted ability.
                actions.push(Action::ActivateAbility {
                    object_id: obj_id,
                    ability_index: ab.ability_index,
                    targets: vec![],
                });
            }
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
                let loyalty_abs = behavior.loyalty_abilities();
                if loyalty_abs.is_empty() { continue; }

                let current_loyalty = state.get_counter_count(obj_id, CounterType::Loyalty);
                for ab in &loyalty_abs {
                    // Check if we can pay the cost.
                    if ab.loyalty_change < 0 && ((-ab.loyalty_change) as u32) > current_loyalty {
                        continue; // Not enough loyalty
                    }
                    actions.push(Action::ActivateLoyaltyAbility {
                        object_id: obj_id,
                        ability_index: ab.ability_index,
                    });
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

    // Collect names banned by Nevermore (spells with that name can't be cast).
    let nevermore_banned: Vec<String> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Nevermore")
        .filter_map(|o| {
            o.instance_oracle_text.as_ref()
                .and_then(|t| t.strip_prefix("nevermore:"))
                .map(|s| s.to_string())
        })
        .collect();

    // Cast spells from hand.
    // Deduplicate untargeted spells — only show one "Cast Kalonian Tusker" even if you have 3.
    // Targeted spells still get one entry per valid target.
    let mut seen_untargeted_casts: Vec<CardId> = Vec::new();
    for obj in state.objects_in_zone(Zone::Hand, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            let data = behavior.card_data();

            // Check Nevermore: spells with the banned name can't be cast.
            if nevermore_banned.iter().any(|n| *n == data.name) {
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

            // Check mana (applying cost reduction effects).
            if let Some(cost) = &data.cost {
                let effective_cost = effective_spell_cost(state, registry, obj.card_id, cost, player);
                if !mana::can_pay(&player_state.mana_pool, &effective_cost) {
                    continue;
                }
            }

            // Generate cast actions with valid targets.
            let target_req = behavior.target_requirement();

            // For untargeted spells, deduplicate by card_id.
            if matches!(target_req, crate::cards::TargetRequirement::None) {
                if seen_untargeted_casts.contains(&obj.card_id) {
                    continue;
                }
                seen_untargeted_casts.push(obj.card_id);
            }

            let cast_actions = generate_cast_actions_with_targets(
                state, player, obj.id, &target_req, behavior,
            );
            if !cast_actions.is_empty() {
                actions.extend(cast_actions);
                let spec = build_cast_target_spec(state, player, obj.id, &target_req, behavior);
                castable_spells.push(crate::actions::CastableSpell {
                    object_id: obj.id,
                    name: data.name.clone(),
                    is_flashback: false,
                    target_spec: spec,
                });
            }
        }
    }

    // Cast spells via flashback from graveyard.
    let mut seen_untargeted_flashbacks: Vec<CardId> = Vec::new();
    for obj in state.objects_in_zone(Zone::Graveyard, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            let data = behavior.card_data();

            // Check for flashback cost, dynamic flashback, or "cast from graveyard" ability.
            let dynamic_fb = state.until_end_of_turn_flashback.iter()
                .find(|(id, _)| *id == obj.id)
                .map(|(_, c)| c.clone());
            let cast_from_gy = behavior.can_cast_from_graveyard();
            let fb_cost = match dynamic_fb {
                Some(ref c) => c,
                None => match &data.flashback_cost {
                    Some(c) => c,
                    None => if cast_from_gy {
                        // Cast from graveyard uses normal mana cost.
                        match &data.cost {
                            Some(c) => c,
                            None => continue,
                        }
                    } else {
                        continue;
                    },
                },
            };

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

            if !mana::can_pay(&player_state.mana_pool, fb_cost) { continue; }

            let target_req = behavior.target_requirement();

            if matches!(target_req, crate::cards::TargetRequirement::None) {
                if seen_untargeted_flashbacks.contains(&obj.card_id) { continue; }
                seen_untargeted_flashbacks.push(obj.card_id);
            }

            let cast_actions = generate_cast_actions_with_targets(
                state, player, obj.id, &target_req, behavior,
            );
            if !cast_actions.is_empty() {
                actions.extend(cast_actions);
                let spec = build_cast_target_spec(state, player, obj.id, &target_req, behavior);
                castable_spells.push(crate::actions::CastableSpell {
                    object_id: obj.id,
                    name: data.name.clone(),
                    is_flashback: true,
                    target_spec: spec,
                });
            }
        }
    }

    // Concede is always last.
    actions.push(Action::Concede);

    LegalActions { actions, combat_prompt: None, castable_spells }
}

/// Check if a permanent can be targeted by a spell from the given caster.
/// Returns false if the target has hexproof and the caster is an opponent.
fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
    if state.has_keyword(target_id, Keyword::Hexproof, registry) {
        let controller = state.get_object(target_id)
            .map(|o| o.controller)
            .unwrap_or(PlayerId(255));
        if controller != caster {
            return false; // hexproof: can't be targeted by opponents
        }
    }
    true
}

/// Generate CastSpell actions with all valid target combinations.
fn generate_cast_actions_with_targets(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    target_req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
) -> Vec<Action> {
    use crate::actions::Target;
    use crate::cards::TargetRequirement;
    let registry = &CardRegistry::with_all_cards();

    match target_req {
        TargetRequirement::None => {
            vec![Action::CastSpell { object_id: spell_id, targets: vec![] }]
        }
        TargetRequirement::AnyTarget => {
            // Can target any creature on the battlefield or any player.
            let mut actions = Vec::new();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                if obj.power.is_some() { // is a creature
                    if !can_be_targeted(state, obj.id, caster, registry) { continue; }
                    let target = Target::Object(obj.id);
                    if behavior.is_valid_target(state, caster, &target, registry) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                        });
                    }
                }
            }
            for player in &state.players {
                if !player.lost {
                    let target = Target::Player(player.id);
                    if behavior.is_valid_target(state, caster, &target, registry) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                        });
                    }
                }
            }
            actions
        }
        TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_) => {
            let mut actions = Vec::new();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                if obj.power.is_some() { // is a creature
                    if !can_be_targeted(state, obj.id, caster, registry) { continue; }
                    let target = Target::Object(obj.id);
                    if behavior.is_valid_target(state, caster, &target, registry) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                        });
                    }
                }
            }
            actions
        }
        TargetRequirement::PlayerOnly => {
            let mut actions = Vec::new();
            for player in &state.players {
                if !player.lost {
                    let target = Target::Player(player.id);
                    if behavior.is_valid_target(state, caster, &target, registry) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                        });
                    }
                }
            }
            actions
        }
        TargetRequirement::Spell => {
            let mut actions = Vec::new();
            for entry in &state.stack {
                let stack_obj_id = match entry.as_spell() {
                    Some(id) => id,
                    None => continue, // Skip triggers — can't target with Counterspell
                };
                // Don't let a spell target itself on the stack.
                if stack_obj_id == spell_id { continue; }
                let target = Target::Object(stack_obj_id);
                if behavior.is_valid_target(state, caster, &target, registry) {
                    actions.push(Action::CastSpell {
                        object_id: spell_id,
                        targets: vec![target],
                    });
                }
            }
            actions
        }
        TargetRequirement::PermanentWithFilter(_) => {
            // Target any permanent on the battlefield matching a filter.
            // Actual filtering is done by the card's is_valid_target.
            let mut actions = Vec::new();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                if !can_be_targeted(state, obj.id, caster, registry) { continue; }
                let target = Target::Object(obj.id);
                if behavior.is_valid_target(state, caster, &target, registry) {
                    actions.push(Action::CastSpell {
                        object_id: spell_id,
                        targets: vec![target],
                    });
                }
            }
            actions
        }
        TargetRequirement::GraveyardCard | TargetRequirement::ExileCard => {
            let targets = valid_targets_for_req(state, caster, spell_id, target_req, behavior, registry);
            targets.into_iter()
                .map(|t| Action::CastSpell { object_id: spell_id, targets: vec![t] })
                .collect()
        }
        TargetRequirement::TwoTargets(ref req1, ref req2) => {
            // Generate Cartesian product of valid targets for each requirement.
            let targets1 = valid_targets_for_req(state, caster, spell_id, req1, behavior, registry);
            let targets2 = valid_targets_for_req(state, caster, spell_id, req2, behavior, registry);
            let mut actions = Vec::new();
            for t1 in &targets1 {
                for t2 in &targets2 {
                    if t1 != t2 {
                        let pair: Vec<crate::actions::Target> = vec![t1.clone(), t2.clone()];
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: pair,
                        });
                    }
                }
            }
            actions
        }
        TargetRequirement::UpToTargets(max, ref inner_req) => {
            // Generate all combinations of 1..=max targets for LLM/random expanded list.
            let options = valid_targets_for_req(state, caster, spell_id, inner_req, behavior, registry);
            let mut actions = Vec::new();
            for k in 1..=(*max).min(options.len()) {
                fn target_combinations(targets: &[crate::actions::Target], k: usize) -> Vec<Vec<crate::actions::Target>> {
                    if k == 0 { return vec![vec![]]; }
                    if targets.len() < k { return vec![]; }
                    let mut result = Vec::new();
                    for i in 0..=targets.len() - k {
                        for mut combo in target_combinations(&targets[i + 1..], k - 1) {
                            combo.insert(0, targets[i].clone());
                            result.push(combo);
                        }
                    }
                    result
                }
                for combo in target_combinations(&options, k) {
                    actions.push(Action::CastSpell {
                        object_id: spell_id,
                        targets: combo,
                    });
                }
            }
            actions
        }
    }
}

/// Helper: collect all valid targets for a single-target requirement.
fn valid_targets_for_req(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> Vec<crate::actions::Target> {
    use crate::actions::Target;
    use crate::cards::TargetRequirement;

    match req {
        TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| o.power.is_some())
                .filter(|o| can_be_targeted(state, o.id, caster, registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::Spell => {
            // Only spells on the stack can be targeted (not triggered abilities).
            state.stack.iter()
                .filter_map(|e| e.as_spell())
                .filter(|&id| id != spell_id)
                .map(|id| Target::Object(id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::PermanentWithFilter(_) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| can_be_targeted(state, o.id, caster, registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::AnyTarget => {
            let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| o.power.is_some())
                .filter(|o| can_be_targeted(state, o.id, caster, registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect();
            for p in &state.players {
                if !p.lost {
                    let t = Target::Player(p.id);
                    if behavior.is_valid_target(state, caster, &t, registry) {
                        targets.push(t);
                    }
                }
            }
            targets
        }
        TargetRequirement::PlayerOnly => {
            state.players.iter()
                .filter(|p| !p.lost)
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCard => {
            // All cards in all graveyards.
            state.objects.values()
                .filter(|o| o.zone == Zone::Graveyard)
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::ExileCard => {
            // All cards in exile owned by the caster.
            state.objects.values()
                .filter(|o| o.zone == Zone::Exile && o.owner == caster)
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        _ => vec![],
    }
}

/// Build a CastTargetSpec for a spell, describing what targets the player needs to choose.
fn build_cast_target_spec(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    target_req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
) -> crate::actions::CastTargetSpec {
    use crate::actions::CastTargetSpec;
    use crate::cards::TargetRequirement;
    let registry = &CardRegistry::with_all_cards();

    match target_req {
        TargetRequirement::None => CastTargetSpec::NoTargets,
        TargetRequirement::TwoTargets(req1, req2) => {
            let t1 = valid_targets_for_req(state, caster, spell_id, req1, behavior, registry);
            let t2 = valid_targets_for_req(state, caster, spell_id, req2, behavior, registry);
            CastTargetSpec::TwoTargets(t1, t2)
        }
        TargetRequirement::UpToTargets(max, inner_req) => {
            let options = valid_targets_for_req(state, caster, spell_id, inner_req, behavior, registry);
            CastTargetSpec::UpToTargets { max: *max, options }
        }
        // All single-target types
        _ => {
            let options = valid_targets_for_req(state, caster, spell_id, target_req, behavior, registry);
            CastTargetSpec::SingleTarget(options)
        }
    }
}

/// Generate valid targets for a targeted activated ability.
fn generate_ability_targets(
    state: &GameState,
    _source_id: ObjectId,
    ab: &crate::cards::ActivatedAbilityDef,
    controller: PlayerId,
    registry: &CardRegistry,
    behavior: &dyn crate::cards::CardBehavior,
) -> Vec<crate::actions::Target> {
    use crate::actions::Target;
    use crate::cards::TargetRequirement;

    let target_req = match &ab.target_requirement {
        Some(req) => req,
        None => return vec![],
    };

    match target_req {
        TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| o.power.is_some())
                .filter(|o| can_be_targeted(state, o.id, controller, registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::PlayerOnly => {
            state.players.iter()
                .filter(|p| !p.lost)
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::AnyTarget => {
            let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| o.power.is_some())
                .filter(|o| can_be_targeted(state, o.id, controller, registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect();
            for p in &state.players {
                if !p.lost {
                    let t = Target::Player(p.id);
                    if behavior.is_valid_target(state, controller, &t, registry) {
                        targets.push(t);
                    }
                }
            }
            targets
        }
        TargetRequirement::PermanentWithFilter(filter) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| can_be_targeted(state, o.id, controller, registry))
                .filter(|o| matches_target_filter(o, filter))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCard => {
            state.objects.values()
                .filter(|o| o.zone == Zone::Graveyard)
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::ExileCard => {
            state.objects.values()
                .filter(|o| o.zone == Zone::Exile && o.owner == controller)
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        _ => vec![],
    }
}

/// Check if a battlefield object matches a TargetFilter.
/// Used by generate_ability_targets to filter targets for activated abilities.
fn matches_target_filter(obj: &crate::state::GameObject, filter: &crate::cards::TargetFilter) -> bool {
    use crate::cards::TargetFilter;
    match filter {
        TargetFilter::Any => true,
        TargetFilter::HasCardType(types) => {
            types.iter().any(|t| obj.card_types.contains(t))
        }
        TargetFilter::Noncreature => obj.power.is_none(),
        TargetFilter::Nonblack => !obj.colors.contains(&crate::types::Color::Black),
        TargetFilter::HasSubtype(subtype) => obj.subtypes.iter().any(|s| s == subtype),
        _ => true, // Other filters not yet needed for abilities.
    }
}

// Attacker/blocker enumeration removed — players now construct combat
// actions from CombatPrompt data. The engine validates on submission.

/// Generate legal discard actions for hand size.
fn legal_discard_actions(state: &GameState, player: PlayerId, discard_count: usize) -> Vec<Action> {
    let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, player)
        .iter().map(|o| o.id).collect();

    if hand.len() <= discard_count {
        // Must discard all.
        return vec![Action::DiscardCards { cards: hand }];
    }

    // Enumerate all combinations of `discard_count` cards from hand.
    let combos = combinations(&hand, discard_count);
    combos.into_iter()
        .map(|cards| Action::DiscardCards { cards })
        .collect()
}

fn combinations(items: &[ObjectId], k: usize) -> Vec<Vec<ObjectId>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    let mut result = Vec::new();
    for i in 0..=items.len() - k {
        let rest = combinations(&items[i + 1..], k - 1);
        for mut combo in rest {
            combo.insert(0, items[i]);
            result.push(combo);
        }
    }
    result
}

fn card_name(state: &GameState, registry: &CardRegistry, obj_id: ObjectId) -> String {
    state.get_object(obj_id)
        .and_then(|o| registry.card_data(o.card_id))
        .map(|d| d.name)
        .unwrap_or_else(|| "?".into())
}

/// Apply an action to the game state and return the new state.
pub fn submit_action(state: &GameState, action: &Action, registry: &CardRegistry) -> GameState {
    let mut new_state = state.clone();
    new_state.events.clear();

    match action {
        Action::PassPriority => {
            let player = new_state.priority_player.unwrap_or(new_state.active_player);
            new_state.events.push(GameEvent::PriorityPassed { player });
            new_state.log(LogLevel::Debug, format!("p{} passes priority", player.0));
            new_state.consecutive_passes += 1;
        }

        Action::PlayLand { object_id } => {
            let player = new_state.priority_player.expect("PlayLand requires priority");
            new_state.move_object(*object_id, Zone::Battlefield);
            // Remove from library order if somehow there (shouldn't be, it's in hand).
            new_state.get_player_mut(player).land_plays_remaining -= 1;
            new_state.events.push(GameEvent::LandPlayed {
                player,
                object: *object_id,
            });
            // EnteredBattlefield is now emitted by move_object.
            // Lands don't have summoning sickness (only creatures care).
            if let Some(obj) = new_state.get_object_mut(*object_id) {
                obj.summoning_sick = false;
            }
            let name = card_name(&new_state, registry, *object_id);
            new_state.log(LogLevel::Info, format!("p{} played {}", player.0, name));
            new_state.consecutive_passes = 0;
        }

        Action::CastSpell { object_id, targets } => {
            let player = new_state.priority_player.expect("CastSpell requires priority");

            // Detect flashback: card is being cast from the graveyard.
            let is_flashback = new_state.get_object(*object_id)
                .map(|o| o.zone == Zone::Graveyard)
                .unwrap_or(false);

            // Pay the appropriate mana cost (applying cost reduction for non-flashback).
            let card_id = new_state.get_object(*object_id).expect("CastSpell object must exist").card_id;
            let data = registry.get(card_id).expect("card must be in registry").card_data();
            let cost = if is_flashback {
                // Check until_end_of_turn_flashback for dynamically granted flashback.
                let dynamic_fb = new_state.until_end_of_turn_flashback.iter()
                    .find(|(id, _)| *id == *object_id)
                    .map(|(_, c)| c.clone());
                dynamic_fb.unwrap_or_else(|| {
                    data.flashback_cost.expect("flashback cast on card without flashback_cost")
                })
            } else {
                let base_cost = data.cost.expect("non-flashback spell must have a mana cost");
                effective_spell_cost(&new_state, registry, card_id, &base_cost, player)
            };

            // Handle X-cost spells: compute X from remaining mana after paying colored requirements.
            let has_x = cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X));
            let x_value = if has_x {
                // Non-X cost components (colored + generic).
                let non_x_cost = ManaCost::new(
                    cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                );
                let pool = &new_state.get_player(player).mana_pool;
                let total_mana = pool.total();
                let non_x_amount = non_x_cost.mana_value();
                let x = total_mana.saturating_sub(non_x_amount);
                Some(x)
            } else {
                None
            };

            if has_x {
                // Pay non-X cost first.
                let non_x_cost = ManaCost::new(
                    cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                );
                mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &non_x_cost)
                    .expect("legal_actions should have verified mana availability");
                // Pay remaining mana as X (drain the pool).
                new_state.get_player_mut(player).mana_pool.empty();
            } else {
                mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &cost)
                    .expect("legal_actions should have verified mana availability");
            }

            // Move to stack and store targets.
            new_state.move_object(*object_id, Zone::Stack);
            {
                let obj = new_state.get_object_mut(*object_id).expect("spell must exist after moving to stack");
                obj.targets = targets.clone();
                if is_flashback {
                    obj.cast_with_flashback = true;
                }
                if let Some(x) = x_value {
                    obj.x_value = Some(x);
                }
            }
            new_state.stack.push(crate::state::StackEntry::Spell(*object_id));

            new_state.events.push(GameEvent::SpellCast {
                player,
                object: *object_id,
            });

            let name = card_name(&new_state, registry, *object_id);
            let suffix = if is_flashback { " (flashback)" } else { "" };
            new_state.log(LogLevel::Event, format!("p{} cast {}{}", player.0, name, suffix));
            new_state.consecutive_passes = 0;
        }

        Action::ActivateManaAbility { object_id, ability_index } => {
            let obj = new_state.get_object(*object_id).expect("activated ability object must exist");
            let card_id = obj.card_id;
            let controller = obj.controller;

            if let Some(behavior) = registry.get(card_id) {
                let abilities = behavior.mana_abilities(&new_state, *object_id);
                if let Some(ability) = abilities.get(*ability_index) {
                    if ability.requires_tap {
                        new_state.get_object_mut(*object_id).expect("object must exist for tapping").tapped = true;
                        new_state.events.push(GameEvent::Tapped { object: *object_id });
                    }
                    for &(mana_type, amount) in &ability.produced {
                        new_state.get_player_mut(controller).mana_pool.add(mana_type, amount);
                        new_state.events.push(GameEvent::ManaAdded {
                            player: controller,
                            mana_type,
                            amount,
                        });
                    }
                    // Call card-specific mana ability callback (e.g., Deranged Assistant mills).
                    behavior.on_activate_mana_ability(&mut new_state, *object_id, *ability_index, registry);
                }
            }

            let name = card_name(&new_state, registry, *object_id);
            new_state.log(LogLevel::Debug, format!("p{} tapped {} for mana", controller.0, name));
        }

        Action::ActivateAbility { object_id, ability_index, targets } => {
            let player = new_state.priority_player.expect("ActivateAbility requires priority");
            let obj = new_state.get_object(*object_id).expect("activated ability object must exist");
            let card_id = obj.card_id;

            // Find the ability — check the permanent's own card, then attached auras.
            let ability = registry.get(card_id)
                .and_then(|b| b.activated_abilities(&new_state, *object_id)
                    .into_iter().find(|a| a.ability_index == *ability_index))
                .or_else(|| {
                    // Check attached auras.
                    new_state.objects.values()
                        .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(*object_id))
                        .find_map(|a| {
                            registry.get(a.card_id)
                                .and_then(|b| b.activated_abilities(&new_state, *object_id)
                                    .into_iter().find(|ab| ab.ability_index == *ability_index))
                        })
                });

            if let Some(ab) = ability {
                // Pay mana cost.
                mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &ab.cost)
                    .expect("legal_actions should have verified mana availability");

                // Pay tap cost.
                if ab.requires_tap {
                    new_state.get_object_mut(*object_id).expect("object must exist for tapping").tapped = true;
                }

                // Pay sacrifice cost.
                use crate::cards::SacrificeCost;
                match &ab.sacrifice_cost {
                    SacrificeCost::None => {}
                    SacrificeCost::SacrificeThis => {
                        crate::destruction::sacrifice(&mut new_state, *object_id, registry);
                    }
                    SacrificeCost::SacrificeCreature => {
                        // For now, auto-sacrifice the first eligible creature.
                        // TODO: Present choice to player when there are multiple options.
                        let creature = new_state.objects_in_zone(Zone::Battlefield, player)
                            .iter()
                            .find(|o| o.power.is_some())
                            .map(|o| o.id);
                        if let Some(cid) = creature {
                            crate::destruction::sacrifice(&mut new_state, cid, registry);
                        }
                    }
                }

                // Track once-per-turn.
                if ab.once_per_turn {
                    if let Some(obj) = new_state.get_object_mut(*object_id) {
                        obj.abilities_activated_this_turn.insert(*ability_index);
                    }
                }

                // Find which behavior to call (card itself or attached aura).
                let behavior_card_id = if registry.get(card_id)
                    .map(|b| !b.activated_abilities(&new_state, *object_id).is_empty())
                    .unwrap_or(false)
                {
                    card_id
                } else {
                    // Must be from an attached aura.
                    new_state.objects.values()
                        .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(*object_id))
                        .find(|a| {
                            registry.get(a.card_id)
                                .map(|b| !b.activated_abilities(&new_state, *object_id).is_empty())
                                .unwrap_or(false)
                        })
                        .map(|a| a.card_id)
                        .unwrap_or(card_id)
                };

                if let Some(behavior) = registry.get(behavior_card_id) {
                    behavior.on_activate_ability(&mut new_state, *object_id, *ability_index, targets, registry);
                }

                let name = card_name(&new_state, registry, *object_id);
                new_state.log(LogLevel::Event, format!("p{} activated ability on {}: {}", player.0, name, ab.description));
            }
        }

        Action::DeclareAttackers { attackers } => {
            if attackers.is_empty() {
                new_state.log(LogLevel::Debug, "No attackers declared".into());
            } else {
                let names: Vec<String> = attackers.iter()
                    .map(|(id, _)| card_name(state, registry, *id))
                    .collect();
                new_state.log(LogLevel::Event, format!("p{} declared attackers: {}", new_state.active_player.0, names.join(", ")));
            }
            combat::declare_attackers(&mut new_state, attackers, registry);

            // Collect forced attackers (creatures with "attacks each combat if able" aura).
            let forced_ids: Vec<crate::ids::ObjectId> = {
                let active = new_state.active_player;
                let mut forced = Vec::new();
                for creature in new_state.objects.values() {
                    if creature.zone != Zone::Battlefield || creature.controller != active
                        || creature.power.is_none() || creature.tapped || creature.summoning_sick {
                        continue;
                    }
                    if new_state.combat.as_ref().map(|c| c.attackers.contains_key(&creature.id)).unwrap_or(false) {
                        continue; // already attacking
                    }
                    // Check for Defender — can't be forced to attack.
                    if new_state.has_keyword(creature.id, crate::types::Keyword::Defender, registry) {
                        continue;
                    }
                    // Check for forced attack effects (e.g., Furor of the Bitten).
                    let must_attack = new_state.has_continuous_effect(creature.id, &|e| {
                        match e {
                            crate::types::ContinuousEffect::ForceAttack { scope } => Some(scope),
                            _ => None,
                        }
                    }, registry);
                    if must_attack {
                        forced.push(creature.id);
                    }
                }
                forced
            };

            // Add forced attackers to combat.
            if !forced_ids.is_empty() {
                let defending = new_state.opponent(new_state.active_player);
                if let Some(ref mut combat) = new_state.combat {
                    for id in &forced_ids {
                        if !combat.attackers.contains_key(id) {
                            combat.attackers.insert(*id, defending);
                            combat.blocker_assignments.insert(*id, Vec::new());
                        }
                    }
                }
                // Tap forced attackers (unless vigilance).
                for id in &forced_ids {
                    let has_vig = new_state.has_keyword(*id, crate::types::Keyword::Vigilance, registry);
                    if !has_vig {
                        if let Some(obj) = new_state.get_object_mut(*id) {
                            if !obj.tapped {
                                obj.tapped = true;
                            }
                        }
                    }
                }
                let names: Vec<String> = forced_ids.iter()
                    .map(|id| card_name(&new_state, registry, *id))
                    .collect();
                new_state.log(LogLevel::Event, format!("Forced attackers: {}", names.join(", ")));
            }

            new_state.awaiting_action = None;
            new_state.consecutive_passes = 0;
        }

        Action::DeclareBlockers { assignments } => {
            // The defending player is the opponent of the active player.
            let defender = new_state.opponent(new_state.active_player);
            if assignments.is_empty() {
                new_state.log(LogLevel::Info, format!("p{} declared no blockers", defender.0));
            } else {
                let descs: Vec<String> = assignments.iter()
                    .map(|(b, a)| format!("{} blocks {}", card_name(state, registry, *b), card_name(state, registry, *a)))
                    .collect();
                new_state.log(LogLevel::Event, format!("p{} declared blockers: {}", defender.0, descs.join(", ")));
            }
            combat::declare_blockers_with_registry(&mut new_state, assignments, registry);
            new_state.awaiting_action = None;
            new_state.consecutive_passes = 0;
        }

        Action::DiscardCards { cards } => {
            let player = match &new_state.awaiting_action {
                Some(AwaitingAction::DiscardToHandSize { player, .. }) => *player,
                _ => new_state.active_player,
            };
            for &card_id in cards {
                new_state.events.push(GameEvent::Discarded { player, object: card_id });
                new_state.move_object(card_id, Zone::Graveyard);
            }
            new_state.awaiting_action = None;
        }

        Action::Concede => {
            if let Some(player) = new_state.priority_player {
                new_state.log(LogLevel::Milestone, format!("p{} concedes", player.0));
                new_state.get_player_mut(player).lost = true;
                new_state.events.push(GameEvent::PlayerLost {
                    player,
                    reason: crate::events::LossReason::Conceded,
                });
            }
        }

        Action::ActivateLoyaltyAbility { object_id, ability_index } => {
            let player = new_state.priority_player.expect("ActivateLoyaltyAbility requires priority");
            if let Some(behavior) = registry.get(
                new_state.get_object(*object_id).map(|o| o.card_id).unwrap_or(crate::ids::CardId(0))
            ) {
                let abilities = behavior.loyalty_abilities();
                if let Some(ab) = abilities.iter().find(|a| a.ability_index == *ability_index) {
                    // Pay loyalty cost: add or remove loyalty counters.
                    let change = ab.loyalty_change;
                    if change > 0 {
                        new_state.add_counters(*object_id, CounterType::Loyalty, change as u32);
                    } else if change < 0 {
                        let remove = (-change) as u32;
                        if let Some(obj) = new_state.get_object_mut(*object_id) {
                            let current = obj.counters.entry(CounterType::Loyalty).or_insert(0);
                            *current = current.saturating_sub(remove);
                        }
                    }
                    // Mark that a loyalty ability was activated this turn on this permanent.
                    if let Some(obj) = new_state.get_object_mut(*object_id) {
                        obj.abilities_activated_this_turn.insert(999); // sentinel for "used loyalty this turn"
                    }
                    behavior.on_loyalty_ability(&mut new_state, *object_id, *ability_index, registry);
                    let name = card_name(&new_state, registry, *object_id);
                    new_state.log(LogLevel::Event, format!("p{} activated loyalty ability on {}: {}", player.0, name, ab.description));
                }
            }
        }

        Action::ResolveChoice { choice: resolved } => {
            use crate::state::ResolutionChoiceKind;
            use crate::actions::ResolvedChoice;
            let awaiting = new_state.awaiting_action.take();
            if let Some(AwaitingAction::ResolutionChoice { choice: kind, .. }) = awaiting {
                match (&kind, resolved) {
                    (ResolutionChoiceKind::PayOrNot { spell_id, source_spell_id, .. },
                     ResolvedChoice::PayDecision(pay)) => {
                        if !*pay {
                            let name = new_state.get_object(*spell_id).map(|o| o.name.clone()).unwrap_or_default();
                            new_state.stack.retain(|e| e.as_spell() != Some(*spell_id));
                            new_state.move_spell_after_resolve(*spell_id);
                            new_state.log(LogLevel::Event, format!("{} was countered", name));
                        } else {
                            // Deduct {1} from the player's mana pool.
                            let controller = new_state.get_object(*spell_id).map(|o| o.controller).unwrap_or(PlayerId(0));
                            let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
                            let _ = mana::auto_pay(&mut new_state.get_player_mut(controller).mana_pool, &cost);
                            new_state.log(LogLevel::Event, "Paid {1} to prevent counter".into());
                        }
                        // Controller discards a card — player chooses which.
                        let controller = new_state.get_object(*spell_id).map(|o| o.controller).unwrap_or(PlayerId(0));
                        let hand: Vec<_> = new_state.objects_in_zone(Zone::Hand, controller)
                            .iter().map(|o| o.id).collect();
                        if hand.len() == 1 {
                            new_state.move_object(hand[0], Zone::Graveyard);
                            new_state.events.push(GameEvent::Discarded { player: controller, object: hand[0] });
                            new_state.log(LogLevel::Event, format!("p{} discarded a card", controller.0));
                        } else if !hand.is_empty() {
                            new_state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                                player: controller,
                                source: *source_spell_id,
                                choice: ResolutionChoiceKind::ChooseCardFromHand {
                                    description: "Frightful Delusion: choose a card to discard".into(),
                                    player: controller,
                                    cards: hand,
                                },
                            });
                            // Move the spell to graveyard before the discard choice.
                            new_state.move_spell_after_resolve(*source_spell_id);
                            return new_state;
                        }
                        new_state.move_spell_after_resolve(*source_spell_id);
                    }
                    (ResolutionChoiceKind::YesNo { source_card, .. },
                     ResolvedChoice::PayDecision(yes)) => {
                        if *yes {
                            // "You may draw a card. If you do, discard a card."
                            let controller = new_state.get_object(*source_card)
                                .map(|o| o.controller).unwrap_or(PlayerId(0));
                            draw_cards(&mut new_state, controller, 1);
                            let hand: Vec<_> = new_state.objects_in_zone(Zone::Hand, controller)
                                .iter().map(|o| o.id).collect();
                            if hand.len() == 1 {
                                new_state.move_object(hand[0], Zone::Graveyard);
                                new_state.events.push(GameEvent::Discarded { player: controller, object: hand[0] });
                                new_state.log(LogLevel::Event, format!("Drew and discarded a card"));
                            } else if !hand.is_empty() {
                                new_state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                                    player: controller,
                                    source: *source_card,
                                    choice: ResolutionChoiceKind::ChooseCardFromHand {
                                        description: "Murder of Crows: choose a card to discard".into(),
                                        player: controller,
                                        cards: hand,
                                    },
                                });
                            }
                        }
                        // If no, nothing happens.
                    }
                    (ResolutionChoiceKind::ChooseTarget { effect, .. },
                     ResolvedChoice::ChosenTarget(target)) => {
                        if let Some(t) = target {
                            apply_pending_effect(&mut new_state, t, effect, registry);
                        }
                    }
                    (ResolutionChoiceKind::ChooseCardFromHand { .. },
                     ResolvedChoice::ChosenCard(discard_id)) => {
                        let name = new_state.get_object(*discard_id).map(|o| o.name.clone()).unwrap_or_default();
                        new_state.move_object(*discard_id, Zone::Graveyard);
                        new_state.events.push(GameEvent::Discarded {
                            player: new_state.get_object(*discard_id).map(|o| o.owner).unwrap_or(PlayerId(0)),
                            object: *discard_id,
                        });
                        new_state.log(LogLevel::Event, format!("Discarded {}", name));
                    }
                    (ResolutionChoiceKind::ChooseFromRevealed { revealed, spell_id, .. },
                     ResolvedChoice::ChosenCard(keep_id)) => {
                        let keep_name = new_state.get_object(*keep_id).map(|o| o.name.clone()).unwrap_or_default();
                        new_state.move_object(*keep_id, Zone::Hand);
                        for &card_id in revealed {
                            if card_id != *keep_id {
                                new_state.move_object(card_id, Zone::Graveyard);
                            }
                        }
                        new_state.log(LogLevel::Event, format!("Kept {}", keep_name));
                        new_state.move_spell_after_resolve(*spell_id);
                    }
                    _ => {}
                }
            }
            new_state.consecutive_passes = 0;
        }
    }

    new_state
}

/// Apply a pending effect from a resolution choice to a target.
pub fn apply_pending_effect(state: &mut GameState, target: &crate::actions::Target, effect: &crate::state::PendingEffect, registry: &CardRegistry) {
    use crate::actions::Target;
    use crate::state::PendingEffect;

    match (target, effect) {
        (Target::Object(id), PendingEffect::DealDamage { amount, source_id, source_name }) => {
            if let Some(obj) = state.get_object_mut(*id) {
                if obj.zone == Zone::Battlefield {
                    obj.damage_marked += amount;
                    obj.damaged_by.push(*source_id);
                    let name = obj.name.clone();
                    state.events.push(GameEvent::NonCombatDamageDealt {
                        source: *source_id,
                        target: crate::events::DamageTarget::Object(*id),
                        amount: *amount,
                    });
                    state.log(LogLevel::Event, format!("{} dealt {} damage to {}", source_name, amount, name));
                }
            }
        }
        (Target::Player(pid), PendingEffect::DealDamage { amount, source_id, source_name }) => {
            let old = state.get_player(*pid).life;
            let new_life = old - *amount as i32;
            state.get_player_mut(*pid).life = new_life;
            state.events.push(GameEvent::NonCombatDamageDealt {
                source: *source_id,
                target: crate::events::DamageTarget::Player(*pid),
                amount: *amount,
            });
            state.events.push(GameEvent::LifeChanged { player: *pid, old, new_life });
            state.log(LogLevel::Event, format!("{} dealt {} damage to p{}", source_name, amount, pid.0));
        }
        (Target::Object(id), PendingEffect::Destroy { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, *id, registry);
            state.log(LogLevel::Event, format!("{} destroyed {}", source_name, name));
        }
        (Target::Object(id), PendingEffect::ReturnToBattlefield { spell_id }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*id, Zone::Battlefield);
            state.log(LogLevel::Event, format!("{} returned to the battlefield", name));
            state.move_spell_after_resolve(*spell_id);
        }
        (Target::Object(id), PendingEffect::AddCounters { count, human_bonus }) => {
            let mut final_count = *count;
            if *human_bonus {
                let is_human = state.get_object(*id)
                    .and_then(|o| registry.card_data(o.card_id))
                    .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                    .unwrap_or(false);
                if is_human {
                    final_count = count * 2;
                }
            }
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.add_counters(*id, crate::types::CounterType::PlusOnePlusOne, final_count);
            state.log(LogLevel::Event,
                format!("Added {} +1/+1 counter{} to {}", final_count, if final_count > 1 { "s" } else { "" }, name));
        }
        (Target::Object(id), PendingEffect::DebuffUntilEOT { power, toughness, source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.until_end_of_turn_effects.push(crate::state::UntilEndOfTurnEffect {
                target: *id,
                power_mod: *power,
                toughness_mod: *toughness,
            });
            state.log(LogLevel::Event, format!("{} gave {} {}/{} until end of turn", source_name, name, power, toughness));
        }
        (Target::Object(id), PendingEffect::CantBlockThisTurn { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.until_end_of_turn_cant_block.push(*id);
            state.log(LogLevel::Event, format!("{} prevents {} from blocking this turn", source_name, name));
        }
        (Target::Player(pid), PendingEffect::Mill { count, source_name }) => {
            mill_cards(state, *pid, *count as usize);
            state.log(LogLevel::Event, format!("{} milled {} card(s) from p{}", source_name, count, pid.0));
        }
        (Target::Object(id), PendingEffect::ExileAndStore { source_id, source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*id, Zone::Exile);
            // Store the exiled creature's ID on the source permanent for LTB retrieval.
            if let Some(source_obj) = state.get_object_mut(*source_id) {
                source_obj.card_state.insert("exiled_creature".into(), *id);
            }
            state.log(LogLevel::Event, format!("{} exiled {}", source_name, name));
        }
        (Target::Player(pid), PendingEffect::DrawAndLoseLife { source_name }) => {
            draw_cards(state, *pid, 1);
            let old = state.get_player(*pid).life;
            let new_life = old - 1;
            state.get_player_mut(*pid).life = new_life;
            state.events.push(GameEvent::LifeChanged { player: *pid, old, new_life });
            state.log(LogLevel::Event, format!("{}: p{} drew a card and lost 1 life", source_name, pid.0));
        }
        (Target::Object(id), PendingEffect::DestroyCreature { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, *id, registry);
            state.log(LogLevel::Event, format!("{} destroyed {}", source_name, name));
        }
        (Target::Object(id), PendingEffect::ExileCurseOfOblivion { remaining }) => {
            let owner = state.get_object(*id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
            state.move_object(*id, Zone::Exile);
            state.log(LogLevel::Event, format!("Curse of Oblivion: exiled a card from p{}'s graveyard", owner.0));
            // If more cards to exile, present another choice.
            if *remaining > 0 {
                let gy_cards: Vec<Target> = state.objects_in_zone(Zone::Graveyard, owner)
                    .iter()
                    .map(|o| Target::Object(o.id))
                    .collect();
                if !gy_cards.is_empty() {
                    state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                        player: owner,
                        source: crate::ids::ObjectId(0), // curse source
                        choice: crate::state::ResolutionChoiceKind::ChooseTarget {
                            description: "Curse of Oblivion: choose another card to exile".into(),
                            options: gy_cards,
                            optional: false,
                            effect: PendingEffect::ExileCurseOfOblivion { remaining: remaining - 1 },
                        },
                    });
                }
            }
        }
        (Target::Object(id), PendingEffect::ReturnToHand { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*id, Zone::Hand);
            state.log(LogLevel::Event, format!("{}: returned {} to hand", source_name, name));
        }
        (Target::Object(id), PendingEffect::PutOnTopOfLibrary { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            let owner = state.get_object(*id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
            state.move_object(*id, Zone::Library);
            // Insert at position 0 (top of library).
            state.get_player_mut(owner).library_order.insert(0, *id);
            state.log(LogLevel::Event, format!("{}: put {} on top of library", source_name, name));
        }
        (Target::Object(id), PendingEffect::SacrificeAndGainLife { beneficiary, spell_id }) => {
            // Get the creature's toughness before sacrificing.
            let toughness = state.effective_toughness(*id, registry)
                .or_else(|| state.get_object(*id).and_then(|o| o.toughness))
                .unwrap_or(0);
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();

            crate::destruction::sacrifice(state, *id, registry);

            // Gain life equal to the creature's toughness.
            if toughness > 0 {
                let old = state.get_player(*beneficiary).life;
                let new_life = old + toughness;
                state.get_player_mut(*beneficiary).life = new_life;
                state.events.push(GameEvent::LifeChanged { player: *beneficiary, old, new_life });
                state.log(LogLevel::Event, format!("Tribute to Hunger: sacrificed {}, p{} gained {} life",
                    name, beneficiary.0, toughness));
            } else {
                state.log(LogLevel::Event, format!("Tribute to Hunger: sacrificed {}", name));
            }

            state.move_spell_after_resolve(*spell_id);
        }
        _ => {}
    }
}

/// Set up a new game: create objects, shuffle libraries, draw opening hands.
pub fn setup_game(config: &GameConfig, registry: &CardRegistry) -> GameState {
    let num_players = config.player_names.len() as u8;
    let mut state = GameState::new(num_players);

    // Set starting life.
    for p in &mut state.players {
        p.life = config.starting_life;
    }

    // Create card objects for each player's deck.
    let mut rng = rand::thread_rng();
    for (player_idx, decklist) in config.decklists.iter().enumerate() {
        let player_id = PlayerId(player_idx as u8);
        let mut library_ids = Vec::new();

        for (card_name, count) in &decklist.entries {
            let card_id = registry.get_id_by_name(card_name)
                .unwrap_or_else(|| panic!("Unknown card: {}", card_name));

            let card_data = registry.card_data(card_id).expect("card must be in registry");

            // Derive colors from mana cost.
            let colors: Vec<Color> = card_data.cost.as_ref()
                .map(|cost| {
                    let mut c = Vec::new();
                    for sym in &cost.symbols {
                        if let ManaSymbol::Colored(color) = sym {
                            if !c.contains(color) {
                                c.push(*color);
                            }
                        }
                    }
                    c
                })
                .unwrap_or_default();

            for _ in 0..*count {
                let obj_id = state.create_object(
                    card_id,
                    player_id,
                    Zone::Library,
                    card_data.power,
                    card_data.toughness,
                );
                let obj = state.get_object_mut(obj_id).expect("object must exist for library draw");
                obj.colors = colors.clone();
                obj.name = card_name.clone();
                obj.keywords = card_data.keywords.clone();
                obj.card_types = card_data.card_types.clone();
                library_ids.push(obj_id);
            }
        }

        // Shuffle the library.
        library_ids.shuffle(&mut rng);
        state.get_player_mut(player_id).library_order = library_ids;
    }

    state.log(LogLevel::Milestone, "Game started".into());

    // Draw opening hands (7 cards each).
    for player_idx in 0..num_players {
        let player_id = PlayerId(player_idx);
        draw_cards(&mut state, player_id, 7);
    }

    state.events.push(GameEvent::GameStarted);
    state.log(LogLevel::Milestone, format!("── Turn 1 (p0) ──"));
    state
}

/// Draw N cards for a player. Logs a single summary entry.
pub fn draw_cards(state: &mut GameState, player: PlayerId, count: usize) {
    let mut drawn = 0;
    for _ in 0..count {
        let card_id = {
            let player_state = state.get_player_mut(player);
            player_state.draw_top_card()
        };
        match card_id {
            Some(id) => {
                state.move_object(id, Zone::Hand);
                state.events.push(GameEvent::CardDrawn { player, object: id });
                drawn += 1;
            }
            None => {
                // Check for Laboratory Maniac: if the player controls one,
                // they win the game instead of losing from empty library draw.
                let has_lab_maniac = state.objects.values().any(|o| {
                    o.zone == Zone::Battlefield
                        && o.controller == player
                        && o.name == "Laboratory Maniac"
                });
                if has_lab_maniac {
                    // Player wins the game instead of drawing from empty library.
                    // Clear the has_drawn_from_empty flag so SBA doesn't kill them.
                    state.get_player_mut(player).has_drawn_from_empty = false;
                    let opponent = state.opponent(player);
                    state.players[opponent.0 as usize].lost = true;
                    state.events.push(GameEvent::PlayerLost {
                        player: opponent,
                        reason: crate::events::LossReason::LifeReachedZero, // closest reason
                    });
                    state.result = Some(crate::state::GameResult::Winner(player));
                    state.log(LogLevel::Milestone,
                        format!("p{} wins the game with Laboratory Maniac!", player.0));
                }
                // Otherwise SBA will catch the empty library draw.
                break;
            }
        }
    }
    if drawn > 0 {
        if drawn == 1 {
            state.log(LogLevel::Info, format!("p{} drew a card", player.0));
        } else {
            state.log(LogLevel::Info, format!("p{} drew {} cards", player.0, drawn));
        }
    }
}

/// Mill N cards from a player's library (move top N cards to graveyard).
pub fn mill_cards(state: &mut GameState, player: PlayerId, count: usize) {
    let mut milled = 0;
    for _ in 0..count {
        let card_id = {
            let player_state = state.get_player_mut(player);
            if player_state.library_order.is_empty() {
                break;
            }
            player_state.library_order.remove(0)
        };
        state.move_object(card_id, Zone::Graveyard);
        milled += 1;
    }
    if milled > 0 {
        state.log(LogLevel::Event, format!("p{} milled {} card{}", player.0, milled, if milled == 1 { "" } else { "s" }));
    }
}

/// Check if a player could cast any spell if they tapped all available mana sources.
/// Used by the auto-pass check to avoid skipping turns where mana abilities are
/// the only listed actions but the player has castable spells.
fn has_castable_with_potential_mana(
    state: &GameState,
    player: PlayerId,
    registry: &CardRegistry,
) -> bool {
    // Build potential mana pool: current pool + all activatable mana abilities.
    let mut potential = state.get_player(player).mana_pool.clone();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            for ma in behavior.mana_abilities(state, obj.id) {
                if !ma.requires_tap || !obj.tapped {
                    for &(mana_type, amount) in &ma.produced {
                        potential.add(mana_type, amount);
                    }
                }
            }
        }
    }

    // Check if any spell in hand could be cast with this potential mana.
    // For instant-speed spells, only count them as meaningful when something
    // interesting is happening (stack items, active combat). This prevents
    // prompting at every step just because the player has an instant + mana.
    let is_sorcery_speed = state.step.is_main_phase()
        && state.stack.is_empty()
        && state.active_player == player;
    let stack_has_items = !state.stack.is_empty();
    // Instants are relevant during Declare Attackers / Declare Blockers
    // (key combat trick windows), but not during Combat Damage / End Combat.
    let in_key_combat_step = state.combat.as_ref()
        .map(|c| !c.attackers.is_empty())
        .unwrap_or(false)
        && matches!(state.step, Step::DeclareAttackers | Step::DeclareBlockers);
    let instants_relevant = stack_has_items || in_key_combat_step;

    for obj in state.objects_in_zone(Zone::Hand, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            let data = behavior.card_data();
            // Check timing — for instants, only consider them when the stack
            // has items (responding to something). Otherwise auto-pass.
            let is_instant = data.card_types.contains(&CardType::Instant);
            let has_flash = data.keywords.contains(&Keyword::Flash);
            let can_cast_timing = if is_instant || has_flash {
                instants_relevant
            } else if data.card_types.contains(&CardType::Sorcery)
                || data.card_types.contains(&CardType::Creature)
                || data.card_types.contains(&CardType::Enchantment)
                || data.card_types.contains(&CardType::Artifact)
            {
                is_sorcery_speed
            } else {
                false
            };
            if !can_cast_timing { continue; }

            // Check if potential mana could pay the cost.
            if let Some(cost) = &data.cost {
                if !mana::can_pay(&potential, cost) {
                    continue;
                }
            }

            // Check if the spell has valid targets (or needs none).
            let target_req = behavior.target_requirement();
            let cast_actions = generate_cast_actions_with_targets(
                state, player, obj.id, &target_req, behavior,
            );
            if !cast_actions.is_empty() {
                return true;
            }
        }
    }

    // Also check activated abilities that cost mana.
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        if let Some(behavior) = registry.get(obj.card_id) {
            for ab in behavior.activated_abilities(state, obj.id) {
                if mana::can_pay(&potential, &ab.cost) {
                    if !ab.requires_tap || !obj.tapped {
                        return true;
                    }
                }
            }
        }
    }

    false
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

    let next = state.step.next();
    match next {
        Some(next_step) => {
            state.step = next_step;
        }
        None => {
            // End of turn: advance to next player's turn.
            let next_player = state.next_player(state.active_player);
            state.active_player = next_player;
            state.turn_number += 1;
            state.step = Step::Untap;
            state.is_first_turn = false;
            state.creature_died_this_turn = false;

            state.events.push(GameEvent::TurnStarted {
                player: next_player,
                turn: state.turn_number,
            });
            state.log(LogLevel::Milestone, format!("── Turn {} (p{}) ──", state.turn_number, next_player.0));
        }
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
                draw_cards(state, active, 1);
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
                .map(|c| !c.attackers.is_empty())
                .unwrap_or(false);

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
                .map(|c| !c.attackers.is_empty())
                .unwrap_or(false);

            if has_attackers {
                combat::deal_combat_damage(state, registry);
            }
            // No priority in combat damage step for Phase 1.
            // (Technically there should be, but no instants yet.)
            state.priority_player = Some(active);
        }

        Step::EndCombat => {
            combat::end_combat(state);
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
            state.until_end_of_turn_effects.clear();
            state.until_end_of_turn_keywords.clear();
            state.until_end_of_turn_cant_block.clear();
            state.until_end_of_turn_protection.clear();
            state.until_end_of_turn_removed_keywords.clear();

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
            let sba_fired = crate::sba::check_state_based_actions_with_registry(state, Some(registry_ref));
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
    // Start with the first turn's turn-based actions.
    state.events.push(GameEvent::TurnStarted {
        player: state.active_player,
        turn: state.turn_number,
    });
    perform_turn_based_actions(state, registry);

    run_game_loop_inner(state, registry, &mut choose_action);
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
    let num_players = state.players.len() as u32;
    let mut auto_pass_count = 0u32;
    const MAX_AUTO_PASSES: u32 = 100;

    loop {
        if state.is_game_over() {
            break;
        }

        // Process triggers from the last action, then SBA+trigger loop.
        triggers::process_triggers(state, registry);
        loop {
            let sba = check_state_based_actions_with_registry(state, Some(registry));
            if sba {
                triggers::process_triggers(state, registry);
            }
            if !sba { break; }
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
        } else {
            match state.priority_player {
                Some(p) => p,
                None => {
                    advance_step(state, registry);
                    continue;
                }
            }
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
                    if !state.stack.is_empty() {
                        let mut new_state = state.clone();
                        stack::resolve_top_of_stack(&mut new_state, registry);
                        *state = new_state;
                        state.consecutive_passes = 0;
                        state.priority_player = Some(state.active_player);
                    } else {
                        // Stack empty, advance step.
                        state.priority_player = None;
                        advance_step(state, registry);
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

            Action::ActivateManaAbility { .. } | Action::ActivateAbility { .. } | Action::ActivateLoyaltyAbility { .. } => {
                // Player retains priority. Don't change anything.
            }

            Action::Concede => {
                // SBAs will handle the game ending.
            }

            Action::PlayLand { .. } | Action::CastSpell { .. } => {
                // Player retains priority after these actions.
            }

            Action::ResolveChoice { .. } => {
                // After resolving a choice, return priority to active player.
                // Triggers may continue processing in the next loop iteration.
                state.priority_player = Some(state.active_player);
            }
        }
    }
}
