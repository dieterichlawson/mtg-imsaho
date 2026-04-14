use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::actions::Target;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Bloodgift Demon — {3}{B}{B} 5/4 flying Demon.
/// At the beginning of your upkeep, target player draws a card and loses 1 life.
pub struct BloodgiftDemon;

impl CardBehavior for BloodgiftDemon {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bloodgift Demon".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Demon".into()],
            power: Some(5),
            toughness: Some(4),
            oracle_text: "Flying\nAt the beginning of your upkeep, target player draws a card and loses 1 life.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "target player draws a card and loses 1 life".into(),
                },
            ],
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        // "Target player" — present choice between all players.
        let targets: Vec<Target> = state.players.iter()
            .filter(|p| !p.lost)
            .filter(|p| !state.player_has_hexproof(p.id, registry) || p.id == controller)
            .map(|p| Target::Player(p.id))
            .collect();
        if targets.is_empty() {
            return;
        }
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "Bloodgift Demon: choose a player to draw a card and lose 1 life".into(),
                options: targets,
                optional: false,
                effect: PendingEffect::DrawAndLoseLife { source_name: "Bloodgift Demon".into() },
            },
        });
    }
}
