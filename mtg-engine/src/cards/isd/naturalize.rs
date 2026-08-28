use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Naturalize — {1}{G} instant. Destroy target artifact or enchantment.
pub struct Naturalize;

impl CardBehavior for Naturalize {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Naturalize".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Destroy target artifact or enchantment.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Artifact, CardType::Enchantment]))
    }

    /// No `is_valid_target`: "an artifact or enchantment on the battlefield" is
    /// exactly `PermanentWithFilter(HasCardType([Artifact, Enchantment]))`,
    /// whose filter arm is `types.iter().any(..)`. `legal_actions` applies it
    /// when offering targets and `stack::is_target_legal` re-applies it with
    /// the zone check on the way down (CR 608.2b).
    ///
    /// As with Ancient Grudge, the requirement says nothing about creatures, so
    /// an artifact creature or an enchantment creature is a legal target.
    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, targets, registry);
    }
}
