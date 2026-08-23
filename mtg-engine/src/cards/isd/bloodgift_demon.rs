use crate::cards::{TargetRequirement, CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
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
                    // CR 603.3b: the target is chosen as the trigger goes on
                    // the stack, so the engine picks it — with hexproof
                    // filtering — rather than `on_upkeep` prompting later.
                    target_requirement: Some(TargetRequirement::PlayerOnly),
                },
            ],
        }
    }

    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::Your,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    /// CR 603.3b: the target arrived with the trigger. `step_trigger_scope`
    /// already restricted this to the controller's own upkeep.
    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        if state.get_object(self_id).is_none_or(|o| o.zone != Zone::Battlefield) {
            return;
        }
        let Some(target) = chosen_targets.first() else { return };
        self.resolve_card_effect(state, self_id, "", target, registry);
    }

    /// "At the beginning of your upkeep, target player draws a card and loses
    /// 1 life." The card count and the life loss are this card's numbers.
    fn resolve_card_effect(&self, state: &mut GameState, _source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Player(pid) = target else { return };
        let _ = crate::engine::draw_cards(state, *pid, 1, registry);
        state.lose_life(*pid, 1);
        state.log(crate::state::LogLevel::Event,
            format!("Bloodgift Demon: p{} drew a card and lost 1 life", pid.0));
    }
}
