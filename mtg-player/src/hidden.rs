//! What each seat is sent of what its player cannot see: nothing (CR 401.2,
//! 402.3). The views and menus a decision would send, compared whole with
//! the same decision's under each change a player cannot see, held to one
//! decision in [`HIDDEN_EVERY`].
//!
//! This lives here rather than in either fuzz harness because there are two:
//! `mtg-runner`'s invariant-checked campaign and the libFuzzer target
//! (`fuzz/fuzz_targets/game.rs`), which holds every input to every check
//! the campaign makes. The engine cannot hold it, as it holds the shared
//! log's check (`invariants::shared_log`): it compares serialized views, and
//! the engine has no `serde_json` outside its tests.

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_engine::view::GameView;

/// The view of `state` each player would be sent, whole.
pub fn every_view(state: &GameState, registry: &CardRegistry) -> Vec<serde_json::Value> {
    state.players.iter()
        .map(|p| serde_json::to_value(GameView::for_player(state, p.id, registry)).unwrap_or(serde_json::Value::Null))
        .collect()
}

/// The fields of two serialized views that differ.
pub fn changed_fields<'a>(a: &'a serde_json::Value, b: &serde_json::Value) -> Vec<&'a str> {
    match (a, b) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => a.iter()
            .filter(|(k, v)| b.get(*k) != Some(*v))
            .map(|(k, _)| k.as_str())
            .collect(),
        _ => Vec::new(),
    }
}

/// `state` with the cards of every hand but `keep`'s dealt anew among its
/// objects, each object taking the next one's card: the same game to
/// everyone who cannot see those hands.
pub fn hands_dealt_anew(state: &GameState, keep: PlayerId) -> GameState {
    let mut dealt = state.clone();
    for other in state.players.iter().filter(|p| p.id != keep) {
        let hand: Vec<mtg_engine::ids::ObjectId> = state.objects_in_zone(mtg_engine::types::Zone::Hand, other.id).iter().map(|o| o.id).collect();
        for (i, id) in hand.iter().enumerate() {
            let mut next = state.get_object(hand[(i + 1) % hand.len()]).expect("in the hand").clone();
            next.id = *id;
            dealt.objects.insert(*id, next);
        }
    }
    dealt
}

/// One decision in this many is held to [`hidden_stays_hidden`]: it
/// builds every view three times and the menu twice more, which on every
/// decision made a fuzz game nearly five times as slow (66.8 s against
/// 14.0 s for 30 games). One in 32 costs 7% (15.0 s), well inside the
/// nightly shard's 90 minutes; a leak that shows at all shows on most
/// decisions of a game.
pub const HIDDEN_EVERY: u64 = 32;

/// What a player is shown, and offered, of what they cannot see: nothing.
/// A library's order is hidden from every player (CR 401.2), and a hand
/// from all but its owner (402.3). So with every library reversed each
/// player's view, as every seat is sent it, must be the same; and so must
/// each viewer's with the others' hands dealt anew among their objects,
/// and the acting player's menu with either change. Not the hands while a
/// resolution's question is pending, which may name a hand to the player
/// it reveals it to (Night Terrors). `legal` is the acting player's menu,
/// when it is the menu and not an observed pass. One message per view or
/// menu that changes. `mtg-engine/tests/view_hidden_information.rs` holds
/// the same rule on boards of its own.
pub fn hidden_stays_hidden(state: &GameState, acting: PlayerId, legal: &engine::LegalActions, registry: &CardRegistry) -> Vec<String> {
    let mut out = Vec::new();
    let seen = every_view(state, registry);
    let mut reversed = state.clone();
    for p in &mut reversed.players {
        p.library_order.reverse();
    }
    for (p, (a, b)) in seen.iter().zip(&every_view(&reversed, registry)).enumerate() {
        if a != b {
            out.push(format!("p{p}'s view reads the order of a library (CR 401.2): its {} change when every library is reversed", changed_fields(a, b).join(", ")));
        }
    }
    let question = matches!(state.awaiting_action, Some(mtg_engine::state::AwaitingAction::ResolutionChoice { .. }));
    if question {
        return out;
    }
    for (v, viewer) in state.players.iter().enumerate() {
        let dealt = hands_dealt_anew(state, viewer.id);
        let after = serde_json::to_value(GameView::for_player(&dealt, viewer.id, registry)).unwrap_or(serde_json::Value::Null);
        if seen[v] != after {
            out.push(format!("p{v}'s view reads the cards of another player's hand (CR 402.3): its {} change when they are dealt anew", changed_fields(&seen[v], &after).join(", ")));
        }
    }
    let menu = |s: &GameState| serde_json::to_value(engine::legal_actions(s, registry)).unwrap_or(serde_json::Value::Null);
    let offered = if legal.is_observed_pass() { menu(state) } else { serde_json::to_value(legal).unwrap_or(serde_json::Value::Null) };
    for (what, variant) in [("every library reversed (CR 401.2)", reversed), ("the other player's hand dealt anew (CR 402.3)", hands_dealt_anew(state, acting))] {
        if menu(&variant) != offered {
            out.push(format!("p{}'s offers change with {what}", acting.0));
        }
    }
    out
}
