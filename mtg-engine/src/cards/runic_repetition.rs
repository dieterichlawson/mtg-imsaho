use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Runic Repetition — {2}{U} Sorcery.
/// Return target exiled card you own with flashback to your hand.
pub struct RunicRepetition;

impl CardBehavior for RunicRepetition {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Runic Repetition".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Return target exiled card you own with flashback to your hand.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::ExileCard
    }

    fn is_valid_target(&self, state: &GameState, caster: crate::ids::PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| {
                        o.zone == Zone::Exile && o.owner == caster
                            && registry.card_data(o.card_id)
                                .map(|d| d.flashback_cost.is_some())
                                .unwrap_or(false)
                    })
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            let name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*target_id, Zone::Hand);
            state.log(crate::state::LogLevel::Event,
                format!("Runic Repetition returned {} from exile to hand", name));
        }
        state.move_spell_after_resolve(object_id);
    }
}
