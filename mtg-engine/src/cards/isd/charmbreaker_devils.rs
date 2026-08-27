use rand::seq::SliceRandom;

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};
use crate::actions::Target;

/// Charmbreaker Devils — {5}{R} 4/4 Devil.
/// At the beginning of your upkeep, return an instant or sorcery card at random
/// from your graveyard to your hand.
/// Whenever you cast an instant or sorcery spell, Charmbreaker Devils gets +4/+0
/// until end of turn.
pub struct CharmbreakerDevils;

impl CardBehavior for CharmbreakerDevils {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Charmbreaker Devils".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Devil".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.\nWhenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "return a random instant or sorcery from graveyard to hand".into(),
                target_requirement: None,
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::SpellCast,
                    description: "Charmbreaker Devils gets +4/+0 until end of turn".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::Your,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        // `step_trigger_scope` already gates this to the controller's own
        // step; re-deriving it here is duplication, not defence.
        // Find instant or sorcery cards in graveyard.
        let mut candidates: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            // "an instant or sorcery **card**" — CR 109.1. `face_data` is
            // already None for a token, but relying on that leaves the rule
            // unsaid.
            .filter(|o| state.is_card(o.id) && state.face_data(o.id, registry)
                .is_some_and(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Instant | CardType::Sorcery))))
            .map(|o| o.id)
            .collect();
        if !candidates.is_empty() {
            let mut rng = rand::thread_rng();
            candidates.shuffle(&mut rng);
            let chosen = candidates[0];
            let name = state.get_object(chosen).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(chosen, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Charmbreaker Devils: returned {name} to hand"));
        }
    }

    // "Whenever you cast an instant or sorcery spell" — both restrictions
    // are part of the trigger condition (CR 603.2) and gate dispatch.
    fn should_trigger_on_spell_cast(&self, state: &GameState, self_id: ObjectId, caster: PlayerId, spell_id: ObjectId, registry: &CardRegistry) -> bool {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return false,
        };
        caster == controller
            && (state.has_card_type(spell_id, CardType::Instant, registry)
                || state.has_card_type(spell_id, CardType::Sorcery, registry))
    }

    fn on_spell_cast(&self, state: &mut GameState, self_id: ObjectId, caster: PlayerId, spell_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 113.7a: the trigger resolves even if the Devils are destroyed in
        // response, the same way their upkeep trigger already did.
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        // Only trigger on your own spells.
        if caster != controller {
            return;
        }
        // Only trigger on instant or sorcery spells.
        let is_instant_or_sorcery = state.get_object(spell_id)
            .and_then(|o| state.face_data(o.id, registry))
            .is_some_and(|d| d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery));
        if !is_instant_or_sorcery {
            return;
        }
        // +4/+0 until end of turn.
        state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
            target: self_id,
            power_mod: 4,
            toughness_mod: 0,
        });
        state.log(crate::state::LogLevel::Event,
            "Charmbreaker Devils: +4/+0 until end of turn".into());
    }
}
