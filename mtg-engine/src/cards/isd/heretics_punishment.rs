use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            oracle_text: "{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
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
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Check target legality. If target is illegal, the ability fizzles.
        if let Some(target) = targets.first() {
            let target_legal = match target {
                Target::Object(target_id) => {
                    state.get_object(*target_id)
                        .is_some_and(|o| o.zone == Zone::Battlefield)
                }
                Target::Player(_) => true,
                // Ruling (2011-09-22): "If the targeted permanent or player is
                // an illegal target by the time the ability resolves, the
                // entire ability won't resolve. No cards will be put into your
                // graveyard, and no damage will be dealt."
                Target::Illegal => false,
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
            let mv = state.face_data(card_obj_id, registry)
                .and_then(|d| d.cost.map(|c| c.mana_value()))
                .unwrap_or(0);
            if mv > max_mv {
                max_mv = mv;
            }
        }

        // "…then mill three cards." Routed through `mill_one` so a creature
        // card among them emits CreatureCardMilled — moving them directly meant
        // an opponent's Undead Alchemist ("whenever a creature card is put into
        // an opponent's graveyard from their library") never saw the three
        // cards this puts into *your* graveyard from *your* library. Which
        // watchers care is the collector's decision, not the miller's.
        let milled: Vec<ObjectId> = state.get_player_mut(controller)
            .library_order.drain(..mill_count).collect();
        for card_id in milled {
            crate::engine::mill_one(state, controller, card_id, registry);
        }

        // Deal damage to target equal to highest mana value.
        if max_mv > 0 {
            let target = targets.first().unwrap();
            let damage_target = match target {
                Target::Object(target_id) => crate::events::DamageTarget::Object(*target_id),
                Target::Player(player_id) => crate::events::DamageTarget::Player(*player_id),
                    // CR 608.2b: a target that is no longer legal is not
                    // dealt damage at all.
                    Target::Illegal => return,
            };
            crate::damage::deal_damage(state, object_id, damage_target, max_mv,
                crate::damage::DamageKind::NonCombat, registry);
        }

        state.log(crate::state::LogLevel::Event,
            format!("Heretic's Punishment milled {mill_count} cards, dealt {max_mv} damage"));
    }
}
