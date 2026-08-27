use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Skirsdag Cultist — {2}{R}{R} 2/2 Human Shaman.
/// {R}, {T}, Sacrifice a creature: Skirsdag Cultist deals 2 damage to any target.
pub struct SkirsdagCultist;

impl CardBehavior for SkirsdagCultist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Skirsdag Cultist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Shaman".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "{R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // Only available on the battlefield.
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{R}, {T}, Sacrifice a creature: Deal 2 damage to any target".into(),
                cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::SacrificeCreature,
                target_requirement: Some(TargetRequirement::AnyTarget),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        // Deal 2 damage to the chosen target.
        if let Some(target) = targets.first() {
            let damage_target = match target {
                Target::Object(target_id) => crate::events::DamageTarget::Object(*target_id),
                Target::Player(player_id) => crate::events::DamageTarget::Player(*player_id),
                    // CR 608.2b: a target that is no longer legal is not
                    // dealt damage at all.
                    Target::Illegal => return,
            };
            crate::damage::deal_damage(state, object_id, damage_target, 2,
                crate::damage::DamageKind::NonCombat, registry);
        }
    }
}
