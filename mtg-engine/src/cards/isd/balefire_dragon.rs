use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Balefire Dragon — {5}{R}{R} 6/6 Dragon. Flying.
/// Whenever Balefire Dragon deals combat damage to a player, it deals that much damage
/// to each creature that player controls.
pub struct BalefireDragon;

impl CardBehavior for BalefireDragon {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Balefire Dragon".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Dragon".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Flying\nWhenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.".into(),
            keywords: vec![Keyword::Flying],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "deal that much damage to each creature that player controls".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, damaged_player: PlayerId, amount: u32, registry: &CardRegistry) {
        // CR 113.7a: killing the Dragon with the trigger on the stack does not
        // save the defending player's board.
        if state.get_object(self_id).is_none() {
            return;
        }

        let creatures: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry) && o.controller == damaged_player)
            .map(|o| o.id)
            .collect();

        let source_name = state.get_object(self_id)
            .map_or_else(|| "Balefire Dragon".into(), |o| o.name.clone());
        for creature_id in creatures {
            let effect = crate::state::PendingEffect::DealDamage {
                amount,
                source_id: self_id,
                source_name: source_name.clone(),
            };
            crate::engine::apply_pending_effect(
                state,
                &crate::actions::Target::Object(creature_id),
                &effect,
                registry,
            );
        }
    }
}
