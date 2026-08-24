use crate::actions::Action;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::mana;
use crate::state::{GameState, LogLevel};
use crate::types::{Zone, CardType, Keyword, Step};
use super::*;

/// Generate legal discard actions for hand size.
pub(crate) fn legal_discard_actions(state: &GameState, player: PlayerId, discard_count: usize) -> Vec<Action> {
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
pub(crate) fn card_name(state: &GameState, _registry: &CardRegistry, obj_id: ObjectId) -> String {
    state.obj_name(obj_id)
}
/// Draw N cards for a player. Logs a single summary entry.
/// Draw `count` cards, returning how many were ACTUALLY drawn.
///
/// The count matters for "draw a card. If you do, ..." — with an empty library
/// nothing is drawn and the rest of the effect does not happen. This returned
/// `()`, so Murder of Crows checked whether the hand was non-empty instead and
/// made a player discard a card they had never drawn.
#[must_use = "'if you do' effects depend on how many cards were actually drawn"]
pub fn draw_cards(state: &mut GameState, player: PlayerId, count: usize, registry: &CardRegistry) -> usize {
    let mut drawn: usize = 0;
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
    drawn
}
/// Mill N cards from a player's library (move top N cards to graveyard).
/// Put one card from a library into its owner's graveyard, emitting
/// `CreatureCardMilled` if it is a creature card.
///
/// Every library-to-graveyard move has to go through here. Cards that milled
/// by hand — Mulch discarding the non-lands it revealed, Cellar Door milling
/// from the BOTTOM, which `mill_cards` cannot express — moved the card
/// directly and skipped the event, so Undead Alchemist's "whenever a creature
/// card is put into an opponent's graveyard from their library" never fired
/// for them.
pub fn mill_one(state: &mut GameState, player: PlayerId, obj_id: ObjectId, registry: &CardRegistry) {
    let is_creature = state.is_creature(obj_id, registry);
    state.get_player_mut(player).library_order.retain(|&id| id != obj_id);
    state.move_object(obj_id, Zone::Graveyard, registry);
    if is_creature {
        state.events.push(crate::events::GameEvent::CreatureCardMilled {
            object: obj_id,
            milled_player: player,
        });
    }
}
pub fn mill_cards(state: &mut GameState, player: PlayerId, count: usize, registry: &CardRegistry) {
    let mut milled = 0;
    for _ in 0..count {
        let obj_id = {
            let player_state = state.get_player_mut(player);
            if player_state.library_order.is_empty() {
                break;
            }
            player_state.library_order[0]
        };
        mill_one(state, player, obj_id, registry);
        milled += 1;
    }
    if milled > 0 {
        state.log(LogLevel::Event, format!("p{} milled {} card{}", player.0, milled, if milled == 1 { "" } else { "s" }));
    }
}
/// Check if a player could cast any spell if they tapped all available mana sources.
/// Used by the auto-pass check to avoid skipping turns where mana abilities are
/// the only listed actions but the player has castable spells.
pub(crate) fn has_castable_with_potential_mana(
    state: &GameState,
    player: PlayerId,
    registry: &CardRegistry,
) -> bool {
    // Build potential mana pool: current pool + all activatable mana abilities.
    let mut potential = state.get_player(player).mana_pool.clone();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        for ma in available_mana_abilities(state, obj.id, registry) {
            for &(mana_type, amount) in &ma.produced {
                potential.add(mana_type, amount);
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
