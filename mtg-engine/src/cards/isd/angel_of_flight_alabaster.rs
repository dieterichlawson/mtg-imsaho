use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
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
            supertypes: vec![],
            subtypes: vec!["Angel".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "Flying\nAt the beginning of your upkeep, return target Spirit card from your graveyard to your hand.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "return target Spirit card from your graveyard to your hand".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        // Collect Spirit cards in graveyard as targets.
        let targets: Vec<Target> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .is_some_and(|d| d.subtypes.iter().any(|s| s == "Spirit"))
                || o.subtypes.iter().any(|s| s == "Spirit")
            })
            .map(|o| Target::Object(o.id))
            .collect();
        // Present choice to controller (mandatory if any valid targets exist).
        crate::cards::helpers::present_target_choice(
            state, self_id, controller, targets,
            PendingEffect::ReturnToHand { source_name: "Angel of Flight Alabaster".into() },
            "Angel of Flight Alabaster: return target Spirit from graveyard to hand",
            false,
        );
    }
}
