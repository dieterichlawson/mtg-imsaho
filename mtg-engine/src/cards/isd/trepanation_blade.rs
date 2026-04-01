use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Trepanation Blade — {3} Artifact — Equipment.
/// Whenever equipped creature attacks, defending player reveals cards from the top
/// of their library until they reveal a land card. That player puts all cards revealed
/// this way into their graveyard. Equipped creature gets +1/+0 until end of turn for
/// each card put into a graveyard this way.
/// Equip {2}.
pub struct TrepanationBlade;

impl CardBehavior for TrepanationBlade {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Trepanation Blade".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec!["Equipment".into()],
            power: None,
            toughness: None,
            oracle_text: "Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. That player puts all cards revealed this way into their graveyard. Equipped creature gets +1/+0 until end of turn for each card put into a graveyard this way.\nEquip {2}".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "mill until land, equipped creature gets +1/+0 per card milled".into(),
                },
            ],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_equipment = true;
        }
    }

    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == caster)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        // self_id is the equipment's ID (from AttacksTrigger resolution).
        let equip = match state.get_object(self_id) {
            Some(o) => o,
            None => return,
        };
        let creature_id = match equip.attached_to {
            Some(id) => id,
            None => return,
        };

        // Find the defending player from combat state.
        let defending_player = state.combat.as_ref()
            .and_then(|c| c.attackers.get(&creature_id).copied());
        let defending_player = match defending_player {
            Some(p) => p,
            None => return,
        };

        // Reveal cards from defending player's library until a land is revealed.
        let mut cards_milled = 0;
        loop {
            let card_id = {
                let player = state.get_player(defending_player);
                if player.library_order.is_empty() {
                    break;
                }
                player.library_order[0]
            };

            // Check if this card is a land.
            let is_land = state.get_object(card_id)
                .map(|o| o.card_types.contains(&CardType::Land))
                .unwrap_or(false);

            // Remove from library and put into graveyard.
            let player = state.get_player_mut(defending_player);
            player.library_order.remove(0);
            state.move_object(card_id, Zone::Graveyard);
            cards_milled += 1;

            if is_land {
                break; // Stop after revealing a land.
            }
        }

        if cards_milled > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Trepanation Blade: p{} milled {} card{}", defending_player.0, cards_milled,
                    if cards_milled == 1 { "" } else { "s" }));

            // Equipped creature gets +1/+0 per card milled until end of turn.
            if state.get_object(creature_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                state.until_end_of_turn_effects.push(
                    crate::state::UntilEndOfTurnEffect {
                        target: creature_id,
                        power_mod: cards_milled,
                        toughness_mod: 0,
                    }
                );
                state.log(crate::state::LogLevel::Event,
                    format!("Trepanation Blade: equipped creature gets +{}/+0", cards_milled));
            }
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };
        if obj.zone == Zone::Battlefield && obj.power.is_none() {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {2}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(2)]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
                once_per_turn: false,
                sorcery_speed_only: true,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*creature_id);
            }
        }
    }
}
