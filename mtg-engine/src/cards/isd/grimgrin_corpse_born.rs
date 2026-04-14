use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Supertype, ContinuousEffect, EffectScope, Zone, CounterType};

/// Grimgrin, Corpse-Born {3}{U}{B} 5/5 Legendary Zombie Warrior.
/// Grimgrin enters tapped and doesn't untap during your untap step.
/// Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
/// Whenever Grimgrin attacks, destroy target creature defending player controls,
/// then put a +1/+1 counter on Grimgrin.
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
            oracle_text: "Grimgrin enters tapped and doesn't untap during your untap step.\nSacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.\nWhenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.".into(),
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
                    description: "destroy target creature defending player controls, then +1/+1 counter".into(),
                },
            ],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        // Enters tapped.
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.tapped = true;
            obj.is_legendary = true;
        }
        state.log(crate::state::LogLevel::Event,
            "Grimgrin, Corpse-Born enters the battlefield tapped".into());
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        }

        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Sacrifice another creature: Untap Grimgrin, +1/+1 counter".into(),
            cost: ManaCost::free(),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeAnotherCreature,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
        }]
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        // The engine already sacrificed another creature as part of paying the cost.
        // Now untap Grimgrin and add a +1/+1 counter.
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.tapped = false;
        }
        state.add_counters(object_id, CounterType::PlusOnePlusOne, 1);
        state.log(crate::state::LogLevel::Event,
            "Grimgrin: sacrificed creature, untapped, +1/+1 counter".into());
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // Get the defending player from combat state, falling back to opponent.
        let defender = state.combat.as_ref()
            .and_then(|c| c.attackers.get(&self_id).copied())
            .unwrap_or_else(|| state.opponent(controller));

        // Collect creatures the defending player controls as potential targets.
        // Filter out creatures with protection from Grimgrin's subtypes.
        let targets: Vec<Target> = state.objects_in_zone(Zone::Battlefield, defender)
            .iter()
            .filter(|o| o.power.is_some())
            .filter(|o| crate::engine::can_be_targeted_by(state, o.id, controller, Some(self_id), registry))
            .map(|o| Target::Object(o.id))
            .collect();

        // Per ruling: "If the defending player controls no creatures when Grimgrin attacks,
        // the last ability will be removed from the stack and have no effect."
        // This means no +1/+1 counter either.
        if targets.is_empty() {
            return;
        }

        // Present target choice to the controller. The effect destroys the target
        // and then adds a +1/+1 counter to Grimgrin.
        crate::cards::helpers::present_target_choice(
            state,
            self_id,
            controller,
            targets,
            PendingEffect::DestroyThenCounter {
                source_id: self_id,
                source_name: "Grimgrin, Corpse-Born".into(),
            },
            "Grimgrin, Corpse-Born: destroy target creature defending player controls",
            false,
        );
    }
}
