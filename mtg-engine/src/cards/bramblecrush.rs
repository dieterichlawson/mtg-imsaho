use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Bramblecrush — {2}{G}{G} sorcery. Destroy target noncreature permanent.
pub struct Bramblecrush;

impl CardBehavior for Bramblecrush {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bramblecrush".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target noncreature permanent.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter("noncreature".into())
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield => o,
                    _ => return false,
                };
                let registry = crate::cards::CardRegistry::with_all_cards();
                registry.card_data(obj.card_id)
                    .map(|d| !d.card_types.contains(&CardType::Creature))
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if let Some(obj) = state.get_object(*target_id) {
                if obj.zone == Zone::Battlefield && obj.power.is_some() {
                    // Creatures are destroyed (indestructible / regeneration apply).
                    crate::destruction::try_destroy(state, *target_id, registry);
                } else if obj.zone == Zone::Battlefield {
                    // Non-creature permanents are just moved to graveyard.
                    state.move_object(*target_id, Zone::Graveyard);
                }
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}
