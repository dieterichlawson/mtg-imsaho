use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Purify the Grave — {W} Instant.
/// Exile target card from a graveyard.
/// Flashback {W}.
pub struct PurifyTheGrave;

impl CardBehavior for PurifyTheGrave {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Purify the Grave".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Exile target card from a graveyard.\nFlashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::White)])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::GraveyardCard
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            let name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*target_id, Zone::Exile, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Purify the Grave exiled {name} from graveyard"));
        }
        state.move_spell_after_resolve(object_id, registry);
    }
}
