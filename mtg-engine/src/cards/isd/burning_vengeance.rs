use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Burning Vengeance — {2}{R} enchantment.
/// Whenever you cast a spell from your graveyard, this enchantment deals 2 damage
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
            oracle_text: "Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.".into(),
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

    fn on_spell_cast(&self, state: &mut GameState, self_id: ObjectId, caster: PlayerId, spell_id: ObjectId, registry: &CardRegistry) {
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

        // "Burning Vengeance deals 2 damage to any target" — present choice.
        let targets = crate::cards::helpers::any_targets(state);
        crate::cards::helpers::present_target_choice(
            state, self_id, controller, targets,
            crate::state::PendingEffect::DealDamage {
                amount: 2,
                source_id: self_id,
                source_name: "Burning Vengeance".into(),
            },
            "Burning Vengeance: deal 2 damage to any target",
            false,
        );
        state.log(crate::state::LogLevel::Event,
            format!("Burning Vengeance deals 2 damage to opponent (flashback spell cast)"));
    }
}
