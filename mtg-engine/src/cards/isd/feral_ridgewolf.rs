use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::{GameState, TemporaryEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Feral Ridgewolf — {2}{R} 1/2 Wolf with Trample. {1}{R}: Feral Ridgewolf gets +2/+0 until end of turn.
pub struct FeralRidgewolf;

impl CardBehavior for FeralRidgewolf {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Feral Ridgewolf".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Wolf".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "Trample\n{1}{R}: This creature gets +2/+0 until end of turn.".into(),
            keywords: vec![Keyword::Trample],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}{R}: +2/+0 until end of turn".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(1),
                    ManaSymbol::Colored(Color::Red),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        state.until_end_of_turn.push(TemporaryEffect::ModifyPT {
            target: object_id,
            power_mod: 2,
            toughness_mod: 0,
        });
    }
}
