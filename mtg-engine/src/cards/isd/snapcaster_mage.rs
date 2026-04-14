use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};
use crate::actions::Target;

/// Snapcaster Mage — {1}{U} 2/1 Human Wizard. Flash.
/// When this creature enters, target instant or sorcery card in your graveyard
/// gains flashback until end of turn. The flashback cost is equal to its mana cost.
pub struct SnapcasterMage;

impl CardBehavior for SnapcasterMage {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Snapcaster Mage".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Flash\nWhen this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![Keyword::Flash],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "grant flashback to instant or sorcery in graveyard".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Find all eligible instant/sorcery cards in graveyard.
        // Per oracle, Snapcaster can target any instant or sorcery — including
        // cards that already have printed flashback (granting a second flashback
        // cost equal to their mana cost).
        let eligible: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller)
            .filter(|o| {
                registry.card_data(o.card_id)
                    .is_some_and(|d| {
                        d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery)
                    })
            })
            .filter(|o| !state.until_end_of_turn.iter().any(|e| matches!(e, crate::state::TemporaryEffect::GrantFlashback { target, .. } if *target == o.id)))
            .map(|o| o.id)
            .collect();

        if eligible.is_empty() {
            return;
        }

        if eligible.len() == 1 {
            // Only one legal option — auto-select.
            let target_id = eligible[0];
            let cost = registry.card_data(state.get_object(target_id).unwrap().card_id)
                .and_then(|d| d.cost.clone())
                .unwrap_or(ManaCost::free());
            state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantFlashback { target: target_id, cost });
            let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("Snapcaster Mage grants flashback to {name}"));
        } else {
            // Multiple eligible — player chooses via ChooseTarget.
            let targets: Vec<crate::actions::Target> = eligible.iter()
                .map(|&id| crate::actions::Target::Object(id))
                .collect();
            crate::cards::helpers::present_target_choice(
                state, object_id, controller, targets,
                crate::state::PendingEffect::GrantFlashback { source_name: "Snapcaster Mage".into() },
                "Snapcaster Mage: choose an instant or sorcery to grant flashback",
                false,
            );
        }
    }
}
