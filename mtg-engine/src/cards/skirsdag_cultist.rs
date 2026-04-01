use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Shaman".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "{R}, {T}, Sacrifice a creature: Skirsdag Cultist deals 2 damage to any target.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        // Only available on the battlefield.
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{R}, {T}, Sacrifice a creature: Deal 2 damage to any target".into(),
                cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::SacrificeCreature,
                target_requirement: Some(TargetRequirement::AnyTarget),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        // Deal 2 damage to the chosen target.
        if let Some(target) = targets.first() {
            match target {
                Target::Object(target_id) => {
                    if let Some(obj) = state.get_object_mut(*target_id) {
                        if obj.zone == Zone::Battlefield {
                            obj.damage_marked += 2;
                            obj.damaged_by.push(_object_id);
                            let name = obj.name.clone();
                            state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                                source: _object_id,
                                target: crate::events::DamageTarget::Object(*target_id),
                                amount: 2,
                            });
                            state.log(crate::state::LogLevel::Event, format!("Skirsdag Cultist dealt 2 damage to {}", name));
                        }
                    }
                }
                Target::Player(player_id) => {
                    let old = state.get_player(*player_id).life;
                    let new_life = old - 2;
                    state.get_player_mut(*player_id).life = new_life;
                    state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                        source: _object_id,
                        target: crate::events::DamageTarget::Player(*player_id),
                        amount: 2,
                    });
                    state.events.push(crate::events::GameEvent::LifeChanged { player: *player_id, old, new_life });
                    state.log(crate::state::LogLevel::Event, format!("Skirsdag Cultist dealt 2 damage to p{}", player_id.0));
                }
            }
        }
    }
}
