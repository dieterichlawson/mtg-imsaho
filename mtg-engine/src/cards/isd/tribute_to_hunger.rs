use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::*;

/// Tribute to Hunger — {2}{B} Instant.
/// Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.
pub struct TributeToHunger;

impl CardBehavior for TributeToHunger {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Tribute to Hunger".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Instant],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn is_valid_target(&self, _state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        // "Target opponent" — can only target opponents, not self.
        match target {
            Target::Player(pid) => *pid != caster,
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id)
            .map(|o| o.controller)
            .unwrap_or(PlayerId(0));

        // Target opponent — use the target if provided, otherwise pick the opponent.
        let opponent = match targets.first() {
            Some(Target::Player(pid)) => *pid,
            _ => state.opponent(controller),
        };

        // Get opponent's creatures.
        let opp_creatures: Vec<Target> = crate::cards::helpers::creatures_controlled_by(state, opponent);

        if opp_creatures.is_empty() {
            // No creatures to sacrifice.
            state.move_spell_after_resolve(object_id, registry);
            return;
        }

        // Opponent chooses which creature to sacrifice. We present the choice to
        // the opponent, and the sacrifice + life gain happen via the pending effect.
        crate::cards::helpers::present_target_choice(
            state,
            object_id,
            opponent,
            opp_creatures,
            PendingEffect::SacrificeAndGainLife {
                beneficiary: controller,
                spell_id: object_id,
            },
            "Tribute to Hunger: choose a creature to sacrifice",
            false, // mandatory
        );
    }
}
