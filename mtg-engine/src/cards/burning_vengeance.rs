use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Burning Vengeance — {2}{R} enchantment.
/// Whenever you cast a spell from your graveyard, Burning Vengeance deals 2 damage
/// to any target.
pub struct BurningVengeance;

impl CardBehavior for BurningVengeance {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Burning Vengeance".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Whenever you cast a spell from your graveyard, Burning Vengeance deals 2 damage to any target.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SpellCast,
                    description: "deal 2 damage to any target".into(),
                },
            ],
        }
    }

    fn on_spell_cast(&self, state: &mut GameState, self_id: ObjectId, caster: PlayerId, spell_id: ObjectId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Only trigger on our own spells.
        if caster != controller {
            return;
        }
        // Only trigger on spells cast from graveyard (flashback).
        let cast_from_gy = state.get_object(spell_id)
            .map(|o| o.cast_with_flashback)
            .unwrap_or(false);
        if !cast_from_gy {
            return;
        }

        // Deal 2 damage to opponent (auto-target in simplified engine).
        let opponent = state.opponent(controller);
        let old_life = state.get_player(opponent).life;
        let new_life = old_life - 2;
        state.get_player_mut(opponent).life = new_life;
        state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
            source: self_id,
            target: crate::events::DamageTarget::Player(opponent),
            amount: 2,
        });
        state.events.push(crate::events::GameEvent::LifeChanged {
            player: opponent,
            old: old_life,
            new_life,
        });
        state.log(crate::state::LogLevel::Event,
            format!("Burning Vengeance deals 2 damage to opponent (flashback spell cast)"));
    }
}
