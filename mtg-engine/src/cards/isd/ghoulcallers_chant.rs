use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            oracle_text: "Choose one —\n• Return target creature card from your graveyard to your hand.\n• Return two target Zombie cards from your graveyard to your hand.".into(),
            ..Default::default()
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
                    .is_some_and(|o| o.zone == Zone::Graveyard && o.owner == caster
                        && state.is_card(o.id))
            }
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        for target in targets {
            if let Target::Object(card_id) = target {
                if let Some(obj) = state.get_object(*card_id) {
                    if obj.zone == Zone::Graveyard {
                        let name = obj.name.clone();
                        state.move_object(*card_id, Zone::Hand, registry);
                        state.log(crate::state::LogLevel::Event,
                            format!("Ghoulcaller's Chant returned {name} to hand"));
                    }
                }
            }
        }
    }
}
