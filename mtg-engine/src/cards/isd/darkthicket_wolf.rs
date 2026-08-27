use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{GameState, TemporaryEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Darkthicket Wolf — {1}{G} 2/2 Wolf. {2}{G}: This creature gets +2/+2 until end of turn.
/// Activate only once each turn.
pub struct DarkthicketWolf;

impl CardBehavior for DarkthicketWolf {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Darkthicket Wolf".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Wolf".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{2}{G}: +2/+2 until end of turn".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(2),
                    ManaSymbol::Colored(Color::Green),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: true,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        state.until_end_of_turn.push(TemporaryEffect::ModifyPT {
            target: object_id,
            power_mod: 2,
            toughness_mod: 2,
        });
    }
}
