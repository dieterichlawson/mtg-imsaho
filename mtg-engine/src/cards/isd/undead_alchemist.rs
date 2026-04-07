use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Undead Alchemist — {3}{U} 4/2 Zombie.
/// If a Zombie you control would deal combat damage to a player, instead that
/// player mills that many cards. Whenever a creature card is put into an
/// opponent's graveyard from their library, exile that card and create a 2/2
/// black Zombie creature token.
///
/// Simplified: The replacement effect is handled in on_any_combat_damage_to_player.
/// Since we can't truly replace damage at this level, we mill the opponent and
/// prevent the life loss by restoring it. The mill-to-exile-and-token part is
/// also handled there.
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
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCombatDamageToPlayer,
                    description: "mill instead of damage, exile creatures for Zombie tokens".into(),
                },
            ],
        }
    }

    fn on_any_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, source_id: ObjectId, damaged_player: PlayerId, amount: u32, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Check if the source is a Zombie we control.
        let source = match state.get_object(source_id) {
            Some(o) if o.controller == controller => o,
            _ => return,
        };
        let is_zombie = registry.card_data(source.card_id)
            .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
            .unwrap_or(false)
            || source.subtypes.iter().any(|s| s == "Zombie");
        if !is_zombie {
            return;
        }
        // "Instead" — restore the life loss from combat damage.
        let current_life = state.get_player(damaged_player).life;
        state.get_player_mut(damaged_player).life = current_life + amount as i32;

        // Mill that many cards.
        let player_state = state.get_player(damaged_player);
        let mill_count = std::cmp::min(amount as usize, player_state.library_order.len());
        let milled_ids: Vec<ObjectId> = player_state.library_order[..mill_count].to_vec();

        // Remove from library.
        let player_state = state.get_player_mut(damaged_player);
        for _ in 0..mill_count {
            player_state.library_order.remove(0);
        }
        // Move to graveyard, then check for creatures.
        for &obj_id in &milled_ids {
            state.move_object(obj_id, Zone::Graveyard, registry);
        }

        // Check each milled card: if it's a creature, exile it and create a Zombie.
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
    }
}
