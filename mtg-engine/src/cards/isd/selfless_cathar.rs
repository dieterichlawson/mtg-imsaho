use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, ActivatedAbilityDef, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Selfless Cathar — {W} 1/1 Human Cleric.
/// {1}{W}, Sacrifice this creature: Creatures you control get +1/+1 until end of turn.
pub struct SelflessCathar;

impl CardBehavior for SelflessCathar {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Selfless Cathar".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "{1}{W}, Sacrifice this creature: Creatures you control get +1/+1 until end of turn.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}{W}, Sacrifice: Creatures you control get +1/+1 until end of turn".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeThis,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        // The creature was already sacrificed by the engine before this is called.
        // Use the controller from the object (even though it's in the graveyard now).
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        // CR 611.2c: a continuous effect created by a resolving spell or
        // ability affects the set of objects that existed when it resolved, and
        // that set never changes. This is the line between Glorious Anthem (a
        // permanent's static ability, which picks up newcomers) and a pump
        // spell (which does not) — so the creatures are snapshotted here rather
        // than matched by a live filter every time P/T is computed.
        for id in state.creatures_controlled_snapshot(controller, registry) {
            state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
                target: id, power_mod: 1, toughness_mod: 1,
            });
        }
    }
}
