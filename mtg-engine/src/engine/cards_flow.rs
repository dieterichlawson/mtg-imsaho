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
            // CR 614: drawing from an empty library can be replaced
            // (Laboratory Maniac wins instead). The effect does whatever it
            // does; either way the draw does not happen.
            crate::replacement::apply(
                state,
                crate::replacement::ReplaceableEvent::DrawsFromEmptyLibrary { player },
                registry,
            );
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
/// Put one card from a library into its owner's graveyard — a mill
/// (CR 701.13a), whether it comes off the top, the bottom, or out of a pile
/// of revealed cards.
///
/// This exists to take the card out of `library_order` as well as moving it;
/// `CreatureCardMilled` is `move_object`'s, emitted for any library-to-
/// graveyard move, so a card that does the move by hand cannot lose the event
/// the way four of them used to.
pub fn mill_one(state: &mut GameState, player: PlayerId, obj_id: ObjectId, registry: &CardRegistry) {
    state.get_player_mut(player).library_order.retain(|&id| id != obj_id);
    state.move_object(obj_id, Zone::Graveyard, registry);
}
/// Mill up to `count` cards, and say how many actually went (CR 701.13b: a
/// player with fewer cards than that mills all of them and does not lose).
///
/// `source` names the card doing it, so there is one line and it is accurate.
/// Six cards used to log their *intended* count next to this function's real
/// one — "Curse of the Bloody Tome: p1 milled 2 cards" beside "p1 milled 1
/// card" — and the card's line, the one naming the source, was the one a
/// reader would trust.
pub fn mill_cards(state: &mut GameState, player: PlayerId, count: usize, source: &str, registry: &CardRegistry) -> usize {
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
        let short = if milled < count { format!(" (of {count} — library ran out)") } else { String::new() };
        state.log(LogLevel::Event, format!(
            "{source}: p{} milled {milled} card{}{short}",
            player.0, if milled == 1 { "" } else { "s" }));
    } else if count > 0 {
        state.log(LogLevel::Event, format!("{source}: p{} has an empty library, nothing to mill", player.0));
    }
    milled
}
/// Have `player` discard `count` cards of their choice — the whole of "target
/// player discards two cards", including the asking.
///
/// A player with fewer cards than that discards all of them (CR 701.8a: you do
/// as much as you can). Where there is nothing to decide — one card left, or
/// none — no prompt is raised; where there is, the choice is the discarding
/// player's, and the engine comes back for the next one when it is answered.
///
/// Returns the number of cards it could not ask for, which is zero unless the
/// hand ran out before the prompt was raised. The rest of the count travels
/// with the choice, in `ChooseCardFromHand::remaining`.
///
/// Brain Weevil used to do this itself: discard one, then chain the second
/// from `on_discard_choice`, keeping the target player between the two in
/// `card_state` as `ObjectId(player.0 as u64)`. Discarding N cards is a rules
/// action, not one card's problem.
pub fn discard_cards(
    state: &mut GameState,
    player: PlayerId,
    count: usize,
    source_id: ObjectId,
    source: &str,
    registry: &CardRegistry,
) {
    if count == 0 {
        return;
    }
    let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, player)
        .iter().map(|o| o.id).collect();

    if hand.is_empty() {
        state.log(LogLevel::Event,
            format!("{source}: p{} has no cards to discard", player.0));
        return;
    }
    // With no more cards than the count, every one of them goes and there is
    // nothing to decide — asking would be a prompt with one answer.
    if hand.len() <= count {
        for card in &hand {
            let name = state.obj_name(*card);
            state.discard_card(*card, registry);
            state.log(LogLevel::Event,
                format!("{source}: p{} discarded {name}", player.0));
            notify_discard(state, source_id, *card, registry);
        }
        if hand.len() < count {
            state.log(LogLevel::Event, format!(
                "{source}: p{} discarded {} of {count} — their hand ran out",
                player.0, hand.len()));
        }
        return;
    }
    state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
        player,
        source: source_id,
        choice: crate::state::ResolutionChoiceKind::ChooseCardFromHand {
            description: format!("{source}: choose a card to discard{}",
                if count > 1 { format!(" ({count} to go)") } else { String::new() }),
            player,
            cards: hand,
            discard_immediately: true,
            remaining: count,
        },
    });
}

/// Tell the source card that a card was discarded for it — Civilized Scholar
/// wants to know whether it was a creature.
pub(crate) fn notify_discard(
    state: &mut GameState,
    source_id: ObjectId,
    discarded: ObjectId,
    registry: &CardRegistry,
) {
    let source_card_id = state.get_object(source_id).map(|o| o.card_id);
    if let Some(behavior) = source_card_id.and_then(|cid| registry.get(cid)) {
        behavior.on_discard_choice(state, source_id, discarded, registry);
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
                state, player, obj.id, &target_req, behavior, registry,
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
