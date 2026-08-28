use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TargetFilter};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Into the Maw of Hell — {4}{R}{R} Sorcery.
/// Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
pub struct IntoTheMawOfHell;

impl CardBehavior for IntoTheMawOfHell {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Into the Maw of Hell".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Destroy target land. Into the Maw of Hell deals 13 damage to target creature.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::TwoTargets(
            Box::new(TargetRequirement::PermanentWithFilter(
                TargetFilter::HasCardType(vec![CardType::Land]),
            )),
            Box::new(TargetRequirement::Creature),
        )
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield => o,
                    _ => return false,
                };
                // Valid if it's a land or a creature.
                let is_land = state.face_data(obj.id, registry)
                    .is_some_and(|d| d.card_types.contains(&CardType::Land));
                let is_creature = state.is_creature(obj.id, registry);
                is_land || is_creature
            }
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // targets[0] = land, targets[1] = creature
        if let Some(Target::Object(land_id)) = targets.first() {
            if state.get_object(*land_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                crate::destruction::try_destroy_by(state, *land_id, "Into the Maw of Hell", registry);
            }
        }
        if let Some(Target::Object(creature_id)) = targets.get(1) {
            if state.get_object(*creature_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                let effect = crate::state::PendingEffect::DealDamage {
                    amount: 13,
                    source_id: object_id,
                    source_name: "Into the Maw of Hell".into(),
                };
                crate::engine::apply_pending_effect(
                    state,
                    &Target::Object(*creature_id),
                    &effect,
                    registry,
                );
            }
        }
    }
}
