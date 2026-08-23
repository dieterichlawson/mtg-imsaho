use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};
use crate::actions::Target;

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
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    // The trigger will be enumerated for ALL spell casts; the
                    // handler filters to flashback casts only by checking the
                    // spell's `cast_with_flashback` flag.
                    target_requirement: Some(TargetRequirement::AnyTarget),
                },
            ],
        }
    }

    // "Whenever you cast a spell from your graveyard" — both restrictions
    // are part of the trigger condition (CR 603.2) and gate dispatch.
    fn should_trigger_on_spell_cast(&self, state: &GameState, self_id: ObjectId, caster: PlayerId, spell_id: ObjectId, _registry: &CardRegistry) -> bool {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return false,
        };
        caster == controller
            && state.get_object(spell_id).is_some_and(|o| o.cast_with_flashback)
    }

    fn on_spell_cast(&self, state: &mut GameState, self_id: ObjectId, caster: PlayerId, spell_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
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
            .is_some_and(|o| o.cast_with_flashback);
        if !cast_from_gy {
            return;
        }
        // CR 603.3d: target was chosen when the trigger went on the stack.
        let Some(target) = chosen_targets.first() else { return };
        let effect = PendingEffect::DealDamage {
            amount: 2,
            source_id: self_id,
            source_name: "Burning Vengeance".into(),
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}
