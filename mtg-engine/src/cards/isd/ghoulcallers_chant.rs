use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Ghoulcaller's Chant — {B} Sorcery.
/// Choose one — Return target creature card from your graveyard to your hand;
/// or return two target Zombie cards from your graveyard to your hand.
pub struct GhoulcallersChant;

impl CardBehavior for GhoulcallersChant {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ghoulcaller's Chant".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Choose one —\n• Return target creature card from your graveyard to your hand.\n• Return two target Zombie cards from your graveyard to your hand.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        // Modal: mode 1 = one creature card; mode 2 = two Zombie cards.
        TargetRequirement::ModalChoice(vec![
            TargetRequirement::GraveyardCreature,
            TargetRequirement::TwoTargets(
                Box::new(TargetRequirement::GraveyardCreatureOfSubtype("Zombie".into())),
                Box::new(TargetRequirement::GraveyardCreatureOfSubtype("Zombie".into())),
            ),
        ])
    }

    fn is_valid_target(&self, state: &GameState, caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        // Both modes require cards in the caster's graveyard.
        // The TargetRequirement handles creature/subtype filtering.
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Graveyard && o.owner == caster)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        for target in targets {
            if let Target::Object(card_id) = target {
                if let Some(obj) = state.get_object(*card_id) {
                    if obj.zone == Zone::Graveyard {
                        let name = obj.name.clone();
                        state.move_object(*card_id, Zone::Hand);
                        state.log(crate::state::LogLevel::Event,
                            format!("Ghoulcaller's Chant returned {} to hand", name));
                    }
                }
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}
