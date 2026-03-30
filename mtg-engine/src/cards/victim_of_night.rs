use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Victim of Night — {B}{B} instant. Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
pub struct VictimOfNight;

impl CardBehavior for VictimOfNight {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Victim of Night".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target non-Vampire, non-Werewolf, non-Zombie creature.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::NotSubtypes(vec!["Vampire".into(), "Werewolf".into(), "Zombie".into()]))
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) => o,
                    None => return false,
                };
                if obj.zone != Zone::Battlefield || obj.power.is_none() { return false; }
                if let Some(data) = registry.card_data(obj.card_id) {
                    !data.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie")
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, object_id, targets, registry);
    }
}
