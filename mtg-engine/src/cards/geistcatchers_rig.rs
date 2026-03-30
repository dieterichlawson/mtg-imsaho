use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
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
            oracle_text: "When Geistcatcher's Rig enters the battlefield, you may have it deal 4 damage to target creature with flying.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "deal 4 damage to target creature with flying".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let opponent = state.opponent(controller);
        // Find opponent's creatures with flying, pick the strongest (highest power).
        let mut flyers: Vec<(ObjectId, i32)> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == opponent && o.power.is_some())
            .filter(|o| state.has_keyword(o.id, Keyword::Flying, registry))
            .filter_map(|o| {
                state.effective_power(o.id, registry).map(|p| (o.id, p))
            })
            .collect();
        flyers.sort_by(|a, b| b.1.cmp(&a.1));
        if let Some((target_id, _)) = flyers.first() {
            if let Some(obj) = state.get_object_mut(*target_id) {
                obj.damage_marked += 4;
            }
            state.events.push(crate::events::GameEvent::CombatDamageDealt {
                source: object_id,
                target: crate::events::DamageTarget::Object(*target_id),
                amount: 4,
            });
            state.log(crate::state::LogLevel::Event, "Geistcatcher's Rig dealt 4 damage to a flyer".into());
        }
        // If no flyers, "may" ability — choose not to use it.
    }
}
