use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Geistcatcher's Rig — {6} 4/5 Construct artifact creature.
/// When Geistcatcher's Rig enters the battlefield, you may have it deal 4 damage
/// to target creature with flying.
pub struct GeistcatchersRig;

impl CardBehavior for GeistcatchersRig {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Geistcatcher's Rig".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(6),
            ])),
            card_types: vec![CardType::Artifact, CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Construct".into()],
            power: Some(4),
            toughness: Some(5),
            oracle_text: "When this creature enters, you may have it deal 4 damage to target creature with flying.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "deal 4 damage to target creature with flying".into(),
                },
            ],
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Find all creatures with flying (any controller — can target any creature with flying).
        let targets: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != object_id)
            .filter(|o| state.has_keyword(o.id, Keyword::Flying, registry))
            .map(|o| Target::Object(o.id))
            .collect();

        if targets.is_empty() {
            // No flyers — "you may" ability, nothing to target.
        } else {
            // Always present choice since the ability is optional ("you may").
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Geistcatcher's Rig: deal 4 damage to target creature with flying".into(),
                    options: targets,
                    optional: true,
                    effect: PendingEffect::DealDamage { amount: 4, source_id: object_id, source_name: "Geistcatcher's Rig".into() },
                },
            });
        }
    }
}
