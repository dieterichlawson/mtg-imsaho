use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};
use crate::actions::Target;

/// Ghoulraiser — {1}{B}{B} 2/2 Zombie.
/// When this creature enters, return a Zombie card at random from your graveyard
/// to your hand.
pub struct Ghoulraiser;

impl CardBehavior for Ghoulraiser {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ghoulraiser".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Zombie".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "When this creature enters, return a Zombie card at random from your graveyard to your hand.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "return a random Zombie from graveyard to hand".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 113.7a: killing Ghoulraiser in response to its own enters
        // trigger does not counter the trigger, and CR 608.2g makes "your
        // graveyard" its last known controller's. This used to return instead,
        // so removal in response ate the card advantage as well as the body.
        //
        // A Ghoulraiser that died on the way is itself a Zombie card in that
        // graveyard by the time the trigger resolves, so it is one of the
        // candidates to be returned at random.
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Find Zombie cards in graveyard (not restricted to creatures).
        let zombies: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            // "a Zombie **card**" — CR 109.1, said rather than left to
            // `face_data` happening to be None for a token.
            .filter(|o| state.is_card(o.id) && state.face_data(o.id, registry)
                .is_some_and(|d| d.subtypes.iter().any(|s| s == "Zombie")))
            .map(|o| o.id)
            .collect();

        if !zombies.is_empty() {
            let Some(chosen) = crate::cards::helpers::choose_at_random(&zombies, 1).first().copied()
            else { return };
            let name = state.get_object(chosen).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(chosen, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Ghoulraiser returned {name} to hand"));
        }
    }
}
