use crate::cards::{TargetRequirement, CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::actions::Target;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

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
            subtypes: vec!["Demon".into()],
            power: Some(5),
            toughness: Some(4),
            oracle_text: "Flying\nAt the beginning of your upkeep, target player draws a card and loses 1 life.".into(),
            keywords: vec![Keyword::Flying],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "target player draws a card and loses 1 life".into(),
                    // CR 603.3d: the target is chosen as the trigger goes on
                    // the stack, so the engine picks it — with hexproof
                    // filtering — rather than `on_upkeep` prompting later.
                    target_requirement: Some(TargetRequirement::PlayerOnly),
                },
            ],
            ..Default::default()
        }
    }

    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::Your,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    /// CR 603.3d: the target arrived with the trigger. `step_trigger_scope`
    /// already restricted this to the controller's own upkeep.
    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 113.7a: "target player draws a card and loses 1 life" is entirely
        // about the target — the Demon is not mentioned. So the ability
        // resolves whether or not the Demon is still on the battlefield, and
        // killing it in response to its own upkeep trigger does not stop the
        // draw. This used to return early when the source had gone.
        let Some(target) = chosen_targets.first() else { return };
        self.resolve_card_effect(state, self_id, "", target, registry);
    }

    /// "At the beginning of your upkeep, target player draws a card and loses
    /// 1 life." The card count and the life loss are this card's numbers.
    fn resolve_card_effect(&self, state: &mut GameState, _source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Player(pid) = target else { return };
        // The summary says what happened, not what was asked: with an empty
        // library no card is drawn (CR 121.3), and "drew a card" under "tried
        // to draw from an empty library" contradicted the line above it
        // (issue #623).
        //
        // The draw and the life loss each have the engine's line, and the
        // loss names this card; a summary of both beside them logged the
        // loss twice (#706).
        // `draw_cards` logs an empty library's failed draw itself (#623).
        let _drawn = crate::engine::draw_cards(state, *pid, 1, registry);
        state.change_life_for(*pid, -1, "Bloodgift Demon");
    }
}
