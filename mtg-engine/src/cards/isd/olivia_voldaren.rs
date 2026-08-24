use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Supertype, Keyword, Zone, CounterType};

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
                target_requirement: None,
                },
            ],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        }

        vec![
            // Ability 0: {1}{R}: Deal 1 damage to another target creature.
            ActivatedAbilityDef {
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
                counter_cost: None,
            },
            // Ability 1: {3}{B}{B}: Gain control of target Vampire.
            ActivatedAbilityDef {
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
                counter_cost: None,
            },
        ]
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = state.get_object(*id);
                obj.is_some_and(|o| o.zone == Zone::Battlefield && o.power.is_some())
            }
            Target::Player(_) => false,
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        match ability_index {
            0 => {
                if let Some(Target::Object(target_id)) = targets.first() {
                    if *target_id == object_id { return; }
                    let on_battlefield = state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield);
                    if !on_battlefield { return; }
                    if state.has_protection_from(*target_id, object_id, registry) { return; }
                    let effect = crate::state::PendingEffect::DealDamage {
                        amount: 1,
                        source_id: object_id,
                        source_name: "Olivia Voldaren".into(),
                    };
                    crate::engine::apply_pending_effect(
                        state,
                        &Target::Object(*target_id),
                        &effect,
                        registry,
                    );
                    if let Some(obj) = state.get_object_mut(*target_id) {
                        if !obj.subtypes.contains(&"Vampire".to_string()) {
                            obj.subtypes.push("Vampire".to_string());
                        }
                    }
                    state.add_counters(object_id, CounterType::PlusOnePlusOne, 1);
                    state.log(crate::state::LogLevel::Event,
                        format!("Olivia Voldaren deals 1 damage to {}, makes it a Vampire, and gets a +1/+1 counter", state.obj_name(*target_id)));
                }
            }
            1 => {
                // {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia.
                if let Some(Target::Object(target_id)) = targets.first() {
                    // Recognise printed Vampires (registry subtype) as well as
                    // creatures Olivia's other ability turned into Vampires
                    // (object-level subtype) — via the characteristics layer.
                    let is_vampire = state.get_object(*target_id)
                        .is_some_and(|o| o.zone == Zone::Battlefield)
                        && state.has_subtype(*target_id, "Vampire", registry);
                    if is_vampire {
                        // Record the original controller so we can revert when Olivia leaves.
                        let original_controller = state.get_object(*target_id).map_or(controller, |o| o.controller);
                        let stolen_name = state.obj_name(*target_id);
                        // Gaining control makes the stolen Vampire summoning-sick
                        // for its new controller (no haste granted here).
                        state.change_control(*target_id, controller);
                        state.log(crate::state::LogLevel::Event,
                            format!("Olivia Voldaren gains control of {stolen_name}"));
                        // Track the stolen creature in Olivia's card_state.
                        // We use "stolen_N" keys to track multiple stolen creatures,
                        // storing the original controller as an ObjectId (abusing the type for PlayerId).
                        let stolen_count = state.get_object(object_id)
                            .map_or(0, |o| o.card_state.keys().filter(|k| k.starts_with("stolen_")).count());
                        let key = format!("stolen_{stolen_count}");
                        if let Some(olivia) = state.get_object_mut(object_id) {
                            olivia.card_state.insert(key.clone(), *target_id);
                            // Also store original controller using "orig_N" key.
                            let orig_key = format!("orig_{stolen_count}");
                            olivia.card_state.insert(orig_key, ObjectId(u64::from(original_controller.0)));
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
                    let orig_key = format!("orig_{idx}");
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
            let orig_controller = crate::ids::PlayerId(u8::try_from(orig_controller_id.0).unwrap_or(u8::MAX));
            if state.get_object(target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                let returned_name = state.obj_name(target_id);
                // Control changes back — summoning-sick for the returnee too.
                state.change_control(target_id, orig_controller);
                state.log(crate::state::LogLevel::Event,
                    format!("Olivia Voldaren left: {} returned to p{}", returned_name, orig_controller.0));
            }
        }
    }
}
