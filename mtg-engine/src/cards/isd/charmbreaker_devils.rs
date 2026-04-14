use rand::seq::SliceRandom;

use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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
            supertypes: vec![],
            subtypes: vec!["Devil".into()],
            power: Some(4),
            toughness: Some(4),
            oracle_text: "At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.\nWhenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "return a random instant or sorcery from graveyard to hand".into(),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::SpellCast,
                    description: "Charmbreaker Devils gets +4/+0 until end of turn".into(),
                },
            ],
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        if state.active_player != controller {
            return;
        }
        // Find instant or sorcery cards in graveyard.
        let mut candidates: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .is_some_and(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Instant | CardType::Sorcery)))
            })
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

    fn on_spell_cast(&self, state: &mut GameState, self_id: ObjectId, caster: PlayerId, spell_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Only trigger on your own spells.
        if caster != controller {
            return;
        }
        // Only trigger on instant or sorcery spells.
        let is_instant_or_sorcery = state.get_object(spell_id)
            .and_then(|o| registry.card_data(o.card_id))
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
