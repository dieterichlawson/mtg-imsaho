use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, LoyaltyAbilityDef, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Supertype, Zone};

/// Liliana of the Veil {1}{B}{B} Legendary Planeswalker — Liliana (3 loyalty).
/// +1: Each player discards a card.
/// -2: Target player sacrifices a creature.
/// -6: Separate all permanents target player controls into two piles. That player sacrifices
///     all permanents in the pile of their choice.
pub struct LilianaOfTheVeil;

impl CardBehavior for LilianaOfTheVeil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Liliana of the Veil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Planeswalker],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Liliana".into()],
            oracle_text: "+1: Each player discards a card.\n−2: Target player sacrifices a creature.\n−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.".into(),
            ..Default::default()
        }
    }

    fn starting_loyalty(&self) -> Option<u32> {
        Some(3)
    }

    fn loyalty_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<LoyaltyAbilityDef> {
        vec![
            LoyaltyAbilityDef {
                ability_index: 0,
                loyalty_change: 1,
                description: "+1: Each player discards a card".into(),
                target_requirement: None,
            },
            LoyaltyAbilityDef {
                ability_index: 1,
                loyalty_change: -2,
                description: "-2: Target player sacrifices a creature".into(),
                target_requirement: Some(TargetRequirement::PlayerOnly),
            },
            LoyaltyAbilityDef {
                ability_index: 2,
                loyalty_change: -6,
                description: "-6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice".into(),
                target_requirement: Some(TargetRequirement::PlayerOnly),
            },
        ]
    }

    fn on_loyalty_ability(&self, state: &mut GameState, self_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::ability_controller(state, self_id);

        match ability_index {
            0 => {
                // +1: Each player discards a card. The active player chooses
                // first, then each other player in turn order (CR 101.4).
                let active = state.active_player;

                // Build list of players who need to discard, starting with active player.
                // Allocated at the most it can hold, as `helpers::ids_where`
                // says why.
                let mut players_to_discard: Vec<PlayerId> = Vec::with_capacity(state.players.len());
                // Active player first.
                if !state.get_player(active).lost {
                    if state.any_in_zone(Zone::Hand, active, |_| true) {
                        players_to_discard.push(active);
                    }
                }
                // Then other players in turn order.
                for pid_idx in 0..state.players.len() {
                    let pid = PlayerId(u8::try_from(pid_idx).unwrap_or(u8::MAX));
                    if pid == active { continue; }
                    if state.get_player(pid).lost { continue; }
                    if state.any_in_zone(Zone::Hand, pid, |_| true) {
                        players_to_discard.push(pid);
                    }
                }

                if players_to_discard.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        "Liliana +1: no player has cards to discard".into());
                    return;
                }

                // CR 101.4: every player chooses in turn order, and the
                // cards leave their hands simultaneously. So the choices are
                // collected first — nothing is discarded until the last
                // player has chosen — otherwise a discard trigger (Murder of
                // Crows) fires and is seen while someone is still choosing.
                Self::store_queue(state, self_id, &players_to_discard, &[]);
                Self::advance(state, self_id, registry);
            }
            1 => {
                // -2: Target player sacrifices a creature.
                let target_player = match targets.first() {
                    Some(Target::Player(pid)) => *pid,
                    _ => return,
                };

                let creatures: Vec<Target> = crate::cards::helpers::creatures_controlled_by(state, target_player, registry);

                if creatures.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana -2: p{} has no creatures to sacrifice", target_player.0));
                    return;
                }

                // The targeted player chooses which creature to sacrifice.
                crate::cards::helpers::present_target_choice(
                    state,
                    self_id,
                    target_player,
                    creatures,
                    PendingEffect::SacrificeCreature {
                        source_name: "Liliana of the Veil".into(),
                    },
                    "Liliana -2: choose a creature to sacrifice",
                    false, // mandatory
                    registry,
                );
            }
            2 => {
                // -6: Separate all permanents target player controls into two piles.
                // That player sacrifices all permanents in the pile of their choice.
                let target_player = match targets.first() {
                    Some(Target::Player(pid)) => *pid,
                    _ => return,
                };

                let permanents = state.ids_in_zone(Zone::Battlefield, target_player, |_| true);

                if permanents.is_empty() {
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana -6: p{} has no permanents", target_player.0));
                    return;
                }

                // Liliana's controller divides the permanents into two piles.
                state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                    player: controller,
                    source: self_id,
                    choice: ResolutionChoiceKind::DividePermanentsIntoPiles {
                        description: format!(
                            "Liliana -6: divide p{}'s permanents into two piles",
                            target_player.0),
                        permanents,
                        target_player,
                        source_id: self_id,
                    },
                });
            }
            _ => {}
        }
    }

    fn on_discard_choice(&self, state: &mut GameState, self_id: ObjectId, discarded_id: ObjectId, registry: &CardRegistry) {
        // The engine did NOT discard this card — `discard_immediately: false`.
        // Record the choice and move on to the next player.
        Self::push_chosen(state, self_id, discarded_id);
        Self::advance(state, self_id, registry);
    }
}

impl LilianaOfTheVeil {
    // The +1's intermediate state lives in `card_state`: a queue of players
    // who have yet to choose, and the cards chosen so far, each a counted
    // list (`cards::store_ids`) rather than one packed value — the previous
    // encoding parsed a comma-joined string of player ids *as a u64*, which
    // silently became 0 for any game with more than one player left to ask.
    const QUEUE: &'static str = "liliana_queue";
    const CHOSEN: &'static str = "liliana_chosen";

    fn store_queue(state: &mut GameState, self_id: ObjectId, queue: &[PlayerId], chosen: &[ObjectId]) {
        let Some(obj) = state.get_object_mut(self_id) else { return };
        let players: Vec<ObjectId> = queue.iter().map(|p| ObjectId(u64::from(p.0))).collect();
        crate::cards::store_ids(&mut obj.card_state, Self::QUEUE, &players);
        crate::cards::store_ids(&mut obj.card_state, Self::CHOSEN, chosen);
    }

    fn load_queue(state: &GameState, self_id: ObjectId) -> (Vec<PlayerId>, Vec<ObjectId>) {
        let Some(obj) = state.get_object(self_id) else { return (vec![], vec![]) };
        let queue = crate::cards::load_ids(&obj.card_state, Self::QUEUE).into_iter()
            .map(|id| PlayerId(u8::try_from(id.0).unwrap_or(u8::MAX)))
            .collect();
        (queue, crate::cards::load_ids(&obj.card_state, Self::CHOSEN))
    }

    fn push_chosen(state: &mut GameState, self_id: ObjectId, card: ObjectId) {
        let (queue, mut chosen) = Self::load_queue(state, self_id);
        chosen.push(card);
        Self::store_queue(state, self_id, &queue, &chosen);
    }

    /// Ask the next player in the queue, or — when the queue is empty —
    /// discard every collected card at once.
    fn advance(state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let (mut queue, mut chosen) = Self::load_queue(state, self_id);

        while !queue.is_empty() {
            let player = queue.remove(0);
            let hand = state.ids_in_zone(Zone::Hand, player, |_| true);

            if hand.is_empty() {
                // A card may have left this player's hand since the ability
                // started resolving.
                state.log(crate::state::LogLevel::Event,
                    format!("Liliana +1: p{} has no cards to discard", player.0));
                continue;
            }
            if hand.len() == 1 {
                // No choice to make — but still no discard yet, because the
                // players after this one have not chosen.
                chosen.push(hand[0]);
                continue;
            }

            Self::store_queue(state, self_id, &queue, &chosen);
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player,
                source: self_id,
                choice: ResolutionChoiceKind::ChooseCardFromHand {
                    description: "Liliana +1: choose a card to discard".into(),
                    player,
                    cards: hand,
                    discard_immediately: false,
                    remaining: 1,
                },
            });
            return;
        }

        // Everyone has chosen. Now the cards leave their hands together.
        for card in &chosen {
            state.discard_card_for(*card, "Liliana +1", registry);
        }
        if let Some(obj) = state.get_object_mut(self_id) {
            crate::cards::clear_ids(&mut obj.card_state, Self::QUEUE);
            crate::cards::clear_ids(&mut obj.card_state, Self::CHOSEN);
        }
    }
}
