use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Undead Alchemist — {3}{U} 4/2 Zombie.
/// If a Zombie you control would deal combat damage to a player, instead that
/// player mills that many cards. Whenever a creature card is put into an
/// opponent's graveyard from their library, exile that card and create a 2/2
/// black Zombie creature token.
///
/// Ability 1 is a replacement effect (`replace_event`): combat damage from a
/// Zombie you control becomes a mill of the same size.
///
/// Ability 2 is a real trigger, not something the replacement effect does on
/// the side. `mill_cards` emits `CreatureCardMilled` for every creature card
/// it puts into a graveyard, whoever milled and for whatever reason, and the
/// collector fires this card's `CreatureCardMilled` trigger for each watcher
/// whose controller is not the milled player — which is what "an **opponent's**
/// graveyard" means. So Curse of the Bloody Tome and Nephalia Drownyard feed
/// it exactly as combat damage does.
pub struct UndeadAlchemist;

impl CardBehavior for UndeadAlchemist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Undead Alchemist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Zombie".into()],
            power: Some(4),
            toughness: Some(2),
            oracle_text: "If a Zombie you control would deal combat damage to a player, instead that player mills that many cards.\nWhenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.".into(),
            triggered_abilities: vec![
                crate::cards::TriggeredAbilityDef {
                    kind: crate::cards::TriggerKind::CreatureCardMilled,
                    description: "exile milled creature, create Zombie token".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_creature_card_milled(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        milled_object: ObjectId,
        _milled_player: PlayerId,
        registry: &CardRegistry,
    ) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        let name = state.obj_name(milled_object);

        // "exile that card" means the card that was put into the graveyard.
        // CR 400.7: once it leaves that graveyard it is a new object and this
        // ability can no longer find it — so a card its owner rescued in
        // response (Ghoulcaller's Chant is in this very set) must not be
        // dragged out of their hand and exiled. Two Alchemists reach the same
        // place from the other direction: per the 2011-09-22 ruling, "the
        // first such ability to resolve will exile that creature card ...
        // subsequent abilities won't exile the creature card, but each will
        // create another Zombie token", and by then it is already in exile.
        //
        // The token is created either way. It is not conditional on the exile.
        let still_in_a_graveyard = state.get_object(milled_object)
            .is_some_and(|o| o.zone == Zone::Graveyard);
        if still_in_a_graveyard {
            state.move_object(milled_object, Zone::Exile, registry);
        }

        state.create_token_with_subtypes(
            "", controller, 2, 2,
            vec![Color::Black],
            vec![CardType::Creature],
            vec![],
            vec!["Zombie".into()],
            registry,
        );
        state.log(crate::state::LogLevel::Event, if still_in_a_graveyard {
            format!("Undead Alchemist: exiled {name} and created a Zombie token")
        } else {
            format!("Undead Alchemist: {name} was no longer in the graveyard to exile, created a Zombie token")
        });
    }

    fn replace_event(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        use crate::replacement::{ReplaceableEvent, Replacement};
        // "If a Zombie you control would deal combat damage to a player,
        // instead that player mills that many cards."
        let ReplaceableEvent::DealsDamage { source, target, amount, combat: true } = event
            else { return None };
        let crate::events::DamageTarget::Player(damaged_player) = target else { return None };
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return None,
        };
        if state.get_object(*source).map(|o| o.controller) != Some(controller)
            || !state.has_subtype(*source, "Zombie", registry)
        {
            return None;
        }

        // mill_cards emits CreatureCardMilled, which the trigger system picks
        // up to fire our on_creature_card_milled (exile + Zombie token).
        crate::engine::mill_cards(state, *damaged_player, *amount as usize, "Undead Alchemist", registry);
        Some(Replacement::Replaced)
    }
}
