use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Creeping Renaissance — {3}{G}{G} Sorcery.
/// Choose a permanent type. Return all cards of the chosen type from your graveyard
/// to your hand. Flashback {5}{G}{G}.
pub struct CreepingRenaissance;

impl CardBehavior for CreepingRenaissance {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Creeping Renaissance".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.\nFlashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Ruling: "The permanent types are artifact, creature, enchantment,
        // land, and planeswalker."
        let options = vec![
            "Creature".to_string(),
            "Artifact".to_string(),
            "Enchantment".to_string(),
            "Land".to_string(),
            "Planeswalker".to_string(),
        ];

        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: object_id,
            choice: ResolutionChoiceKind::ChooseCardType {
                description: "Creeping Renaissance: choose a permanent type".into(),
                options,
                controller,
            },
        });
    }

    /// "Return all cards of the chosen type from your graveyard to your hand."
    ///
    /// This used to be the body of the engine's `ChooseCardType` handler, which
    /// made "return them from your graveyard" the only thing naming a permanent
    /// type could ever mean.
    fn on_card_type_choice(&self, state: &mut GameState, self_id: ObjectId, chosen_type: &str, registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        let card_type = match chosen_type {
            "Artifact" => CardType::Artifact,
            "Enchantment" => CardType::Enchantment,
            "Land" => CardType::Land,
            "Planeswalker" => CardType::Planeswalker,
            _ => CardType::Creature,
        };
        // "all **cards** of the chosen type from **your** graveyard": CR 109.1
        // excludes a token, and a graveyard is keyed by owner (CR 404.3).
        let to_return: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .map(|o| o.id)
            .filter(|id| state.is_card(*id) && state.has_card_type(*id, card_type, registry))
            .collect();
        let count = to_return.len();
        for id in to_return {
            state.move_object(id, Zone::Hand, registry);
        }
        state.log(crate::state::LogLevel::Event,
            format!("Creeping Renaissance: chose {chosen_type}, returned {count} card(s) to hand"));
    }
}
