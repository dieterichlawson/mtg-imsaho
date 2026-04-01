use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::*;

/// Nevermore — {1}{W}{W} Enchantment.
/// As Nevermore enters the battlefield, choose a nonland card name.
/// Spells with the chosen name can't be cast.
///
/// Stores the chosen card name in instance_oracle_text (prefixed with "nevermore:").
/// The engine checks for Nevermore in legal_actions to prevent casting spells with that name.
pub struct Nevermore;

impl CardBehavior for Nevermore {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Nevermore".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As Nevermore enters the battlefield, choose a nonland card name.\nSpells with the chosen name can't be cast.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Build list of all nonland card names from the registry as options.
        // Use cards from opponent's hand as options (most strategically relevant).
        let opponent = state.opponent(controller);
        let mut options: Vec<Target> = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        // Prioritize cards in opponent's hand.
        for obj in state.objects_in_zone(Zone::Hand, opponent) {
            if let Some(data) = registry.card_data(obj.card_id) {
                if !data.card_types.contains(&CardType::Land) && seen_names.insert(data.name.clone()) {
                    options.push(Target::Object(obj.id));
                }
            }
        }

        // Also add cards in opponent's library for more options.
        for obj in state.objects_in_zone(Zone::Library, opponent) {
            if let Some(data) = registry.card_data(obj.card_id) {
                if !data.card_types.contains(&CardType::Land) && seen_names.insert(data.name.clone()) {
                    options.push(Target::Object(obj.id));
                }
            }
        }

        if options.len() <= 1 {
            // Auto-select if only one option.
            if let Some(Target::Object(ref_id)) = options.first() {
                let chosen_name = state.get_object(*ref_id)
                    .and_then(|o| registry.card_data(o.card_id))
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| "Lightning Bolt".into());
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.instance_oracle_text = Some(format!("nevermore:{}", chosen_name));
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Nevermore names \"{}\"", chosen_name));
            } else {
                // No options — name a default.
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.instance_oracle_text = Some("nevermore:Lightning Bolt".into());
                }
                state.log(crate::state::LogLevel::Event,
                    "Nevermore names \"Lightning Bolt\" (no nonland cards found)".into());
            }
        } else {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseCardName {
                    description: "Nevermore: choose a nonland card name".into(),
                    options,
                    source_id: object_id,
                },
            });
        }
    }
}
