use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Dissipate — {1}{U}{U} instant. Counter target spell. Exile it instead of graveyard.
pub struct Dissipate;

impl CardBehavior for Dissipate {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Dissipate".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Spell
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        crate::cards::helpers::spell_target_is_legal(state, target)
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            // "exile it instead of putting it into its owner's graveyard".
            crate::cards::helpers::counter_spell_exiling(state, *target_id, registry);
        }
    }
}
