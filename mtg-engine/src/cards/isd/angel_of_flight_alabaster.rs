use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Angel of Flight Alabaster — {4}{W} 4/4 flying Angel.
/// At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
pub struct AngelOfFlightAlabaster;

impl CardBehavior for AngelOfFlightAlabaster {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Angel of Flight Alabaster".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Angel".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Flying\nAt the beginning of your upkeep, return target Spirit card from your graveyard to your hand.".into(),
            keywords: vec![Keyword::Flying],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "return target Spirit card from your graveyard to your hand".into(),
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    // Use GraveyardCardOwnedByCaster + is_valid_target to filter to Spirits.
                    target_requirement: Some(TargetRequirement::GraveyardCardOwnedByCaster),
                },
            ],
            ..Default::default()
        }
    }

    /// Filter graveyard cards to Spirit cards only.
    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        let Target::Object(id) = target else { return false; };
        let Some(obj) = state.get_object(*id) else { return false; };
        if obj.zone != Zone::Graveyard {
            return false;
        }
        state.has_subtype(obj.id, "Spirit", registry)
    }

    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::Your,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    /// `step_trigger_scope` gates this to the controller's own step, and
    /// CR 113.7a means the ability resolves whether or not the Angel is still
    /// around — so neither the controller nor the source is needed here, which
    /// is why the source parameter is unused.
    fn on_upkeep(&self, state: &mut GameState, _self_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 603.3d: target was chosen when the trigger went on the stack.
        let Some(target) = chosen_targets.first() else { return };
        let effect = PendingEffect::ReturnToHand {
            source_name: "Angel of Flight Alabaster".into(),
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}
