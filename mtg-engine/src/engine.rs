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
    /// Human-readable description of why the player has priority or needs to act.
    pub context: Option<String>,
}

/// Check if Rooftop Storm is on the battlefield and provides a {0} alternative cost
/// for a Zombie creature spell cast by the given player.
fn rooftop_storm_applies(state: &GameState, registry: &CardRegistry, card_id: CardId, caster: PlayerId) -> bool {
    // Check if the spell is a Zombie creature spell.
    let is_zombie_creature = registry.card_data(card_id).map(|d| {
        d.card_types.contains(&CardType::Creature)
            && d.subtypes.iter().any(|s| s == "Zombie")
    }).unwrap_or(false);
    if !is_zombie_creature {
        return false;
    }
    // Check if the caster controls a Rooftop Storm on the battlefield.
    state.objects.values().any(|o| {
        o.zone == Zone::Battlefield
            && o.controller == caster
            && o.name == "Rooftop Storm"
    })
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

    // Rooftop Storm's alternative cost is handled via the alternative_cost field
    // on CastSpell actions (see rooftop_storm_applies() and action generation).
    // It is NOT a cost reduction — it's an alternative cost chosen at cast time.

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
        return LegalActions { actions: vec![], combat_prompt: None, castable_spells: vec![], context: None };
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
                    context: Some("DECLARE ATTACKERS".into()),
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
                    context: Some("DECLARE BLOCKERS".into()),
                }
            }
            AwaitingAction::DiscardToHandSize { player, discard_count } => {
                LegalActions {
                    actions: legal_discard_actions(state, *player, *discard_count),
                    combat_prompt: None,
                    castable_spells: vec![],
                    context: Some(format!("DISCARD {} CARD{}", discard_count,
                        if *discard_count == 1 { "" } else { "S" })),
                }
            }
            AwaitingAction::ResolutionChoice { choice, source, .. } => {
                use crate::state::ResolutionChoiceKind;
                use crate::actions::ResolvedChoice;
                let source_name = card_name(state, registry, *source);
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
                    ResolutionChoiceKind::ChooseFromLibrary { options, .. } => {
                        options.iter()
                            .map(|&id| Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(id) })
                            .collect()
                    }
                    ResolutionChoiceKind::ChooseCardType { options, .. } => {
                        (0..options.len())
                            .map(|i| Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(i) })
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
                    ResolutionChoiceKind::ChoosePile { .. } => {
                        // Two options: choose pile 1 or pile 2.
                        vec![
                            Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0) },
                            Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(1) },
                        ]
                    }
                };
                let context = match choice {
                    ResolutionChoiceKind::ChooseTarget { description, .. } => description.clone(),
                    ResolutionChoiceKind::PayOrNot { description, .. } => description.clone(),
                    ResolutionChoiceKind::YesNo { .. } => format!("{}: choose yes or no", source_name),
                    ResolutionChoiceKind::ChooseCardFromHand { description, .. } => description.clone(),
                    ResolutionChoiceKind::ChooseFromRevealed { .. } => format!("{}: choose a card", source_name),
                    ResolutionChoiceKind::ChooseFromLibrary { .. } => format!("{}: search library", source_name),
                    ResolutionChoiceKind::ChooseCardType { .. } => format!("{}: choose a card type", source_name),
                    ResolutionChoiceKind::DividePermanentsIntoPiles { .. } => format!("{}: divide into piles", source_name),
                    ResolutionChoiceKind::ChoosePile { .. } => format!("{}: choose a pile", source_name),
                };
                LegalActions { actions, combat_prompt: None, castable_spells: vec![], context: Some(context) }
            }
        };
    }

    let player = match state.priority_player {
        Some(p) => p,
        None => return LegalActions { actions: vec![], combat_prompt: None, castable_spells: vec![], context: None },
    };

    let mut actions = Vec::new();
    let mut castable_spells = Vec::new();

    // PassPriority is always available when you have priority.
    actions.push(Action::PassPriority);

    // Check for Stony Silence: no abilities of artifacts can be activated, including mana abilities.
    let stony_silence_active = state.objects.values().any(|o| {
        o.zone == Zone::Battlefield && o.name == "Stony Silence"
    });

    // Mana abilities: can activate anytime you have priority.
    // Deduplicate by card_id — if you have 5 untapped Forests, only show one "Tap Forest".
    let mut seen_mana_abilities: Vec<(CardId, usize)> = Vec::new();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        // Stony Silence: skip mana abilities from artifacts.
        if stony_silence_active {
            let is_artifact = registry.card_data(obj.card_id)
                .map(|d| d.card_types.contains(&CardType::Artifact))
                .unwrap_or(false)
                || obj.card_types.contains(&CardType::Artifact);
            if is_artifact { continue; }
        }
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
            for ab in behavior.activated_abilities(state, obj_id, registry) {
                abilities.push((obj_card_id, ab));
            }
        }
        // If this is an Evil Twin that has copied another creature, its card_id now
        // points to the copied creature. We must also invoke Evil Twin's own behavior
        // to surface the "{U}{B}, {T}: Destroy" ability stored there.
        let is_evil_twin_copy = state.get_object(obj_id)
            .map(|o| o.card_state.contains_key("is_evil_twin"))
            .unwrap_or(false);
        if is_evil_twin_copy {
            if let Some(evil_twin_card_id) = registry.get_id_by_name("Evil Twin") {
                if evil_twin_card_id != obj_card_id {
                    if let Some(behavior) = registry.get(evil_twin_card_id) {
                        for ab in behavior.activated_abilities(state, obj_id, registry) {
                            abilities.push((evil_twin_card_id, ab));
                        }
                    }
                }
            }
        }
        for attached in state.objects.values() {
            if attached.zone == Zone::Battlefield && attached.attached_to == Some(obj_id) {
                if let Some(behavior) = registry.get(attached.card_id) {
                    for ab in behavior.activated_abilities(state, obj_id, registry) {
                        abilities.push((attached.card_id, ab));
                    }
                }
            }
        }

        for (source_card_id, ab) in abilities {
            // Check mana cost. For X-cost abilities, check that non-X portion is affordable.
            let has_x_cost = ab.cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X));
            if has_x_cost {
                let non_x_cost = ManaCost::new(
                    ab.cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                );
                // Need at least non-X cost + 1 extra mana for X >= 1.
                // Actually, X can be 0, so just need the non-X portion.
                // But X=0 is usually pointless — still allow it for correctness.
                if !mana::can_pay(mana_pool, &non_x_cost) { continue; }
            } else {
                if !mana::can_pay(mana_pool, &ab.cost) { continue; }
            }
            // Check tap cost and summoning sickness.
            // Per MTG rules, creatures with summoning sickness cannot use
            // abilities with {T} in the cost (unless they have haste).
            if ab.requires_tap {
                if obj_tapped { continue; }
                if obj.summoning_sick && !state.has_keyword(obj.id, Keyword::Haste, registry) {
                    continue;
                }
            }
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
                SacrificeCost::SacrificeAnotherCreature => {
                    // Must control at least one other creature to sacrifice.
                    let has_other_creature = state.objects_in_zone(Zone::Battlefield, player)
                        .iter()
                        .any(|o| o.power.is_some() && o.id != obj_id);
                    if !has_other_creature { continue; }
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
                let loyalty_abs = behavior.loyalty_abilities(state, obj_id);
                if loyalty_abs.is_empty() { continue; }

                let current_loyalty = state.get_counter_count(obj_id, CounterType::Loyalty);
                for ab in &loyalty_abs {
                    // Check if we can pay the cost.
                    if ab.loyalty_change < 0 && ((-ab.loyalty_change) as u32) > current_loyalty {
                        continue; // Not enough loyalty
                    }
                    if let Some(ref target_req) = ab.target_requirement {
                        // Targeted loyalty ability: generate one action per valid target.
                        let targets = valid_targets_for_req(state, player, obj_id, target_req, &*behavior, registry);
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

    // Collect names banned by Nevermore (spells with that name can't be cast).
    let nevermore_banned: Vec<String> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Nevermore")
        .filter_map(|o| {
            // Primary: instance_oracle_text set by on_enter_battlefield.
            if let Some(name) = o.instance_oracle_text.as_ref()
                .and_then(|t| t.strip_prefix("nevermore:"))
                .map(|s| s.to_string())
            {
                return Some(name);
            }
            // Secondary: card_state["named_card"] stores a CardId as ObjectId (used in tests).
            if let Some(oid) = o.card_state.get("named_card") {
                let card_id = crate::ids::CardId(oid.0 as u32);
                return registry.card_data(card_id).map(|d| d.name);
            }
            None
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
            // Also check if Rooftop Storm provides an alternative {0} cost.
            let has_rooftop_alt = rooftop_storm_applies(state, registry, obj.card_id, player);
            let can_pay_normal = if let Some(cost) = &data.cost {
                let effective_cost = effective_spell_cost(state, registry, obj.card_id, cost, player);
                mana::can_pay(&player_state.mana_pool, &effective_cost)
            } else {
                true
            };
            if !can_pay_normal && !has_rooftop_alt {
                continue;
            }

            // Check additional costs.
            use crate::cards::AdditionalCost;
            let eligible_sacrifices: Vec<ObjectId> = match &data.additional_cost {
                Some(AdditionalCost::SacrificeCreature) => {
                    let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, player)
                        .iter()
                        .filter(|o| o.power.is_some())
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
                                && (o.power.is_some() || registry.card_data(o.card_id)
                                    .map(|d| d.card_types.contains(&CardType::Creature))
                                    .unwrap_or(false))
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

            // If the spell requires a creature sacrifice, expand each action
            // into one per eligible creature.
            if !eligible_sacrifices.is_empty() {
                let base_actions = std::mem::take(&mut cast_actions);
                for action in base_actions {
                    if let Action::CastSpell { object_id, targets, .. } = action {
                        for &sac_id in &eligible_sacrifices {
                            cast_actions.push(Action::CastSpell {
                                object_id,
                                targets: targets.clone(),
                                sacrifice: Some(sac_id),
                                exile_count: None, exile_ids: vec![], alternative_cost: None,
                            });
                        }
                    }
                }
            }

            // For ExileXFromGraveyard, expand each cast action into one per graveyard subset.
            // The player chooses which specific cards to exile (not just how many).
            if matches!(&data.additional_cost, Some(AdditionalCost::ExileXFromGraveyard)) {
                let gy_cards: Vec<ObjectId> = state.objects.values()
                    .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != obj.id)
                    .map(|o| o.id)
                    .collect();
                let gy_count = gy_cards.len();
                let base_actions = std::mem::take(&mut cast_actions);
                for action in base_actions {
                    if let Action::CastSpell { object_id, targets, sacrifice, .. } = action {
                        // X=0: cast exiling nothing
                        cast_actions.push(Action::CastSpell {
                            object_id,
                            targets: targets.clone(),
                            sacrifice,
                            exile_count: Some(0),
                            exile_ids: vec![],
                            alternative_cost: None,
                        });
                        // For each X from 1 to gy_count, enumerate all C(gy_count, X) subsets
                        for x in 1..=gy_count {
                            for combo in combinations(&gy_cards, x) {
                                cast_actions.push(Action::CastSpell {
                                    object_id,
                                    targets: targets.clone(),
                                    sacrifice,
                                    exile_count: Some(x as u32),
                                    exile_ids: combo,
                                    alternative_cost: None,
                                });
                            }
                        }
                    }
                }
            }

            // Rooftop Storm: generate alternative {0} cost actions for Zombie creatures.
            // The player chooses between the normal cost and the free alternative cost.
            let is_zombie_creature = data.card_types.contains(&CardType::Creature)
                && data.subtypes.iter().any(|s| s == "Zombie");
            let has_rooftop_alt = is_zombie_creature && state.objects.values().any(|o| {
                o.zone == Zone::Battlefield && o.controller == player && o.name == "Rooftop Storm"
            });
            let can_pay_normal = if let Some(cost) = &data.cost {
                let effective_cost = effective_spell_cost(state, registry, obj.card_id, cost, player);
                mana::can_pay(&player_state.mana_pool, &effective_cost)
            } else {
                false
            };
            if has_rooftop_alt {
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
                                alternative_cost: Some(ManaCost::free()),
                            })
                        } else {
                            None
                        }
                    }).collect();
                    cast_actions.extend(alt_actions);
                } else {
                    // Player can't pay normally — replace all actions with alternative cost versions.
                    for action in &mut cast_actions {
                        if let Action::CastSpell { alternative_cost, .. } = action {
                            *alternative_cost = Some(ManaCost::free());
                        }
                    }
                }
            }

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

            // Check Nevermore: spells with the banned name can't be cast, even via flashback.
            if nevermore_banned.iter().any(|n| *n == data.name) {
                continue;
            }

            // Check for flashback cost, dynamic flashback, or "cast from graveyard" ability.
            let dynamic_fb = state.until_end_of_turn.iter()
                .find_map(|e| if let crate::state::TemporaryEffect::GrantFlashback { target, cost } = e {
                    if *target == obj.id { Some(cost.clone()) } else { None }
                } else { None });
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

            // Check additional cost eligibility for graveyard casts.
            {
                use crate::cards::AdditionalCost;
                if let Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) = &data.additional_cost {
                    // Count creature cards in graveyard (excluding the spell itself).
                    let creature_count = state.objects.values()
                        .filter(|o| {
                            o.zone == Zone::Graveyard && o.owner == player && o.id != obj.id
                                && (o.power.is_some() || registry.card_data(o.card_id)
                                    .map(|d| d.card_types.contains(&CardType::Creature))
                                    .unwrap_or(false))
                        })
                        .count();
                    if creature_count < *n { continue; }
                }
            }

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
                    is_flashback: !cast_from_gy,
                    target_spec: spec,
                });
            }
        }
    }

    // Concede is always last.
    actions.push(Action::Concede);

    // Build context string based on game state.
    let context = if !state.stack.is_empty() {
        // Responding to something on the stack.
        let top_name = match state.stack.last() {
            Some(crate::state::StackEntry::Spell(id)) => card_name(state, registry, *id),
            Some(crate::state::StackEntry::Trigger(t)) => t.display_name(registry),
            None => "?".into(),
        };
        let caster = match state.stack.last() {
            Some(crate::state::StackEntry::Spell(id)) =>
                state.get_object(*id).map(|o| o.controller),
            Some(crate::state::StackEntry::Trigger(t)) => Some(t.controller()),
            None => None,
        };
        let who = match caster {
            Some(p) if p == player => "your".into(),
            Some(p) => format!("p{}'s", p.0),
            None => "?".into(),
        };
        format!("RESPOND TO {} {}", who, top_name)
    } else {
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
            format!("OPPONENT'S TURN: {}", step_name)
        }
    };

    LegalActions { actions, combat_prompt: None, castable_spells, context: Some(context) }
}

/// Check if a permanent can be targeted by a spell from the given caster.
/// Returns false if the target has hexproof and the caster is an opponent.
fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
    can_be_targeted_by(state, target_id, caster, None, registry)
}

/// Check targeting legality, including protection from the source.
/// `source_id` is the spell or permanent whose ability is targeting.
pub fn can_be_targeted_by(state: &GameState, target_id: ObjectId, caster: PlayerId, source_id: Option<ObjectId>, registry: &CardRegistry) -> bool {
    if state.has_keyword(target_id, Keyword::Hexproof, registry) {
        let controller = state.get_object(target_id)
            .map(|o| o.controller)
            .unwrap_or(PlayerId(255));
        if controller != caster {
            return false; // hexproof: can't be targeted by opponents
        }
    }
    // Check protection from the source.
    if let Some(sid) = source_id {
        if state.has_protection_from(target_id, sid, registry) {
            return false;
        }
    }
    true
}

/// Check if a player can be targeted by a given caster.
/// Players with hexproof can't be targeted by opponents.
fn can_target_player(state: &GameState, target_player: PlayerId, caster: PlayerId, registry: &CardRegistry) -> bool {
    if target_player != caster && state.player_has_hexproof(target_player, registry) {
        return false;
    }
    true
}

/// Determine which mode of a ModalChoice was selected, based on the chosen targets.
/// For each mode, checks if all chosen targets are valid. Returns the first matching
/// mode index, defaulting to 0 if ambiguous (e.g. empty targets valid for all modes).
fn detect_modal_choice_mode(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    targets: &[crate::actions::Target],
    modes: &[crate::cards::TargetRequirement],
    behavior: &dyn crate::cards::CardBehavior,
) -> usize {
    let registry = &CardRegistry::with_all_cards();
    // For non-empty targets, find the first mode whose valid targets contain all chosen targets.
    if !targets.is_empty() {
        for (i, mode_req) in modes.iter().enumerate() {
            let valid = valid_targets_for_mode(state, caster, spell_id, mode_req, behavior, registry);
            if targets.iter().all(|t| valid.contains(t)) {
                return i;
            }
        }
    }
    // For empty targets (or no mode matched), default to mode 0.
    0
}

/// Get valid targets for a single mode requirement, unwrapping UpToTargets.
fn valid_targets_for_mode(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    mode_req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> Vec<crate::actions::Target> {
    use crate::cards::TargetRequirement;
    match mode_req {
        TargetRequirement::UpToTargets(_, inner) => valid_targets_for_req(state, caster, spell_id, inner, behavior, registry),
        other => valid_targets_for_req(state, caster, spell_id, other, behavior, registry),
    }
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
            vec![Action::CastSpell { object_id: spell_id, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None }]
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
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
                        });
                    }
                }
            }
            for player in &state.players {
                if !player.lost && can_target_player(state, player.id, caster, registry) {
                    let target = Target::Player(player.id);
                    if behavior.is_valid_target(state, caster, &target, registry) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
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
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
                        });
                    }
                }
            }
            actions
        }
        TargetRequirement::PlayerOnly => {
            let mut actions = Vec::new();
            for player in &state.players {
                if !player.lost && can_target_player(state, player.id, caster, registry) {
                    let target = Target::Player(player.id);
                    if behavior.is_valid_target(state, caster, &target, registry) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
                        });
                    }
                }
            }
            actions
        }
        TargetRequirement::PlayerOrPlaneswalker => {
            let mut actions = Vec::new();
            // Players
            for player in &state.players {
                if !player.lost && can_target_player(state, player.id, caster, registry) {
                    let target = Target::Player(player.id);
                    if behavior.is_valid_target(state, caster, &target, registry) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
                        });
                    }
                }
            }
            // Planeswalkers on the battlefield
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                let is_pw = obj.card_types.contains(&CardType::Planeswalker)
                    || registry.card_data(obj.card_id)
                        .map(|d| d.card_types.contains(&CardType::Planeswalker))
                        .unwrap_or(false);
                if is_pw {
                    if !can_be_targeted(state, obj.id, caster, registry) { continue; }
                    let target = Target::Object(obj.id);
                    if behavior.is_valid_target(state, caster, &target, registry) {
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: vec![target],
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
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
                        sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
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
                        sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
                    });
                }
            }
            actions
        }
        TargetRequirement::GraveyardCard | TargetRequirement::ExileCard
        | TargetRequirement::GraveyardCreature | TargetRequirement::GraveyardCreatureOfSubtype(_)
        | TargetRequirement::GraveyardCardOwnedByCaster | TargetRequirement::GraveyardCardOwnedByOpponent => {
            let targets = valid_targets_for_req(state, caster, spell_id, target_req, behavior, registry);
            targets.into_iter()
                .map(|t| Action::CastSpell { object_id: spell_id, targets: vec![t], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None })
                .collect()
        }
        TargetRequirement::ModalChoice(ref modes) => {
            let mut actions = Vec::new();
            for mode_req in modes {
                actions.extend(generate_cast_actions_with_targets(state, caster, spell_id, mode_req, behavior));
            }
            actions
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
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
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
            // Start from 0 to allow "up to N" to mean "0 or more" (e.g., Memory's Journey
            // can be cast targeting just a player with 0 cards).
            for k in 0..=(*max).min(options.len()) {
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
                        sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None,
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
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
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
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::AnyTarget => {
            let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| o.power.is_some())
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect();
            for p in &state.players {
                if !p.lost && can_target_player(state, p.id, caster, registry) {
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
                .filter(|p| can_target_player(state, p.id, caster, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::PlayerOrPlaneswalker => {
            let mut targets: Vec<Target> = state.players.iter()
                .filter(|p| !p.lost)
                .filter(|p| can_target_player(state, p.id, caster, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                let is_pw = obj.card_types.contains(&CardType::Planeswalker)
                    || registry.card_data(obj.card_id)
                        .map(|d| d.card_types.contains(&CardType::Planeswalker))
                        .unwrap_or(false);
                if is_pw && can_be_targeted(state, obj.id, caster, registry) {
                    let t = Target::Object(obj.id);
                    if behavior.is_valid_target(state, caster, &t, registry) {
                        targets.push(t);
                    }
                }
            }
            targets
        }
        TargetRequirement::GraveyardCard => {
            // All cards in all graveyards.
            state.objects.values()
                .filter(|o| o.zone == Zone::Graveyard)
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCreature => {
            // Creature cards in all graveyards. Check both object and registry data.
            state.objects.values()
                .filter(|o| {
                    o.zone == Zone::Graveyard
                        && (o.power.is_some()
                            || registry.card_data(o.card_id)
                                .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                                .unwrap_or(false))
                })
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCreatureOfSubtype(ref subtype) => {
            // Creature cards with a specific subtype in all graveyards.
            // Check subtypes on both the object and the registry card data.
            state.objects.values()
                .filter(|o| {
                    o.zone == Zone::Graveyard
                        && (o.power.is_some()
                            || registry.card_data(o.card_id)
                                .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                                .unwrap_or(false))
                        && (o.subtypes.iter().any(|s| s == subtype)
                            || registry.card_data(o.card_id)
                                .map(|d| d.subtypes.iter().any(|s| s == subtype))
                                .unwrap_or(false))
                })
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCardOwnedByCaster => {
            // Cards in the caster's own graveyard.
            state.objects.values()
                .filter(|o| o.zone == Zone::Graveyard && o.owner == caster)
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCardOwnedByOpponent => {
            // Cards in any opponent's graveyard.
            state.objects.values()
                .filter(|o| o.zone == Zone::Graveyard && o.owner != caster)
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
        TargetRequirement::ModalChoice(ref modes) => {
            // Collect all possible targets across all modes.
            let mut all_options = Vec::new();
            for mode_req in modes {
                all_options.extend(valid_targets_for_req(state, caster, spell_id, mode_req, behavior, registry));
            }
            all_options.dedup();
            CastTargetSpec::SingleTarget(all_options)
        }
        // All single-target types
        _ => {
            let options = valid_targets_for_req(state, caster, spell_id, target_req, behavior, registry);
            CastTargetSpec::SingleTarget(options)
        }
    }
}

/// Check if a creature matches a TargetFilter for ability targeting.
fn matches_ability_target_filter(
    state: &GameState,
    obj: &crate::state::GameObject,
    filter: &crate::cards::TargetFilter,
    controller: PlayerId,
    source_id: ObjectId,
    registry: &CardRegistry,
) -> bool {
    use crate::cards::TargetFilter;
    match filter {
        TargetFilter::Any => true,
        TargetFilter::Another => obj.id != source_id,
        TargetFilter::YouControl => obj.controller == controller,
        TargetFilter::YouDontControl => obj.controller != controller,
        TargetFilter::Nonblack => {
            // Check if the creature's mana cost contains black.
            if let Some(data) = registry.card_data(obj.card_id) {
                if let Some(ref cost) = data.cost {
                    !cost.symbols.iter().any(|s| matches!(s, ManaSymbol::Colored(Color::Black)))
                } else {
                    true // No cost = not black
                }
            } else {
                true
            }
        }
        TargetFilter::NotSubtypes(types) => {
            let subtypes = &obj.subtypes;
            !types.iter().any(|t| subtypes.contains(t))
        }
        TargetFilter::PowerAtLeast(n) => {
            state.effective_power(obj.id, registry).unwrap_or(0) >= *n
        }
        TargetFilter::Attacking => {
            state.combat.as_ref().map(|c| c.attackers.contains_key(&obj.id)).unwrap_or(false)
        }
        TargetFilter::HasSubtype(subtype) => {
            obj.subtypes.contains(subtype)
        }
        TargetFilter::SameNameAsSource => {
            // Only target creatures with the same name as the source permanent.
            state.get_object(source_id)
                .map(|source| source.name == obj.name)
                .unwrap_or(false)
        }
        _ => true, // Other filters not commonly used for ability targeting
    }
}

/// Generate valid targets for a targeted activated ability.
fn generate_ability_targets(
    state: &GameState,
    source_id: ObjectId,
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
        TargetRequirement::Creature => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| o.power.is_some())
                .filter(|o| can_be_targeted(state, o.id, controller, registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::CreatureWithFilter(filter) => {
            // For equipment equip abilities, exclude the creature already attached to this equipment.
            let already_attached: Option<ObjectId> = state.get_object(source_id)
                .filter(|o| o.is_equipment)
                .and_then(|o| o.attached_to);
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| o.power.is_some())
                .filter(|o| can_be_targeted(state, o.id, controller, registry))
                .filter(|o| matches_ability_target_filter(state, o, filter, controller, source_id, registry))
                .filter(|o| already_attached.map(|a| a != o.id).unwrap_or(true))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::PlayerOnly => {
            state.players.iter()
                .filter(|p| !p.lost)
                .filter(|p| can_target_player(state, p.id, controller, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::PlayerOrPlaneswalker => {
            let mut targets: Vec<Target> = state.players.iter()
                .filter(|p| !p.lost)
                .filter(|p| can_target_player(state, p.id, controller, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                let is_pw = obj.card_types.contains(&CardType::Planeswalker)
                    || registry.card_data(obj.card_id)
                        .map(|d| d.card_types.contains(&CardType::Planeswalker))
                        .unwrap_or(false);
                if is_pw && can_be_targeted(state, obj.id, controller, registry) {
                    let t = Target::Object(obj.id);
                    if behavior.is_valid_target(state, controller, &t, registry) {
                        targets.push(t);
                    }
                }
            }
            targets
        }
        TargetRequirement::AnyTarget => {
            let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| o.power.is_some())
                .filter(|o| can_be_targeted(state, o.id, controller, registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect();
            for p in &state.players {
                if !p.lost && can_target_player(state, p.id, controller, registry) {
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
        .map(|o| registry.card_data(o.card_id)
            .map(|d| d.name)
            .unwrap_or_else(|| o.name.clone()))
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
            new_state.move_object(*object_id, Zone::Battlefield, registry);
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

        Action::CastSpell { object_id, targets, sacrifice, exile_count, exile_ids, alternative_cost } => {
            let player = new_state.priority_player.expect("CastSpell requires priority");

            // Detect flashback vs cast-from-graveyard.
            // Flashback: card has flashback_cost or dynamically granted flashback.
            // Cast-from-graveyard: card has can_cast_from_graveyard() (Skaab Ruinator) — uses normal mana cost.
            let card_id = new_state.get_object(*object_id).expect("CastSpell object must exist").card_id;
            let data = registry.get(card_id).expect("card must be in registry").card_data();
            let behavior = registry.get(card_id).expect("card must be in registry");
            let in_graveyard = new_state.get_object(*object_id)
                .map(|o| o.zone == Zone::Graveyard)
                .unwrap_or(false);
            let is_cast_from_graveyard = in_graveyard && behavior.can_cast_from_graveyard();
            let is_flashback = in_graveyard && !is_cast_from_graveyard;

            // Pay the appropriate mana cost (applying cost reduction for non-flashback).
            // If an alternative_cost is provided (e.g. Rooftop Storm's {0}), use it directly.
            let cost = if let Some(alt) = alternative_cost {
                alt.clone()
            } else if is_flashback {
                // Check until_end_of_turn for dynamically granted flashback.
                let dynamic_fb = new_state.until_end_of_turn.iter()
                    .find_map(|e| if let crate::state::TemporaryEffect::GrantFlashback { target, cost } = e {
                        if *target == *object_id { Some(cost.clone()) } else { None }
                    } else { None });
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

            // Pay additional costs (sacrifice) at cast time, before the spell goes on the stack.
            if let Some(sac_id) = sacrifice {
                let sac_name = card_name(&new_state, registry, *sac_id);
                crate::destruction::sacrifice(&mut new_state, *sac_id, registry);
                new_state.log(LogLevel::Event,
                    format!("Sacrificed {} as additional cost", sac_name));
            } else {
                // Backward compatibility: if sacrifice is None but the spell has
                // AdditionalCost::SacrificeCreature, auto-sacrifice the first creature.
                use crate::cards::AdditionalCost;
                let needs_sac = registry.get(card_id)
                    .map(|b| matches!(b.card_data().additional_cost, Some(AdditionalCost::SacrificeCreature)))
                    .unwrap_or(false);
                if needs_sac {
                    let creature = new_state.objects_in_zone(Zone::Battlefield, player)
                        .iter()
                        .find(|o| o.power.is_some())
                        .map(|o| o.id);
                    if let Some(cid) = creature {
                        let sac_name = card_name(&new_state, registry, cid);
                        crate::destruction::sacrifice(&mut new_state, cid, registry);
                        new_state.log(LogLevel::Event,
                            format!("Sacrificed {} as additional cost", sac_name));
                    }
                }
            }

            // Handle ExileCreaturesFromGraveyard additional cost (Skaab Ruinator, Corpse Lunge, etc.).
            {
                use crate::cards::AdditionalCost;
                if let Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) = registry.get(card_id)
                    .map(|b| b.card_data().additional_cost).flatten()
                {
                    // Pick highest-power creatures first (better default for Corpse Lunge).
                    let mut exile_candidates: Vec<(ObjectId, i32)> = new_state.objects.values()
                        .filter(|o| {
                            o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id
                                && (o.power.is_some() || registry.card_data(o.card_id)
                                    .map(|d| d.card_types.contains(&CardType::Creature))
                                    .unwrap_or(false))
                        })
                        .map(|o| (o.id, o.power.unwrap_or(0)))
                        .collect();
                    exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first
                    let exile_candidates: Vec<_> = exile_candidates.into_iter().take(n).collect();

                    // Store the first exiled creature's power for cards that need it
                    // (Corpse Lunge uses the power to determine damage).
                    if let Some((_, power)) = exile_candidates.first() {
                        if let Some(obj) = new_state.get_object_mut(*object_id) {
                            obj.card_state.insert("exiled_power".into(), ObjectId(*power as u64));
                        }
                    }

                    for (exile_id, _) in &exile_candidates {
                        let name = card_name(&new_state, registry, *exile_id);
                        new_state.move_object(*exile_id, Zone::Exile, registry);
                        new_state.log(LogLevel::Event,
                            format!("Exiled {} from graveyard as additional cost", name));
                    }
                }
            }

            // Handle ExileXFromGraveyard additional cost (Harvest Pyre).
            // The player chose X via exile_count in the action.
            {
                use crate::cards::AdditionalCost;
                let needs_exile_x = registry.get(card_id)
                    .map(|b| matches!(b.card_data().additional_cost, Some(AdditionalCost::ExileXFromGraveyard)))
                    .unwrap_or(false);
                if needs_exile_x {
                    // If specific cards were chosen (via exile_ids), exile those exactly.
                    // Otherwise fall back to auto-selecting the first exile_count cards (legacy behavior).
                    let graveyard_cards: Vec<ObjectId> = if !exile_ids.is_empty() {
                        exile_ids.clone()
                    } else {
                        let x = exile_count.unwrap_or(0) as usize;
                        new_state.objects.values()
                            .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id)
                            .map(|o| o.id)
                            .take(x)
                            .collect()
                    };
                    let count = graveyard_cards.len() as u32;
                    for gid in &graveyard_cards {
                        new_state.move_object(*gid, Zone::Exile, registry);
                    }
                    // Store the count on the spell for resolution.
                    if let Some(obj) = new_state.get_object_mut(*object_id) {
                        obj.card_state.insert("exile_count".into(), ObjectId(count as u64));
                    }
                    new_state.log(LogLevel::Event,
                        format!("Exiled {} cards from graveyard as additional cost", count));
                }
            }

            // Move to stack and store targets.
            new_state.move_object(*object_id, Zone::Stack, registry);
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

            // For ModalChoice spells, determine and store which mode was chosen
            // by checking which mode's valid targets match the actual targets.
            if let Some(behavior) = registry.get(card_id) {
                if let crate::cards::TargetRequirement::ModalChoice(ref modes) = behavior.target_requirement() {
                    let chosen = detect_modal_choice_mode(&new_state, player, *object_id, targets, modes, behavior);
                    if let Some(obj) = new_state.get_object_mut(*object_id) {
                        obj.chosen_mode = Some(chosen);
                    }
                }
            }

            new_state.stack.push(crate::state::StackEntry::Spell(*object_id));

            new_state.events.push(GameEvent::SpellCast {
                player,
                object: *object_id,
            });

            // Track spells cast this turn (for werewolf transform conditions etc.)
            *new_state.num_spells_cast_this_turn.entry(player).or_insert(0) += 1;

            let name = card_name(&new_state, registry, *object_id);
            let suffix = if is_flashback { " (flashback)" } else { "" };
            let target_str = if targets.is_empty() {
                String::new()
            } else {
                let names: Vec<String> = targets.iter().map(|t| match t {
                    crate::actions::Target::Object(id) => card_name(&new_state, registry, *id),
                    crate::actions::Target::Player(pid) => format!("p{}", pid.0),
                }).collect();
                format!(" targeting {}", names.join(", "))
            };
            new_state.log(LogLevel::Event, format!("p{} cast {}{}{}", player.0, name, suffix, target_str));
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
            let pool = &new_state.get_player(controller).mana_pool;
            let pool_str: Vec<String> = pool.mana.iter()
                .filter(|(_, &v)| v > 0)
                .map(|(t, v)| format!("{:?}:{}", t, v))
                .collect();
            new_state.log(LogLevel::Info, format!("p{} tapped {} for mana (pool: {})",
                controller.0, name, if pool_str.is_empty() { "empty".into() } else { pool_str.join(" ") }));
        }

        Action::ActivateAbility { object_id, ability_index, targets } => {
            let player = new_state.priority_player.expect("ActivateAbility requires priority");
            let obj = new_state.get_object(*object_id).expect("activated ability object must exist");
            let card_id = obj.card_id;

            // Find the ability — check the permanent's own card, Evil Twin override, then attached auras.
            let is_evil_twin_copy = new_state.get_object(*object_id)
                .map(|o| o.card_state.contains_key("is_evil_twin"))
                .unwrap_or(false);
            let ability = registry.get(card_id)
                .and_then(|b| b.activated_abilities(&new_state, *object_id, registry)
                    .into_iter().find(|a| a.ability_index == *ability_index))
                .or_else(|| {
                    // Evil Twin copies another creature: its card_id changes to the copied
                    // creature's card_id, so we must also check Evil Twin's own behavior.
                    if is_evil_twin_copy {
                        registry.get_id_by_name("Evil Twin")
                            .filter(|&et_id| et_id != card_id)
                            .and_then(|et_id| {
                                registry.get(et_id)
                                    .and_then(|b| b.activated_abilities(&new_state, *object_id, registry)
                                        .into_iter().find(|a| a.ability_index == *ability_index))
                            })
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    // Check attached auras.
                    new_state.objects.values()
                        .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(*object_id))
                        .find_map(|a| {
                            registry.get(a.card_id)
                                .and_then(|b| b.activated_abilities(&new_state, *object_id, registry)
                                    .into_iter().find(|ab| ab.ability_index == *ability_index))
                        })
                });

            if let Some(ab) = ability {
                // Pay mana cost (with X-cost support).
                let has_x_cost = ab.cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X));
                if has_x_cost {
                    let non_x_cost = ManaCost::new(
                        ab.cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                    );
                    let pool = &new_state.get_player(player).mana_pool;
                    let total_mana = pool.total();
                    let non_x_amount = non_x_cost.mana_value();
                    let x = total_mana.saturating_sub(non_x_amount);
                    mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &non_x_cost)
                        .expect("legal_actions should have verified mana availability");
                    new_state.get_player_mut(player).mana_pool.empty();
                    new_state.last_activated_x_value = Some(x);
                } else {
                    mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &ab.cost)
                        .expect("legal_actions should have verified mana availability");
                    new_state.last_activated_x_value = None;
                }

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
                    SacrificeCost::SacrificeAnotherCreature => {
                        // Sacrifice another creature (not the source permanent).
                        // For now, auto-sacrifice the first eligible creature.
                        // TODO: Present choice to player when there are multiple options.
                        let creature = new_state.objects_in_zone(Zone::Battlefield, player)
                            .iter()
                            .find(|o| o.power.is_some() && o.id != *object_id)
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

                // Find which behavior to call (card itself, Evil Twin override, or attached aura).
                let behavior_card_id = if registry.get(card_id)
                    .map(|b| !b.activated_abilities(&new_state, *object_id, registry).is_empty())
                    .unwrap_or(false)
                {
                    card_id
                } else if is_evil_twin_copy {
                    // Evil Twin has copied another creature: dispatch to Evil Twin's behavior.
                    registry.get_id_by_name("Evil Twin")
                        .filter(|&et_id| et_id != card_id)
                        .unwrap_or(card_id)
                } else {
                    // Must be from an attached aura.
                    new_state.objects.values()
                        .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(*object_id))
                        .find(|a| {
                            registry.get(a.card_id)
                                .map(|b| !b.activated_abilities(&new_state, *object_id, registry).is_empty())
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
            combat::declare_blockers_with_registry(&mut new_state, assignments, registry);
            // Log after validation so only legal blocks appear in the log.
            let actual_blockers: Vec<(ObjectId, ObjectId)> = new_state.combat.as_ref()
                .map(|c| c.blocker_assignments.iter()
                    .flat_map(|(&att, blockers)| blockers.iter().map(move |&b| (b, att)))
                    .collect())
                .unwrap_or_default();
            if actual_blockers.is_empty() {
                new_state.log(LogLevel::Info, format!("p{} declared no blockers", defender.0));
            } else {
                let descs: Vec<String> = actual_blockers.iter()
                    .map(|(b, a)| format!("{} blocks {}", card_name(state, registry, *b), card_name(state, registry, *a)))
                    .collect();
                new_state.log(LogLevel::Event, format!("p{} declared blockers: {}", defender.0, descs.join(", ")));
            }
            new_state.awaiting_action = None;
            new_state.consecutive_passes = 0;
        }

        Action::DiscardCards { cards } => {
            let is_hand_size = matches!(&new_state.awaiting_action,
                Some(AwaitingAction::DiscardToHandSize { .. }));
            let player = match &new_state.awaiting_action {
                Some(AwaitingAction::DiscardToHandSize { player, .. }) => *player,
                _ => new_state.active_player,
            };
            let names: Vec<String> = cards.iter()
                .map(|&id| card_name(&new_state, registry, id))
                .collect();
            for &card_id in cards {
                new_state.events.push(GameEvent::Discarded { player, object: card_id });
                new_state.move_object(card_id, Zone::Graveyard, registry);
            }
            if is_hand_size {
                new_state.log(LogLevel::Event,
                    format!("p{} discarded {} (cleanup)", player.0, names.join(", ")));
            } else {
                for name in &names {
                    new_state.log(LogLevel::Event, format!("p{} discarded {}", player.0, name));
                }
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

        Action::ActivateLoyaltyAbility { object_id, ability_index, targets } => {
            let player = new_state.priority_player.expect("ActivateLoyaltyAbility requires priority");
            if let Some(behavior) = registry.get(
                new_state.get_object(*object_id).map(|o| o.card_id).unwrap_or(crate::ids::CardId(0))
            ) {
                let abilities = behavior.loyalty_abilities(&new_state, *object_id);
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
                    behavior.on_loyalty_ability(&mut new_state, *object_id, *ability_index, targets, registry);
                    let name = card_name(&new_state, registry, *object_id);
                    new_state.log(LogLevel::Event, format!("p{} activated loyalty ability on {}: {}", player.0, name, ab.description));
                }
            }
        }

        Action::ResolveChoice { choice: resolved } => {
            use crate::state::ResolutionChoiceKind;
            use crate::actions::ResolvedChoice;
            let awaiting = new_state.awaiting_action.take();
            if let Some(AwaitingAction::ResolutionChoice { choice: kind, source: choice_source, .. }) = awaiting {
                match (&kind, resolved) {
                    (ResolutionChoiceKind::PayOrNot { spell_id, source_spell_id, .. },
                     ResolvedChoice::PayDecision(pay)) => {
                        if !*pay {
                            let name = new_state.get_object(*spell_id).map(|o| o.name.clone()).unwrap_or_default();
                            new_state.stack.retain(|e| e.as_spell() != Some(*spell_id));
                            new_state.move_spell_after_resolve(*spell_id, registry);
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
                            new_state.move_object(hand[0], Zone::Graveyard, registry);
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
                            new_state.move_spell_after_resolve(*source_spell_id, registry);
                            return new_state;
                        }
                        new_state.move_spell_after_resolve(*source_spell_id, registry);
                    }
                    (ResolutionChoiceKind::YesNo { source_card, .. },
                     ResolvedChoice::PayDecision(yes)) => {
                        // Dispatch to the card's on_yes_no_choice hook.
                        let source_card_id = new_state.get_object(*source_card).map(|o| o.card_id);
                        if let Some(behavior) = source_card_id.and_then(|cid| registry.get(cid)) {
                            behavior.on_yes_no_choice(&mut new_state, *source_card, *yes, registry);
                        }
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
                        new_state.move_object(*discard_id, Zone::Graveyard, registry);
                        new_state.events.push(GameEvent::Discarded {
                            player: new_state.get_object(*discard_id).map(|o| o.owner).unwrap_or(PlayerId(0)),
                            object: *discard_id,
                        });
                        new_state.log(LogLevel::Event, format!("Discarded {}", name));
                        // Notify the source card about the discard (e.g., Civilized Scholar
                        // checks if the discarded card was a creature to trigger transform).
                        let source_card_id = new_state.get_object(choice_source).map(|o| o.card_id);
                        if let Some(behavior) = source_card_id.and_then(|cid| registry.get(cid)) {
                            behavior.on_discard_choice(&mut new_state, choice_source, *discard_id, registry);
                        }
                    }
                    (ResolutionChoiceKind::ChooseFromRevealed { revealed, spell_id, .. },
                     ResolvedChoice::ChosenCard(keep_id)) => {
                        let keep_name = new_state.get_object(*keep_id).map(|o| o.name.clone()).unwrap_or_default();
                        new_state.move_object(*keep_id, Zone::Hand, registry);
                        for &card_id in revealed {
                            if card_id != *keep_id {
                                new_state.move_object(card_id, Zone::Graveyard, registry);
                            }
                        }
                        new_state.log(LogLevel::Event, format!("Kept {}", keep_name));
                        new_state.move_spell_after_resolve(*spell_id, registry);
                    }
                    (ResolutionChoiceKind::ChooseFromLibrary { searcher, .. },
                     ResolvedChoice::ChosenCard(chosen_id)) => {
                        let chosen_name = new_state.get_object(*chosen_id).map(|o| o.name.clone()).unwrap_or_default();
                        let player = new_state.get_player_mut(*searcher);
                        player.library_order.retain(|&id| id != *chosen_id);
                        new_state.move_object(*chosen_id, Zone::Hand, registry);
                        new_state.log(LogLevel::Event, format!("Searched library and found {}", chosen_name));
                        // Shuffle library after searching.
                        {
                            use rand::seq::SliceRandom;
                            let mut rng = rand::thread_rng();
                            new_state.get_player_mut(*searcher).library_order.shuffle(&mut rng);
                        }
                    }
                    (ResolutionChoiceKind::ChooseCardType { options, spell_id, controller, .. },
                     ResolvedChoice::ChosenIndex(index)) => {
                        let chosen_type = options.get(*index).cloned().unwrap_or_default();
                        let card_type = match chosen_type.as_str() {
                            "Creature" => CardType::Creature,
                            "Artifact" => CardType::Artifact,
                            "Enchantment" => CardType::Enchantment,
                            "Land" => CardType::Land,
                            "Planeswalker" => CardType::Planeswalker,
                            _ => CardType::Creature,
                        };
                        let to_return: Vec<ObjectId> = new_state.objects_in_zone(Zone::Graveyard, *controller)
                            .iter()
                            .filter(|o| {
                                // Check object's own card_types first, fall back to registry
                                if !o.card_types.is_empty() {
                                    o.card_types.contains(&card_type)
                                } else {
                                    registry.card_data(o.card_id)
                                        .map(|d| d.card_types.contains(&card_type))
                                        .unwrap_or(false)
                                }
                            })
                            .map(|o| o.id)
                            .collect();
                        let count = to_return.len();
                        for id in to_return {
                            new_state.move_object(id, Zone::Hand, registry);
                        }
                        new_state.log(LogLevel::Event,
                            format!("Creeping Renaissance: chose {}. Returned {} cards from graveyard to hand",
                                chosen_type, count));
                        new_state.move_spell_after_resolve(*spell_id, registry);
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
                            .filter_map(|id| new_state.get_object(*id).map(|o| o.name.clone()))
                            .collect();
                        let pile_2_names: Vec<String> = pile_2.iter()
                            .filter_map(|id| new_state.get_object(*id).map(|o| o.name.clone()))
                            .collect();
                        new_state.log(LogLevel::Event,
                            format!("Liliana -6: Pile 1: [{}], Pile 2: [{}]",
                                if pile_1_names.is_empty() { "empty".into() } else { pile_1_names.join(", ") },
                                if pile_2_names.is_empty() { "empty".into() } else { pile_2_names.join(", ") }));

                        // Now the target player chooses which pile to sacrifice.
                        new_state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                            player: *target_player,
                            source: *source_id,
                            choice: ResolutionChoiceKind::ChoosePile {
                                description: format!(
                                    "Liliana -6: Choose a pile to sacrifice.\nPile 1: [{}]\nPile 2: [{}]",
                                    if pile_1_names.is_empty() { "empty".into() } else { pile_1_names.join(", ") },
                                    if pile_2_names.is_empty() { "empty".into() } else { pile_2_names.join(", ") }),
                                pile_1,
                                pile_2,
                                source_id: *source_id,
                            },
                        });
                    }
                    (ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. },
                     ResolvedChoice::ChosenIndex(index)) => {
                        // Target player chose which pile to sacrifice.
                        let chosen_pile = if *index == 0 { pile_1 } else { pile_2 };
                        let pile_label = if *index == 0 { "Pile 1" } else { "Pile 2" };
                        new_state.log(LogLevel::Event,
                            format!("Liliana -6: chose to sacrifice {}", pile_label));
                        for &perm_id in chosen_pile {
                            let name = new_state.get_object(perm_id).map(|o| o.name.clone()).unwrap_or_default();
                            if new_state.get_object(perm_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                                crate::destruction::sacrifice(&mut new_state, perm_id, registry);
                                new_state.log(LogLevel::Event,
                                    format!("Liliana -6: sacrificed {}", name));
                            }
                        }
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
            // Check for "prevent damage, remove counter" replacement (Unbreathing Horde).
            let has_prevent = state.has_continuous_effect(*id, &|e| {
                match e {
                    crate::types::ContinuousEffect::PreventDamageRemoveCounter { scope } => Some(scope),
                    _ => None,
                }
            }, registry);
            if has_prevent {
                let counter_count = state.get_object(*id)
                    .and_then(|o| o.counters.get(&crate::types::CounterType::PlusOnePlusOne).copied())
                    .unwrap_or(0);
                if counter_count > 0 {
                    if let Some(obj) = state.get_object_mut(*id) {
                        let entry = obj.counters.entry(crate::types::CounterType::PlusOnePlusOne).or_insert(0);
                        *entry = entry.saturating_sub(1);
                        if *entry == 0 {
                            obj.counters.remove(&crate::types::CounterType::PlusOnePlusOne);
                        }
                    }
                    let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
                    state.log(LogLevel::Event,
                        format!("{}: damage prevented, removed a +1/+1 counter", name));
                }
                // Damage prevented — skip normal damage application.
            } else if state.has_protection_from(*id, *source_id, registry) {
                // Protection prevents damage from the source.
                let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
                state.log(LogLevel::Event,
                    format!("{}: damage from {} prevented by protection", name, source_name));
            } else if let Some(obj) = state.get_object_mut(*id) {
                if obj.zone == Zone::Battlefield {
                    // Check if target is a planeswalker — damage removes loyalty counters.
                    let is_planeswalker = registry.card_data(obj.card_id)
                        .map(|d| d.card_types.contains(&CardType::Planeswalker))
                        .unwrap_or(false);
                    let name = obj.name.clone();

                    if is_planeswalker {
                        // Remove loyalty counters equal to damage.
                        let loyalty = obj.counters.entry(crate::types::CounterType::Loyalty).or_insert(0);
                        *loyalty = loyalty.saturating_sub(*amount);
                        if *loyalty == 0 {
                            obj.counters.remove(&crate::types::CounterType::Loyalty);
                        }
                    } else {
                        obj.damage_marked += amount;
                    }
                    obj.damaged_by.push(*source_id);
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
            state.move_object(*id, Zone::Battlefield, registry);
            state.log(LogLevel::Event, format!("{} returned to the battlefield", name));
            state.move_spell_after_resolve(*spell_id, registry);
        }
        (Target::Object(id), PendingEffect::AddCounters { count, human_bonus }) => {
            let mut final_count = *count;
            if *human_bonus {
                let is_human = state.get_object(*id)
                    .map(|o| {
                        let obj_has = o.subtypes.iter().any(|s| s == "Human");
                        let card_has = registry.card_data(o.card_id)
                            .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                            .unwrap_or(false);
                        obj_has || card_has
                    })
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
            state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
                target: *id,
                power_mod: *power,
                toughness_mod: *toughness,
            });
            state.log(LogLevel::Event, format!("{} gave {} {}/{} until end of turn", source_name, name, power, toughness));
        }
        (Target::Object(id), PendingEffect::CantBlockThisTurn { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.until_end_of_turn.push(crate::state::TemporaryEffect::CantBlock { target: *id });
            state.log(LogLevel::Event, format!("{} prevents {} from blocking this turn", source_name, name));
        }
        (Target::Player(pid), PendingEffect::Mill { count, source_name }) => {
            mill_cards(state, *pid, *count as usize, registry);
            state.log(LogLevel::Event, format!("{} milled {} card(s) from p{}", source_name, count, pid.0));
        }
        (Target::Object(id), PendingEffect::ExileAndStore { source_id, source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*id, Zone::Exile, registry);
            // Store the exiled creature's ID on the source permanent for LTB retrieval.
            if let Some(source_obj) = state.get_object_mut(*source_id) {
                source_obj.card_state.insert("exiled_creature".into(), *id);
            }
            state.log(LogLevel::Event, format!("{} exiled {}", source_name, name));
        }
        (Target::Object(id), PendingEffect::ExileCardAndCleanup { spell_id, source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*id, Zone::Exile, registry);
            state.log(LogLevel::Event, format!("{} exiled {} from hand", source_name, name));
            state.move_spell_after_resolve(*spell_id, registry);
        }
        (Target::Player(pid), PendingEffect::DrawAndLoseLife { source_name }) => {
            draw_cards(state, *pid, 1, registry);
            let old = state.get_player(*pid).life;
            let new_life = old - 1;
            state.get_player_mut(*pid).life = new_life;
            state.events.push(GameEvent::LifeChanged { player: *pid, old, new_life });
            state.log(LogLevel::Event, format!("{}: p{} drew a card and lost 1 life", source_name, pid.0));
        }
        (Target::Player(pid), PendingEffect::DrainLife { controller, source_name }) => {
            // Target player loses 1 life.
            let old = state.get_player(*pid).life;
            let new_life = old - 1;
            state.get_player_mut(*pid).life = new_life;
            state.events.push(GameEvent::LifeChanged { player: *pid, old, new_life });
            // Controller gains 1 life.
            let old_self = state.get_player(*controller).life;
            let new_self = old_self + 1;
            state.get_player_mut(*controller).life = new_self;
            state.events.push(GameEvent::LifeChanged { player: *controller, old: old_self, new_life: new_self });
            state.log(LogLevel::Event, format!("{}: p{} lost 1 life, p{} gained 1 life", source_name, pid.0, controller.0));
        }
        (Target::Object(id), PendingEffect::DestroyCreature { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, *id, registry);
            state.log(LogLevel::Event, format!("{} destroyed {}", source_name, name));
        }
        (Target::Object(id), PendingEffect::ExileCurseOfOblivion { remaining }) => {
            let owner = state.get_object(*id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
            state.move_object(*id, Zone::Exile, registry);
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
            state.move_object(*id, Zone::Hand, registry);
            state.log(LogLevel::Event, format!("{}: returned {} to hand", source_name, name));
        }
        (Target::Object(id), PendingEffect::PutOnTopOfLibrary { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            let owner = state.get_object(*id).map(|o| o.owner).unwrap_or(crate::ids::PlayerId(0));
            state.move_object(*id, Zone::Library, registry);
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

            state.move_spell_after_resolve(*spell_id, registry);
        }
        (Target::Object(id), PendingEffect::ExileFromGraveyardGainLife { controller }) => {
            let is_creature = state.get_object(*id)
                .map(|o| {
                    registry.card_data(o.card_id)
                        .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                        .unwrap_or(o.power.is_some())
                })
                .unwrap_or(false);
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*id, Zone::Exile, registry);
            state.log(LogLevel::Event, format!("Graveyard Shovel: exiled {} from graveyard", name));

            if is_creature {
                let old_life = state.get_player(*controller).life;
                let new_life = old_life + 2;
                state.get_player_mut(*controller).life = new_life;
                state.events.push(GameEvent::LifeChanged {
                    player: *controller,
                    old: old_life,
                    new_life,
                });
                state.log(LogLevel::Event,
                    format!("Graveyard Shovel: p{} gained 2 life (creature exiled)", controller.0));
            }
        }
        (Target::Object(id), PendingEffect::SacrificeAndTutor { garruk_id }) => {
            use crate::state::ResolutionChoiceKind;
            // Garruk -1: sacrifice the chosen creature, then search library for a creature card.
            let sac_name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            let controller = state.get_object(*garruk_id).map(|o| o.controller).unwrap_or(PlayerId(0));
            crate::destruction::sacrifice(state, *id, registry);
            state.log(LogLevel::Event,
                format!("Garruk, the Veil-Cursed: sacrificed {}", sac_name));

            // Find all creature cards in library for the player to choose from.
            let creature_options: Vec<ObjectId> = state.get_player(controller).library_order.iter()
                .filter(|&&lib_id| {
                    if let Some(obj) = state.get_object(lib_id) {
                        if !obj.card_types.is_empty() {
                            obj.card_types.contains(&CardType::Creature)
                        } else {
                            registry.card_data(obj.card_id)
                                .map(|d| d.card_types.contains(&CardType::Creature))
                                .unwrap_or(false)
                        }
                    } else {
                        false
                    }
                })
                .copied()
                .collect();

            if creature_options.is_empty() {
                state.log(LogLevel::Event,
                    "Garruk, the Veil-Cursed: no creature card found in library".into());
                // Still shuffle even if nothing found.
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                state.get_player_mut(controller).library_order.shuffle(&mut rng);
            } else if creature_options.len() == 1 {
                // Only one option — auto-select and shuffle.
                let found_id = creature_options[0];
                let found_name = state.get_object(found_id).map(|o| o.name.clone()).unwrap_or_default();
                let player = state.get_player_mut(controller);
                player.library_order.retain(|&lid| lid != found_id);
                state.move_object(found_id, Zone::Hand, registry);
                state.log(LogLevel::Event,
                    format!("Garruk, the Veil-Cursed: searched and found {}", found_name));
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                state.get_player_mut(controller).library_order.shuffle(&mut rng);
            } else {
                // Multiple options — present choice to player.
                state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                    player: controller,
                    source: *garruk_id,
                    choice: ResolutionChoiceKind::ChooseFromLibrary {
                        description: "Garruk, the Veil-Cursed: choose a creature card from your library".into(),
                        options: creature_options,
                        searcher: controller,
                        source_id: *garruk_id,
                    },
                });
            }
        }
        (Target::Object(id), PendingEffect::SacrificeCreature { source_name }) => {
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::sacrifice(state, *id, registry);
            state.log(LogLevel::Event, format!("{}: sacrificed {}", source_name, name));
        }
        (Target::Object(id), PendingEffect::DestroyThenCounter { source_id, source_name }) => {
            // Destroy the target creature, then add a +1/+1 counter to the source.
            // The counter is added regardless of whether destruction succeeds
            // (e.g. indestructible/regenerate), per MTG rules.
            let name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, *id, registry);
            state.log(LogLevel::Event, format!("{} destroyed {}", source_name, name));
            // Add +1/+1 counter to the source permanent.
            state.add_counters(*source_id, crate::types::CounterType::PlusOnePlusOne, 1);
            state.log(LogLevel::Event,
                format!("{}: +1/+1 counter from attack trigger", source_name));
        }
        (Target::Object(target_id), PendingEffect::CopyCreature { source_id }) => {
            // Copy the target creature's characteristics onto the source permanent.
            let (name, power, toughness, card_id, card_types, subtypes, keywords, colors, is_evil_twin) =
                match state.get_object(*target_id) {
                    Some(o) => {
                        let kw = registry.card_data(o.card_id)
                            .map(|d| d.keywords.clone())
                            .unwrap_or_default();
                        let evil_twin = o.card_state.contains_key("is_evil_twin");
                        (o.name.clone(), o.power, o.toughness, o.card_id,
                         o.card_types.clone(), o.subtypes.clone(), kw, o.colors.clone(), evil_twin)
                    }
                    None => return,
                };

            if let Some(obj) = state.get_object_mut(*source_id) {
                obj.name = name.clone();
                obj.power = power;
                obj.toughness = toughness;
                obj.card_id = card_id;
                obj.keywords = keywords;
                obj.card_types = card_types;
                obj.subtypes = subtypes;
                obj.colors = colors;
                // Always set the "is_evil_twin" marker on the source: CopyCreature is
                // only ever created by Evil Twin's ETB trigger, so the source is always
                // an Evil Twin that needs the destroy ability regardless of which
                // creature it copies. This also handles the case where another creature
                // copies an Evil Twin (the target carries the marker).
                let _ = is_evil_twin; // retained from target for documentation; source always gets it
                obj.card_state.insert("is_evil_twin".into(), ObjectId(1));
            }
            state.log(LogLevel::Event,
                format!("Evil Twin enters as a copy of {}", name));
        }
        (Target::Object(id), PendingEffect::KeepOneDestroyRest {
            remaining_players, kept_so_far, source_name,
        }) => {
            // Record this player's choice.
            let mut kept = kept_so_far.clone();
            kept.push(*id);
            let chosen_name = state.get_object(*id).map(|o| o.name.clone()).unwrap_or_default();
            let chooser = state.get_object(*id).map(|o| o.controller).unwrap_or(PlayerId(0));
            state.log(LogLevel::Event, format!("{}: p{} keeps {}", source_name, chooser.0, chosen_name));

            if remaining_players.is_empty() {
                // All players have chosen. Destroy every creature not in the kept set.
                let all_creatures: Vec<ObjectId> = state.objects.values()
                    .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
                    .map(|o| o.id)
                    .collect();
                for cid in all_creatures {
                    if !kept.contains(&cid) {
                        crate::destruction::try_destroy(state, cid, registry);
                    }
                }
            } else {
                // Chain to the next player's choice.
                let next_player = remaining_players[0];
                let rest = remaining_players[1..].to_vec();

                let options: Vec<crate::actions::Target> = state.objects.values()
                    .filter(|o| o.zone == Zone::Battlefield && o.controller == next_player && o.power.is_some())
                    .map(|o| crate::actions::Target::Object(o.id))
                    .collect();

                if options.len() <= 1 {
                    // 0 or 1 creature — auto-keep and continue.
                    if let Some(crate::actions::Target::Object(auto_id)) = options.first() {
                        kept.push(*auto_id);
                        let auto_name = state.get_object(*auto_id).map(|o| o.name.clone()).unwrap_or_default();
                        state.log(LogLevel::Event, format!("{}: p{} keeps {} (only creature)", source_name, next_player.0, auto_name));
                    }
                    // Continue chaining: apply as if this was a recursive call with rest.
                    // Use a simple loop to handle all auto-selects.
                    let mut remaining = rest;
                    loop {
                        if remaining.is_empty() {
                            // All done — destroy the rest.
                            let all_creatures: Vec<ObjectId> = state.objects.values()
                                .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
                                .map(|o| o.id)
                                .collect();
                            for cid in all_creatures {
                                if !kept.contains(&cid) {
                                    crate::destruction::try_destroy(state, cid, registry);
                                }
                            }
                            break;
                        }
                        let np = remaining[0];
                        let nr = remaining[1..].to_vec();
                        let np_options: Vec<crate::actions::Target> = state.objects.values()
                            .filter(|o| o.zone == Zone::Battlefield && o.controller == np && o.power.is_some())
                            .map(|o| crate::actions::Target::Object(o.id))
                            .collect();
                        if np_options.len() <= 1 {
                            if let Some(crate::actions::Target::Object(auto_id)) = np_options.first() {
                                kept.push(*auto_id);
                                let auto_name = state.get_object(*auto_id).map(|o| o.name.clone()).unwrap_or_default();
                                state.log(LogLevel::Event, format!("{}: p{} keeps {} (only creature)", source_name, np.0, auto_name));
                            }
                            remaining = nr;
                        } else {
                            // Present choice to this player.
                            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                                player: np,
                                source: ObjectId(0),
                                choice: crate::state::ResolutionChoiceKind::ChooseTarget {
                                    description: format!("{}: choose a creature you control to keep", source_name),
                                    options: np_options,
                                    optional: false,
                                    effect: PendingEffect::KeepOneDestroyRest {
                                        remaining_players: nr,
                                        kept_so_far: kept.clone(),
                                        source_name: source_name.clone(),
                                    },
                                },
                            });
                            break;
                        }
                    }
                } else {
                    // Present choice to the next player.
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player: next_player,
                        source: ObjectId(0),
                        choice: crate::state::ResolutionChoiceKind::ChooseTarget {
                            description: format!("{}: choose a creature you control to keep", source_name),
                            options,
                            optional: false,
                            effect: PendingEffect::KeepOneDestroyRest {
                                remaining_players: rest,
                                kept_so_far: kept,
                                source_name: source_name.clone(),
                            },
                        },
                    });
                }
            }
        }
        (Target::Object(curse_id), PendingEffect::ChooseCurseThenAttach { searcher, source }) => {
            // Player chose which Curse from library — now present the "target player" choice.
            // Filter out players with hexproof (e.g. Witchbane Orb); they can't be targeted.
            let player_targets: Vec<crate::actions::Target> = (0..state.players.len())
                .map(|i| PlayerId(i as u8))
                .filter(|&pid| !state.player_has_hexproof(pid, registry) || pid == *searcher)
                .map(|pid| crate::actions::Target::Player(pid))
                .collect();
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: *searcher,
                source: *source,
                choice: crate::state::ResolutionChoiceKind::ChooseTarget {
                    description: "Bitterheart Witch: choose a player to attach the Curse to".into(),
                    options: player_targets,
                    optional: false,
                    effect: PendingEffect::AttachCurseToPlayer {
                        curse_id: *curse_id,
                        searcher: *searcher,
                    },
                },
            });
        }
        (Target::Player(pid), PendingEffect::AttachCurseToPlayer { curse_id, searcher }) => {
            let name = state.get_object(*curse_id).map(|o| o.name.clone()).unwrap_or_default();
            // Remove from library.
            state.get_player_mut(*searcher).library_order.retain(|&id| id != *curse_id);
            // Put on battlefield attached to the chosen player.
            state.move_object(*curse_id, Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(*curse_id) {
                obj.attached_to_player = Some(*pid);
                obj.summoning_sick = false;
            }
            state.log(LogLevel::Event,
                format!("Bitterheart Witch: attached {} to p{}", name, pid.0));
            // Shuffle library.
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(*searcher).library_order.shuffle(&mut rng);
        }
        (Target::Object(target_id), PendingEffect::GrantFlashback { source_name }) => {
            // Grant flashback to the chosen card until end of turn.
            if let Some(obj) = state.get_object(*target_id) {
                let card_id = obj.card_id;
                let cost = registry.card_data(card_id)
                    .and_then(|d| d.cost.clone())
                    .unwrap_or(ManaCost::free());
                let name = obj.name.clone();
                state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantFlashback { target: *target_id, cost });
                state.log(LogLevel::Event,
                    format!("{} grants flashback to {}", source_name, name));
            }
        }
        (Target::Object(land_id), PendingEffect::GhostQuarterSearch { searcher }) => {
            // Put the chosen basic land onto the battlefield, then shuffle.
            let name = state.get_object(*land_id).map(|o| o.name.clone()).unwrap_or_default();
            state.get_player_mut(*searcher).library_order.retain(|&id| id != *land_id);
            state.move_object(*land_id, Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(*land_id) {
                obj.summoning_sick = false;
            }
            state.log(LogLevel::Event,
                format!("Ghost Quarter: p{} searched for {}", searcher.0, name));
            // Shuffle the library.
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(*searcher).library_order.shuffle(&mut rng);
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
        draw_cards(&mut state, player_id, 7, registry);
    }

    state.events.push(GameEvent::GameStarted);
    state.log(LogLevel::Milestone, format!("── Turn 1 (p0) ──"));
    state
}

/// Draw N cards for a player. Logs a single summary entry.
pub fn draw_cards(state: &mut GameState, player: PlayerId, count: usize, registry: &CardRegistry) {
    let mut drawn = 0;
    for _ in 0..count {
        let card_id = {
            let player_state = state.get_player_mut(player);
            player_state.draw_top_card()
        };
        match card_id {
            Some(id) => {
                state.move_object(id, Zone::Hand, registry);
                state.events.push(GameEvent::CardDrawn { player, object: id });
                drawn += 1;
            }
            None => {
                // Check for ReplaceEmptyDraw replacement effect (e.g. Laboratory Maniac):
                // if the player controls a permanent with this effect, they win instead.
                let has_replace_empty_draw = state.objects.values().any(|o| {
                    o.zone == Zone::Battlefield
                        && o.controller == player
                        && registry.get(o.card_id)
                            .map(|b| b.replacement_effects().contains(&crate::types::ReplacementEffect::ReplaceEmptyDraw))
                            .unwrap_or(false)
                });
                if has_replace_empty_draw {
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
                    let source_name = state.objects.values()
                        .find(|o| o.zone == Zone::Battlefield && o.controller == player
                            && registry.get(o.card_id)
                                .map(|b| b.replacement_effects().contains(&crate::types::ReplacementEffect::ReplaceEmptyDraw))
                                .unwrap_or(false))
                        .map(|o| o.name.clone())
                        .unwrap_or_default();
                    state.log(LogLevel::Milestone,
                        format!("p{} wins the game with {}!", player.0, source_name));
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
pub fn mill_cards(state: &mut GameState, player: PlayerId, count: usize, registry: &CardRegistry) {
    let mut milled = 0;
    for _ in 0..count {
        let card_id = {
            let player_state = state.get_player_mut(player);
            if player_state.library_order.is_empty() {
                break;
            }
            player_state.library_order.remove(0)
        };
        state.move_object(card_id, Zone::Graveyard, registry);
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
                // Instants can be cast at sorcery speed too (main phase, empty stack).
                instants_relevant || is_sorcery_speed
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
            for ab in behavior.activated_abilities(state, obj.id, registry) {
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
                draw_cards(state, active, 1, registry);
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
            let sba = check_state_based_actions(state, registry);
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
