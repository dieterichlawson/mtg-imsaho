//! The rules engine: legal actions, applying a chosen action, and the loop
//! that drives a game from setup to a result.
//!
//! Split by what each part is *for*, since this file was once 4,576 lines and
//! over half of it was two functions:
//!
//! - [`actions`] — applying a chosen [`Action`], one module per variant.
//! - [`legal`] — enumerating what a player may do right now.
//! - [`costs`] — what a spell costs to cast (CR 601.2f).
//! - [`mana_sources`] — producing mana and paying costs with it.
//! - [`targeting`] — what a spell or ability may point at.
//! - [`effects`] — applying a resolved effect.
//! - [`cards_flow`] — drawing, milling, discarding.

mod actions;
mod legal;
mod cards_flow;
mod costs;
mod effects;
mod mana_sources;
mod targeting;

pub use cards_flow::{discard_cards, draw_cards, mill_cards, mill_one};
pub use effects::apply_pending_effect;
pub use costs::{
    AdditionalCostPlan, CastMethod, SpellCost, additional_cost_plan, alternative_costs,
    cost_to_cast, effective_spell_cost, pay_exile_creatures,
};
pub use mana_sources::{
    activate_mana_source, available_mana_abilities, can_pay_with_sources, pay_cost_with_sources,
};
pub use targeting::can_be_targeted_by;
pub(crate) use targeting::can_target_player;

pub(crate) use cards_flow::{card_name, has_castable_with_potential_mana, legal_discard_actions, notify_discard};
pub(crate) use effects::{finalize_spell_cast, finish_spell_resolution_if_idle};
pub(crate) use mana_sources::{
    activatable_mana_abilities, execute_tap_plan_and_pay,
    gather_mana_sources, plan_autotap_for_cost, prevents_artifact_abilities,
};
pub(crate) use targeting::{
    matches_target_filter,
    build_cast_target_spec, combinations, detect_modal_choice_mode, generate_ability_targets,
    generate_cast_actions_with_targets,
    valid_targets_for_req,
};


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
use crate::types::{Zone, CardType, ContinuousEffect, Keyword, Step};

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
    /// Seed for every random decision the game makes — shuffles, coin flips,
    /// "at random" choices. `None` draws a fresh one, so games differ; setting
    /// it replays a game exactly.
    pub rng_seed: Option<u64>,
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

    // If we're waiting for a specific action (attackers, blockers, discard),
    // the only legal moves are the ones that answer it.
    if let Some(la) = legal::awaiting::legal_actions_while_awaiting(state, registry) {
        return la;
    }

    let Some(player) = state.priority_player else {
        return LegalActions { actions: vec![], combat_prompt: None, castable_spells: vec![], activatable_abilities: vec![], context: None, resolution_prompt: None };
    };

    let mut actions = Vec::new();
    let mut castable_spells = Vec::new();

    // PassPriority is always available when you have priority.
    actions.push(Action::PassPriority);

    // Stony Silence: no ability of an artifact may be activated, mana
    // abilities included.
    let prevent_artifact_abilities = prevents_artifact_abilities(state, registry);
    let ctx = legal::Ctx {
        state,
        registry,
        player,
        prevent_artifact_abilities,
        is_sorcery_speed: state.step.is_main_phase()
            && state.stack.is_empty()
            && state.active_player == player,
        mana_sources: gather_mana_sources(state, player, registry, prevent_artifact_abilities),
        hand_costs: state.objects_in_zone(Zone::Hand, player).iter()
            .filter_map(|obj| {
                let data = registry.get(obj.card_id)?.card_data();
                data.cost.as_ref()
                    .map(|c| effective_spell_cost(state, registry, obj.card_id, c, player))
            })
            .collect(),
        // Nevermore stores its chosen name as an instance effect, but reading
        // only instance effects meant a card that declared the ban on its face
        // would have been ignored.
        casting_banned: state.global_effects(registry).into_iter()
            .filter_map(|e| match e {
                ContinuousEffect::PreventCastingNamed { name } => Some(name),
                _ => None,
            })
            .collect(),
    };

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



    legal::abilities::activated(&ctx, &mut actions);

    legal::abilities::loyalty(&ctx, &mut actions);




    legal::casting::from_hand(&ctx, &mut actions, &mut castable_spells);

    legal::casting::flashback(&ctx, &mut actions, &mut castable_spells);

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
    // Sorted: `ability_map` is a HashMap, and this list is what the player
    // picks an ability from by position. Draining it in map order offered the
    // same abilities under different indices on a replay of the same game.
    let mut ability_entries: Vec<_> = ability_map.into_iter().collect();
    ability_entries.sort_by_key(|((object_id, source_card_id, ability_index), _)| {
        (object_id.0, source_card_id.map_or(0, |c| c.0), *ability_index)
    });
    let activatable_abilities: Vec<crate::actions::ActivatableAbility> = ability_entries
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

        Action::DeclareAttackers { attackers, planeswalker_attacks } =>
            actions::combat::declare_attackers(&mut new_state, attackers, planeswalker_attacks, registry),
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
/// One of the two places the engine reaches outside itself for randomness,
/// and for the same reason as the other (`setup_game` seeding
/// `GameState::rng_state`): this answers a question asked *before* there is a
/// game to ask it of. Everything the game itself decides at random — coin
/// flips, "at random" choices, shuffles — draws from the seeded stream on
/// `GameState`, so a game replays exactly from its seed.
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

    // Seed the game's randomness before anything draws from it. Without a
    // seed from the caller, one from the OS — so an unconfigured game is a
    // different game each time, and a configured one is the same game every
    // time.
    state.rng_state = config.rng_seed.unwrap_or_else(|| {
        use rand::Rng;
        rand::thread_rng().gen()
    });

    // Create card objects for each player's deck.
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
        state.shuffle(&mut library_ids);
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
                .filter(|o| !state.untaps_normally(o.id, registry))
                .map(|o| o.id)
                .collect();

            // Untap all permanents the active player controls, except locked ones.
            let to_untap: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, active)
                .iter()
                .filter(|o| o.tapped && !locked_ids.contains(&o.id))
                .map(|o| o.id)
                .collect();

            for id in to_untap {
                state.untap(id);
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
            // CR 510.5: this flag is what made `advance_step` come back here
            // for the second combat damage step, so entering the step consumes
            // it. It used to be cleared inside the `has_attackers` branch
            // below — and when combat emptied between the two steps (an
            // attacker regenerating, CR 701.15) the branch was skipped, the
            // flag survived, and `advance_step` chose Step::CombatDamage
            // again, forever. The game never reached end of combat: about one
            // random game in twenty-five ground to a halt cycling this step.
            let second_damage_step = std::mem::take(&mut state.combat_damage_step_pending);

            let has_attackers = state.combat.as_ref()
                .is_some_and(|c| !c.attackers.is_empty());

            if has_attackers {
                if second_damage_step {
                    // Second combat damage step (CR 510.5): regular damage
                    // from creatures that didn't deal first-strike damage,
                    // plus double strikers.
                    combat::deal_regular_damage_pass(state, registry);
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
            // CR 514.2: remove all damage marked on permanents, and with it
            // the turn's record of who dealt it.
            //
            // Every permanent, not only those with damage still marked: a
            // creature that regenerated has no marked damage but was still
            // dealt some this turn, and a planeswalker's damage removes
            // loyalty rather than marking. Filtering on `damage_marked > 0`
            // left both carrying a `damaged_by` into the next turn, where
            // "dealt damage by this creature **this turn**" would read it.
            let on_battlefield: Vec<ObjectId> = state.all_objects_in_zone(Zone::Battlefield)
                .iter()
                .map(|o| o.id)
                .collect();

            for id in on_battlefield {
                let obj = state.get_object_mut(id).expect("object must exist for damage clear");
                obj.damage_marked = 0;
                obj.dealt_deathtouch_damage = false;
                obj.damaged_by.clear();
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

        // CR 500.2: a step ends only when the stack is empty. Every fallback
        // below that would advance the step must first resolve what is on the
        // stack — a trigger put on the stack during a combat declaration used
        // to ride through the rest of combat, cleanup, and into the NEXT
        // turn's combat before resolving (Geist of Saint Traft's attack
        // trigger created its Angel a turn late, attacking its own
        // controller's opponent-of-the-moment; found by seeded fuzzing,
        // br vs wu coverage decks, seed 290).
        let advance_or_resolve = |state: &mut GameState, registry: &CardRegistry| {
            if state.stack.is_empty() {
                advance_step(state, registry);
            } else {
                let mut new_state = state.clone();
                stack::resolve_top_of_stack(&mut new_state, registry);
                *state = new_state;
            }
        };

        // If no one has priority and there's no awaiting action, advance step.
        if state.priority_player.is_none() && state.awaiting_action.is_none() {
            advance_or_resolve(state, registry);
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
            advance_or_resolve(state, registry);
            continue;
        };

        let legal = legal_actions(state, registry);
        if legal.actions.is_empty() && legal.combat_prompt.is_none() {
            advance_or_resolve(state, registry);
            continue;
        }

        // Auto-declare zero attackers when there are no eligible creatures.
        if let Some(crate::actions::CombatPrompt::ChooseAttackers {
            ref eligible, ref must_attack, ..
        }) = legal.combat_prompt {
            if eligible.is_empty() && must_attack.is_empty() {
                *state = submit_action(state, &Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] }, registry);
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
                advance_or_resolve(state, registry);
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
