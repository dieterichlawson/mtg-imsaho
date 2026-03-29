use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Unburial Rites — {4}{B} sorcery. Return target creature card from your graveyard to the battlefield.
/// Simplified: no targeting at engine level; finds the best creature in controller's graveyard.
pub struct UnburialRites;

impl CardBehavior for UnburialRites {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Unburial Rites".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Return target creature card from your graveyard to the battlefield.".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White)])),
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
        // Find creature cards in controller's graveyard.
        let targets: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.power.is_some() && o.id != object_id)
            .map(|o| Target::Object(o.id))
            .collect();

        if targets.is_empty() {
            // No creatures in graveyard, spell fizzles.
            state.move_spell_after_resolve(object_id);
        } else if targets.len() == 1 {
            // Auto-return the only creature.
            if let Target::Object(id) = targets[0] {
                let name = state.get_object(id).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(id, Zone::Battlefield);
                state.log(LogLevel::Event, format!("{} returned to the battlefield", name));
            }
            state.move_spell_after_resolve(object_id);
        } else {
            // Multiple choices -- ask the player.
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Unburial Rites: return a creature card from your graveyard to the battlefield".into(),
                    options: targets,
                    optional: false,
                    effect: PendingEffect::ReturnToBattlefield { spell_id: object_id },
                },
            });
            // Don't clean up spell yet -- ResolveChoice handler does it.
        }
    }
}
