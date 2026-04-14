use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Crossway Vampire — 3/2 for {1}{R}{R}. Vampire.
/// When this creature enters, target creature can't block this turn.
pub struct CrosswayVampire;

impl CardBehavior for CrosswayVampire {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Crossway Vampire".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "When this creature enters, target creature can't block this turn.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "target creature can't block this turn".into(),
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // "Target creature" — any creature, including self (Oracle doesn't say "another").
        let targets = crate::cards::helpers::creature_targets(state, object_id, controller, registry);
        crate::cards::helpers::present_target_choice(
            state, object_id, controller, targets,
            crate::state::PendingEffect::CantBlockThisTurn { source_name: "Crossway Vampire".into() },
            "Crossway Vampire: target creature can't block this turn",
            false, // mandatory
        );
    }
}
