//! The dealt-anew oracle changes only what a player cannot see.
//!
//! `mtg_player::hidden::hands_dealt_anew` builds "the same game" for a
//! viewer by giving each object in another player's hand the next one's
//! card. The card is hidden (CR 402.3), but the object's history is not.
//! Each of its zone changes was public, and a public row can read the
//! count. For example, `damage_sources` drops a source that has changed
//! zones since it dealt the damage (CR 400.7, #1090). Before this, the
//! oracle moved the zone-change count along with the card. So a hand
//! object whose count happened to match a damage stamp brought a damage
//! record back, and the oracle reported a leak the engine did not make
//! (#1091, #1096).

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::ids::PlayerId;
use mtg_engine::types::Zone;
use mtg_player::hidden::{hands_dealt_anew, hidden_stays_hidden};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

#[test]
fn a_hand_object_keeps_its_own_zone_changes_when_its_card_is_dealt_anew() {
    let registry = CardRegistry::with_all_cards();
    let deck = Decklist { entries: vec![("Markov Patrician".into(), 20), ("Kindercatch".into(), 20)] };
    let config = GameConfig {
        player_names: vec!["a".into(), "b".into()],
        decklists: vec![deck.clone(), deck],
        starting_life: 20,
        starting_player: Some(P0),
        rng_seed: Some(1),
    };
    let mut state = engine::setup_game(&config, &registry);

    // p0's hand: the first object is a source that has changed zones twice
    // since it dealt damage (stamp 0). The next object's count is 0, so
    // dealing the next card into the source's slot must not hand it that 0.
    let hand: Vec<_> = state.objects_in_zone(Zone::Hand, P0).iter().map(|o| o.id).collect();
    assert!(hand.len() >= 2);
    let (source, next) = (hand[0], hand[1]);
    state.objects.get_mut(&source).unwrap().zone_change_count = 2;
    state.objects.get_mut(&next).unwrap().zone_change_count = 0;

    // A p1 creature on the battlefield, damaged by `source` before it moved.
    let victim = state.objects_in_zone(Zone::Hand, P1)[0].id;
    {
        let v = state.objects.get_mut(&victim).unwrap();
        v.zone = Zone::Battlefield;
        v.damage_marked = 3;
        v.damaged_by = vec![source];
        v.damaged_by_zone_changes = vec![0];
    }
    assert!(state.damage_sources(victim).is_empty(), "the source is a new object (CR 400.7)");

    let dealt = hands_dealt_anew(&state, P1);
    assert_eq!(dealt.get_object(source).unwrap().zone_change_count, 2,
        "the slot keeps its own public history; only the card is dealt anew");
    assert_eq!(dealt.get_object(source).unwrap().card_id, state.get_object(next).unwrap().card_id,
        "the card does move");
    assert!(dealt.damage_sources(victim).is_empty());

    let legal = engine::legal_actions(&state, &registry);
    let found = hidden_stays_hidden(&state, state.priority_player.unwrap_or(state.active_player), &legal, &registry);
    assert!(found.is_empty(), "{found:?}");
}

/// A trigger on the stack names its subject ("that creature") as it was
/// when the trigger went on the stack. Read live, a subject bounced while
/// the trigger waited was named through its object in a hidden hand, and
/// the stack row changed when that hand was dealt anew (#1095).
#[test]
fn a_waiting_triggers_row_names_its_subject_as_it_was_not_through_a_hidden_hand() {
    use mtg_engine::state::StackEntry;
    use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
    let registry = CardRegistry::with_all_cards();
    let deck = Decklist { entries: vec![("Markov Patrician".into(), 20), ("Kindercatch".into(), 20)] };
    let config = GameConfig {
        player_names: vec!["a".into(), "b".into()],
        decklists: vec![deck.clone(), deck],
        starting_life: 20,
        starting_player: Some(P0),
        rng_seed: Some(1),
    };
    let mut state = engine::setup_game(&config, &registry);
    let hand: Vec<_> = state.objects_in_zone(Zone::Hand, P0).iter().map(|o| o.id).collect();
    let name = |s: &mtg_engine::state::GameState, id| s.get_object(id).unwrap().name.clone();
    // The subject is followed in the hand by a different card, so dealing
    // the hand anew changes the card its object holds once it is back.
    let watcher = *hand.last().unwrap();
    let at = (0..hand.len() - 2).find(|&i| name(&state, hand[i]) != name(&state, hand[i + 1]))
        .expect("two different cards side by side");
    let subject = hand[at];

    // The subject: a p0 creature that dealt combat damage, watched by a
    // p0 permanent's "put a counter on that creature".
    state.move_object(subject, Zone::Battlefield, &registry);
    state.move_object(watcher, Zone::Battlefield, &registry);
    let subject_name = name(&state, subject);
    let card = state.get_object(watcher).unwrap().card_id;
    state.put_on_stack(StackEntry::Trigger(PendingTrigger::new(
        TriggerSource::new(watcher, card, P0, "put a +1/+1 counter on that creature"),
        TriggerEvent::AnyCombatDamageToPlayer { dealer: subject, damaged_player: P1, amount: 2 },
    )));

    // Bounced while the trigger waits.
    state.move_object(subject, Zone::Hand, &registry);
    let StackEntry::Trigger(t) = state.stack.last().unwrap() else { panic!("a trigger") };
    let row = t.description(&registry, Some(&state));
    assert!(row.contains(&format!("{subject_name} (#{})", subject.0)), "{row}");

    let dealt = hands_dealt_anew(&state, P1);
    assert_ne!(name(&dealt, subject), subject_name, "the hand object holds another card once dealt anew");
    let StackEntry::Trigger(t) = dealt.stack.last().unwrap() else { panic!("a trigger") };
    assert_eq!(t.description(&registry, Some(&dealt)), row, "the row is the same game's");

    let legal = engine::legal_actions(&state, &registry);
    let found = hidden_stays_hidden(&state, state.priority_player.unwrap_or(state.active_player), &legal, &registry);
    assert!(found.is_empty(), "{found:?}");
}

/// A token whose own ability defines its P/T (Gutter Grime's Ooze) reads
/// that ability from the card stamped when it was made, not through its
/// creator's id. With the creator bounced to a hidden hand, the Ooze's row
/// followed whatever card that hand object held (#1095).
#[test]
fn an_oozes_row_does_not_read_its_creator_through_a_hidden_hand() {
    let registry = CardRegistry::with_all_cards();
    let deck = Decklist { entries: vec![("Gutter Grime".into(), 20), ("Kindercatch".into(), 20)] };
    let config = GameConfig {
        player_names: vec!["a".into(), "b".into()],
        decklists: vec![deck.clone(), deck],
        starting_life: 20,
        starting_player: Some(P0),
        rng_seed: Some(1),
    };
    let mut state = engine::setup_game(&config, &registry);
    let hand: Vec<_> = state.objects_in_zone(Zone::Hand, P0).iter().map(|o| o.id).collect();
    let name = |s: &mtg_engine::state::GameState, id| s.get_object(id).unwrap().name.clone();
    let at = (0..hand.len() - 1).find(|&i| name(&state, hand[i]) == "Gutter Grime" && name(&state, hand[i + 1]) != "Gutter Grime")
        .expect("a Gutter Grime followed by another card");
    let grime = hand[at];

    state.move_object(grime, Zone::Battlefield, &registry);
    let ooze = state.create_token_with_defined_pt(grime, "", P0, vec![], vec![mtg_engine::types::CardType::Creature],
        vec![], vec!["Ooze".into()], &registry)[0];
    state.add_counters(ooze, mtg_engine::types::CounterType::PlusOnePlusOne, 1);
    state.move_object(grime, Zone::Hand, &registry);

    let row = |s: &mtg_engine::state::GameState| {
        let v = mtg_engine::view::GameView::for_player(s, P1, &registry);
        let p = v.battlefield.iter().find(|p| p.object_id == ooze).expect("the Ooze").clone();
        (p.oracle_text, p.star_pt, p.power, p.toughness)
    };
    let seen = row(&state);
    assert!(seen.0.contains("slime counters on Gutter Grime"), "{seen:?}");
    let dealt = hands_dealt_anew(&state, P1);
    assert_ne!(name(&dealt, grime), "Gutter Grime", "the hand object holds another card once dealt anew");
    assert_eq!(row(&dealt), seen, "the Ooze's row is the same game's");

    let legal = engine::legal_actions(&state, &registry);
    let found = hidden_stays_hidden(&state, state.priority_player.unwrap_or(state.active_player), &legal, &registry);
    assert!(found.is_empty(), "{found:?}");
}
