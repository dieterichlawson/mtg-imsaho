use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Grimgrin, Corpse-Born {3}{U}{B} 5/5 Legendary Zombie Warrior.
/// Grimgrin enters the battlefield tapped and doesn't untap during your untap step.
/// Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
/// Whenever Grimgrin attacks, destroy target creature defending player controls.
/// Grimgrin gets a +1/+1 counter.
pub struct GrimgrinCorpseBorn;

impl CardBehavior for GrimgrinCorpseBorn {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Grimgrin, Corpse-Born".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Zombie".into(), "Warrior".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "Grimgrin, Corpse-Born enters the battlefield tapped and doesn't untap during your untap step.\nSacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.\nWhenever Grimgrin attacks, destroy target creature defending player controls. Put a +1/+1 counter on Grimgrin.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::PreventUntap {
                    scope: EffectScope::OnSelf,
                },
            ],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "destroy target creature defending player controls, +1/+1 counter".into(),
                },
            ],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        // Enters tapped.
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.tapped = true;
            obj.is_legendary = true;
        }
        state.log(crate::state::LogLevel::Event,
            "Grimgrin, Corpse-Born enters the battlefield tapped".into());
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        };

        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Sacrifice another creature: Untap Grimgrin, +1/+1 counter".into(),
            cost: ManaCost::free(),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Sacrifice another creature (pick the first available that isn't Grimgrin).
        let sacrifice_target: Option<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
            .iter()
            .find(|o| o.id != object_id && o.power.is_some())
            .map(|o| o.id);

        if let Some(sac_id) = sacrifice_target {
            let sac_name = state.get_object(sac_id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::sacrifice(state, sac_id, registry);
            // Untap Grimgrin.
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.tapped = false;
            }
            // +1/+1 counter.
            state.add_counters(object_id, CounterType::PlusOnePlusOne, 1);
            state.log(crate::state::LogLevel::Event,
                format!("Grimgrin: sacrificed {}, untapped, +1/+1 counter", sac_name));
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // Destroy target creature defending player controls.
        let defender = state.opponent(controller);
        let target: Option<ObjectId> = state.objects_in_zone(Zone::Battlefield, defender)
            .iter()
            .find(|o| o.power.is_some())
            .map(|o| o.id);

        if let Some(target_id) = target {
            let target_name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, target_id, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Grimgrin attacks: destroys {}", target_name));
            // +1/+1 counter (only if target was valid — "then" means sequential resolution).
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
            state.log(crate::state::LogLevel::Event,
                "Grimgrin: +1/+1 counter from attack trigger".into());
        }
    }
}
