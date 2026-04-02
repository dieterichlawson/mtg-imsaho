use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Heretic's Punishment — {4}{R} Enchantment.
/// {3}{R}: Mill three cards, then Heretic's Punishment deals damage to any target
/// equal to the highest mana value among the milled cards.
pub struct HereticsPunishment;

impl CardBehavior for HereticsPunishment {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Heretic's Punishment".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{3}{R}: Mill three cards, deal damage equal to highest mana value".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(3),
                    ManaSymbol::Colored(Color::Red),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::AnyTarget),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Check target legality. If target is illegal, the ability fizzles.
        if let Some(target) = targets.first() {
            let target_legal = match target {
                Target::Object(target_id) => {
                    state.get_object(*target_id)
                        .map(|o| o.zone == Zone::Battlefield)
                        .unwrap_or(false)
                }
                Target::Player(_) => true,
            };
            if !target_legal {
                state.log(crate::state::LogLevel::Event,
                    "Heretic's Punishment ability fizzles — target is illegal".to_string());
                return;
            }
        } else {
            return;
        }

        // Mill three cards first.
        let player = state.get_player(controller);
        let mill_count = std::cmp::min(3, player.library_order.len());
        let to_mill: Vec<ObjectId> = player.library_order[..mill_count].to_vec();

        // Compute highest mana value among cards to be milled before moving them.
        let mut max_mv: u32 = 0;
        for &card_obj_id in &to_mill {
            let card_id = state.get_object(card_obj_id).map(|o| o.card_id).unwrap_or(crate::ids::CardId(0));
            let mv = registry.card_data(card_id)
                .and_then(|d| d.cost.map(|c| c.mana_value()))
                .unwrap_or(0);
            if mv > max_mv {
                max_mv = mv;
            }
        }

        // Move milled cards to graveyard.
        let milled: Vec<ObjectId> = state.get_player_mut(controller)
            .library_order.drain(..mill_count).collect();
        for card_id in milled {
            state.move_object(card_id, Zone::Graveyard);
        }

        // Deal damage to target equal to highest mana value.
        if max_mv > 0 {
            let target = targets.first().unwrap();
            match target {
                Target::Object(target_id) => {
                    if let Some(obj) = state.get_object_mut(*target_id) {
                        if obj.zone == Zone::Battlefield {
                            obj.damage_marked += max_mv;
                            obj.damaged_by.push(object_id);
                        }
                    }
                    state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                        source: object_id,
                        target: crate::events::DamageTarget::Object(*target_id),
                        amount: max_mv,
                    });
                }
                Target::Player(player_id) => {
                    let old = state.get_player(*player_id).life;
                    let new_life = old - max_mv as i32;
                    state.get_player_mut(*player_id).life = new_life;
                    state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                        source: object_id,
                        target: crate::events::DamageTarget::Player(*player_id),
                        amount: max_mv,
                    });
                    state.events.push(crate::events::GameEvent::LifeChanged {
                        player: *player_id,
                        old,
                        new_life,
                    });
                }
            }
        }

        state.log(crate::state::LogLevel::Event,
            format!("Heretic's Punishment milled {} cards, dealt {} damage", mill_count, max_mv));
    }
}
