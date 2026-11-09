//! What a player is shown of what they cannot see: nothing. A library's
//! order is hidden from everyone (CR 401.2), and a hand from all but its
//! owner (402.3). So a player's view must be the same however each library
//! is ordered, and whatever cards the other player holds or will draw. Each
//! test changes only what the viewer cannot see, and compares the views
//! whole, as every seat is sent them.

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use mtg_engine::view::GameView;

/// Each player's hand and library, by name, on a board with a creature
/// each: what is hidden is all that varies between two calls.
fn board(registry: &CardRegistry, hands: [&[&str]; 2], libraries: [&[&str]; 2]) -> GameState {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for p in [P0, P1] {
        named_permanent(&mut state, registry, "Grizzly Bears", p);
    }
    for p in [P0, P1] {
        for name in hands[p.0 as usize] {
            spell_in_hand(&mut state, registry, name, p);
        }
        for name in libraries[p.0 as usize] {
            stock_library_with(&mut state, registry, p, name, 1);
        }
    }
    state
}

fn view(state: &GameState, registry: &CardRegistry, player: PlayerId) -> serde_json::Value {
    serde_json::to_value(GameView::for_player(state, player, registry)).expect("a view serializes")
}

const HAND: [&str; 3] = ["Lightning Bolt", "Think Twice", "Island"];
const LIBRARY: [&str; 4] = ["Moon Heron", "Forest", "Brimstone Volley", "Swamp"];

/// Every library reordered, each player's view unchanged: not the other's
/// library, which it shows only by size, and not its own, whose cards it
/// lists, but not in the order they will be drawn (CR 401.2).
#[test]
fn no_view_shows_the_order_of_a_library() {
    let registry = registry();
    let state = board(&registry, [&HAND, &HAND], [&LIBRARY, &LIBRARY]);
    for viewer in [P0, P1] {
        let seen = view(&state, &registry, viewer);
        for whose in [P0, P1] {
            for turn in [|l: &mut Vec<ObjectId>| l.reverse(), |l: &mut Vec<ObjectId>| l.rotate_left(1)] {
                let mut reordered = state.clone();
                turn(&mut reordered.get_player_mut(whose).library_order);
                assert_eq!(view(&reordered, &registry, viewer), seen,
                    "p{}'s view changed when p{}'s library was reordered", viewer.0, whose.0);
            }
        }
    }
}

/// The other player's hand and library held other cards, each player's
/// view unchanged: it shows the other's hand and library by size alone
/// (CR 402.3), double-faced cards among them (Delver of Secrets, Daybreak
/// Ranger), whose other faces must not show either.
#[test]
fn no_view_shows_the_cards_the_other_player_holds_or_will_draw() {
    let registry = registry();
    let others_hand: [&str; 3] = ["Delver of Secrets", "Daybreak Ranger", "Forest"];
    let others_library: [&str; 4] = ["Daybreak Ranger", "Delver of Secrets", "Plains", "Mountain"];
    for viewer in [P0, P1] {
        let other = PlayerId(1 - viewer.0);
        let mut hands: [&[&str]; 2] = [&HAND, &HAND];
        let mut libraries: [&[&str]; 2] = [&LIBRARY, &LIBRARY];
        let seen = view(&board(&registry, hands, libraries), &registry, viewer);
        hands[other.0 as usize] = &others_hand;
        libraries[other.0 as usize] = &others_library;
        assert_eq!(view(&board(&registry, hands, libraries), &registry, viewer), seen,
            "p{}'s view changed with p{}'s hand and library", viewer.0, other.0);
    }
}

/// The same two boards played on, every player passing, through the other
/// player's untap, upkeep and draw into their main phase: what each drew
/// is theirs alone, and the views, their logs among them, must still be
/// the same.
#[test]
fn no_view_shows_what_the_other_player_draws() {
    let registry = registry();
    let others_hand: [&str; 3] = ["Delver of Secrets", "Daybreak Ranger", "Forest"];
    let others_library: [&str; 4] = ["Daybreak Ranger", "Delver of Secrets", "Plains", "Mountain"];
    let played_on = |hands: [&[&str]; 2], libraries: [&[&str]; 2]| {
        use mtg_engine::actions::{Action, CombatPrompt};
        let mut state = board(&registry, hands, libraries);
        let mut decisions = 0;
        mtg_engine::engine::run_game_loop(&mut state, &registry, |s: &GameState, _: PlayerId, legal: &mtg_engine::engine::LegalActions| {
            decisions += 1;
            match &legal.combat_prompt {
                _ if decisions > 100 || (s.turn_number > 1 && s.step == Step::PrecombatMain) => Action::AbandonGame,
                Some(CombatPrompt::ChooseAttackers { .. }) => Action::DeclareAttackers { attackers: Vec::new(), planeswalker_attacks: Vec::new() },
                Some(CombatPrompt::ChooseBlockers { .. }) => Action::DeclareBlockers { assignments: Vec::new() },
                None => Action::PassPriority,
            }
        });
        assert_eq!((state.turn_number, state.step), (2, Step::PrecombatMain), "played into the next turn's main phase");
        state
    };
    for viewer in [P0, P1] {
        let other = PlayerId(1 - viewer.0);
        let mut hands: [&[&str]; 2] = [&HAND, &HAND];
        let mut libraries: [&[&str]; 2] = [&LIBRARY, &LIBRARY];
        let seen = view(&played_on(hands, libraries), &registry, viewer);
        hands[other.0 as usize] = &others_hand;
        libraries[other.0 as usize] = &others_library;
        assert_eq!(view(&played_on(hands, libraries), &registry, viewer), seen,
            "p{}'s view changed with what p{} held and drew", viewer.0, other.0);
    }
}

/// The other player asked a question about cards only they may see: which
/// card of their hand to discard (CR 402.3), what to do with the top card
/// of their library they looked at (Delver of Secrets: "look at", not
/// "reveal", CR 701.16), and which card of their library a search finds
/// (701.19). The viewer's view must be the same whatever those cards are,
/// and the asked player's must name them.
#[test]
fn no_view_names_the_cards_of_a_question_put_to_the_other_player() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    let registry = registry();
    let others_hand: [&str; 3] = ["Delver of Secrets", "Daybreak Ranger", "Forest"];
    let others_library: [&str; 4] = ["Daybreak Ranger", "Delver of Secrets", "Plains", "Mountain"];
    for viewer in [P0, P1] {
        let other = PlayerId(1 - viewer.0);
        let asking = |state: &GameState, kind: usize| {
            let mut state = state.clone();
            let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, other).iter().map(|o| o.id).collect();
            let library = state.get_player(other).library_order.clone();
            let source = state.objects_in_zone(Zone::Battlefield, other)[0].id;
            let choice = match kind {
                0 => ResolutionChoiceKind::ChooseCardFromHand {
                    description: "choose a card to discard".into(), player: other, cards: hand,
                    discard_immediately: true, remaining: 1,
                },
                1 => ResolutionChoiceKind::ChooseFromLookedAt { description: "the top card".into(), looked_at: vec![library[0]] },
                _ => ResolutionChoiceKind::ChooseFromLibrary {
                    description: "search".into(), options: library, searcher: other, source_id: source,
                    destination: Zone::Hand, tapped: false,
                },
            };
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice { player: other, source, choice });
            state
        };
        let mut hands: [&[&str]; 2] = [&HAND, &HAND];
        let mut libraries: [&[&str]; 2] = [&LIBRARY, &LIBRARY];
        let before = board(&registry, hands, libraries);
        hands[other.0 as usize] = &others_hand;
        libraries[other.0 as usize] = &others_library;
        let after = board(&registry, hands, libraries);
        for (kind, what) in ["a discard", "a look at the top card", "a search"].iter().enumerate() {
            assert_eq!(view(&asking(&after, kind), &registry, viewer), view(&asking(&before, kind), &registry, viewer),
                "p{}'s view named the cards of {what} put to p{}", viewer.0, other.0);
            assert!(!GameView::for_player(&asking(&after, kind), other, &registry).revealed_names.is_empty(),
                "p{}, asked {what}, is told the names of what they choose among", other.0);
        }
    }
}
