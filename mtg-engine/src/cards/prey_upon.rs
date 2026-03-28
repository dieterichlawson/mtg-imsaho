use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Prey Upon — {G} sorcery. Target creature you control fights target creature you don't control.
pub struct PreyUpon;

impl CardBehavior for PreyUpon {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Prey Upon".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target creature you control fights target creature you don't control.".into(),
            keywords: vec![],
            flashback_cost: None,
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::TwoTargets(
            Box::new(TargetRequirement::CreatureWithFilter("you control".into())),
            Box::new(TargetRequirement::CreatureWithFilter("you don't control".into())),
        )
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        if targets.len() == 2 {
            if let (Target::Object(a), Target::Object(b)) = (&targets[0], &targets[1]) {
                let caster = state.get_object(object_id).map(|o| o.controller);
                let a_mine = caster.and_then(|c| state.get_object(*a).map(|o| o.controller == c)).unwrap_or(false);

                // Handle both target orderings: (mine, theirs) or (theirs, mine).
                let (my_creature, their_creature) = if a_mine {
                    (*a, *b)
                } else {
                    (*b, *a)
                };

                let registry = crate::cards::CardRegistry::with_all_cards();
                crate::combat::fight(state, my_creature, their_creature, &registry);
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}
