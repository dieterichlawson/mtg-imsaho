use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};
use crate::actions::Target;

/// Moldgraf Monstrosity {4}{G}{G}{G} 8/8 Insect with Trample.
/// When Moldgraf Monstrosity dies, exile it, then return two creature cards at random
/// from your graveyard to the battlefield.
pub struct MoldgrafMonstrosity;

impl CardBehavior for MoldgrafMonstrosity {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moldgraf Monstrosity".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Insect".into()],
            power: Some(8),
            toughness: Some(8),
            oracle_text: "Trample\nWhen this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.".into(),
            keywords: vec![Keyword::Trample],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "exile, return two random creatures from graveyard".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 603.10c: "your" means last-known controller, not owner.
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // "Exile it" applies to the card in the graveyard, and only there.
        // Two Monstrosities dying together each put a trigger on the stack;
        // the first can return the second to the battlefield, and the second
        // trigger must then leave it alone rather than exiling a live
        // creature. Same if something else exiled the card first.
        //
        // The return happens either way — a trigger that can't do the first
        // part still does as much as it can (CR 608.2).
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Graveyard) {
            state.move_object(object_id, Zone::Exile, registry);
            state.log(crate::state::LogLevel::Event,
                "Moldgraf Monstrosity: exiled on death".into());
        }

        // Find creature cards in the graveyard (excluding the Monstrosity itself, which is now exiled).
        let creatures_in_gy: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            // "return two creature **cards** at random from your graveyard" —
            // CR 109.1, so a token waiting on the next SBA check is not one.
            .filter(|o| state.is_card(o.id) && state.is_creature(o.id, registry)
                && o.id != object_id)
            .map(|o| o.id)
            .collect();

        // "return two creature cards AT RANDOM".
        let to_return = crate::cards::helpers::choose_at_random(&creatures_in_gy, 2);
        for cid in &to_return {
            let name = state.get_object(*cid).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object_under_control(*cid, Zone::Battlefield, controller, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Moldgraf Monstrosity: {name} returned to the battlefield"));
        }
    }
}
