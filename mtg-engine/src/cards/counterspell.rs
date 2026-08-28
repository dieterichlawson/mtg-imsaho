use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Counterspell — {U}{U} instant. Counter target spell.
pub struct Counterspell;

impl CardBehavior for Counterspell {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Counterspell".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Counter target spell.".into(),
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
            crate::cards::helpers::counter_spell(state, *target_id, registry);
        }
    }
}
