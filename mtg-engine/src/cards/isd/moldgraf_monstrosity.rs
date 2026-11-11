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
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        // "Exile it" applies to the card in the graveyard, and only there.
        // Two Monstrosities dying together each put a trigger on the stack;
        // the first can return the second to the battlefield, and the second
        // trigger must then leave it alone rather than exiling a live
        // creature. Same if something else exiled the card first.
        //
        // The return happens either way — a trigger that can't do the first
        // part still does as much as it can (CR 608.2).
        // A Monstrosity that has died, come back and died again since is a
        // new object too, and not the one this "it" names (CR 400.7).
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Graveyard) && !state.is_new_source(object_id) {
            state.move_object(object_id, Zone::Exile, registry);
            state.log(crate::state::LogLevel::Event,
                "Moldgraf Monstrosity: exiled on death".into());
        }

        // Find creature cards in the graveyard (excluding the Monstrosity
        // itself, which is now exiled), in the graveyard's order, by a loop
        // rather than a filter collected (Bitterheart Witch's search says why).
        let mut creatures_in_gy: Vec<ObjectId> = Vec::new();
        for o in state.objects_in_zone(Zone::Graveyard, controller) {
            // "return two creature **cards** at random from your graveyard" —
            // CR 109.1, so a token waiting on the next SBA check is not one.
            if state.is_card(o.id) && state.is_creature(o.id, registry) && o.id != object_id {
                creatures_in_gy.push(o.id);
            }
        }

        // "return two creature cards AT RANDOM".
        let to_return = state.choose_at_random(&creatures_in_gy, 2);
        // Both in one event (CR 614.12): what each enters as is decided by
        // what was there before either.
        let names: Vec<String> = to_return.iter().map(|&cid| state.get_object(cid).map(|o| o.name.clone()).unwrap_or_default()).collect();
        let moved = state.move_onto_battlefield_together(&to_return, controller, registry);
        for ((cid, name), moved) in to_return.iter().zip(names).zip(moved) {
            if !moved {
                // Its entry waits on what it enters as a copy of (CR
                // 614.12b), asked once the other has entered: it may copy
                // only a creature there before either (Evil Twin's ruling).
                let before: Vec<ObjectId> = state.all_objects_in_zone(Zone::Battlefield).iter()
                    .filter(|o| !to_return.contains(&o.id) && state.is_creature(o.id, registry))
                    .map(|o| o.id)
                    .collect();
                state.pending_entry_options.insert(*cid, before);
                continue;
            }
            state.log(crate::state::LogLevel::Event,
                format!("Moldgraf Monstrosity: {name} returned to the battlefield"));
        }
    }
}
