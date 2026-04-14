use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};
use crate::actions::Target;

/// Fiend Hunter — {1}{W}{W} 1/3 Human Cleric.
/// When Fiend Hunter enters the battlefield, you may exile another target creature.
/// When Fiend Hunter leaves the battlefield, return the exiled card to the battlefield
/// under its owner's control.
pub struct FiendHunter;

impl CardBehavior for FiendHunter {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Fiend Hunter".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(3),
            oracle_text: "When this creature enters, you may exile another target creature.\nWhen this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "you may exile another target creature".into(),
                target_requirement: None,
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::LeavesBattlefield,
                    description: "return exiled card to the battlefield".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // "Another target creature" — any creature except Fiend Hunter itself.
        // Can target own creatures (Oracle doesn't restrict to opponents).
        let targets = crate::cards::helpers::creature_targets_except(state, object_id, object_id, controller, registry);
        // "You may" — always present choice, even with 1 target.
        crate::cards::helpers::present_optional_target_choice(
            state, object_id, controller, targets,
            PendingEffect::ExileAndStore { source_id: object_id, source_name: "Fiend Hunter".into() },
            "Fiend Hunter: you may exile another target creature",
        );
    }

    fn on_leave_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let exiled_id = state.get_object(object_id)
            .and_then(|o| o.card_state.get("exiled_creature").copied());
        if let Some(target_id) = exiled_id {
            if state.get_object(target_id).is_some_and(|o| o.zone == Zone::Exile) {
                let returned_name = state.obj_name(target_id);
                state.move_object(target_id, Zone::Battlefield, registry);
                // "under its owner's control" — reset controller to owner
                if let Some(obj) = state.get_object_mut(target_id) {
                    obj.controller = obj.owner;
                }
                state.log(crate::state::LogLevel::Event, format!("{returned_name} returned to the battlefield"));
            }
        }
    }
}
