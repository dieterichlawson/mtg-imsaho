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
