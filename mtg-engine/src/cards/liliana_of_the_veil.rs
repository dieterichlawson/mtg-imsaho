use crate::cards::{CardBehavior, CardData, CardRegistry, LoyaltyAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Liliana of the Veil {1}{B}{B} Planeswalker — Liliana (3 loyalty).
/// +1: Each player discards a card.
/// -2: Target player sacrifices a creature.
/// -6: Separate all permanents target player controls into two piles. That player sacrifices
///     all permanents in the pile of your choice.
///
/// Simplified: +1 each player discards (auto-picks). -2 opponent sacrifices a creature.
/// -6 opponent sacrifices half their permanents (simplified pile division).
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
            power: None,
            toughness: None,
            oracle_text: "+1: Each player discards a card.\n-2: Target player sacrifices a creature.\n-6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of your choice.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn starting_loyalty(&self) -> Option<u32> {
        Some(3)
    }

    fn loyalty_abilities(&self) -> Vec<LoyaltyAbilityDef> {
        vec![
            LoyaltyAbilityDef {
                ability_index: 0,
                loyalty_change: 1,
                description: "+1: Each player discards a card".into(),
            },
            LoyaltyAbilityDef {
                ability_index: 1,
                loyalty_change: -2,
                description: "-2: Target player sacrifices a creature".into(),
            },
            LoyaltyAbilityDef {
                ability_index: 2,
                loyalty_change: -6,
                description: "-6: Separate permanents into two piles, sacrifice one pile".into(),
            },
        ]
    }

    fn on_loyalty_ability(&self, state: &mut GameState, self_id: ObjectId, ability_index: usize, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        let opponent = state.opponent(controller);

        match ability_index {
            0 => {
                // +1: Each player discards a card.
                for pid_idx in 0..state.players.len() {
                    let pid = crate::ids::PlayerId(pid_idx as u8);
                    if state.get_player(pid).lost { continue; }
                    let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, pid)
                        .iter().map(|o| o.id).collect();
                    if let Some(&card_id) = hand.first() {
                        let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
                        state.move_object(card_id, Zone::Graveyard);
                        state.events.push(crate::events::GameEvent::Discarded {
                            player: pid,
                            object: card_id,
                        });
                        state.log(crate::state::LogLevel::Event,
                            format!("Liliana +1: p{} discarded {}", pid.0, name));
                    }
                }
            }
            1 => {
                // -2: Target player sacrifices a creature.
                // Simplified: opponent sacrifices.
                let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, opponent)
                    .iter()
                    .filter(|o| o.power.is_some())
                    .map(|o| o.id)
                    .collect();
                if let Some(&creature_id) = creatures.first() {
                    let name = state.get_object(creature_id).map(|o| o.name.clone()).unwrap_or_default();
                    crate::destruction::sacrifice(state, creature_id, registry);
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana -2: p{} sacrificed {}", opponent.0, name));
                }
            }
            2 => {
                // -6: Opponent sacrifices half their permanents (simplified).
                let permanents: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, opponent)
                    .iter()
                    .map(|o| o.id)
                    .collect();
                let half = permanents.len() / 2;
                let to_sacrifice: Vec<ObjectId> = permanents.into_iter().take(half.max(1)).collect();
                for &perm_id in &to_sacrifice {
                    let name = state.get_object(perm_id).map(|o| o.name.clone()).unwrap_or_default();
                    crate::destruction::sacrifice(state, perm_id, registry);
                    state.log(crate::state::LogLevel::Event,
                        format!("Liliana -6: p{} sacrificed {}", opponent.0, name));
                }
            }
            _ => {}
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[crate::actions::Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        // Set starting loyalty.
        state.add_counters(object_id, CounterType::Loyalty, 3);
        // Mark card_types on the object for SBA checking.
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.card_types = vec![CardType::Planeswalker];
        }
        state.log(crate::state::LogLevel::Event,
            "Liliana of the Veil enters with 3 loyalty".into());
    }
}
