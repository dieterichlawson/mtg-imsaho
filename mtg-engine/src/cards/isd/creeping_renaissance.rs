use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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

        // Present a choice of permanent types.
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
}
