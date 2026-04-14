use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::NotSubtypes(vec!["Vampire".into(), "Werewolf".into(), "Zombie".into()]))
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let Some(obj) = state.get_object(*id) else { return false; };
                if obj.zone != Zone::Battlefield || obj.power.is_none() { return false; }
                // Check both registry subtypes (regular cards) and object subtypes (tokens).
                let excluded = ["Vampire", "Werewolf", "Zombie"];
                let has_excluded_registry = registry.card_data(obj.card_id)
                    .is_some_and(|d| d.subtypes.iter().any(|s| excluded.contains(&s.as_str())));
                let has_excluded_obj = obj.subtypes.iter().any(|s| excluded.contains(&s.as_str()));
                !has_excluded_registry && !has_excluded_obj
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, object_id, targets, registry);
    }
}
