//! The log every seat is sent names no card no one has seen
//! (`invariants::shared_log`, CR 400.2): each test writes one line between
//! two decisions and asks the check what it says of it.

mod common;
use common::*;
use mtg_engine::actions::Action;
use mtg_engine::engine;
use mtg_engine::invariants::shared_log::{ids_named, SharedLog};
use mtg_engine::state::LogLevel;
use mtg_engine::types::*;

/// What the check says of `line`, written at `level` between `before` and
/// `after`, with `before` the first decision it has seen.
fn said(before: &GameState, after: &GameState, level: LogLevel, line: &str) -> Vec<String> {
    let mut log = SharedLog::new();
    assert!(log.check(None, before).is_empty(), "the first decision has no lines before it");
    let mut after = after.clone();
    after.log(level, line.to_string());
    log.check(Some(before), &after)
}

/// A board with a Grizzly Bears on each side, and `name` on top of p1's
/// library: its id.
fn with_top_card(name: &str) -> (mtg_engine::cards::CardRegistry, GameState, ObjectId) {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for p in [P0, P1] {
        named_permanent(&mut state, &registry, "Grizzly Bears", p);
    }
    let id = library_card(&mut state, &registry, P1, name);
    (registry, state, id)
}

/// `name` put on top of `player`'s library, named as a game names it.
fn library_card(state: &mut GameState, registry: &mtg_engine::cards::CardRegistry, player: PlayerId, name: &str) -> ObjectId {
    let id = stock_library_with(state, registry, player, name, 1)[0];
    state.get_object_mut(id).expect("just made").name = name.to_string();
    id
}

#[test]
fn a_log_line_names_the_ids_after_each_hash() {
    let ids = |line| ids_named(line).collect::<Vec<_>>();
    assert_eq!(ids("p1 searched their library and revealed Island (#48)"), vec![ObjectId(48)]);
    assert_eq!(ids("Lightning Bolt (#12) deals 3 damage to Grizzly Bears (#7)"), vec![ObjectId(12), ObjectId(7)]);
    assert_eq!(ids("resolving spell #3"), vec![ObjectId(3)]);
    assert_eq!(ids("p0 drew a card"), Vec::<ObjectId>::new());
    assert_eq!(ids("a # alone, and #x"), Vec::<ObjectId>::new());
}

/// A card drawn, named with its id in a line both seats are sent, is
/// flagged, by its id and by its name; the same line kept to its player
/// (`Private`) or saying the card was revealed is not.
#[test]
fn a_card_drawn_and_named_is_flagged_unless_private_or_revealed() {
    let (registry, before, heron) = with_top_card("Moon Heron");
    let mut after = before.clone();
    after.move_object(heron, Zone::Hand, &registry);
    let line = format!("p1 drew Moon Heron (#{})", heron.0);
    let found = said(&before, &after, LogLevel::Info, &line);
    assert_eq!(found.len(), 2, "flagged by id and by name: {found:?}");
    assert!(found.iter().all(|f| f.contains("Moon Heron")), "{found:?}");
    assert!(said(&before, &after, LogLevel::Private, &line).is_empty(), "a Private line is the player's own");
    let revealed = format!("p1 searched their library and revealed Moon Heron (#{})", heron.0);
    assert!(said(&before, &after, LogLevel::Event, &revealed).is_empty(), "a revealed card is public (CR 701.16a)");
}

/// A card looked at and left where it is, named by its name alone, as
/// Delver of Secrets's look is written, is flagged; a name a card in view
/// bears is not, since the line may be about that card.
#[test]
fn a_hidden_card_named_by_name_alone_is_flagged_unless_one_in_view_bears_it() {
    let (_, before, _) = with_top_card("Moon Heron");
    let found = said(&before, &before, LogLevel::Debug, "Delver of Secrets: top card is Moon Heron");
    assert_eq!(found.len(), 1, "{found:?}");
    let (_, before, _) = with_top_card("Grizzly Bears");
    assert!(said(&before, &before, LogLevel::Debug, "Delver of Secrets: top card is Grizzly Bears").is_empty(),
        "a Grizzly Bears is on the battlefield, so the name says nothing of the library");
}

/// A name chosen aloud (Nevermore's) is quoted, and a name read inside a
/// longer one in view ("Kessig Wolf" in "Kessig Wolf Run") is not a name.
#[test]
fn quoted_names_and_names_inside_longer_ones_in_view_are_not_flagged() {
    let (_, before, _) = with_top_card("Moon Heron");
    assert!(said(&before, &before, LogLevel::Event, "Nevermore (#99) names \"Moon Heron\"").is_empty());
    let (registry, mut before, _) = with_top_card("Kessig Wolf");
    let run = named_permanent(&mut before, &registry, "Kessig Wolf Run", P1);
    assert!(said(&before, &before, LogLevel::Info, &format!("p1 played Kessig Wolf Run (#{})", run.0)).is_empty());
}

/// A card once in view and returned to its owner's hand may be named: the
/// other player saw it go there.
#[test]
fn a_card_seen_in_view_and_returned_to_hand_may_be_named() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let heron = named_permanent(&mut state, &registry, "Moon Heron", P1);
    let mut log = SharedLog::new();
    assert!(log.check(None, &state).is_empty());
    let mut bounced = state.clone();
    bounced.move_object(heron, Zone::Hand, &registry);
    assert!(log.check(Some(&state), &bounced).is_empty());
    let mut later = bounced.clone();
    later.log(LogLevel::Debug, format!("Moon Heron (#{}) is in p1's hand", heron.0));
    assert!(log.check(Some(&bounced), &later).is_empty(), "seen on the battlefield, so known");
}

/// A cast announced and cancelled names its card, which announcing made
/// public (CR 601.2a) though the engine keeps a cast waiting on its X in the
/// hand; nor is another copy in a library flagged by that name.
#[test]
fn a_cast_cancelled_may_name_its_card_and_its_name() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let play = spell_in_hand(&mut state, &registry, "Devil's Play", P0);
    library_card(&mut state, &registry, P0, "Devil's Play");
    add_mana(&mut state, P0, &[(ManaType::Red, 3)]);
    let legal = engine::legal_actions(&state, &registry);
    let cast = legal.actions.iter()
        .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == play))
        .expect("Devil's Play is castable");
    let waiting = engine::submit_action(&state, cast, &registry);
    assert!(waiting.pending_spell_cast.as_ref().is_some_and(|c| c.object_id == play), "the cast waits on its X");
    let found = said(&state, &waiting, LogLevel::Event, &format!("Devil's Play (#{}): cast cancelled", play.0));
    assert!(found.is_empty(), "{found:?}");
}

/// A card that left a hand between the two decisions, as a land played does,
/// is in view now and may be named: only what is hidden at both is held to
/// the check.
#[test]
fn a_card_played_from_hand_may_be_named() {
    let registry = registry();
    let before = game_at_step(Step::PrecombatMain, P0);
    let mut before = before;
    let forest = spell_in_hand(&mut before, &registry, "Forest", P0);
    before.get_object_mut(forest).expect("in hand").name = "Forest".to_string();
    let mut after = before.clone();
    after.move_object(forest, Zone::Battlefield, &registry);
    let found = said(&before, &after, LogLevel::Info, &format!("p0 played Forest (#{})", forest.0));
    assert!(found.is_empty(), "{found:?}");
}

/// A name a line reveals is seen from then on, and only that name: Mulch's
/// reveal names Moon Heron, so a later line naming it by name says nothing
/// new, while one naming the Delver of Secrets beside it in the library
/// does.
#[test]
fn a_name_revealed_is_seen_and_no_other() {
    let (registry, mut state, _) = with_top_card("Moon Heron");
    library_card(&mut state, &registry, P1, "Delver of Secrets");
    let mut log = SharedLog::new();
    assert!(log.check(None, &state).is_empty());
    let mut revealed = state.clone();
    revealed.log(LogLevel::Event, "Mulch revealed: Moon Heron".into());
    assert!(log.check(Some(&state), &revealed).is_empty(), "a reveal is public (CR 701.16a)");
    let mut later = revealed.clone();
    later.log(LogLevel::Event, "Mulch: Moon Heron put into hand".into());
    assert!(log.check(Some(&revealed), &later).is_empty(), "Moon Heron was revealed, so its name is known");
    let mut leak = later.clone();
    leak.log(LogLevel::Debug, "top card is Delver of Secrets".into());
    let found = log.check(Some(&later), &leak);
    assert_eq!(found.len(), 1, "Delver of Secrets was never revealed: {found:?}");
}
