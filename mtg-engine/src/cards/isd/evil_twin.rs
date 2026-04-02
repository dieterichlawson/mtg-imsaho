use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::*;

/// Evil Twin {2}{U}{B} 0/0 Shapeshifter.
/// You may have Evil Twin enter the battlefield as a copy of any creature on the battlefield,
/// except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
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
            oracle_text: "You may have Evil Twin enter the battlefield as a copy of any creature on the battlefield, except it has \"{U}{B}, {T}: Destroy target creature with the same name as this creature.\"".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "you may copy a creature".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Collect all creatures on the battlefield except Evil Twin itself.
        let targets = crate::cards::helpers::creature_targets_except(state, object_id);

        // "You may" — present an optional choice. If no creatures exist or the
        // player declines, Evil Twin stays as a 0/0 and dies to SBA.
        if !targets.is_empty() {
            // Mark as Evil Twin so the destroy ability is available after copying.
            // This is set before the copy choice resolves so that card_state persists
            // regardless of which creature is copied.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.card_state.insert("is_evil_twin".into(), ObjectId(1));
            }

            crate::cards::helpers::present_optional_target_choice(
                state,
                object_id,
                controller,
                targets,
                PendingEffect::CopyCreature { source_id: object_id },
                "Evil Twin: you may choose a creature to copy",
            );
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
            target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::SameNameAsSource)),
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        // Basic validation: target must be a creature on the battlefield.
        // The SameNameAsSource filter in matches_ability_target_filter handles
        // the name-matching restriction for the activated ability.
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
        if let Some(Target::Object(target_id)) = targets.first() {
            let target_name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, *target_id, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Evil Twin: destroyed {} (same name)", target_name));
        }
    }
}
