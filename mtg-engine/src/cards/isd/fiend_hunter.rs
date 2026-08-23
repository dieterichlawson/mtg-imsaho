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
                    // CR 603.3d: declaring the requirement makes the engine
                    // lock the target as the trigger goes on the stack.
                    target_requirement: Some(crate::cards::TargetRequirement::CreatureWithFilter(
                        crate::cards::TargetFilter::Another,
                    )),
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

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, chosen_targets: &[Target], _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // CR 603.3d: the target was chosen when the trigger went on the
        // stack; legality was re-checked before resolution. Only the "you
        // may" decision remains — offer exactly the locked target (or
        // decline), never a fresh pick from the current battlefield.
        let Some(target) = chosen_targets.first().cloned() else { return };
        crate::cards::helpers::present_optional_target_choice(
            state, object_id, controller, vec![target],
            PendingEffect::CardEffect { source_id: object_id, key: String::new() },
            "Fiend Hunter: you may exile the targeted creature",
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

    /// "When this creature enters, exile another target creature." The exiled
    /// creature's id is remembered on this permanent so the leaves-the-
    /// battlefield trigger can return it — the key is this card's own
    /// convention, so the bookkeeping belongs here.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let name = state.obj_name(*id);
        state.move_object(*id, Zone::Exile, registry);
        if let Some(source_obj) = state.get_object_mut(source_id) {
            source_obj.card_state.insert("exiled_creature".into(), *id);
        }
        state.log(crate::state::LogLevel::Event, format!("Fiend Hunter exiled {name}"));
    }
}
