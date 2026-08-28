use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Avacynian Priest — {1}{W} 1/2 Human Cleric. {1}, {T}: Tap target non-Human creature.
pub struct AvacynianPriest;

impl CardBehavior for AvacynianPriest {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Avacynian Priest".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "{1}, {T}: Tap target non-Human creature.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}, {T}: Tap target non-Human creature".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(1),
                ]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::Creature),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    /// Filter targets to exclude Human creatures.
    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .is_some_and(|o| {
                        let is_human = state.has_subtype(o.id, "Human", registry);
                        o.zone == Zone::Battlefield
                            && state.is_creature(o.id, registry)
                            && !is_human
                    })
            }
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        for target in targets {
            if let Target::Object(target_id) = target {
                state.tap(*target_id);
            }
        }
    }
}
