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
        // CR 112.8: a card in a graveyard is controlled by its owner, and
        // `objects_in_zone` filters graveyards by owner — so reading a stale
        // `controller` left over from a steal effect would count the
        // opponent's graveyard instead of this card's owner's.
        let owner = state.get_object(object_id)?.owner;
        let creature_cards_in_gy = i32::try_from(state.objects_in_zone(Zone::Graveyard, owner)
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
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        // Only trigger on your upkeep.
        // `step_trigger_scope` already gates this to the controller's own
        // step; re-deriving it here is duplication, not defence.
        crate::engine::mill_cards(state, controller, 2, registry);
        state.log(crate::state::LogLevel::Event, "Splinterfright: milled 2 cards".into());
    }
}
