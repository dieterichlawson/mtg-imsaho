use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Mindshrieker — {1}{U} 1/1 Spirit Bird with Flying.
/// {2}: Target player mills a card. Mindshrieker gets +X/+X until end of turn,
/// where X is the milled card's mana value.
pub struct Mindshrieker;

impl CardBehavior for Mindshrieker {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mindshrieker".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into(), "Bird".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Flying\n{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{2}: Target player mills a card. Mindshrieker gets +X/+X (X = mana value)".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(2),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::PlayerOnly),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Player(player_id)) = targets.first() {
            // Mill one card from target player, tracking what was milled.
            let milled_card_id = {
                let player_state = state.get_player_mut(*player_id);
                if player_state.library_order.is_empty() {
                    return;
                }
                player_state.library_order.remove(0)
            };
            state.move_object(milled_card_id, Zone::Graveyard);
            state.log(crate::state::LogLevel::Event,
                format!("p{} milled 1 card", player_id.0));

            // Look up the milled card's mana value.
            let mana_value = {
                let card_id = state.get_object(milled_card_id).map(|o| o.card_id);
                card_id.and_then(|cid| registry.get(cid))
                    .and_then(|b| b.card_data().cost.as_ref().map(|c| c.mana_value()))
                    .unwrap_or(0) as i32
            };

            // Apply +X/+X until end of turn.
            if mana_value > 0 {
                if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                    state.until_end_of_turn.push(
                        crate::state::TemporaryEffect::ModifyPT {
                            target: object_id,
                            power_mod: mana_value,
                            toughness_mod: mana_value,
                        }
                    );
                    state.log(crate::state::LogLevel::Event,
                        format!("Mindshrieker gets +{}/+{} (milled card's mana value)", mana_value, mana_value));
                }
            }
        }
    }
}
