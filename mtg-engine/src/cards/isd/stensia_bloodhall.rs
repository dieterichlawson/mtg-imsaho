use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Stensia Bloodhall — Land.
/// {T}: Add {C}.
/// {3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
pub struct StensiaBloodhall;

impl CardBehavior for StensiaBloodhall {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Stensia Bloodhall".into(),
            cost: None,
            card_types: vec![CardType::Land],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{T}: Add {C}.\n{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ManaAbilityDef {
                ability_index: 0,
                description: "Add {C}".into(),
                produced: vec![(ManaType::Colorless, 1)],
                requires_tap: true,
            }]
        } else {
            vec![]
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
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
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        match targets.first() {
            Some(Target::Player(player_id)) => {
                let old_life = state.get_player(*player_id).life;
                let new_life = old_life - 2;
                state.get_player_mut(*player_id).life = new_life;
                state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                    source: object_id,
                    target: crate::events::DamageTarget::Player(*player_id),
                    amount: 2,
                });
                state.events.push(crate::events::GameEvent::LifeChanged {
                    player: *player_id,
                    old: old_life,
                    new_life,
                });
                state.log(crate::state::LogLevel::Event,
                    format!("Stensia Bloodhall deals 2 damage to p{}", player_id.0));
            }
            Some(Target::Object(target_id)) => {
                // Planeswalker: damage removes loyalty counters.
                if let Some(obj) = state.get_object_mut(*target_id) {
                    if obj.zone == Zone::Battlefield {
                        let loyalty = obj.counters.entry(CounterType::Loyalty).or_insert(0);
                        *loyalty = loyalty.saturating_sub(2);
                    }
                }
                state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                    source: object_id,
                    target: crate::events::DamageTarget::Object(*target_id),
                    amount: 2,
                });
                let target_name = state.get_object(*target_id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "?".into());
                state.log(crate::state::LogLevel::Event,
                    format!("Stensia Bloodhall deals 2 damage to {}", target_name));
            }
            None => {}
        }
    }
}
