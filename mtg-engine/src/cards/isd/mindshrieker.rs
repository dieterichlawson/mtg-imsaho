use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

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
            subtypes: vec!["Spirit".into(), "Bird".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Flying\n{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.".into(),
            keywords: vec![Keyword::Flying],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
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
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Player(player_id)) = targets.first() {
            // Mill one card from target player, remembering which so the
            // mana value can be read afterwards. Routed through `mill_one` so
            // a creature card among them emits CreatureCardMilled — moving it
            // directly meant Undead Alchemist never saw it.
            let Some(&milled_card_id) = state.get_player(*player_id).library_order.first() else {
                return;
            };
            crate::engine::mill_one(state, *player_id, milled_card_id, registry);
            state.log(crate::state::LogLevel::Event,
                format!("p{} milled 1 card", player_id.0));

            // Look up the milled card's mana value.
            let mana_value = {
                let card_id = state.get_object(milled_card_id).map(|o| o.card_id);
                i32::try_from(card_id.and_then(|cid| registry.get(cid))
                    .and_then(|b| b.card_data().cost.as_ref().map(crate::types::ManaCost::mana_value))
                    .unwrap_or(0)).unwrap_or(i32::MAX)
            };

            // Apply +X/+X until end of turn.
            if mana_value > 0
                && state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield)
            {
                state.until_end_of_turn.push(
                    crate::state::TemporaryEffect::ModifyPT {
                        target: object_id,
                        power_mod: mana_value,
                        toughness_mod: mana_value,
                    }
                );
                state.log(crate::state::LogLevel::Event,
                    format!("Mindshrieker gets +{mana_value}/+{mana_value} (milled card's mana value)"));
            }
        }
    }
}
