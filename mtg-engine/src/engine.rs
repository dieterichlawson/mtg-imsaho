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
use crate::types::{Zone, CardType, Supertype, ManaCost, ManaSymbol, ContinuousEffect, Keyword, CounterType, Step, Color};

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

/// Gather all available mana sources for a player, classified by opportunity cost.
pub(crate) fn gather_mana_sources(
    state: &GameState,
    player: PlayerId,
    registry: &CardRegistry,
    prevent_artifact_abilities: bool,
) -> Vec<mana::ManaSource> {
    use mana::{ManaSource, ManaSourceKind};

    let mut sources = Vec::new();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        // Stony Silence: skip mana abilities from artifacts.
        if prevent_artifact_abilities
            && state.has_card_type(obj.id, CardType::Artifact, registry) {
            continue;
        }
        if let Some(behavior) = registry.get(obj.card_id) {
            let abilities = behavior.mana_abilities(state, obj.id);
            if abilities.is_empty() { continue; }

            // Classify the source kind.
            let has_side_effects = abilities.iter().any(|a| a.has_side_effects);
            let is_creature = state.is_creature(obj.id, registry);
            let has_utility = !behavior.activated_abilities(state, obj.id, registry).is_empty();
            let is_basic = state.face_data(obj.id, registry)
                .is_some_and(|d| d.supertypes.contains(&Supertype::Basic));

            let source_kind = if has_side_effects {
                ManaSourceKind::HasSideEffects
            } else if is_creature {
                ManaSourceKind::Creature
            } else if has_utility {
                ManaSourceKind::HasUtilityAbility
            } else if is_basic {
                ManaSourceKind::BasicMana
            } else {
                ManaSourceKind::NonBasicMana
            };

            sources.push(ManaSource {
                object_id: obj.id,
                abilities,
                source_kind,
            });
        }
    }
    sources
}

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
        }).collect();
        format!(" targeting {}", names.join(", "))
    };
    state.log(LogLevel::Event, format!("p{} cast {}{}{}", player.0, name, suffix, target_str));
    state.consecutive_passes = 0;
}

/// Activate a single mana source (tap + add mana + side effects).
/// Shared by both `ActivateManaAbility` and `CastSpell` `tap_plan` execution.
pub(crate) fn activate_mana_source(
    state: &mut GameState,
    source_id: ObjectId,
    ability_index: usize,
    registry: &CardRegistry,
) {
    let obj = state.get_object(source_id).expect("mana source must exist");
    let card_id = obj.card_id;
    let controller = obj.controller;

    if let Some(behavior) = registry.get(card_id) {
        let abilities = behavior.mana_abilities(state, source_id);
        if let Some(ability) = abilities.iter().find(|a| a.ability_index == ability_index) {
            if ability.requires_tap {
                state.get_object_mut(source_id).expect("object must exist for tapping").tapped = true;
                state.events.push(GameEvent::Tapped { object: source_id });
            }
            for &(mana_type, amount) in &ability.produced {
                state.get_player_mut(controller).mana_pool.add(mana_type, amount);
                state.events.push(GameEvent::ManaAdded {
                    player: controller,
                    mana_type,
                    amount,
                });
            }
            behavior.on_activate_mana_ability(state, source_id, ability_index, registry);
        }
    }

    let name = card_name(state, registry, source_id);
    let pool = &state.get_player(controller).mana_pool;
    let pool_str: Vec<String> = pool.mana.iter()
        .filter(|(_, &v)| v > 0)
        .map(|(t, v)| format!("{t:?}:{v}"))
        .collect();
    state.log(LogLevel::Info, format!("p{} tapped {} for mana (pool: {})",
        controller.0, name, if pool_str.is_empty() { "empty".into() } else { pool_str.join(" ") }));
}

/// Compute an autotap plan for paying `cost` from `player`'s pool + untapped
/// mana sources, without modifying state. Returns `None` if the cost is not
/// autotap-reachable. Callers pair this with [`execute_tap_plan_and_pay`] to
/// actually commit. Used by card handlers (e.g. Screeching Bat's may-pay
/// upkeep) that need to know whether a mid-resolution cost is affordable
/// *before* asking the player — pool-only checks miss the common case where
/// the pool is empty but enough untapped sources are available (CR 106.4:
/// mana empties between steps).
#[must_use]
pub(crate) fn plan_autotap_for_cost(
    state: &GameState,
    player: PlayerId,
    cost: &ManaCost,
    registry: &CardRegistry,
) -> Option<Vec<(ObjectId, usize)>> {
    let sources = gather_mana_sources(state, player, registry, false);
    let pool = &state.get_player(player).mana_pool;
    mana::compute_autotap(cost, pool, &sources, &[])
}

/// Execute a tap plan and deduct `cost` from the resulting pool.
/// Returns `true` on success, `false` if `auto_pay` fails (which shouldn't
/// happen when `tap_plan` came from `compute_autotap` on the same `cost`).
pub(crate) fn execute_tap_plan_and_pay(
    state: &mut GameState,
    player: PlayerId,
    cost: &ManaCost,
    tap_plan: &[(ObjectId, usize)],
    registry: &CardRegistry,
) -> bool {
    for &(src_id, ability_index) in tap_plan {
        activate_mana_source(state, src_id, ability_index, registry);
    }
    mana::auto_pay(&mut state.get_player_mut(player).mana_pool, cost).is_ok()
}

/// Find alternative costs provided by continuous effects on permanents the caster controls.
/// Returns a list of alternative `ManaCosts` that the caster may use for the given spell.
fn alternative_costs_from_effects(state: &GameState, registry: &CardRegistry, card_id: CardId, caster: PlayerId) -> Vec<ManaCost> {
    use crate::types::{ContinuousEffect, SpellFilter};

    let card_data = registry.card_data(card_id);
    let is_creature = card_data.as_ref()
        .is_some_and(|d| d.card_types.contains(&CardType::Creature));
    let subtypes: Vec<String> = card_data.as_ref()
        .map(|d| d.subtypes.clone())
        .unwrap_or_default();

    let mut alt_costs = Vec::new();
    for obj in state.objects.values() {
        if obj.zone != Zone::Battlefield || obj.controller != caster {
            continue;
        }
        if let Some(behavior) = registry.get(obj.card_id) {
            for effect in &behavior.card_data().continuous_effects {
                if let ContinuousEffect::AlternativeCost { cost, filter } = effect {
                    let applies = match filter {
                        SpellFilter::CreatureSpells => is_creature,
                        SpellFilter::CreatureWithSubtype(sub) => {
                            is_creature && subtypes.iter().any(|s| s == sub)
                        }
                    };
                    if applies {
                        alt_costs.push(cost.clone());
                    }
                }
            }
        }
    }
    alt_costs
}

/// Compute the effective mana cost of a spell after applying cost reduction effects.
/// Returns a reduced `ManaCost` (generic portion lowered, colored requirements unchanged).
#[must_use]
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
        .is_some_and(|d| d.card_types.contains(&CardType::Creature));
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

    // Check for PreventArtifactAbilities effect (e.g. Stony Silence):
    // no abilities of artifacts can be activated, including mana abilities.
    let prevent_artifact_abilities = state.objects.values().any(|o| {
        if o.zone != Zone::Battlefield { return false; }
        // Check instance effects.
        if let Some(ref effects) = o.instance_continuous_effects {
            if effects.iter().any(|e| matches!(e, ContinuousEffect::PreventArtifactAbilities)) {
                return true;
            }
        }
        // Check card data effects.
        registry.get(o.card_id)
            .is_some_and(|b| b.card_data().continuous_effects.iter()
                .any(|e| matches!(e, ContinuousEffect::PreventArtifactAbilities)))
    });

    // Mana abilities: can activate anytime you have priority.
    // Deduplicate by card_id — if you have 5 untapped Forests, only show one "Tap Forest".
    let mut seen_mana_abilities: Vec<(CardId, usize)> = Vec::new();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        // Stony Silence: skip mana abilities from artifacts.
        if prevent_artifact_abilities {
            if state.has_card_type(obj.id, CardType::Artifact, registry) { continue; }
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
            let has_x_cost = ab.cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X));
            // Autotap sources to consider for this specific ability.
            let ability_sources: Vec<_> = if ability_has_sac_this {
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
                    ManaCost::new(
                        ab.cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                    )
                } else {
                    ab.cost.clone()
                };
                if !mana::can_pay(mana_pool, &cost_to_check) { continue; }
                Vec::new()
            } else if has_x_cost {
                let non_x_cost = ManaCost::new(
                    ab.cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                );
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
                .is_some_and(|c| c.symbols.iter().any(|s| matches!(s, ManaSymbol::X)));
            let (can_pay_normal, normal_tap_plan) = if has_x {
                // X-cost spells: only autotap for the non-X portion. After the
                // agent chooses X, a second autotap pass covers the X generic.
                if let Some(cost) = &data.cost {
                    let effective_cost = effective_spell_cost(state, registry, obj.card_id, cost, player);
                    let non_x_cost = ManaCost::new(
                        effective_cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                    );
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
            let fb_has_x = fb_cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X));
            let fb_non_x_cost;
            let fb_cost_for_autotap: &ManaCost = if fb_has_x {
                fb_non_x_cost = ManaCost::new(
                    fb_cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                );
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

/// Check targeting legality, including protection from the source.
/// `source_id` is the spell or permanent whose ability is targeting.
#[must_use]
pub fn can_be_targeted_by(state: &GameState, target_id: ObjectId, caster: PlayerId, source_id: Option<ObjectId>, registry: &CardRegistry) -> bool {
    if state.has_keyword(target_id, Keyword::Hexproof, registry) {
        let controller = state.get_object(target_id)
            .map_or(PlayerId(255), |o| o.controller);
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

/// Determine which mode of a `ModalChoice` was selected, based on the chosen targets.
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

/// Get valid targets for a single mode requirement, unwrapping `UpToTargets`.
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

/// Generate `CastSpell` actions with all valid target combinations.
fn generate_cast_actions_with_targets(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    target_req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
) -> Vec<Action> {
    use crate::cards::TargetRequirement;
    let registry = &CardRegistry::with_all_cards();

    match target_req {
        TargetRequirement::None => {
            vec![Action::CastSpell { object_id: spell_id, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] }]
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
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
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
                        sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
                    });
                }
            }
            actions
        }
        // All single-target requirement kinds share the canonical target
        // enumeration in `valid_targets_for_req` — one target per action.
        _ => {
            valid_targets_for_req(state, caster, spell_id, target_req, behavior, registry)
                .into_iter()
                .map(|t| Action::CastSpell { object_id: spell_id, targets: vec![t], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] })
                .collect()
        }
    }
}

/// Helper: collect all valid targets for a single-target requirement.
pub(crate) fn valid_targets_for_req(
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
        TargetRequirement::Creature => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry))
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::CreatureWithFilter(filter) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry))
                .filter(|o| matches_target_filter(state, o, filter, caster, Some(spell_id), registry))
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::Spell => {
            // Only spells on the stack can be targeted (not triggered abilities).
            state.stack.iter()
                .filter_map(super::state::StackEntry::as_spell)
                .filter(|&id| id != spell_id)
                .map(Target::Object)
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::PermanentWithFilter(filter) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| matches_target_filter(state, o, filter, caster, Some(spell_id), registry))
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::AnyTarget => {
            let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry)
                    || state.has_card_type(o.id, CardType::Planeswalker, registry))
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
                let is_pw = state.has_card_type(obj.id, CardType::Planeswalker, registry);
                if is_pw && can_be_targeted_by(state, obj.id, caster, Some(spell_id), registry) {
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
            // Creature cards in caster's graveyard.
            state.objects.values()
                .filter(|o| {
                    o.zone == Zone::Graveyard
                        && o.owner == caster
                        && state.is_creature(o.id, registry)
                })
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCreatureOfSubtype(ref subtype) => {
            // Creature cards with a specific subtype in all graveyards.
            state.objects.values()
                .filter(|o| {
                    o.zone == Zone::Graveyard
                        && state.is_creature(o.id, registry)
                        && state.has_subtype(o.id, subtype, registry)
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

/// Build a `CastTargetSpec` for a spell, describing what targets the player needs to choose.
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

/// Check whether an object satisfies a `TargetFilter`.
///
/// The single canonical filter matcher — used by spell targeting, ability
/// targeting, and resolution-time legality checks (stack.rs). All
/// characteristic lookups go through the `GameState` characteristics layer,
/// so non-token permanents (empty object-level fields) and transformed DFCs
/// are handled uniformly.
///
/// `source_id` is the permanent or spell the targeting originates from; it
/// only affects `Another` and `SameNameAsSource`. Pass `None` when no source
/// is available (resolution-time recheck), which leaves `Another`
/// unrestricted.
pub(crate) fn matches_target_filter(
    state: &GameState,
    obj: &crate::state::GameObject,
    filter: &crate::cards::TargetFilter,
    controller: PlayerId,
    source_id: Option<ObjectId>,
    registry: &CardRegistry,
) -> bool {
    use crate::cards::TargetFilter;
    match filter {
        TargetFilter::Any => true,
        TargetFilter::YouControl => obj.controller == controller,
        TargetFilter::YouDontControl => obj.controller != controller,
        TargetFilter::Nonblack => !state.colors_of(obj.id, registry).contains(&Color::Black),
        TargetFilter::NotSubtypes(types) => {
            let subtypes = state.subtypes_of(obj.id, registry);
            !types.iter().any(|t| subtypes.contains(t))
        }
        TargetFilter::PowerAtLeast(n) => {
            state.effective_power(obj.id, registry).unwrap_or(0) >= *n
        }
        TargetFilter::Attacking => {
            state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&obj.id))
        }
        TargetFilter::Noncreature => !state.is_creature(obj.id, registry),
        TargetFilter::HasCardType(types) => {
            types.iter().any(|t| state.has_card_type(obj.id, *t, registry))
        }
        TargetFilter::SubtypeOrCardType { subtypes, card_types } => {
            subtypes.iter().any(|s| state.has_subtype(obj.id, s, registry))
                || card_types.iter().any(|t| state.has_card_type(obj.id, *t, registry))
        }
        TargetFilter::HasSubtype(subtype) => state.has_subtype(obj.id, subtype, registry),
        TargetFilter::HasKeyword(keyword) => state.has_keyword(obj.id, *keyword, registry),
        TargetFilter::Another => source_id.is_none_or(|s| obj.id != s),
        TargetFilter::SameNameAsSource => {
            source_id
                .and_then(|s| state.get_object(s))
                .is_some_and(|source| source.name == obj.name)
        }
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

    let Some(target_req) = &ab.target_requirement else { return vec![]; };

    match target_req {
        TargetRequirement::Creature => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry))
                .filter(|o| can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::CreatureWithFilter(filter) => {
            // CR 702.6a: equip is "Attach this permanent to target creature you
            // control" — nothing excludes the creature it is already attached
            // to. This used to filter that creature out as a UX shortcut, which
            // made a legal play unavailable: re-equipping to the same host is
            // the point whenever the equip COST is what you want (Demonmail
            // Hauberk sacrificing a different creature), and with only one
            // creature on the battlefield it removed the ability entirely.
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry))
                .filter(|o| can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
                .filter(|o| matches_target_filter(state, o, filter, controller, Some(source_id), registry))
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
                let is_pw = state.has_card_type(obj.id, CardType::Planeswalker, registry);
                if is_pw && can_be_targeted_by(state, obj.id, controller, Some(source_id), registry) {
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
                .filter(|o| state.is_creature(o.id, registry)
                    || state.has_card_type(o.id, CardType::Planeswalker, registry))
                .filter(|o| can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
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
                .filter(|o| can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
                .filter(|o| matches_target_filter(state, o, filter, controller, Some(source_id), registry))
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

fn card_name(state: &GameState, _registry: &CardRegistry, obj_id: ObjectId) -> String {
    state.obj_name(obj_id)
}

/// Apply an action to the game state and return the new state.
///
/// # Panics
/// Panics if the action requires preconditions that are not met in `state` —
/// for example, submitting `PlayLand` when no player currently has priority,
/// submitting an action that references an object or registry id that does
/// not exist, or otherwise violating invariants that `legal_actions` would
/// have filtered out.
#[must_use]
pub fn submit_action(state: &GameState, action: &Action, registry: &CardRegistry) -> GameState {
    use crate::cards::SacrificeCost;

    let mut new_state = state.clone();
    new_state.events.clear();
    new_state.trigger_event_index = 0;

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

        Action::CastSpell { object_id, targets, sacrifice, exile_count, exile_ids, alternative_cost, tap_plan } => {
            let player = new_state.priority_player.expect("CastSpell requires priority");

            // Detect flashback vs cast-from-graveyard.
            // Flashback: card has flashback_cost or dynamically granted flashback.
            // Cast-from-graveyard: card has can_cast_from_graveyard() (Skaab Ruinator) — uses normal mana cost.
            let card_id = new_state.get_object(*object_id).expect("CastSpell object must exist").card_id;
            let data = registry.get(card_id).expect("card must be in registry").card_data();
            let behavior = registry.get(card_id).expect("card must be in registry");
            let in_graveyard = new_state.get_object(*object_id)
                .is_some_and(|o| o.zone == Zone::Graveyard);
            let is_cast_from_graveyard = in_graveyard && behavior.can_cast_from_graveyard();
            let is_flashback = in_graveyard && !is_cast_from_graveyard;

            // Resolve the appropriate mana cost (applying cost reduction for
            // non-flashback). If an alternative_cost is provided (e.g.
            // Rooftop Storm's {0}), use it directly.
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
            let has_x = cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X));

            if has_x {
                let non_x_cost = ManaCost::new(
                    cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                );
                // Probe max_x without touching state: simulate tap_plan's
                // mana output + existing pool + any remaining untapped
                // sources after the tap_plan runs.
                let probe_options = {
                    // Apply tap_plan and non-X payment in a clone to see
                    // what's left for X.
                    let mut probe = new_state.clone();
                    for &(source_id, ability_index) in tap_plan {
                        activate_mana_source(&mut probe, source_id, ability_index, registry);
                    }
                    let _ = mana::auto_pay(&mut probe.get_player_mut(player).mana_pool, &non_x_cost);
                    crate::funding::build_options(&probe, player, registry)
                };

                if probe_options.max_x > 0 {
                    // Stash context; leave spell in hand. Set up the prompt.
                    let spell_name = card_name(&new_state, registry, *object_id);
                    new_state.pending_spell_cast = Some(crate::state::PendingSpellCast {
                        object_id: *object_id,
                        player,
                        card_id,
                        targets: targets.clone(),
                        sacrifice: *sacrifice,
                        exile_ids: exile_ids.clone(),
                        exile_count: *exile_count,
                        tap_plan: tap_plan.clone(),
                        alternative_cost: alternative_cost.clone(),
                        non_x_mana_cost: non_x_cost,
                        is_flashback,
                    });
                    new_state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                        player,
                        source: *object_id,
                        choice: crate::state::ResolutionChoiceKind::ChooseXFunding {
                            description: format!("{spell_name}: choose X funding (0-{})", probe_options.max_x),
                            options: probe_options,
                            source_id: *object_id,
                            is_ability: false,
                        },
                    });
                    // Nothing else happens until the player submits the
                    // funding response — spell stays in hand, no mana
                    // paid, no taps executed.
                    return new_state;
                }
                // max_x == 0: no legal funding choice, X is forced to 0.
                // Fall through to the eager path and pay as X=0.
            }

            // Rules-strict exile-cost casting: for spells with an
            // `ExileXFromGraveyard` or `ExileCreaturesFromGraveyard(n)`
            // additional cost, set up a `ChooseExileFromGraveyard` prompt
            // and leave the spell in hand until the player submits
            // `ChosenExileSet`. Mirrors the ChooseXFunding flow above.
            //
            // The prompt is only set up if the caller left `exile_ids`
            // empty AND hasn't specified an exile_count (for variable-X
            // exile cost). That lets tests and other code paths submit
            // an already-resolved `CastSpell` with specific exile_ids
            // (or an explicit X count) and bypass the prompt.
            {
                use crate::cards::AdditionalCost;
                let additional = data.additional_cost.clone();
                let needs_exile_prompt = exile_ids.is_empty() && match &additional {
                    Some(AdditionalCost::ExileXFromGraveyard) => exile_count.is_none(),
                    Some(AdditionalCost::ExileCreaturesFromGraveyard(_)) => true,
                    _ => false,
                };
                if needs_exile_prompt {
                    let (gy_options, min, max) = match &additional {
                        Some(AdditionalCost::ExileXFromGraveyard) => {
                            // Any card in the caster's graveyard is eligible,
                            // except the spell itself (if cast from GY).
                            let opts: Vec<ObjectId> = new_state.objects.values()
                                .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id)
                                .map(|o| o.id)
                                .collect();
                            let n = opts.len();
                            (opts, 0usize, n)
                        }
                        Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) => {
                            // Only creature cards in GY.
                            let opts: Vec<ObjectId> = new_state.objects.values()
                                .filter(|o| {
                                    o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id
                                        && state.is_creature(o.id, registry)
                                })
                                .map(|o| o.id)
                                .collect();
                            (opts, *n, *n)
                        }
                        _ => unreachable!(),
                    };

                    let spell_name = card_name(&new_state, registry, *object_id);
                    let description = match &additional {
                        Some(AdditionalCost::ExileXFromGraveyard) => format!(
                            "{spell_name}: choose 0-{} cards to exile from your graveyard (each exiled card adds to the spell's X)",
                            gy_options.len()
                        ),
                        Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) => format!(
                            "{spell_name}: choose exactly {n} creature{} to exile from your graveyard",
                            if *n == 1 { "" } else { "s" }
                        ),
                        _ => unreachable!(),
                    };

                    // For X-cost spells, the non_x_mana_cost is the stripped
                    // cost. For non-X spells, use the full cost.
                    let non_x_mana_cost = if has_x {
                        ManaCost::new(
                            cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                        )
                    } else {
                        cost.clone()
                    };

                    new_state.pending_spell_cast = Some(crate::state::PendingSpellCast {
                        object_id: *object_id,
                        player,
                        card_id,
                        targets: targets.clone(),
                        sacrifice: *sacrifice,
                        exile_ids: exile_ids.clone(),
                        exile_count: *exile_count,
                        tap_plan: tap_plan.clone(),
                        alternative_cost: alternative_cost.clone(),
                        non_x_mana_cost,
                        is_flashback,
                    });
                    new_state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                        player,
                        source: *object_id,
                        choice: crate::state::ResolutionChoiceKind::ChooseExileFromGraveyard {
                            description,
                            options: gy_options,
                            min,
                            max,
                            source_id: *object_id,
                        },
                    });
                    // Spell stays in hand; no mana tapped or paid yet.
                    return new_state;
                }
            }

            // Eager path: non-X spells and X-cost spells with max_x == 0.
            // Execute tap_plan, pay mana, pay additional costs, move to stack.
            for &(source_id, ability_index) in tap_plan {
                activate_mana_source(&mut new_state, source_id, ability_index, registry);
            }

            if has_x {
                // max_x == 0 case: pay only the non-X portion.
                let non_x_cost = ManaCost::new(
                    cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                );
                mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &non_x_cost)
                    .expect("legal_actions should have verified mana availability");
            } else {
                mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &cost)
                    .expect("legal_actions should have verified mana availability");
            }

            // Pay additional costs (sacrifice) at cast time, before the spell goes on the stack.
            if let Some(sac_id) = sacrifice {
                let sac_name = card_name(&new_state, registry, *sac_id);
                crate::destruction::sacrifice(&mut new_state, *sac_id, registry);
                new_state.log(LogLevel::Event,
                    format!("Sacrificed {sac_name} as additional cost"));
            } else {
                // Backward compatibility: if sacrifice is None but the spell has
                // AdditionalCost::SacrificeCreature, auto-sacrifice the first creature.
                use crate::cards::AdditionalCost;
                let needs_sac = registry.get(card_id)
                    .is_some_and(|b| matches!(b.card_data().additional_cost, Some(AdditionalCost::SacrificeCreature)));
                if needs_sac {
                    let creature = new_state.objects_in_zone(Zone::Battlefield, player)
                        .iter()
                        .find(|o| new_state.is_creature(o.id, registry))
                        .map(|o| o.id);
                    if let Some(cid) = creature {
                        let sac_name = card_name(&new_state, registry, cid);
                        crate::destruction::sacrifice(&mut new_state, cid, registry);
                        new_state.log(LogLevel::Event,
                            format!("Sacrificed {sac_name} as additional cost"));
                    }
                }
            }

            // Handle ExileCreaturesFromGraveyard additional cost (Skaab Ruinator, Corpse Lunge, etc.).
            {
                use crate::cards::AdditionalCost;
                if let Some(AdditionalCost::ExileCreaturesFromGraveyard(n)) = registry.get(card_id)
                    .and_then(|b| b.card_data().additional_cost)
                {
                    // Use player-chosen exile_ids if provided, otherwise fall back to auto-pick.
                    let to_exile: Vec<ObjectId> = if exile_ids.is_empty() {
                        let mut exile_candidates: Vec<(ObjectId, i32)> = new_state.objects.values()
                            .filter(|o| {
                                o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id
                                    && state.is_creature(o.id, registry)
                            })
                            .map(|o| (o.id, o.power.unwrap_or(0)))
                            .collect();
                        exile_candidates.sort_by(|a, b| b.1.cmp(&a.1));
                        exile_candidates.into_iter().take(n).map(|(id, _)| id).collect()
                    } else {
                        exile_ids.clone()
                    };

                    // Store the first exiled creature's power for cards that need it
                    // (Corpse Lunge uses the power to determine damage).
                    // Use `effective_power` so characteristic-defining-ability creatures
                    // (Boneyard Wurm, Splinterfright, etc.) whose P/T is a function of
                    // graveyard state — which CR 208.2 says "works in all zones" —
                    // store their CDA-computed power instead of the base 0.
                    if let Some(&first_exile) = to_exile.first() {
                        let power = new_state.effective_power(first_exile, registry).unwrap_or(0);
                        if let Some(obj) = new_state.get_object_mut(*object_id) {
                            obj.card_state.insert("exiled_power".into(), ObjectId(u64::try_from(power).unwrap_or(0)));
                        }
                    }

                    for exile_id in &to_exile {
                        let name = card_name(&new_state, registry, *exile_id);
                        new_state.move_object(*exile_id, Zone::Exile, registry);
                        new_state.log(LogLevel::Event,
                            format!("Exiled {name} from graveyard as additional cost"));
                    }
                }
            }

            // Handle ExileXFromGraveyard additional cost (Harvest Pyre).
            // The player chose X via exile_count in the action.
            {
                use crate::cards::AdditionalCost;
                let needs_exile_x = registry.get(card_id)
                    .is_some_and(|b| matches!(b.card_data().additional_cost, Some(AdditionalCost::ExileXFromGraveyard)));
                if needs_exile_x {
                    // If specific cards were chosen (via exile_ids), exile those exactly.
                    // Otherwise fall back to auto-selecting the first exile_count cards (legacy behavior).
                    let graveyard_cards: Vec<ObjectId> = if exile_ids.is_empty() {
                        let x = exile_count.unwrap_or(0) as usize;
                        new_state.objects.values()
                            .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id)
                            .map(|o| o.id)
                            .take(x)
                            .collect()
                    } else {
                        exile_ids.clone()
                    };
                    let count = u32::try_from(graveyard_cards.len()).unwrap_or(u32::MAX);
                    for gid in &graveyard_cards {
                        new_state.move_object(*gid, Zone::Exile, registry);
                    }
                    // Store the count on the spell for resolution.
                    if let Some(obj) = new_state.get_object_mut(*object_id) {
                        obj.card_state.insert("exile_count".into(), ObjectId(u64::from(count)));
                    }
                    new_state.log(LogLevel::Event,
                        format!("Exiled {count} cards from graveyard as additional cost"));
                }
            }

            // Move to stack and store targets.
            new_state.move_object(*object_id, Zone::Stack, registry);
            {
                let obj = new_state.get_object_mut(*object_id).expect("spell must exist after moving to stack");
                obj.targets.clone_from(targets);
                if is_flashback {
                    obj.cast_with_flashback = true;
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

            if has_x {
                // Eager path reached here only when max_x == 0 (X forced to
                // 0). The funding-prompt path already returned earlier.
                if let Some(obj) = new_state.get_object_mut(*object_id) {
                    obj.x_value = Some(0);
                }
            }
            finalize_spell_cast(&mut new_state, player, *object_id, is_flashback, targets, registry);
        }

        Action::ActivateManaAbility { object_id, ability_index } => {
            activate_mana_source(&mut new_state, *object_id, *ability_index, registry);
        }

        Action::ActivateAbility { object_id, ability_index, targets, tap_plan, sacrifice, source_card_id, .. } => {
            let player = new_state.priority_player.expect("ActivateAbility requires priority");

            // Execute autotap plan: tap mana sources to fill the mana pool before
            // we attempt to pay the ability's mana cost. This mirrors CastSpell.
            for &(source_id, ma_idx) in tap_plan {
                activate_mana_source(&mut new_state, source_id, ma_idx, registry);
            }

            let obj = new_state.get_object(*object_id).expect("activated ability object must exist");
            let card_id = obj.card_id;
            let copy_grantor = new_state.get_object(*object_id).and_then(|o| o.copy_grantor);

            // Resolve which card's behavior contributed this ability:
            // - Some(cid): caller explicitly disambiguated the source — used by
            //   legal_actions to mark aura-granted abilities. Look up in cid only.
            // - None: backward-compat chained lookup (native → copy-grantor
            //   override → attached auras). Used by tests and code paths that
            //   don't need to disambiguate (only one contributes the ability).
            let (behavior_card_id, ability) = if let Some(cid) = *source_card_id {
                let ab = registry.get(cid)
                    .and_then(|b| b.activated_abilities(&new_state, *object_id, registry)
                        .into_iter().find(|a| a.ability_index == *ability_index));
                (cid, ab)
            } else {
                let native = registry.get(card_id)
                    .and_then(|b| b.activated_abilities(&new_state, *object_id, registry)
                        .into_iter().find(|a| a.ability_index == *ability_index));
                if native.is_some() {
                    (card_id, native)
                } else if copy_grantor.is_some() {
                    // CR 706.2: an ability the copy effect added — dispatch to
                    // the card whose copy effect granted it.
                    let g_id = copy_grantor.filter(|&g| g != card_id);
                    let ab = g_id.and_then(|cid| registry.get(cid))
                        .and_then(|b| b.activated_abilities(&new_state, *object_id, registry)
                            .into_iter().find(|a| a.ability_index == *ability_index));
                    if let Some(ab) = ab {
                        (g_id.unwrap_or(card_id), Some(ab))
                    } else {
                        // Fall through to attached lookup below.
                        let mut found = (card_id, None);
                        for attached in new_state.objects.values()
                            .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(*object_id))
                        {
                            if let Some(ab) = registry.get(attached.card_id)
                                .and_then(|b| b.activated_abilities(&new_state, *object_id, registry)
                                    .into_iter().find(|a| a.ability_index == *ability_index))
                            {
                                found = (attached.card_id, Some(ab));
                                break;
                            }
                        }
                        found
                    }
                } else {
                    // Walk attached auras/equipment.
                    let mut found = (card_id, None);
                    for attached in new_state.objects.values()
                        .filter(|a| a.zone == Zone::Battlefield && a.attached_to == Some(*object_id))
                    {
                        if let Some(ab) = registry.get(attached.card_id)
                            .and_then(|b| b.activated_abilities(&new_state, *object_id, registry)
                                .into_iter().find(|a| a.ability_index == *ability_index))
                        {
                            found = (attached.card_id, Some(ab));
                            break;
                        }
                    }
                    found
                }
            };

            if let Some(ab) = ability {
                // Pay mana cost (with X-cost support). For X-cost abilities
                // we pay only the non-X portion here; the X generic is paid
                // later via the ChooseXFunding flow (CR 602.1: the cost is
                // announced & paid before the ability resolves).
                let has_x_cost = ab.cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X));
                if has_x_cost {
                    let non_x_cost = ManaCost::new(
                        ab.cost.symbols.iter().filter(|s| !matches!(s, ManaSymbol::X)).cloned().collect()
                    );
                    mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &non_x_cost)
                        .expect("legal_actions should have verified mana availability");
                } else {
                    mana::auto_pay(&mut new_state.get_player_mut(player).mana_pool, &ab.cost)
                        .expect("legal_actions should have verified mana availability");
                    new_state.last_activated_x_value = None;
                }

                // Pay tap cost.
                if ab.requires_tap {
                    new_state.get_object_mut(*object_id).expect("object must exist for tapping").tapped = true;
                }

                // Pay sacrifice cost. The player chose which creature to sacrifice
                // when picking the action — legal_actions enumerated one
                // ActivateAbility per (target, sacrifice) combo, so the choice is
                // already encoded in `sacrifice`. We just sacrifice it here.
                match &ab.sacrifice_cost {
                    SacrificeCost::None => {}
                    SacrificeCost::SacrificeThis => {
                        crate::destruction::sacrifice(&mut new_state, *object_id, registry);
                    }
                    SacrificeCost::SacrificeCreature | SacrificeCost::SacrificeAnotherCreature => {
                        let sac_id = sacrifice
                            .expect("legal_actions must populate sacrifice for sacrifice-cost abilities");
                        crate::destruction::sacrifice(&mut new_state, sac_id, registry);
                    }
                }

                // Track once-per-turn.
                if ab.once_per_turn {
                    if let Some(obj) = new_state.get_object_mut(*object_id) {
                        obj.abilities_activated_this_turn.insert(*ability_index);
                    }
                }

                if has_x_cost {
                    // Defer on_activate_ability and the activation log until
                    // funding completes — the ability's effect reads
                    // `last_activated_x_value`, which isn't set until then.
                    // See the ChooseXFunding handler for the continuation.
                    let options = crate::funding::build_options(&new_state, player, registry);
                    let name = card_name(&new_state, registry, *object_id);
                    if options.max_x > 0 {
                        new_state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                            player,
                            source: *object_id,
                            choice: crate::state::ResolutionChoiceKind::ChooseXFunding {
                                description: format!("{name}: choose X funding (0-{})", options.max_x),
                                options,
                                source_id: *object_id,
                                is_ability: true,
                            },
                        });
                        // Store context needed to fire the ability's effect
                        // once funding completes.
                        new_state.pending_ability_effect = Some(crate::state::PendingAbilityEffect {
                            source_id: *object_id,
                            ability_index: *ability_index,
                            behavior_card_id,
                            targets: targets.clone(),
                            description: ab.description.clone(),
                            activator: player,
                        });
                    } else {
                        // No mana available; force X = 0 and fire the effect.
                        new_state.last_activated_x_value = Some(0);
                        if let Some(behavior) = registry.get(behavior_card_id) {
                            behavior.on_activate_ability(&mut new_state, *object_id, *ability_index, targets, registry);
                        }
                        if new_state.stack.last().is_some_and(|e| matches!(e, crate::state::StackEntry::Ability { .. })) {
                            crate::stack::resolve_top_of_stack(&mut new_state, registry);
                        }
                        let name = card_name(&new_state, registry, *object_id);
                        new_state.log(LogLevel::Event, format!("p{} activated ability on {}: {}", player.0, name, ab.description));
                    }
                } else {
                    if let Some(behavior) = registry.get(behavior_card_id) {
                        behavior.on_activate_ability(&mut new_state, *object_id, *ability_index, targets, registry);
                    }
                    if new_state.stack.last().is_some_and(|e| matches!(e, crate::state::StackEntry::Ability { .. })) {
                        crate::stack::resolve_top_of_stack(&mut new_state, registry);
                    }
                    let name = card_name(&new_state, registry, *object_id);
                    new_state.log(LogLevel::Event, format!("p{} activated ability on {}: {}", player.0, name, ab.description));
                }
            }
        }

        Action::DeclareAttackers { attackers } => {
            // Validate declarations: only the active player's eligible
            // creatures (untapped, not summoning-sick without haste, no
            // defender/Pacifism — CR 508.1a) may attack, and only a valid
            // defender may be attacked. The engine is the authority; it does
            // not trust the submitted list. Illegal entries are dropped,
            // mirroring how blocker validation filters illegal blocks.
            let eligible = combat::eligible_attackers(&new_state, new_state.active_player, registry);
            let valid_defender = new_state.opponent(new_state.active_player);
            let attackers: Vec<(ObjectId, PlayerId)> = attackers.iter()
                .filter(|(id, def)| eligible.contains(id) && *def == valid_defender)
                .copied()
                .collect();
            let attackers = &attackers[..];
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
                        || !new_state.is_creature(creature.id, registry) || creature.tapped || creature.summoning_sick {
                        continue;
                    }
                    if new_state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&creature.id)) {
                        continue; // already attacking
                    }
                    // Check for Defender — can't be forced to attack.
                    if new_state.has_keyword(creature.id, crate::types::Keyword::Defender, registry) {
                        continue;
                    }
                    // Respect "can't attack" effects (e.g. Bonds of Faith).
                    if !new_state.can_attack(creature.id, registry) {
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

        Action::MulliganKeep => {
            let player = match &new_state.awaiting_action {
                Some(AwaitingAction::MulliganDecision { player }) => *player,
                _ => panic!("MulliganKeep without MulliganDecision awaiting"),
            };
            let mull_count = new_state.get_player(player).mulligan_count;
            new_state.log(LogLevel::Event,
                format!("p{} keeps ({} mulligan{})", player.0, mull_count,
                    if mull_count == 1 { "" } else { "s" }));
            new_state.get_player_mut(player).mulligan_kept = true;
            // Record this player's bottom obligation now so it'll be drained
            // once every player has finished the keep/mull sub-phase.
            new_state.pending_mulligan_bottoms.push((player, mull_count as usize));
            // Advance the within-round position past this player.
            new_state.mulligan_round_position += 1;
            advance_mulligan_phase(&mut new_state, registry);
            new_state.consecutive_passes = 0;
        }

        Action::MulliganMull => {
            let player = match &new_state.awaiting_action {
                Some(AwaitingAction::MulliganDecision { player }) => *player,
                _ => panic!("MulliganMull without MulliganDecision awaiting"),
            };
            assert!(new_state.get_player(player).mulligan_count < crate::state::LONDON_MULLIGAN_CAP, "MulliganMull attempted after reaching the mulligan cap");
            // Put the entire hand on the bottom of the library (temporarily;
            // it will be shuffled immediately) and draw seven fresh cards.
            let hand_ids: Vec<ObjectId> = new_state.objects_in_zone(Zone::Hand, player)
                .iter().map(|o| o.id).collect();
            for id in &hand_ids {
                new_state.move_object(*id, Zone::Library, registry);
                // Append to the end of the library_order (bottom).
                let lib = &mut new_state.get_player_mut(player).library_order;
                if !lib.contains(id) {
                    lib.push(*id);
                }
            }
            // Shuffle.
            let mut rng = rand::thread_rng();
            new_state.get_player_mut(player).library_order.shuffle(&mut rng);
            // Redraw seven.
            draw_cards(&mut new_state, player, 7, registry);
            new_state.get_player_mut(player).mulligan_count += 1;
            let mull_count = new_state.get_player(player).mulligan_count;
            new_state.log(LogLevel::Event,
                format!("p{} mulligans to {}", player.0, 7 - i32::try_from(mull_count).unwrap_or(i32::MAX)));
            // Mark that this round had a mulligan, then advance past this
            // player. The next player in turn order (who hasn't already kept)
            // will be asked. The mulled player will be re-asked next round.
            new_state.mulligan_round_mulled = true;
            new_state.mulligan_round_position += 1;
            advance_mulligan_phase(&mut new_state, registry);
            new_state.consecutive_passes = 0;
        }

        Action::BottomCards { cards } => {
            let (player, count) = match &new_state.awaiting_action {
                Some(AwaitingAction::BottomAfterMulligan { player, count }) => (*player, *count),
                _ => panic!("BottomCards without BottomAfterMulligan awaiting"),
            };
            assert_eq!(cards.len(), count,
                "BottomCards: expected {} cards, got {}", count, cards.len());
            // Validate the chosen cards are all in this player's hand and distinct.
            let hand_ids: Vec<ObjectId> = new_state.objects_in_zone(Zone::Hand, player)
                .iter().map(|o| o.id).collect();
            let mut seen = std::collections::HashSet::new();
            for id in cards {
                assert!(hand_ids.contains(id),
                    "BottomCards: card {:?} not in p{}'s hand", id, player.0);
                assert!(seen.insert(*id),
                    "BottomCards: duplicate card {id:?}");
            }
            // Move each card from hand to library and append to the bottom, in
            // the order given (so `cards[0]` ends up bottom-most of the group).
            for &card_id in cards {
                new_state.move_object(card_id, Zone::Library, registry);
                let lib = &mut new_state.get_player_mut(player).library_order;
                lib.retain(|&id| id != card_id);
                lib.push(card_id);
            }
            // Bottoming is a hidden action — the opponent must not see which
            // specific cards were sent to the bottom. Log only the count.
            new_state.log(LogLevel::Event,
                format!("p{} bottomed {} card{}", player.0, count,
                    if count == 1 { "" } else { "s" }));
            new_state.awaiting_action = None;
            advance_mulligan_phase(&mut new_state, registry);
            new_state.consecutive_passes = 0;
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
                new_state.get_object(*object_id).map_or(crate::ids::CardId(0), |o| o.card_id)
            ) {
                let abilities = behavior.loyalty_abilities(&new_state, *object_id);
                if let Some(ab) = abilities.iter().find(|a| a.ability_index == *ability_index) {
                    // Pay loyalty cost: add or remove loyalty counters.
                    let change = ab.loyalty_change;
                    if change > 0 {
                        new_state.add_counters(*object_id, CounterType::Loyalty, u32::try_from(change).unwrap_or(0));
                    } else if change < 0 {
                        let remove = u32::try_from(-change).unwrap_or(0);
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
                        if *pay {
                            // Deduct {1} from the player's mana pool.
                            let controller = new_state.get_object(*spell_id).map_or(PlayerId(0), |o| o.controller);
                            let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
                            let _ = mana::auto_pay(&mut new_state.get_player_mut(controller).mana_pool, &cost);
                            new_state.log(LogLevel::Event, "Paid {1} to prevent counter".into());
                        } else {
                            let name = new_state.obj_name(*spell_id);
                            new_state.stack.retain(|e| e.as_spell() != Some(*spell_id));
                            new_state.move_spell_after_resolve(*spell_id, registry);
                            new_state.log(LogLevel::Event, format!("{name} was countered"));
                        }
                        // Controller discards a card — player chooses which.
                        let controller = new_state.get_object(*spell_id).map_or(PlayerId(0), |o| o.controller);
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
                     ResolvedChoice::YesNoDecision(yes)) => {
                        // Dispatch to the card's on_yes_no_choice hook.
                        let source_card_id = new_state.get_object(*source_card).map(|o| o.card_id);
                        if let Some(behavior) = source_card_id.and_then(|cid| registry.get(cid)) {
                            behavior.on_yes_no_choice(&mut new_state, *source_card, *yes, registry);
                        }
                    }
                    (ResolutionChoiceKind::ChooseTarget { effect, .. },
                     ResolvedChoice::ChosenTarget(chosen)) => {
                        if let Some(t) = chosen {
                            apply_pending_effect(&mut new_state, t, effect, registry);
                        }
                        // If this was an "enters as a copy" choice (Evil Twin)
                        // and the player declined, the copy never resolves —
                        // disarm the SBA guard so the printed 0/0 can die.
                        // (On accept, the CopyCreature handler already did.)
                        if let crate::state::PendingEffect::CopyCreature { source_id } = effect {
                            if let Some(obj) = new_state.get_object_mut(*source_id) {
                                obj.entering_copy_source = false;
                            }
                        }
                    }
                    (ResolutionChoiceKind::ChooseCardFromHand { .. },
                     ResolvedChoice::ChosenCard(discard_id)) => {
                        let name = new_state.obj_name(*discard_id);
                        new_state.move_object(*discard_id, Zone::Graveyard, registry);
                        new_state.events.push(GameEvent::Discarded {
                            player: new_state.get_object(*discard_id).map_or(PlayerId(0), |o| o.owner),
                            object: *discard_id,
                        });
                        new_state.log(LogLevel::Event, format!("Discarded {name}"));
                        // Notify the source card about the discard (e.g., Civilized Scholar
                        // checks if the discarded card was a creature to trigger transform).
                        let source_card_id = new_state.get_object(choice_source).map(|o| o.card_id);
                        if let Some(behavior) = source_card_id.and_then(|cid| registry.get(cid)) {
                            behavior.on_discard_choice(&mut new_state, choice_source, *discard_id, registry);
                        }
                    }
                    (ResolutionChoiceKind::ChooseFromRevealed { revealed, spell_id, .. },
                     ResolvedChoice::ChosenCard(keep_id)) => {
                        let keep_name = new_state.obj_name(*keep_id);
                        new_state.move_object(*keep_id, Zone::Hand, registry);
                        for &card_id in revealed {
                            if card_id != *keep_id {
                                new_state.move_object(card_id, Zone::Graveyard, registry);
                            }
                        }
                        new_state.log(LogLevel::Event, format!("Kept {keep_name}"));
                        new_state.move_spell_after_resolve(*spell_id, registry);
                    }
                    (ResolutionChoiceKind::ChooseFromLibrary { searcher, destination, tapped, .. },
                     ResolvedChoice::ChosenCard(chosen_id)) => {
                        crate::cards::helpers::finish_library_search(
                            &mut new_state, *searcher, *chosen_id, *destination, *tapped, registry);
                    }
                    (ResolutionChoiceKind::ChooseCardType { options, spell_id, controller, .. },
                     ResolvedChoice::ChosenIndex(index, _)) => {
                        let chosen_type = options.get(*index).cloned().unwrap_or_default();
                        let card_type = match chosen_type.as_str() {
                            "Artifact" => CardType::Artifact,
                            "Enchantment" => CardType::Enchantment,
                            "Land" => CardType::Land,
                            "Planeswalker" => CardType::Planeswalker,
                            _ => CardType::Creature,
                        };
                        let to_return: Vec<ObjectId> = new_state.objects_in_zone(Zone::Graveyard, *controller)
                            .iter()
                            .map(|o| o.id)
                            .filter(|id| new_state.has_card_type(*id, card_type, registry))
                            .collect();
                        let count = to_return.len();
                        for id in to_return {
                            new_state.move_object(id, Zone::Hand, registry);
                        }
                        new_state.log(LogLevel::Event,
                            format!("Creeping Renaissance: chose {chosen_type}. Returned {count} cards from graveyard to hand"));
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
                            .map(|id| new_state.obj_name(*id))
                            .collect();
                        let pile_2_names: Vec<String> = pile_2.iter()
                            .map(|id| new_state.obj_name(*id))
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
                     ResolvedChoice::ChosenIndex(index, _)) => {
                        // Target player chose which pile to sacrifice.
                        let chosen_pile = if *index == 0 { pile_1 } else { pile_2 };
                        let pile_label = if *index == 0 { "Pile 1" } else { "Pile 2" };
                        new_state.log(LogLevel::Event,
                            format!("Liliana -6: chose to sacrifice {pile_label}"));
                        for &perm_id in chosen_pile {
                            let name = new_state.obj_name(perm_id);
                            if new_state.get_object(perm_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                                crate::destruction::sacrifice(&mut new_state, perm_id, registry);
                                new_state.log(LogLevel::Event,
                                    format!("Liliana -6: sacrificed {name}"));
                            }
                        }
                    }
                    (ResolutionChoiceKind::ChooseCardName { options, source_id, .. },
                     ResolvedChoice::ChosenIndex(index, _)) => {
                        let chosen_name = options.get(*index).cloned().unwrap_or_default();
                        // Store the restriction as an instance continuous effect on the source.
                        if let Some(obj) = new_state.get_object_mut(*source_id) {
                            let effect = ContinuousEffect::PreventCastingNamed { name: chosen_name.clone() };
                            obj.instance_continuous_effects = Some(vec![effect]);
                        }
                        new_state.log(LogLevel::Event,
                            format!("Nevermore names \"{chosen_name}\""));
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
                        let player = new_state.priority_player
                            .unwrap_or(new_state.active_player);
                        // Validate the response shape. For X-funding this
                        // should be impossible-by-construction with the
                        // string-enum schema, but validate defensively.
                        crate::funding::validate(response, options)
                            .expect("ChooseXFunding response must be valid (player implementations should pre-validate)");

                        if *is_ability {
                            let x = crate::funding::apply(&mut new_state, player, options, response, registry);
                            new_state.log(LogLevel::Event, format!("Funded X = {x}"));
                            new_state.last_activated_x_value = Some(x);
                            let pending = new_state.pending_ability_effect.take()
                                .expect("pending_ability_effect must be set for X-cost ability funding");
                            if let Some(behavior) = registry.get(pending.behavior_card_id) {
                                behavior.on_activate_ability(
                                    &mut new_state,
                                    pending.source_id,
                                    pending.ability_index,
                                    &pending.targets,
                                    registry,
                                );
                            }
                            if new_state.stack.last().is_some_and(|e| matches!(e, crate::state::StackEntry::Ability { .. })) {
                                crate::stack::resolve_top_of_stack(&mut new_state, registry);
                            }
                            let name = card_name(&new_state, registry, pending.source_id);
                            new_state.log(
                                LogLevel::Event,
                                format!(
                                    "p{} activated ability on {}: {}",
                                    pending.activator.0, name, pending.description
                                ),
                            );
                        } else {
                            // Spells: pull the stashed casting context and
                            // run the full casting sequence atomically.
                            let pending = new_state.pending_spell_cast.take()
                                .expect("pending_spell_cast must be set for X-cost spell funding");
                            debug_assert_eq!(pending.object_id, *source_id);
                            debug_assert_eq!(pending.player, player);

                            // Step 1: execute tap_plan for the non-X cost.
                            for (src_id, ability_index) in &pending.tap_plan {
                                activate_mana_source(&mut new_state, *src_id, *ability_index, registry);
                            }

                            // Step 2: pay the non-X mana portion.
                            mana::auto_pay(
                                &mut new_state.get_player_mut(player).mana_pool,
                                &pending.non_x_mana_cost,
                            ).expect("non-X mana should be payable after tap_plan");

                            // Step 3: apply the funding response (pays X by
                            // tapping funding sources + draining pool).
                            let x = crate::funding::apply(&mut new_state, player, options, response, registry);
                            new_state.log(LogLevel::Event, format!("Funded X = {x}"));

                            // Step 4: pay additional costs (sacrifice / exile).
                            if let Some(sac_id) = pending.sacrifice {
                                let sac_name = card_name(&new_state, registry, sac_id);
                                crate::destruction::sacrifice(&mut new_state, sac_id, registry);
                                new_state.log(LogLevel::Event,
                                    format!("Sacrificed {sac_name} as additional cost"));
                            }
                            let additional = registry.get(pending.card_id)
                                .and_then(|b| b.card_data().additional_cost);
                            if let Some(crate::cards::AdditionalCost::ExileCreaturesFromGraveyard(n)) = additional {
                                let to_exile: Vec<ObjectId> = if pending.exile_ids.is_empty() {
                                    let mut cands: Vec<(ObjectId, i32)> = new_state.objects.values()
                                        .filter(|o| {
                                            o.zone == Zone::Graveyard && o.owner == player
                                                && o.id != pending.object_id
                                                && state.is_creature(o.id, registry)
                                        })
                                        .map(|o| (o.id, new_state.effective_power(o.id, registry).unwrap_or(0)))
                                        .collect();
                                    cands.sort_by(|a, b| b.1.cmp(&a.1));
                                    cands.into_iter().take(n).map(|(id, _)| id).collect()
                                } else {
                                    pending.exile_ids.clone()
                                };
                                if let Some(&first) = to_exile.first() {
                                    let power = new_state.effective_power(first, registry).unwrap_or(0);
                                    if let Some(obj) = new_state.get_object_mut(pending.object_id) {
                                        obj.card_state.insert("exiled_power".into(),
                                            ObjectId(u64::try_from(power).unwrap_or(0)));
                                    }
                                }
                                for eid in &to_exile {
                                    let name = card_name(&new_state, registry, *eid);
                                    new_state.move_object(*eid, Zone::Exile, registry);
                                    new_state.log(LogLevel::Event,
                                        format!("Exiled {name} from graveyard as additional cost"));
                                }
                            } else if matches!(additional, Some(crate::cards::AdditionalCost::ExileXFromGraveyard)) {
                                let gy_cards: Vec<ObjectId> = if pending.exile_ids.is_empty() {
                                    let count = pending.exile_count.unwrap_or(0) as usize;
                                    new_state.objects.values()
                                        .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != pending.object_id)
                                        .map(|o| o.id)
                                        .take(count)
                                        .collect()
                                } else {
                                    pending.exile_ids.clone()
                                };
                                let count = u32::try_from(gy_cards.len()).unwrap_or(u32::MAX);
                                for gid in &gy_cards {
                                    new_state.move_object(*gid, Zone::Exile, registry);
                                }
                                if let Some(obj) = new_state.get_object_mut(pending.object_id) {
                                    obj.card_state.insert("exile_count".into(), ObjectId(u64::from(count)));
                                }
                                new_state.log(LogLevel::Event,
                                    format!("Exiled {count} cards from graveyard as additional cost"));
                            }

                            // Step 5: move spell to stack, set metadata,
                            // push StackEntry.
                            new_state.move_object(pending.object_id, Zone::Stack, registry);
                            {
                                let obj = new_state.get_object_mut(pending.object_id)
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
                                        &new_state, player, pending.object_id, &pending.targets, modes, behavior,
                                    );
                                    if let Some(obj) = new_state.get_object_mut(pending.object_id) {
                                        obj.chosen_mode = Some(chosen);
                                    }
                                }
                            }
                            new_state.stack.push(crate::state::StackEntry::Spell(pending.object_id));

                            // Step 6: fire SpellCast + bookkeeping.
                            finalize_spell_cast(
                                &mut new_state, player, pending.object_id,
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
                            new_state.log(LogLevel::Event,
                                format!("Exile-choice rejected: {err}; cast cancelled"));
                            new_state.pending_spell_cast = None;
                            return new_state;
                        }

                        let pending = new_state.pending_spell_cast.take()
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
                        new_state.awaiting_action = None;
                        return submit_action(&new_state, &cast, registry);
                    }
                    // Player cancelled a cast mid-prompt (rarely reached —
                    // only when a fixed-count exile choice couldn't be
                    // satisfied after validation retries).
                    (_, ResolvedChoice::CancelCast) => {
                        new_state.log(LogLevel::Event, "Cast cancelled".into());
                        new_state.pending_spell_cast = None;
                    }
                    _ => {}
                }
            }
            new_state.consecutive_passes = 0;
        }
    }

    finish_spell_resolution_if_idle(&mut new_state, registry);

    new_state
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
                        state.pending_triggers.push(
                            crate::triggers::PendingTrigger::EnteredBattlefield {
                                object_id: *source_id,
                                card_id,
                                controller,
                                description: format!("{copy_name} (copy) enters the battlefield"),
                                chosen_targets: Vec::new(),
                            }
                        );
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
                t.set_chosen_targets(vec![chosen_target.clone()]);
                state.stack.push(crate::state::StackEntry::Trigger(t));
            }
            // Continue processing the remaining pending triggers (may set up
            // another awaiting_action prompt for the next target choice).
            crate::triggers::process_pending_trigger_pushes(state, registry);
        }
        _ => {}
    }
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
        draw_cards(&mut state, player_id, 7, registry);
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

/// Draw N cards for a player. Logs a single summary entry.
pub fn draw_cards(state: &mut GameState, player: PlayerId, count: usize, registry: &CardRegistry) {
    let mut drawn = 0;
    for _ in 0..count {
        let card_id = {
            let player_state = state.get_player_mut(player);
            player_state.draw_top_card()
        };
        if let Some(id) = card_id {
            state.move_object(id, Zone::Hand, registry);
            state.events.push(GameEvent::CardDrawn { player, object: id });
            drawn += 1;
        } else {
            // Check for ReplaceEmptyDraw replacement effect (e.g. Laboratory Maniac):
            // if the player controls a permanent with this effect, they win instead.
            let has_replace_empty_draw = state.objects.values().any(|o| {
                o.zone == Zone::Battlefield
                    && o.controller == player
                    && registry.get(o.card_id)
                        .is_some_and(|b| b.replacement_effects().contains(&crate::types::ReplacementEffect::ReplaceEmptyDraw))
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
                            .is_some_and(|b| b.replacement_effects().contains(&crate::types::ReplacementEffect::ReplaceEmptyDraw)))
                    .map(|o| o.name.clone())
                    .unwrap_or_default();
                state.log(LogLevel::Milestone,
                    format!("p{} wins the game with {}!", player.0, source_name));
            }
            // Otherwise SBA will catch the empty library draw.
            break;
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
        let obj_id = {
            let player_state = state.get_player_mut(player);
            if player_state.library_order.is_empty() {
                break;
            }
            player_state.library_order.remove(0)
        };
        // Check if it's a creature card before moving (for mill-watcher triggers).
        let is_creature = state.get_object(obj_id)
            .is_some_and(|o| {
                state.is_creature(o.id, registry)
            });
        state.move_object(obj_id, Zone::Graveyard, registry);
        if is_creature {
            state.events.push(crate::events::GameEvent::CreatureCardMilled {
                object: obj_id,
                milled_player: player,
            });
        }
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
        .is_some_and(|c| !c.attackers.is_empty())
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
                if mana::can_pay(&potential, &ab.cost) && (!ab.requires_tap || !obj.tapped) {
                    return true;
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
