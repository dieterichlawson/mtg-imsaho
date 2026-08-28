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
            oracle_text: "When this artifact enters, destroy all Curses attached to you.\nYou have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "Witchbane Orb: destroy all Curses attached to you".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn grants_player_hexproof(&self) -> bool { true }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Find all curses attached to the controller.
        let curses: Vec<ObjectId> = state.all_objects_in_zone(Zone::Battlefield).into_iter()
            .filter(|o| {
                o.attached_to_player == Some(controller)
                    && state.face_data(o.id, registry)
                        .is_some_and(|d| d.subtypes.iter().any(|s| s == "Curse"))
            })
            .map(|o| o.id)
            .collect();

        // "Destroy all Curses attached to you" — one event (CR 700.2c), so
        // `try_destroy_all` rather than a loop.
        let names: Vec<String> = curses.iter()
            .map(|&id| state.obj_name(id))
            .collect();
        let results = crate::destruction::try_destroy_all(state, &curses, registry);
        // Say what actually happened to each. A Curse could be indestructible
        // or regenerate; the log used to announce every one of them destroyed
        // regardless.
        for (name, (_, result)) in names.into_iter().zip(results) {
            let line = match result {
                crate::destruction::DestroyResult::Died =>
                    format!("Witchbane Orb destroyed {name}"),
                crate::destruction::DestroyResult::Regenerated =>
                    format!("Witchbane Orb could not destroy {name} — it regenerated"),
                crate::destruction::DestroyResult::Indestructible =>
                    format!("Witchbane Orb could not destroy {name} — it is indestructible"),
            };
            state.log(crate::state::LogLevel::Event, line);
        }
    }
}
