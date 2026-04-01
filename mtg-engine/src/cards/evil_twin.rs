use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TargetFilter, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Evil Twin {2}{U}{B} 0/0 Shapeshifter.
/// You may have Evil Twin enter the battlefield as a copy of any creature on the battlefield,
/// except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
///
/// Simplified: On ETB, copies the first creature an opponent controls (or any creature on the
/// battlefield). The destroy ability targets any creature with the same name.
pub struct EvilTwin;

impl CardBehavior for EvilTwin {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Evil Twin".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Shapeshifter".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "You may have Evil Twin enter as a copy of any creature on the battlefield, except it has \"{U}{B}, {T}: Destroy target creature with the same name as this creature.\"".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Find a creature to copy (prefer opponent's creatures).
        let target: Option<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != object_id)
            .max_by_key(|o| if o.controller != controller { 1 } else { 0 })
            .map(|o| o.id);

        if let Some(target_id) = target {
            let (name, power, toughness, card_id, subtypes) = match state.get_object(target_id) {
                Some(o) => (o.name.clone(), o.power, o.toughness, o.card_id, o.subtypes.clone()),
                None => return,
            };
            let (keywords, reg_subtypes) = registry.card_data(card_id)
                .map(|d| (d.keywords.clone(), d.subtypes.clone()))
                .unwrap_or_default();

            if let Some(obj) = state.get_object_mut(object_id) {
                obj.name = name.clone();
                obj.power = power;
                obj.toughness = toughness;
                obj.card_id = card_id;
                obj.keywords = keywords;
                // Merge subtypes.
                let mut all_subtypes = reg_subtypes;
                for s in subtypes {
                    if !all_subtypes.contains(&s) {
                        all_subtypes.push(s);
                    }
                }
                obj.subtypes = all_subtypes;
                // Mark as Evil Twin via card_state so we know it has the destroy ability.
                obj.card_state.insert("is_evil_twin".into(), ObjectId(1));
            }
            state.log(crate::state::LogLevel::Event,
                format!("Evil Twin enters as a copy of {}", name));
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        // Only show the destroy ability if this is an Evil Twin copy.
        if !obj.card_state.contains_key("is_evil_twin") {
            return vec![];
        }

        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{U}{B}, {T}: Destroy target creature with the same name".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Black),
            ]),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::Any)),
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        // Target must be a creature with the same name as this Evil Twin.
        // We need to find the Evil Twin on the battlefield to check the name.
        // Since is_valid_target doesn't get self_id, we accept any creature here
        // and check in on_activate_ability.
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .map(|o| o.zone == Zone::Battlefield && o.power.is_some())
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let my_name = state.get_object(object_id).map(|o| o.name.clone()).unwrap_or_default();

        if let Some(Target::Object(target_id)) = targets.first() {
            let target_name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
            if target_name == my_name {
                crate::destruction::try_destroy(state, *target_id, registry);
                state.log(crate::state::LogLevel::Event,
                    format!("Evil Twin: destroyed {} (same name)", target_name));
            } else {
                state.log(crate::state::LogLevel::Event,
                    format!("Evil Twin: {} doesn't share a name with {}", target_name, my_name));
            }
        }
    }
}
