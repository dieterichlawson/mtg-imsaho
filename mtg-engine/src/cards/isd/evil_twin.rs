use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            oracle_text: "You may have this creature enter as a copy of any creature on the battlefield, except it has \"{U}{B}, {T}: Destroy target creature with the same name as this creature.\"".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "you may copy a creature".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    /// Marks this permanent as one that "enters as a copy" via a player
    /// choice (CR 614.1d). `move_object` reads this at entry to arm the
    /// transient SBA copy-guard (`entering_copy_source`) before any SBA runs;
    /// the guard is cleared when the copy choice concludes below / in the
    /// CopyCreature handler.
    fn enters_as_copy(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // "a copy of ANY CREATURE on the battlefield" — a choice, not a
        // target (CR 115.1, 614.12b), so hexproof and protection do not
        // restrict it. Using the targeting helper hid an opponent's hexproof
        // creature, which is a perfectly legal thing to copy.
        let targets = crate::cards::helpers::creature_choices_except(state, object_id, registry);

        // "You may" — present an optional choice. If no creatures exist or the
        // player declines, Evil Twin stays as a 0/0 and dies to SBA.
        if targets.is_empty() {
            // No choice will be presented — disarm the guard now so SBA can
            // clean up the 0/0.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.entering_copy_source = false;
            }
        } else {
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

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        // The granted ability exists only on a permanent that actually entered
        // as a copy (CR 706.2 "except it has ..."). An Evil Twin whose copy
        // choice was declined is a plain 0/0 with no ability. The engine
        // consults this behavior for a copy whose `copy_grantor` is this card,
        // so the presence of a grantor is the whole test.
        if obj.copy_grantor.is_none() {
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
                    .is_some_and(|o| o.zone == Zone::Battlefield && o.power.is_some())
            }
            Target::Player(_) => false,
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            let target_name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, *target_id, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Evil Twin: destroyed {target_name} (same name)"));
        }
    }
}
