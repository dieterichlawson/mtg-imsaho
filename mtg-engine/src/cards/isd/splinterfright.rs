use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};
use crate::actions::Target;

/// Splinterfright — {2}{G} */* Elemental. Trample.
/// Splinterfright's power and toughness are each equal to the number of
/// creature cards in your graveyard.
/// At the beginning of your upkeep, mill two cards.
pub struct Splinterfright;

impl CardBehavior for Splinterfright {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Splinterfright".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Elemental".into()],
            // */* — the CDA (dynamic_pt) defines actual P/T. Some(0) is needed so
            // the engine recognizes this as a creature (power.is_some() is used as proxy).
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Trample\nSplinterfright's power and toughness are each equal to the number of creature cards in your graveyard.\nAt the beginning of your upkeep, mill two cards. (Put the top two cards of your library into your graveyard.)".into(),
            keywords: vec![Keyword::Trample],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "mill two cards".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Option<(i32, i32)> {
        // CR 109.5: "you" on an object is its controller — and for a static
        // ability, the *current* controller of the object it is on. A CDA is a
        // static ability (CR 604.3), so a stolen Splinterfright is the size of
        // the thief's graveyard, not its owner's.
        //
        // The same field answers the ruling's other half — "the ability works
        // in all zones... if Splinterfright is in your graveyard, it will
        // count itself" — because CR 108.4 gives a card off the battlefield no
        // controller, and the zone change resets `controller` to `owner` on
        // the way out. So one read covers both, where reading `owner` was only
        // ever right in one of them.
        let you = state.get_object(object_id)?.controller;
        let creature_cards_in_gy = i32::try_from(state.objects_in_zone(Zone::Graveyard, you)
            .iter()
            .filter(|o| state.is_creature(o.id, registry) && state.is_card(o.id))
            .count()).unwrap_or(i32::MAX);
        Some((creature_cards_in_gy, creature_cards_in_gy))
    }

    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::Your,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        // Only trigger on your upkeep.
        // `step_trigger_scope` already gates this to the controller's own
        // step; re-deriving it here is duplication, not defence.
        crate::engine::mill_cards(state, controller, 2, "Splinterfright", registry);
    }
}
