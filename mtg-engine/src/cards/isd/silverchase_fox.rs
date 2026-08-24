use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, ActivatedAbilityDef, SacrificeCost, TargetRequirement, TargetFilter};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Silverchase Fox — {1}{W} 2/2 Fox.
/// {1}{W}, Sacrifice Silverchase Fox: Exile target enchantment.
pub struct SilverchaseFox;

impl CardBehavior for SilverchaseFox {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Silverchase Fox".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Fox".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "{1}{W}, Sacrifice this creature: Exile target enchantment.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}{W}, Sacrifice: Exile target enchantment".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeThis,
            target_requirement: Some(TargetRequirement::PermanentWithFilter(
                TargetFilter::HasCardType(vec![CardType::Enchantment]),
            )),
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                let exiled_name = state.obj_name(*target_id);
                state.move_object(*target_id, Zone::Exile, registry);
                state.log(crate::state::LogLevel::Event, format!("Silverchase Fox exiled {exiled_name}"));
            }
        }
    }
}
