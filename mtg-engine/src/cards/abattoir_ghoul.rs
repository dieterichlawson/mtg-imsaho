use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Abattoir Ghoul — {3}{B} 3/2 Zombie. First strike.
/// Whenever a creature dealt damage by Abattoir Ghoul this turn dies,
/// you gain life equal to that creature's toughness.
pub struct AbattoirGhoul;

impl CardBehavior for AbattoirGhoul {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Abattoir Ghoul".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "First strike\nWhenever a creature dealt damage by Abattoir Ghoul this turn dies, you gain life equal to that creature's toughness.".into(),
            keywords: vec![Keyword::FirstStrike],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "gain life equal to that creature's toughness".into(),
                },
            ],
        }
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, dead_id: ObjectId, _dead_controller: PlayerId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Check if the dead creature was dealt damage by Abattoir Ghoul this turn.
        let was_damaged_by_me = state.get_object(dead_id)
            .map(|o| o.damaged_by.contains(&self_id))
            .unwrap_or(false);
        if !was_damaged_by_me {
            return;
        }
        // Gain life equal to that creature's toughness.
        let toughness = state.effective_toughness(dead_id, registry).unwrap_or(0).max(0);
        if toughness > 0 {
            let old = state.get_player(controller).life;
            let new_life = old + toughness;
            state.get_player_mut(controller).life = new_life;
            state.events.push(crate::events::GameEvent::LifeChanged { player: controller, old, new_life });
            state.log(crate::state::LogLevel::Event,
                format!("Abattoir Ghoul: gained {} life from creature death", toughness));
        }
    }
}
