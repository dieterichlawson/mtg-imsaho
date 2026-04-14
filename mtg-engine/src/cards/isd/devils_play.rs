use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Devil's Play — {X}{R} Sorcery.
/// Devil's Play deals X damage to any target.
/// Flashback {X}{R}{R}{R}.
///
/// Simplified: Since the engine doesn't yet support choosing X at cast time,
/// X is computed as the total mana the player had minus the colored requirement ({R})
/// when cast normally, or minus {R}{R}{R} when cast via flashback.
/// The X value is stored on the spell object by the engine's casting code.
pub struct DevilsPlay;

impl CardBehavior for DevilsPlay {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Devil's Play".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::X,
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Devil's Play deals X damage to any target.\nFlashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::X,
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::AnyTarget
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // Read X from the stored x_value on the spell object.
        let x = state.get_object(object_id)
            .and_then(|o| o.x_value)
            .unwrap_or(0);
        if x > 0 {
            crate::cards::helpers::resolve_damage(state, object_id, targets, x, registry);
        } else {
            state.move_spell_after_resolve(object_id, registry);
        }
    }
}
