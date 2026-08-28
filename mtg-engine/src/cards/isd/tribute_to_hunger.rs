use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
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

    /// "Target opponent", not "target player" (CR 102.1) — see
    /// `TargetRequirement::OpponentOnly`. This used to be `PlayerOnly` plus an
    /// `is_valid_target` of `*pid != caster`.
    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::OpponentOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // "Target opponent" — the opponent is the one the caster chose, and
        // there is no other. This used to fall back to `state.opponent(controller)`
        // for anything else, which invents a target the caster never declared;
        // the rule for a target that stopped being legal is CR 608.2b, the
        // spell does not resolve at all, and `stack::resolve_spell` applies it
        // before this is ever called.
        let Some(&Target::Player(opponent)) = targets.first() else { return };

        // Get opponent's creatures.
        let opp_creatures: Vec<Target> = crate::cards::helpers::creatures_controlled_by(state, opponent, registry);

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
            registry,
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
