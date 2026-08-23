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
            supertypes: vec![],
            subtypes: vec!["Demon".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Flying\nMorbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EndStep,
                    description: "if morbid, destroy target non-Demon creature".into(),
                    target_requirement: Some(TargetRequirement::CreatureWithFilter(
                        TargetFilter::NotSubtypes(vec!["Demon".into()])
                    )),
                },
            ],
        }
    }

    /// Filter to non-Demon creatures on the battlefield. Additionally enforce
    /// the morbid condition here: if no creature died this turn, no creature
    /// is a legal target, so the trigger is removed from the stack per CR 603.3c.
    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        if !state.creature_died_this_turn {
            return false;
        }
        let Target::Object(id) = target else { return false; };
        let Some(obj) = state.get_object(*id) else { return false; };
        if obj.zone != Zone::Battlefield || obj.power.is_none() {
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
        // Morbid — only destroy if a creature died this turn (CR 603.4 intervening-if,
        // also enforced at stack-time via is_valid_target).
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
