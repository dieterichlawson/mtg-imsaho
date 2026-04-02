use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TargetFilter};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Destroy target land. Into the Maw of Hell deals 13 damage to target creature.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
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
                let is_land = registry.card_data(obj.card_id)
                    .map(|d| d.card_types.contains(&CardType::Land))
                    .unwrap_or(false);
                let is_creature = obj.power.is_some();
                is_land || is_creature
            }
            _ => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // targets[0] = land, targets[1] = creature
        if let Some(Target::Object(land_id)) = targets.first() {
            if state.get_object(*land_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                let name = state.get_object(*land_id).map(|o| o.name.clone()).unwrap_or_default();
                crate::destruction::try_destroy(state, *land_id, registry);
                state.log(crate::state::LogLevel::Event,
                    format!("Into the Maw of Hell destroyed {}", name));
            }
        }
        if let Some(Target::Object(creature_id)) = targets.get(1) {
            if let Some(obj) = state.get_object_mut(*creature_id) {
                if obj.zone == Zone::Battlefield {
                    obj.damage_marked += 13;
                    obj.damaged_by.push(object_id);
                    let name = obj.name.clone();
                    state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                        source: object_id,
                        target: crate::events::DamageTarget::Object(*creature_id),
                        amount: 13,
                    });
                    state.log(crate::state::LogLevel::Event,
                        format!("Into the Maw of Hell dealt 13 damage to {}", name));
                }
            }
        }
        state.move_spell_after_resolve(object_id);
    }
}
