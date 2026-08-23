use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Zone};
use crate::actions::Target;

/// Witchbane Orb — {4} Artifact.
/// When Witchbane Orb enters the battlefield, destroy all Curses attached to you.
/// You have hexproof.
pub struct WitchbaneOrb;

impl CardBehavior for WitchbaneOrb {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Witchbane Orb".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "When this artifact enters, destroy all Curses attached to you.\nYou have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "Witchbane Orb: destroy all Curses attached to you".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn grants_player_hexproof(&self) -> bool { true }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Find all curses attached to the controller.
        let curses: Vec<ObjectId> = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.attached_to_player == Some(controller)
                    && state.face_data(o.id, registry)
                        .is_some_and(|d| d.subtypes.iter().any(|s| s == "Curse"))
            })
            .map(|o| o.id)
            .collect();

        for curse_id in &curses {
            let name = state.get_object(*curse_id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, *curse_id, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Witchbane Orb destroyed {name}"));
        }
    }
}
