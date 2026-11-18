use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{GameState, TemporaryEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Vampiric Fury — {1}{R} instant. Vampire creatures you control get +2/+0 and gain first strike until end of turn.
pub struct VampiricFury;

impl CardBehavior for VampiricFury {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Vampiric Fury".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Vampire creatures you control get +2/+0 and gain first strike until end of turn. (They deal combat damage before creatures without first strike.)".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // Find the controller of this spell.
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // CR 611.2c: a continuous effect created by a resolving spell or
        // ability affects the set of objects that existed when it resolved, and
        // that set never changes. This is the line between Glorious Anthem (a
        // permanent's static ability, which picks up newcomers) and a pump
        // spell (which does not) — so the creatures are snapshotted here rather
        // than matched by a live filter every time P/T is computed.
        //
        // Each Vampire of that set in a loop, not a filter collected, as
        // Bitterheart Witch searches (a proof walks a filter's inner `find`
        // again for each item collected).
        for id in state.creatures_controlled_snapshot(controller, registry) {
            if !state.has_subtype(id, "Vampire", registry) {
                continue;
            }
            state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
                target: id, power_mod: 2, toughness_mod: 0,
            });
            state.until_end_of_turn.push(TemporaryEffect::GrantKeyword {
                target: id, keyword: Keyword::FirstStrike,
            });
        }

    }
}
