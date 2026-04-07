use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Olivia Voldaren — {2}{B}{R} 3/3 Legendary Vampire with Flying.
/// {1}{R}: Deal 1 damage to another target creature. That creature becomes a Vampire in addition
/// to its other types. Put a +1/+1 counter on Olivia Voldaren.
/// {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
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
            oracle_text: "Flying\n{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.\n{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::LeavesBattlefield,
                    description: "return stolen creatures to their owners".into(),
                },
            ],
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
            target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::Another)),
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
            target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::HasSubtype("Vampire".into()))),
            once_per_turn: false,
            sorcery_speed_only: false,
        });

        abilities
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
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
                    if let Some(target_obj) = state.get_object(*target_id) {
                        if target_obj.zone == Zone::Battlefield {
                            // Deal 1 damage.
                            if let Some(obj) = state.get_object_mut(*target_id) {
                                obj.damage_marked += 1;
                                obj.damaged_by.push(object_id);
                                // Add Vampire subtype if not already present.
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
                            state.log(crate::state::LogLevel::Event,
                                format!("Olivia Voldaren deals 1 damage to {}, makes it a Vampire, and gets a +1/+1 counter", state.obj_name(*target_id)));
                        }
                    }
                }
            }
            1 => {
                // {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia.
                if let Some(Target::Object(target_id)) = targets.first() {
                    let is_vampire = state.get_object(*target_id)
                        .map(|o| o.zone == Zone::Battlefield && o.subtypes.contains(&"Vampire".to_string()))
                        .unwrap_or(false);
                    if is_vampire {
                        // Record the original controller so we can revert when Olivia leaves.
                        let original_controller = state.get_object(*target_id).map(|o| o.controller).unwrap_or(controller);
                        let stolen_name = state.obj_name(*target_id);
                        if let Some(obj) = state.get_object_mut(*target_id) {
                            obj.controller = controller;
                        }
                        state.log(crate::state::LogLevel::Event,
                            format!("Olivia Voldaren gains control of {}", stolen_name));
                        // Track the stolen creature in Olivia's card_state.
                        // We use "stolen_N" keys to track multiple stolen creatures,
                        // storing the original controller as an ObjectId (abusing the type for PlayerId).
                        let stolen_count = state.get_object(object_id)
                            .map(|o| o.card_state.keys().filter(|k| k.starts_with("stolen_")).count())
                            .unwrap_or(0);
                        let key = format!("stolen_{}", stolen_count);
                        if let Some(olivia) = state.get_object_mut(object_id) {
                            olivia.card_state.insert(key.clone(), *target_id);
                            // Also store original controller using "orig_N" key.
                            let orig_key = format!("orig_{}", stolen_count);
                            olivia.card_state.insert(orig_key, ObjectId(original_controller.0 as u64));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn on_leave_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        // When Olivia leaves the battlefield, return all stolen creatures to their original controllers.
        let stolen_entries: Vec<(ObjectId, ObjectId)> = {
            if let Some(olivia) = state.get_object(object_id) {
                let mut entries = vec![];
                let stolen_keys: Vec<String> = olivia.card_state.keys()
                    .filter(|k| k.starts_with("stolen_"))
                    .cloned()
                    .collect();
                for key in stolen_keys {
                    let idx = key.strip_prefix("stolen_").unwrap_or("0");
                    let orig_key = format!("orig_{}", idx);
                    if let (Some(&target_id), Some(&orig_controller_id)) =
                        (olivia.card_state.get(&key), olivia.card_state.get(&orig_key)) {
                        entries.push((target_id, orig_controller_id));
                    }
                }
                entries
            } else {
                vec![]
            }
        };

        for (target_id, orig_controller_id) in stolen_entries {
            let orig_controller = crate::ids::PlayerId(orig_controller_id.0 as u8);
            if state.get_object(target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                let returned_name = state.obj_name(target_id);
                if let Some(obj) = state.get_object_mut(target_id) {
                    obj.controller = orig_controller;
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Olivia Voldaren left: {} returned to p{}", returned_name, orig_controller.0));
            }
        }
    }
}
