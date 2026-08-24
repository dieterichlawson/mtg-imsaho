use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, Zone, ManaType, ManaCost, ManaSymbol, Color, CounterType};

/// Gavony Township — Land.
/// {T}: Add {C}.
/// {2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
pub struct GavonyTownship;

impl CardBehavior for GavonyTownship {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Gavony Township".into(),
            card_types: vec![CardType::Land],
            oracle_text: "{T}: Add {C}.\n{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.".into(),
            ..Default::default()
        }
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}".into(),
            produced: vec![(ManaType::Colorless, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }]
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ActivatedAbilityDef {
                ability_index: 1,
                description: "{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(2),
                    ManaSymbol::Colored(Color::Green),
                    ManaSymbol::Colored(Color::White),
                ]),
                requires_tap: true,
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
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);
        let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .filter(|o| o.power.is_some())
            .map(|o| o.id)
            .collect();
        for cid in creatures {
            state.add_counters(cid, CounterType::PlusOnePlusOne, 1);
        }
        state.log(crate::state::LogLevel::Event,
            format!("Gavony Township puts a +1/+1 counter on each creature p{} controls", controller.0));
    }
}
