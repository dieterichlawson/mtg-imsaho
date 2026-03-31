use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

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
            oracle_text: "Exile target card from a graveyard.\nFlashback {W}".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::White)])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        // Target any card in a graveyard. We use Creature as an approximation — the engine
        // primarily deals with creature targets. For now, we auto-exile in on_resolve.
        TargetRequirement::None
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let opponent = state.opponent(controller);

        // Exile a card from opponent's graveyard (auto-select first card found).
        let target_card = state.objects_in_zone(Zone::Graveyard, opponent)
            .iter()
            .map(|o| o.id)
            .next();

        if let Some(target_id) = target_card {
            let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(target_id, Zone::Exile);
            state.log(crate::state::LogLevel::Event,
                format!("Purify the Grave exiled {} from graveyard", name));
        } else {
            // If opponent's graveyard is empty, try our own.
            let own_target = state.objects_in_zone(Zone::Graveyard, controller)
                .iter()
                .map(|o| o.id)
                .next();
            if let Some(target_id) = own_target {
                let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(target_id, Zone::Exile);
                state.log(crate::state::LogLevel::Event,
                    format!("Purify the Grave exiled {} from graveyard", name));
            }
        }

        state.move_spell_after_resolve(object_id);
    }
}
