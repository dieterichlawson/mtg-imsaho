use crate::cards::{CardBehavior, CardData, CardRegistry};
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
            oracle_text: "As this enchantment enters, choose a nonland card name.\nSpells with the chosen name can't be cast.".into(),
            ..Default::default()
        }
    }

    /// CR 614.12: the name is chosen **as** Nevermore enters, not from a
    /// trigger. `has_etb_handler` stays false so no `EntersBattlefield` trigger
    /// is created for it.
    fn chooses_as_it_enters(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Collect all implemented nonland card names.
        // Nevermore names a CARD, not a permanent — this reads the registry by
        // name with no game object involved, so the characteristics layer
        // (which resolves an object's active face) does not apply.
        // A card is named by its front face, so it is the front face's types
        // that say whether it is a land. The list is allocated at its
        // largest and filled in a loop: a filtered `collect` allocates when
        // it keeps its first name, which a proof has to take both ways.
        let mut names = Vec::with_capacity(registry.card_count());
        for (name, id) in registry.names_and_ids() {
            if !registry.printed_card_types(id, false).contains(&CardType::Land) {
                names.push(name.to_string());
            }
        }
        let card_names = crate::cards::helpers::sorted(names);

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
