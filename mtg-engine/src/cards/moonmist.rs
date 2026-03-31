use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Moonmist — {1}{G} Instant.
/// Transform all Humans. Prevent all combat damage that would be dealt this turn
/// by creatures other than Werewolves and Wolves.
///
/// Simplified: transforms all Human Werewolf DFCs (sets is_transformed = true).
/// Combat damage prevention for non-Wolf/non-Werewolf creatures is not implemented
/// (would require a combat damage prevention system that doesn't exist yet).
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
            oracle_text: "Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // Transform all Human Werewolf DFCs (those that have a back face).
        let humans_to_transform: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && !o.is_transformed)
            .filter(|o| {
                // Check if it's a Human with a back face (i.e., a DFC).
                let has_human_subtype = o.subtypes.iter().any(|s| s == "Human")
                    || registry.card_data(o.card_id)
                        .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                        .unwrap_or(false);
                let has_back_face = registry.get(o.card_id)
                    .and_then(|b| b.back_face_data())
                    .is_some();
                has_human_subtype && has_back_face
            })
            .map(|o| o.id)
            .collect();

        let count = humans_to_transform.len();
        for hid in humans_to_transform {
            if let Some(obj) = state.get_object_mut(hid) {
                obj.is_transformed = true;
            }
            // Update characteristics from back face.
            if let Some(behavior) = state.get_object(hid).and_then(|o| registry.get(o.card_id)) {
                if let Some(back) = behavior.back_face_data() {
                    if let Some(obj) = state.get_object_mut(hid) {
                        obj.name = back.name.clone();
                        if let Some(p) = back.power {
                            obj.power = Some(p);
                        }
                        if let Some(t) = back.toughness {
                            obj.toughness = Some(t);
                        }
                        obj.keywords = back.keywords.clone();
                        obj.subtypes = back.subtypes.clone();
                    }
                }
            }
        }

        if count > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Moonmist transformed {} Human(s)", count));
        }
        // Note: combat damage prevention for non-Wolf/Werewolf is not implemented.

        state.move_spell_after_resolve(object_id);
    }
}
