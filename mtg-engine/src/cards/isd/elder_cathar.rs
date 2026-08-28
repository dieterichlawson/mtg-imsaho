use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};

/// Elder Cathar — {2}{W} 2/2 Human Soldier.
/// When Elder Cathar dies, put a +1/+1 counter on target creature you control.
/// If that creature is a Human, put two +1/+1 counters on it instead.
pub struct ElderCathar;

impl CardBehavior for ElderCathar {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Elder Cathar".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "put +1/+1 counters on target creature you control".into(),
                    // CR 603.3b: the target is chosen when the trigger goes on
                    // the stack, so the engine picks it — not `on_dies`.
                    target_requirement: Some(TargetRequirement::Creature),
                },
            ],
            ..Default::default()
        }
    }

    /// CR 603.3b: the target was chosen when the trigger went on the stack, so
    /// it arrives in `chosen_targets`. CR 603.3c already removed the trigger if
    /// there were no legal targets, and CR 608.2b re-checked legality on the
    /// way down, neither of which happened while this selected its own target
    /// at resolution.
    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        let Some(target) = chosen_targets.first() else { return };
        self.apply_counters(state, target, registry);
        let _ = object_id;
    }

    /// "target creature you control" — restrict the engine's creature
    /// enumeration to this card's controller.
    ///
    /// `caster` is the trigger's controller, which for a death trigger is the
    /// Cathar's *last known* controller (CR 608.2g) — leaving the battlefield
    /// resets the object's own `controller` to its owner (CR 400.7), so this
    /// must not be re-derived from the source here.
    ///
    /// There is no self-exclusion clause because none is needed: the Cathar is
    /// already in the graveyard when its own death trigger picks targets, so
    /// the zone check covers it.
    fn is_valid_target(&self, state: &GameState, caster: crate::ids::PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        let Target::Object(id) = target else { return false };
        state.get_object(*id).is_some_and(|o| {
            o.zone == Zone::Battlefield
                && o.controller == caster
                && state.is_creature(o.id, registry)
        })
    }

}

impl ElderCathar {
    /// "...put a +1/+1 counter on target creature you control. If that
    /// creature is a Human, put two +1/+1 counters on it instead." The Human
    /// check is this card's rule, so it lives here rather than as a flag on a
    /// shared engine effect.
    fn apply_counters(&self, state: &mut GameState, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let count = if state.has_subtype(*id, "Human", registry) { 2 } else { 1 };
        state.add_counters(*id, CounterType::PlusOnePlusOne, count);
        state.log(LogLevel::Event,
            format!("Elder Cathar's death granted {} +1/+1 counter{}",
                count, if count > 1 { "s" } else { "" }));
    }
}
