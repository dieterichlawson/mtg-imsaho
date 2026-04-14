use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};
use crate::actions::Target;

/// Nevermore — {1}{W}{W} Enchantment.
/// As this enchantment enters, choose a nonland card name.
/// Spells with the chosen name can't be cast.
///
/// Presents the player with a choice of all implemented nonland card names.
/// The chosen name is stored as a `PreventCastingNamed` instance continuous
/// effect. The engine checks for this effect in `legal_actions`.
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
            oracle_text: "As this enchantment enters, choose a nonland card name.\nSpells with the chosen name can't be cast.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "choose a nonland card name".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Collect all implemented nonland card names.
        let mut card_names: Vec<String> = registry.all_names().into_iter()
            .filter(|name| {
                registry.get_id_by_name(name)
                    .and_then(|id| registry.card_data(id))
                    .is_some_and(|d| !d.card_types.contains(&CardType::Land))
            })
            .map(std::string::ToString::to_string)
            .collect();
        card_names.sort();

        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: object_id,
            choice: ResolutionChoiceKind::ChooseCardName {
                description: "Nevermore: choose a nonland card name (spells with that name can't be cast)".into(),
                options: card_names,
                source_id: object_id,
            },
        });
    }
}
