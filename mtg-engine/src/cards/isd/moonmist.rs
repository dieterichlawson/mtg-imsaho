use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Moonmist — {1}{G} Instant.
/// Transform all Humans. Prevent all combat damage that would be dealt this turn
/// by creatures other than Werewolves and Wolves.
pub struct Moonmist;

impl CardBehavior for Moonmist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moonmist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // "Transform all Humans." — any creature currently with the Human subtype
        // should transform, regardless of which face is showing. This includes:
        // - Front-face Humans (transform to back face)
        // - Back-face creatures that are still Human (transform back to front face,
        //   e.g., Thraben Militia is Human on its back face)
        let humans_to_transform: Vec<(ObjectId, bool)> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield)
            .filter(|o| {
                // Check if the creature currently has the Human subtype on its
                // active face. Check instance subtypes first, then also check
                // the registry for the correct face (transform-aware).
                let has_human_subtype = if o.subtypes.iter().any(|s| s == "Human") {
                    true
                } else if o.is_transformed {
                    registry.get(o.card_id)
                        .and_then(super::super::CardBehavior::back_face_data)
                        .is_some_and(|d| d.subtypes.iter().any(|s| s == "Human"))
                } else {
                    registry.card_data(o.card_id)
                        .is_some_and(|d| d.subtypes.iter().any(|s| s == "Human"))
                };
                // Must be a DFC (has a back face).
                let has_back_face = registry.get(o.card_id)
                    .and_then(super::super::CardBehavior::back_face_data)
                    .is_some();
                has_human_subtype && has_back_face
            })
            .map(|o| (o.id, o.is_transformed))
            .collect();

        let count = humans_to_transform.len();
        for (hid, was_transformed) in &humans_to_transform {
            if let Some(obj) = state.get_object_mut(*hid) {
                obj.is_transformed = !obj.is_transformed;
            }
            if let Some(behavior) = state.get_object(*hid).and_then(|o| registry.get(o.card_id)) {
                if *was_transformed {
                    // Was on back face → transform to front face. Restore front face data.
                    let front = behavior.card_data();
                    if let Some(obj) = state.get_object_mut(*hid) {
                        obj.name.clone_from(&front.name);
                        obj.power = front.power;
                        obj.toughness = front.toughness;
                        obj.keywords.clone_from(&front.keywords);
                        obj.subtypes.clone_from(&front.subtypes);
                    }
                } else {
                    // Was on front face → transform to back face. Apply back face data.
                    if let Some(back) = behavior.back_face_data() {
                        if let Some(obj) = state.get_object_mut(*hid) {
                            obj.name.clone_from(&back.name);
                            if let Some(p) = back.power {
                                obj.power = Some(p);
                            }
                            if let Some(t) = back.toughness {
                                obj.toughness = Some(t);
                            }
                            obj.keywords.clone_from(&back.keywords);
                            obj.subtypes.clone_from(&back.subtypes);
                        }
                    }
                }
            }
        }

        if count > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Moonmist transformed {count} Human(s)"));
        }
        // Prevent all combat damage from non-Wolf/non-Werewolf creatures this turn.
        state.until_end_of_turn.push(crate::state::TemporaryEffect::PreventNonWolfWerewolfCombatDamage);
        state.log(crate::state::LogLevel::Event,
            "Moonmist: preventing combat damage from non-Wolf/non-Werewolf creatures this turn".into());

        state.move_spell_after_resolve(object_id, registry);
    }
}
