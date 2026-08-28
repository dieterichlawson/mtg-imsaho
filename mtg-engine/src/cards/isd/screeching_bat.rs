use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::cards::helpers;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};
use crate::actions::Target;

/// Screeching Bat {2}{B} 2/2 Bat with Flying // Stalking Vampire 5/5 Vampire.
/// Both faces: "At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform."
pub struct ScreechingBat;

impl ScreechingBat {
    fn transform_cost() -> ManaCost {
        ManaCost::new(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Colored(Color::Black),
            ManaSymbol::Colored(Color::Black),
        ])
    }
}

impl CardBehavior for ScreechingBat {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Screeching Bat".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Bat".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.".into(),
            keywords: vec![Keyword::Flying],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "you may pay {2}{B}{B} to transform".into(),
                    target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData {
            name: "Stalking Vampire".into(),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Vampire".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "you may pay {2}{B}{B} to transform".into(),
                    target_requirement: None,
                },
            ],
            ..Default::default()
        })
    }


    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::Your,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // `step_trigger_scope` already gates this to the controller's own
        // step; re-deriving it here is duplication, not defence.
        // "You may pay {2}{B}{B}. If you do, transform."
        //
        // Check autotap reachability, not pool-floating. Per CR 106.4 mana
        // pools empty between steps, so the pool is typically empty at
        // upkeep. We need to offer the may-pay prompt whenever the player
        // has enough untapped mana sources — the tap plan is executed in
        // `on_yes_no_choice` below.
        let cost = Self::transform_cost();
        let Some(tap_plan) = crate::engine::plan_autotap_for_cost(state, controller, &cost, registry) else {
            return;
        };

        let current_name = state.obj_name(self_id);
        let plan_suffix = helpers::format_tap_plan_names(state, &tap_plan);
        let description = if plan_suffix.is_empty() {
            format!("{current_name}: pay {{2}}{{B}}{{B}} to transform?")
        } else {
            format!("{current_name}: pay {{2}}{{B}}{{B}} ({plan_suffix}) to transform?")
        };

        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::YesNo {
                description,
                source_card: self_id,
            },
        });
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        if !yes {
            state.log(LogLevel::Event,
                format!("{}: chose not to pay", state.obj_name(self_id)));
            return;
        }

        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // Recompute the tap plan and execute it. State hasn't changed since
        // `on_upkeep` set up the prompt (no intervening triggers / priority),
        // so the plan is the same — recomputing is simpler than stashing.
        let cost = Self::transform_cost();
        let Some(tap_plan) = crate::engine::plan_autotap_for_cost(state, controller, &cost, registry) else {
            state.log(LogLevel::Event, "Could not pay {2}{B}{B} to transform".into());
            return;
        };
        if !crate::engine::execute_tap_plan_and_pay(state, controller, &cost, &tap_plan, registry) {
            state.log(LogLevel::Event, "Could not pay {2}{B}{B} to transform".into());
            return;
        }

        // Transform — uses the generic helper to update name, keywords, and subtypes.
        helpers::apply_transform(state, self_id, registry);
    }

    fn should_transform(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> bool {
        false
    }
}
