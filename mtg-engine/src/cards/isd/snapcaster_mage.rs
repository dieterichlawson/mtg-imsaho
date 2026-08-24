use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};
use crate::actions::Target;

/// Snapcaster Mage — {1}{U} 2/1 Human Wizard. Flash.
/// When this creature enters, target instant or sorcery card in your graveyard
/// gains flashback until end of turn. The flashback cost is equal to its mana cost.
pub struct SnapcasterMage;

impl CardBehavior for SnapcasterMage {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Snapcaster Mage".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Flash\nWhen this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)".into(),
            keywords: vec![Keyword::Flash],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "grant flashback to instant or sorcery in graveyard".into(),
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::GraveyardCardOwnedByCaster),
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    /// Filter graveyard cards to only instants and sorceries that don't
    /// already have flashback granted by a prior Snapcaster trigger this turn.
    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        let Target::Object(id) = target else { return false; };
        let Some(obj) = state.get_object(*id) else { return false; };
        let is_instant_or_sorcery = state.face_data(obj.id, registry)
            .is_some_and(|d| d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery));
        if !is_instant_or_sorcery { return false; }
        // A card that already has a flashback grant is still a legal target:
        // "target instant or sorcery card in your graveyard" has no clause
        // excluding it, and CR 702.33 explicitly allows several instances of
        // flashback at once. Refusing meant a second Snapcaster found no legal
        // target and its trigger was removed under CR 603.3c.
        true
    }

    fn on_enter_battlefield(&self, state: &mut GameState, _object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 603.3d: target was chosen when the trigger went on the stack.
        let Some(Target::Object(target_id)) = chosen_targets.first() else { return };
        // CR 702.33a: a flashback cost is "equal to its mana cost". A card with NO
        // mana cost has no flashback cost and simply cannot be cast this way —
        // substituting a free cost made it castable for {0}.
        let Some(cost) = state.face_data(*target_id, registry).and_then(|d| d.cost.clone()) else { return };
        let Some(name) = state.get_object(*target_id).map(|o| o.name.clone()) else { return };
        state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantFlashback { target: *target_id, cost });
        state.log(crate::state::LogLevel::Event,
            format!("Snapcaster Mage grants flashback to {name}"));
    }
}
