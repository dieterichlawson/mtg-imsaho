use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, Zone, ManaType, ManaCost, ManaSymbol, Color};

/// Stensia Bloodhall — Land.
/// {T}: Add {C}.
/// {3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
pub struct StensiaBloodhall;

impl CardBehavior for StensiaBloodhall {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stensia Bloodhall".into(),
            card_types: vec![CardType::Land],
            oracle_text: "{T}: Add {C}.\n{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.".into(),
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
                description: "{3}{B}{R}, {T}: Deal 2 damage to target player or planeswalker".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(3),
                    ManaSymbol::Colored(Color::Black),
                    ManaSymbol::Colored(Color::Red),
                ]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::PlayerOrPlaneswalker),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(target) = targets.first() {
            let effect = crate::state::PendingEffect::DealDamage {
                amount: 2,
                source_id: object_id,
                source_name: "Stensia Bloodhall".into(),
            };
            crate::engine::apply_pending_effect(state, target, &effect, registry);
        }
    }
}
