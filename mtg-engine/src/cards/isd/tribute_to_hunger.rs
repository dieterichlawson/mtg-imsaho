use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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
            oracle_text: "Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn is_valid_target(&self, _state: &GameState, caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        // "Target opponent" — can only target opponents, not self.
        match target {
            Target::Player(pid) => *pid != caster,
            Target::Object(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id)
            .map_or(PlayerId(0), |o| o.controller);

        // Target opponent — use the target if provided, otherwise pick the opponent.
        let opponent = match targets.first() {
            Some(Target::Player(pid)) => *pid,
            _ => state.opponent(controller),
        };

        // Get opponent's creatures.
        let opp_creatures: Vec<Target> = crate::cards::helpers::creatures_controlled_by(state, opponent);

        if opp_creatures.is_empty() {
            // No creatures to sacrifice.
            return;
        }

        // Opponent chooses which creature to sacrifice. We present the choice to
        // the opponent, and the sacrifice + life gain happen via the pending effect.
        crate::cards::helpers::present_target_choice(
            state,
            object_id,
            opponent,
            opp_creatures,
            PendingEffect::CardEffect { source_id: object_id, key: String::new() },
            "Tribute to Hunger: choose a creature to sacrifice",
            false, // mandatory
        );
    }

    /// "Target opponent sacrifices a creature. You gain life equal to its
    /// toughness." The toughness is read before the sacrifice — last known
    /// information (CR 608.2h) — and both halves are this card's text.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let toughness = state.effective_toughness(*id, registry)
            .or_else(|| state.get_object(*id).and_then(|o| o.toughness))
            .unwrap_or(0);
        let name = state.obj_name(*id);
        let beneficiary = crate::cards::helpers::controller_of(state, source_id);

        crate::destruction::sacrifice(state, *id, registry);

        if toughness > 0 {
            state.gain_life(beneficiary, toughness);
            state.log(crate::state::LogLevel::Event,
                format!("Tribute to Hunger: sacrificed {name}, p{} gained {toughness} life", beneficiary.0));
        } else {
            state.log(crate::state::LogLevel::Event,
                format!("Tribute to Hunger: sacrificed {name}"));
        }
    }
}
