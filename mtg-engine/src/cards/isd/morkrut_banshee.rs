use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Morkrut Banshee — 4/4 for {3}{B}{B}. Spirit.
/// Morbid — When Morkrut Banshee enters the battlefield, if a creature died this turn,
/// target creature gets -4/-4 until end of turn.
pub struct MorkrutBanshee;

impl CardBehavior for MorkrutBanshee {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Morkrut Banshee".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Morbid — When this creature enters, if a creature died this turn, target creature gets -4/-4 until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "if morbid, target creature gets -4/-4 until end of turn".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        if !state.creature_died_this_turn {
            return;
        }

        let controller = crate::cards::helpers::controller_of(state, object_id);
        // "Target creature" — can target ANY creature including itself.
        let targets = crate::cards::helpers::creature_targets(state);
        crate::cards::helpers::present_target_choice(
            state, object_id, controller, targets,
            PendingEffect::DebuffUntilEOT { power: -4, toughness: -4, source_name: "Morkrut Banshee".into() },
            "Morkrut Banshee: target creature gets -4/-4 until end of turn",
            false, // mandatory, not "you may"
        );
    }
}
