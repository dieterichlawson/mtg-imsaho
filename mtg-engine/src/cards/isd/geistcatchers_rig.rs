use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, CardType, Keyword};

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
            subtypes: vec!["Construct".into()],
            power: Some(4),
            toughness: Some(5),
            oracle_text: "When this creature enters, you may have it deal 4 damage to target creature with flying.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "deal 4 damage to target creature with flying".into(),
                    // CR 603.3d: declaring the requirement makes the engine
                    // lock the target as the trigger goes on the stack, which
                    // is also where hexproof and protection are filtered out.
                    target_requirement: Some(crate::cards::TargetRequirement::CreatureWithFilter(
                        crate::cards::TargetFilter::HasKeyword(Keyword::Flying),
                    )),
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // CR 603.3d: the target was chosen when the trigger went on the stack
        // and its legality re-checked before resolution. Only the "you may"
        // decision is left — offer the locked target, never a fresh pick.
        let Some(target) = chosen_targets.first().cloned() else { return };
        crate::cards::helpers::present_optional_target_choice(
            state, object_id, controller, vec![target],
            PendingEffect::DealDamage {
                amount: 4,
                source_id: object_id,
            },
            "Geistcatcher's Rig: you may deal 4 damage to the targeted creature",
            registry,
        );
    }
}
