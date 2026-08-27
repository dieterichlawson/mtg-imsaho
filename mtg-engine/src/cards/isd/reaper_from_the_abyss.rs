use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Reaper from the Abyss — {3}{B}{B}{B} 6/6 flying Demon.
/// Morbid — At the beginning of each end step, if a creature died this turn,
/// destroy target non-Demon creature.
pub struct ReaperFromTheAbyss;

impl CardBehavior for ReaperFromTheAbyss {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Reaper from the Abyss".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Demon".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Flying\nMorbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.".into(),
            keywords: vec![Keyword::Flying],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EndStep,
                    description: "if morbid, destroy target non-Demon creature".into(),
                    target_requirement: Some(TargetRequirement::CreatureWithFilter(
                        TargetFilter::NotSubtypes(vec!["Demon".into()])
                    )),
                },
            ],
            ..Default::default()
        }
    }

    /// Morbid is an intervening-if clause (CR 603.4): if no creature died this
    /// turn the ability does not trigger, so the condition belongs here, at
    /// dispatch time, and not in `is_valid_target`.
    ///
    /// It used to live there, on the reasoning that "no legal target" removes
    /// the trigger under CR 603.3c and reaches the same board state. It does —
    /// but 603.3c puts the ability on the stack first and then removes it, and
    /// the engine says so in the game log. A Reaper that sat through an end
    /// step with nothing dead reported "Trigger removed: no legal targets" for
    /// an ability that, by 603.4, never triggered at all.
    fn should_trigger(&self, state: &GameState, _self_id: ObjectId, kind: &TriggerKind, _registry: &CardRegistry) -> bool {
        match kind {
            TriggerKind::EndStep => state.creature_died_this_turn,
            _ => true,
        }
    }

    /// "target non-Demon **creature**" — a property of the target alone.
    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        let Target::Object(id) = target else { return false; };
        let Some(obj) = state.get_object(*id) else { return false; };
        // `is_creature` is the accessor for this: card types, plus the
        // object-level P/T sentinel that tokens and `*/*` creatures carry.
        // This used to inline half of it as `!state.is_creature(obj.id, registry)`.
        if obj.zone != Zone::Battlefield || !state.is_creature(*id, registry) {
            return false;
        }
        !state.has_subtype(obj.id, "Demon", registry)
    }

    fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 112.7a: the ability is on the stack independently of the Reaper,
        // so it resolves even if the Reaper has since been destroyed — the
        // creature being destroyed is a different permanent and the Reaper's
        // whereabouts are irrelevant to it. This used to return early here.
        let _ = self_id;
        // CR 603.4 checks the intervening-if a second time on resolution.
        if !state.creature_died_this_turn {
            return;
        }
        // CR 603.3d: target was chosen when the trigger went on the stack.
        let Some(target) = chosen_targets.first() else { return };
        let effect = PendingEffect::DestroyCreature {
            source_name: "Reaper from the Abyss".into(),
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}
