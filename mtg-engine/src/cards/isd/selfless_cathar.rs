use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, ActivatedAbilityDef, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Selfless Cathar — {W} 1/1 Human Soldier.
/// {1}{W}, Sacrifice Selfless Cathar: Creatures you control get +1/+1 until end of turn.
pub struct SelflessCathar;

impl CardBehavior for SelflessCathar {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Selfless Cathar".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "{1}{W}, Sacrifice this creature: Creatures you control get +1/+1 until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}{W}, Sacrifice: Creatures you control get +1/+1 until end of turn".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeThis,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        // The creature was already sacrificed by the engine before this is called.
        // Use the controller from the object (even though it's in the graveyard now).
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap();

        let creature_ids: Vec<ObjectId> = state.objects.values()
            .filter(|obj| {
                obj.zone == Zone::Battlefield
                    && obj.controller == controller
                    && obj.power.is_some()
            })
            .map(|obj| obj.id)
            .collect();

        for id in creature_ids {
            state.until_end_of_turn.push(
                crate::state::TemporaryEffect::ModifyPT {
                    target: id,
                    power_mod: 1,
                    toughness_mod: 1,
                }
            );
        }
    }
}
