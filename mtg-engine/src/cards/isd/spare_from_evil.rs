use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, CreatureFilter};

/// Spare from Evil — {1}{W} instant.
/// Creatures you control gain protection from non-Human creatures until end of turn.
pub struct SpareFromEvil;

impl CardBehavior for SpareFromEvil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Spare from Evil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Creatures you control gain protection from non-Human creatures until end of turn.".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Grant protection from non-Human creatures until end of turn.
        // CR 611.2c: a continuous effect created by a resolving spell or
        // ability affects the set of objects that existed when it resolved, and
        // that set never changes. This is the line between Glorious Anthem (a
        // permanent's static ability, which picks up newcomers) and a pump
        // spell (which does not) — so the creatures are snapshotted here rather
        // than matched by a live filter every time P/T is computed.
        // "protection from non-Human *creatures*" — both halves matter. Written
        // as bare `Not(HasSubtype("Human"))` this also matched every instant,
        // sorcery, artifact and land, so a Brimstone Volley could not target
        // the creature at all.
        let filter = CreatureFilter::And(vec![
            CreatureFilter::HasCardType(CardType::Creature),
            CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into()))),
        ]);
        for id in state.creatures_controlled_snapshot(controller, registry) {
            state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantProtection {
                target: id, filter: filter.clone(),
            });
        }

        state.log(crate::state::LogLevel::Event,
            "Spare from Evil: creatures gain protection from non-Human creatures until end of turn".into());

    }
}
