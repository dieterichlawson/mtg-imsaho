use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Undead Alchemist — {3}{U} 4/2 Zombie.
/// If a Zombie you control would deal combat damage to a player, instead that
/// player mills that many cards. Whenever a creature card is put into an
/// opponent's graveyard from their library, exile that card and create a 2/2
/// black Zombie creature token.
///
/// Ability 1 is a replacement effect: combat damage from Zombies is replaced
/// with milling. Implemented via replace_combat_damage_to_player.
///
/// Ability 2 (mill-watcher trigger for non-combat mill sources) is not yet
/// implemented as a standalone trigger — currently the exile-and-token logic
/// is inlined in the replacement effect for the combat mill path only.
pub struct UndeadAlchemist;

impl CardBehavior for UndeadAlchemist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Undead Alchemist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into()],
            power: Some(4),
            toughness: Some(2),
            oracle_text: "If a Zombie you control would deal combat damage to a player, instead that player mills that many cards.\nWhenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn replace_combat_damage_to_player(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        source_id: ObjectId,
        damaged_player: PlayerId,
        amount: u32,
        registry: &CardRegistry,
    ) -> bool {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return false,
        };
        // Only replace damage from Zombies we control.
        let source = match state.get_object(source_id) {
            Some(o) if o.controller == controller => o,
            _ => return false,
        };
        let is_zombie = registry.card_data(source.card_id)
            .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
            .unwrap_or(false)
            || source.subtypes.iter().any(|s| s == "Zombie");
        if !is_zombie {
            return false;
        }

        // Mill that many cards instead of dealing damage.
        let player_state = state.get_player(damaged_player);
        let mill_count = std::cmp::min(amount as usize, player_state.library_order.len());
        let milled_ids: Vec<ObjectId> = player_state.library_order[..mill_count].to_vec();

        let player_state = state.get_player_mut(damaged_player);
        for _ in 0..mill_count {
            player_state.library_order.remove(0);
        }
        for &obj_id in &milled_ids {
            state.move_object(obj_id, Zone::Graveyard, registry);
        }

        // Ability 2 (inline for combat-mill path): exile milled creatures, create Zombies.
        for &obj_id in &milled_ids {
            let is_creature = state.get_object(obj_id)
                .map(|o| {
                    registry.card_data(o.card_id)
                        .map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                        .unwrap_or(o.power.is_some())
                })
                .unwrap_or(false);
            if is_creature {
                state.move_object(obj_id, Zone::Exile, registry);
                state.create_token_with_subtypes(
                    "Zombie", controller, 2, 2,
                    vec![Color::Black],
                    vec![CardType::Creature],
                    vec![],
                    vec!["Zombie".into()],
                    registry,
                );
            }
        }

        state.log(crate::state::LogLevel::Event,
            format!("Undead Alchemist: Zombie dealt {} combat damage, milled {} cards instead",
                amount, mill_count));
        true // Damage fully replaced
    }
}
