use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::*;

/// Curse of the Pierced Heart — {1}{R} Enchantment — Aura Curse.
/// Enchant player.
/// At the beginning of enchanted player's upkeep, Curse of the Pierced Heart
/// deals 1 damage to that player or a planeswalker that player controls.
pub struct CurseOfThePiercedHeart;

impl CardBehavior for CurseOfThePiercedHeart {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curse of the Pierced Heart".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into(), "Curse".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant player\nAt the beginning of enchanted player's upkeep, Curse of the Pierced Heart deals 1 damage to that player or a planeswalker that player controls.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "deal 1 damage to enchanted player or their planeswalker".into(),
                },
            ],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets);
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let cursed_player = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.attached_to_player,
            _ => return,
        };
        let cursed_player = match cursed_player {
            Some(p) => p,
            None => return,
        };
        // Only trigger on the enchanted player's upkeep.
        if state.active_player != cursed_player {
            return;
        }
        let controller = state.get_object(self_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Build targets: the cursed player + any planeswalkers they control.
        let mut targets: Vec<Target> = vec![Target::Player(cursed_player)];
        for obj in state.all_objects_in_zone(Zone::Battlefield) {
            if obj.controller == cursed_player && obj.card_types.contains(&CardType::Planeswalker) {
                targets.push(Target::Object(obj.id));
            }
        }

        let effect = PendingEffect::DealDamage {
            amount: 1,
            source_id: self_id,
            source_name: "Curse of the Pierced Heart".into(),
        };
        crate::cards::helpers::present_target_choice(
            state, self_id, controller, targets, effect,
            "Curse of the Pierced Heart: deal 1 damage to enchanted player or their planeswalker",
            false,
        );
    }
}
