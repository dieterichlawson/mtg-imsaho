use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Mikaeus, the Lunarch {X}{W} 0/0 Legendary Human Cleric.
/// Mikaeus enters the battlefield with X +1/+1 counters on it.
/// {T}: Put a +1/+1 counter on Mikaeus.
/// {T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature
/// you control.
pub struct MikaeusTheLunarch;

impl CardBehavior for MikaeusTheLunarch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mikaeus, the Lunarch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::X,
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Mikaeus enters with X +1/+1 counters on it.\n{T}: Put a +1/+1 counter on Mikaeus.\n{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        // Move to battlefield.
        state.move_object(object_id, Zone::Battlefield);
        // Enter with X +1/+1 counters.
        let x = state.get_object(object_id).and_then(|o| o.x_value).unwrap_or(0);
        if x > 0 {
            state.add_counters(object_id, CounterType::PlusOnePlusOne, x);
            state.log(crate::state::LogLevel::Event,
                format!("Mikaeus, the Lunarch enters with {} +1/+1 counters", x));
        }
        // Set legendary flag.
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_legendary = true;
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        };

        let mut abilities = vec![];

        // Ability 0: {T}: Put a +1/+1 counter on Mikaeus.
        abilities.push(ActivatedAbilityDef {
            ability_index: 0,
            description: "{T}: Put a +1/+1 counter on Mikaeus".into(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
        });

        // Ability 1: {T}, Remove a +1/+1 counter: Put a +1/+1 counter on each other creature.
        let has_counter = state.get_counter_count(object_id, CounterType::PlusOnePlusOne) > 0;
        if has_counter {
            abilities.push(ActivatedAbilityDef {
                ability_index: 1,
                description: "{T}, Remove a +1/+1 counter: +1/+1 counter on each other creature you control".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
            });
        }

        abilities
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        match ability_index {
            0 => {
                // Put a +1/+1 counter on Mikaeus.
                state.add_counters(object_id, CounterType::PlusOnePlusOne, 1);
                state.log(crate::state::LogLevel::Event,
                    "Mikaeus, the Lunarch: +1/+1 counter added".into());
            }
            1 => {
                // Remove a +1/+1 counter from Mikaeus.
                if let Some(obj) = state.get_object_mut(object_id) {
                    let count = obj.counters.entry(CounterType::PlusOnePlusOne).or_insert(0);
                    if *count > 0 {
                        *count -= 1;
                    }
                }
                // Put a +1/+1 counter on each other creature you control.
                let other_creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
                    .iter()
                    .filter(|o| o.id != object_id && o.power.is_some())
                    .map(|o| o.id)
                    .collect();
                for cid in &other_creatures {
                    state.add_counters(*cid, CounterType::PlusOnePlusOne, 1);
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Mikaeus, the Lunarch: +1/+1 counter on {} other creatures", other_creatures.len()));
            }
            _ => {}
        }
    }
}
