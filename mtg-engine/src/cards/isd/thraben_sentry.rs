use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::cards::helpers;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};
use crate::actions::Target;

/// Thraben Sentry {3}{W} 2/2 Human Soldier with Vigilance // Thraben Militia 5/4 Human Soldier.
/// Whenever another creature you control dies, you may transform Thraben Sentry.
pub struct ThrabenSentry;

impl CardBehavior for ThrabenSentry {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Thraben Sentry".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Vigilance\nWhenever another creature you control dies, you may transform this creature.".into(),
            keywords: vec![Keyword::Vigilance],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "may transform Thraben Sentry".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Thraben Militia".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(5),
            toughness: Some(4),
            // CR 204.2: the back face has no mana cost, so its color is the
            // indicator printed beside its type line — white.
            color_indicator: vec![Color::White],
            oracle_text: "Trample".into(),
            keywords: vec![Keyword::Trample],
            ..Default::default()
        })
    }


    /// "Whenever another creature **you control** dies" — a condition on the
    /// event (CR 603.2). The front-face check beside it is CR 712.8d: the
    /// ability is printed on Thraben Sentry, not on Thraben Militia, so a
    /// transformed Sentry does not have it to trigger.
    fn should_trigger_on_creature_dies(&self, state: &GameState, self_id: ObjectId, _dead_id: ObjectId, dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _dead_subtypes: &[String], _registry: &CardRegistry) -> bool {
        let is_transformed = state.get_object(self_id).is_some_and(|o| o.is_transformed);
        !is_transformed && dead_controller == crate::cards::helpers::controller_of(state, self_id)
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], _registry: &CardRegistry) {
        // Whether it triggered was settled as the creature died. What is left
        // is the effect, and there are two ways it has nothing to do.
        //
        // The Sentry is no longer on the battlefield: there is nothing there
        // to transform (CR 400.7).
        if !crate::cards::helpers::still_on_battlefield(state, self_id) {
            return;
        }
        // Or it is already transformed. Ruling: "If multiple creatures you
        // control die simultaneously, Thraben Sentry's ability will trigger
        // that many times. Only the first one to resolve will cause it to
        // transform." Both triggered — they were collected while it was still
        // a Sentry — and the second must not flip it back, or ask.
        if state.get_object(self_id).is_some_and(|o| o.is_transformed) {
            return;
        }
        let controller = crate::cards::helpers::controller_of(state, self_id);
        // "You may transform Thraben Sentry" — present a choice to the player.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description: "Transform Thraben Sentry into Thraben Militia?".into(),
                source_card: self_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            state.log(crate::state::LogLevel::Event,
                "Thraben Sentry: chose not to transform".into());
            return;
        }
        // Transform using the helper so that keywords and subtypes are updated correctly.
        helpers::apply_transform(state, self_id, registry);
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}
