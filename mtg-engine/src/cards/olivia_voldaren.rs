use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Olivia Voldaren — {2}{B}{R} 3/3 Legendary Vampire with Flying.
/// {1}{R}: Deal 1 damage to target creature. That creature becomes a Vampire in addition
/// to its other types. Put a +1/+1 counter on Olivia Voldaren.
/// {3}{B}{B}: Gain control of target Vampire.
pub struct OliviaVoldaren;

impl CardBehavior for OliviaVoldaren {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Olivia Voldaren".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Flying\n{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.\n{3}{B}{B}: Gain control of target Vampire.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        };

        let mut abilities = vec![];

        // Ability 0: {1}{R}: Deal 1 damage to another target creature.
        abilities.push(ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}{R}: Deal 1 damage to target creature, make it a Vampire, +1/+1 counter on Olivia".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::Any)),
            once_per_turn: false,
            sorcery_speed_only: false,
        });

        // Ability 1: {3}{B}{B}: Gain control of target Vampire.
        abilities.push(ActivatedAbilityDef {
            ability_index: 1,
            description: "{3}{B}{B}: Gain control of target Vampire".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::SubtypeOrCardType { subtypes: vec!["Vampire".into()], card_types: vec![] })),
            once_per_turn: false,
            sorcery_speed_only: false,
        });

        abilities
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        // Both abilities target creatures, but ability 1 only targets Vampires.
        // Since we can't distinguish which ability is being activated in is_valid_target,
        // we accept any creature here. The on_activate_ability checks Vampire for ability 1.
        match target {
            Target::Object(id) => {
                let obj = state.get_object(*id);
                obj.map(|o| o.zone == Zone::Battlefield && o.power.is_some()).unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        match ability_index {
            0 => {
                // {1}{R}: Deal 1 damage to ANOTHER target creature. Make it a Vampire. +1/+1 counter on Olivia.
                if let Some(Target::Object(target_id)) = targets.first() {
                    if *target_id == object_id { return; } // "another" — can't target self
                    if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                        // Deal 1 damage.
                        state.mark_damage_on_creature(*target_id, 1, object_id);
                        // Add Vampire subtype if not already present.
                        if let Some(obj) = state.get_object_mut(*target_id) {
                            if !obj.subtypes.contains(&"Vampire".to_string()) {
                                obj.subtypes.push("Vampire".to_string());
                            }
                        }
                        state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                            source: object_id,
                            target: crate::events::DamageTarget::Object(*target_id),
                            amount: 1,
                        });
                        // +1/+1 counter on Olivia.
                        state.add_counters(object_id, CounterType::PlusOnePlusOne, 1);
                        let target_name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
                        state.log(crate::state::LogLevel::Event,
                            format!("Olivia Voldaren deals 1 damage to {}, makes it a Vampire, and gets a +1/+1 counter", target_name));
                    }
                }
            }
            1 => {
                // {3}{B}{B}: Gain control of target Vampire.
                if let Some(Target::Object(target_id)) = targets.first() {
                    let is_vampire = state.get_object(*target_id)
                        .map(|o| o.zone == Zone::Battlefield && o.subtypes.contains(&"Vampire".to_string()))
                        .unwrap_or(false);
                    if is_vampire {
                        // Change controller permanently (well, as long as Olivia is on the battlefield).
                        if let Some(obj) = state.get_object_mut(*target_id) {
                            let target_name = obj.name.clone();
                            obj.controller = controller;
                            state.log(crate::state::LogLevel::Event,
                                format!("Olivia Voldaren gains control of {}", target_name));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
