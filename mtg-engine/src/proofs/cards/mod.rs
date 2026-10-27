//! Every card, proved against its Oracle text.
//!
//! A card proof runs the card's own code, unchanged, on a game state whose
//! relevant parts are left to Kani: which target the spell holds, which
//! player controls what, whether a creature died this turn. It asserts what
//! the card's text says must happen, and that nothing else does.
//!
//! What the card's text says happens is, in the rules, a sequence of
//! actions: deal 1 damage to that target, destroy that creature, draw a
//! card (CR 120, 701). The engine performs each one; the card asks for it.
//! So a card proof replaces the engine function that performs an action
//! with a recorder ([`record_effect`] and the others here), and asserts
//! that the card asked for exactly the actions its text names, with the
//! right objects, players and amounts, under the conditions its text
//! states. That the engine then performs each action as the rules say is
//! proved once per action, in `proofs::actions`, rather than once per card
//! that asks for it, and with the same recorders standing in for the
//! actions that action itself asks for. The two together are the card's proof; neither half
//! is a test of the other.
//!
//! A recorder changes nothing. A card that reads the board back after an
//! action (whether the destroyed creature died, say) is proved with a
//! recorder that also does what the action's own proof says it does, for
//! every outcome that proof allows.
//!
//! Three things are taken out of every proof: `format!`, whose machinery
//! CBMC does not get through ([`no_format`] stands in), the game log, which
//! is for people and which no rule reads ([`no_log`]), and dropping the
//! state at the end, which frees memory and decides nothing
//! (`std::mem::forget`). The engine's maps are
//! `vec_map::VecMap` under Kani, which `proofs::vec_map` holds to
//! `BTreeMap`.
//!
//! Bounds, for every proof here unless its own doc says otherwise: two
//! players, two objects (the card's own and one other, [`any_object`],
//! whose every characteristic is Kani's to choose), and loops of up to six
//! iterations (`unwind(7)`: the five colours, the six kinds of counter);
//! Kani checks that no loop runs longer, and an unwinding assertion fails
//! otherwise.

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::damage::DamageKind;
use crate::destruction::DestroyResult;
use crate::events::DamageTarget;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameState, LibraryPosition, PendingEffect};
use crate::types::{CardType, Color, CounterType, Keyword, ManaType, Zone};

mod activated;
mod auras;
mod board_spells;
mod check_lands;
mod choices;
mod combat_triggers;
mod counted_pt;
mod curses;
mod damage_spells;
mod death_and_attack_triggers;
mod death_watchers;
mod destroy_spells;
mod doom_blade;
mod effect_spells;
mod enters;
mod equipment;
mod graveyard_spells;
mod looking;
mod mana;
mod planeswalkers;
mod replacements;
mod simple_spells;
mod statics;
mod steps;
mod targeted_spells;
mod targets;
mod transforming;
mod watchers;
mod werewolves;

/// An action a card asked the engine for.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    /// `engine::apply_pending_effect`: `effect`, to `target`.
    Effect(Target, PendingEffect),
    /// `destruction::try_destroy`: destroy that permanent (CR 701.8).
    Destroy(ObjectId),
    /// `damage::deal_damage`: `amount` damage of `kind`, dealt by `source`
    /// to `target` (CR 120).
    Damage { source: ObjectId, target: DamageTarget, amount: u32, kind: DamageKind },
    /// `GameState::move_object`: that object to that zone (CR 400.7).
    Move(ObjectId, Zone),
    /// `GameState::attach`: the first object attached to the second (CR
    /// 701.3).
    Attach(ObjectId, ObjectId),
    /// `engine::draw_cards`: that player draws that many (CR 121.1).
    Draw(PlayerId, usize),
    /// `engine::mill_cards`: that player mills that many (CR 701.17a).
    Mill(PlayerId, usize),
    /// `GameState::create_tokens_with_subtypes`: `n` tokens with those
    /// characteristics, under `controller` (CR 111.1).
    Tokens {
        n: u32,
        controller: PlayerId,
        power: i32,
        toughness: i32,
        colors: Vec<Color>,
        types: Vec<CardType>,
        keywords: Vec<Keyword>,
        subtypes: Vec<String>,
    },
    /// `helpers::counter_spell`, or `counter_spell_exiling` when `exile`:
    /// that spell countered (CR 701.6a).
    Counter { spell: ObjectId, exile: bool },
    /// `GameState::add_counters`: that many counters of that kind on that
    /// object (CR 122.1).
    Counters(ObjectId, CounterType, u32),
    /// `helpers::apply_transform`: that permanent transformed (CR 701.27).
    Transform(ObjectId),
    /// `GameState::remove_counters`: up to that many counters of that kind
    /// removed from that object.
    RemoveCounters(ObjectId, CounterType, u32),
    /// `GameState::tap` or `tap_for`: that permanent tapped.
    Tap(ObjectId),
    /// `GameState::untap` or `untap_for`: that permanent untapped.
    Untap(ObjectId),
    /// `GameState::change_life` or `change_life_for`, which `gain_life` and
    /// `lose_life` call: that player's life total changed by that much (CR
    /// 119.3).
    Life(PlayerId, i32),
    /// `GameState::discard_card` or `discard_card_for`: that card
    /// discarded.
    Discard(ObjectId),
    /// `GameState::choose_at_random`: that many of those chosen at random.
    Random(Vec<ObjectId>, usize),
    /// `GameState::put_into_library`: that card put into its owner's
    /// library there.
    ToLibrary(ObjectId, LibraryPosition),
    /// `GameState::move_object_under_control`: that object to that zone,
    /// under that player's control (CR 110.2).
    MoveUnder(ObjectId, Zone, PlayerId),
    /// `helpers::shuffle_library`: that player's library shuffled.
    Shuffle(PlayerId),
    /// `GameState::add_regeneration_shield`: a regeneration shield on that
    /// permanent.
    Regenerate(ObjectId),
    /// `GameState::add_mana`: that much mana of that type in that player's
    /// pool (CR 106.4).
    Mana(PlayerId, ManaType, u32),
    /// `destruction::sacrifice_by`: that permanent sacrificed.
    Sacrifice(ObjectId),
    /// `destruction::try_destroy_all`: those permanents destroyed at once,
    /// with the outcome the recorder gave each.
    DestroyAll(Vec<(ObjectId, DestroyResult)>),
    /// `GameState::arrives_tapped`, asked for `count` objects with
    /// consecutive ids from `first` one after another: each put onto the
    /// battlefield tapped. One record for the run, so that thirteen tokens
    /// entering tapped are not thirteen records.
    ArrivesTapped { first: ObjectId, count: u32 },
    /// `engine::mill_one`: that card milled, from its owner's library to
    /// their graveyard.
    MillCard(ObjectId),
    /// `destruction::try_destroy_no_regen`: that permanent destroyed, and
    /// it can't be regenerated.
    DestroyNoRegen(ObjectId),
    /// `damage::queue_damage`: damage to be dealt with the rest queued (CR
    /// 120.1, all at once).
    Queue { source: ObjectId, target: DamageTarget, amount: u32, kind: DamageKind },
    /// `damage::process_pending_damage`: the queued damage dealt.
    DealQueued,
    /// `GameState::flip_coin`: a coin flipped (CR 705), won or lost as the
    /// recorder said.
    FlipCoin(bool),
    /// `helpers::tokens_enter_combat_attacking`: those tokens put onto the
    /// battlefield tapped and attacking, by `source`'s ability, under
    /// `controller`, who chooses what each attacks (CR 508.4).
    EnterAttacking { source: ObjectId, controller: PlayerId, tokens: Vec<ObjectId> },
    /// `GameState::player_loses`: that player loses the game, for that
    /// reason (CR 104.3).
    Loses(PlayerId, crate::events::LossReason),
    /// `GameState::end_game`: the game over, with that result (CR 104.1).
    GameEnds(crate::state::GameResult),
    /// `helpers::present_target_choice` or `present_optional_target_choice`:
    /// `player` asked to choose one of `options` for `source`'s effect, or
    /// (if `optional`) none; the effect then applied to the one chosen.
    Choose { source: ObjectId, player: PlayerId, options: Vec<Target>, effect: PendingEffect, optional: bool },
    /// `GameState::set_base_pt`: that permanent's power and toughness set
    /// to these, where given (CR 613.4b).
    SetPT(ObjectId, Option<i32>, Option<i32>),
    /// `engine::discard_cards`: that player discards that many cards, which
    /// they choose, for `source`.
    DiscardN { player: PlayerId, count: usize, source: ObjectId },
    /// `helpers::search_library`: `player` searches their library for one
    /// of `candidates`, to put into `to` (tapped, if `tapped`), or (if
    /// `optional`) does not search; then shuffles if they searched.
    Search { source: ObjectId, player: PlayerId, candidates: Vec<ObjectId>, to: Zone, tapped: bool, optional: bool },
    /// `combat::fight`: the two creatures fight (CR 701.12).
    Fight(ObjectId, ObjectId),
    /// `GameState::change_control`: that permanent now controlled by that
    /// player (CR 110.2).
    Control(ObjectId, PlayerId),
    /// `engine::pay_cost_with_sources`: that player pays that mana cost,
    /// tapping mana sources for it as needed (CR 601.2g-h), or cannot, as
    /// the recorder said.
    Pay(PlayerId, crate::types::ManaCost, bool),
    /// `GameState::create_token_copy`: a token that is a copy of that
    /// permanent, under that player's control (CR 111.10, 707.2).
    TokenCopy(ObjectId, PlayerId),
    /// `GameState::create_token_with_defined_pt`: a token whose power and
    /// toughness `defined_by`'s ability defines (CR 604.3), under
    /// `controller`, with those characteristics.
    DefinedToken { defined_by: ObjectId, controller: PlayerId, colors: Vec<Color>, types: Vec<CardType>, subtypes: Vec<String> },
    /// `GameState::gain_control_while_source_controlled`: that permanent
    /// controlled by `source`'s controller for as long as they control
    /// `source` (CR 611.2b, 613.2).
    ControlWhile { object: ObjectId, source: ObjectId },
}

/// The actions asked for so far, in order: a fixed array and a count
/// rather than a `Vec`, for the reason `proofs::decide` gives.
const MAX_ACTS: usize = 8;
static mut ACTS: [Option<Act>; MAX_ACTS] = [const { None }; MAX_ACTS];
static mut ACTED: usize = 0;

/// Whether the card asked for exactly `expected`, in order.
pub fn asked_for(expected: &[Act]) -> bool {
    // SAFETY: a proof runs on one thread, and only this module names ACTS.
    let (acts, n) = unsafe { (&*std::ptr::addr_of!(ACTS), *std::ptr::addr_of!(ACTED)) };
    n == expected.len() && acts.iter().zip(expected).all(|(a, e)| a.as_ref() == Some(e))
}

/// The `i`th action asked for, if there were that many: the action
/// itself, not a copy, since copying a list whose length the solver
/// leaves open allocates a buffer of that open size, which it models byte
/// by byte.
pub fn act(i: usize) -> Option<&'static Act> {
    // SAFETY: as in `acts`; the log is only written by the recorders, and
    // a proof reads it once the card is done asking.
    unsafe { (*std::ptr::addr_of!(ACTS)).get(i).and_then(Option::as_ref) }
}

/// The most objects a list a recorder keeps holds.
const MAX_LISTED: usize = 8;

/// `ids`, as a list of [`MAX_LISTED`] allocated up front and filled entry
/// by entry: one buffer of a size the solver knows, where `to_vec` would
/// allocate one of the open length of `ids`.
pub fn listed(ids: &[ObjectId]) -> Vec<ObjectId> {
    assert!(ids.len() <= MAX_LISTED, "a list longer than a recorder keeps");
    let mut list = Vec::with_capacity(MAX_LISTED);
    for id in ids {
        list.push(*id);
    }
    list
}

/// How many actions were asked for.
pub fn acted() -> usize {
    // SAFETY: as in `acts`.
    unsafe { *std::ptr::addr_of!(ACTED) }
}

fn record(act: Act) {
    // SAFETY: as in `acts`.
    unsafe {
        let n = *std::ptr::addr_of!(ACTED);
        assert!(n < MAX_ACTS, "more actions than a proof here records");
        (*std::ptr::addr_of_mut!(ACTS))[n] = Some(act);
        *std::ptr::addr_of_mut!(ACTED) = n + 1;
    }
}

/// Stands in for `engine::apply_pending_effect`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_effect(_: &mut GameState, target: &Target, effect: &PendingEffect, _: &CardRegistry) {
    record(Act::Effect(target.clone(), effect.clone()));
}

/// Stands in for `destruction::try_destroy`, with every outcome destroying
/// can have, since the recorder decides none of them.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_destroy(_: &mut GameState, id: ObjectId, _: &CardRegistry) -> DestroyResult {
    record(Act::Destroy(id));
    any_destroy_result()
}

/// Stands in for `damage::deal_damage`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_damage(_: &mut GameState, source: ObjectId, target: DamageTarget, amount: u32, kind: DamageKind, _: &CardRegistry) {
    record(Act::Damage { source, target, amount, kind });
}

/// Stands in for `GameState::move_object`. A move is put off, and returns
/// false, only for a card that chooses a copy as it enters (CR 614.12b,
/// `move_object`'s doc), and no object on a board here is of one.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_move(_: &mut GameState, id: ObjectId, to: Zone, _: &CardRegistry) -> bool {
    record(Act::Move(id, to));
    true
}

/// Stands in for `GameState::attach`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_attach(_: &mut GameState, id: ObjectId, host: ObjectId) -> bool {
    record(Act::Attach(id, host));
    true
}

/// Stands in for `engine::draw_cards`, and does what drawing does to the
/// zones, for a card that reads its hand afterwards: the top card of the
/// library to the hand, `count` times or until the library is empty (CR
/// 121.1, 121.4). How many were drawn is what it returns.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_draw(state: &mut GameState, player: PlayerId, count: usize, _: &CardRegistry) -> usize {
    record(Act::Draw(player, count));
    let mut drawn = 0;
    while drawn < count {
        // A branch on the player, not an index by it: `any_object` says why.
        let top = if player == PlayerId(0) { state.players[0].draw_top_card() } else { state.players[1].draw_top_card() };
        let Some(id) = top else { break };
        state.get_object_mut(id).expect("a card in a library").zone = Zone::Hand;
        drawn += 1;
    }
    drawn
}

/// Stands in for `engine::mill_cards`: what it returns, the cards that
/// would be milled (the top `count` of the library, as many as there are,
/// CR 701.17a), for a card that reads what was milled; left where they
/// are, as a recorder changes nothing.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_mill(state: &mut GameState, player: PlayerId, count: usize, _: &str, _: &CardRegistry) -> Vec<ObjectId> {
    record(Act::Mill(player, count));
    // A branch on the player, not an index by it: `any_object` says why.
    let library = if player == PlayerId(0) { &state.players[0].library_order } else { &state.players[1].library_order };
    library.iter().take(count).copied().collect()
}

/// Stands in for `GameState::create_tokens_with_subtypes`: the ids the
/// tokens would have, the next `n` (`GameState::create_object`), none of
/// which is an object on the board, so that a card asking for something
/// done to each token asks for it by id.
#[allow(dead_code, clippy::too_many_arguments)] // named only by `kani::stub`
pub fn record_tokens(
    state: &mut GameState, n: u32, _: &str, controller: PlayerId, power: i32, toughness: i32,
    colors: Vec<Color>, types: Vec<CardType>, keywords: Vec<Keyword>, subtypes: Vec<String>, _: &CardRegistry,
) -> Vec<ObjectId> {
    record(Act::Tokens { n, controller, power, toughness, colors, types, keywords, subtypes });
    new_ids(state, n)
}

/// Stands in for `GameState::create_tokens_with_subtypes` as
/// [`record_tokens`] does, and makes the tokens as well, for a card that
/// reads the board back afterwards: each a token of no card, on the
/// battlefield under `controller`, with exactly the characteristics asked
/// for (CR 111.3). What else creating them does (replacement effects,
/// triggers) is the action's own proof's. Up to two, which is every board
/// here that reads them back.
#[allow(dead_code, clippy::too_many_arguments)] // named only by `kani::stub`
pub fn make_tokens(
    state: &mut GameState, n: u32, _: &str, controller: PlayerId, power: i32, toughness: i32,
    colors: Vec<Color>, types: Vec<CardType>, keywords: Vec<Keyword>, subtypes: Vec<String>, _: &CardRegistry,
) -> Vec<ObjectId> {
    assert!(n <= 2, "more tokens made than a proof here reads back");
    record(Act::Tokens { n, controller, power, toughness, colors: colors.clone(), types: types.clone(), keywords: keywords.clone(), subtypes: subtypes.clone() });
    let mut ids = Vec::with_capacity(2);
    for _ in 0..n {
        let id = state.create_object(CardId(0), controller, Zone::Battlefield, Some(power), Some(toughness));
        let o = state.get_object_mut(id).expect("just created");
        o.is_token = true;
        o.colors = colors.clone();
        o.card_types = types.clone();
        o.keywords = keywords.clone();
        o.subtypes = subtypes.clone();
        ids.push(id);
    }
    ids
}

/// The ids of the next `n` objects the game would create.
pub fn new_ids(state: &GameState, n: u32) -> Vec<ObjectId> {
    let mut ids = Vec::with_capacity(n as usize);
    for k in 0..u64::from(n) {
        ids.push(ObjectId(state.next_object_id + k));
    }
    ids
}

/// Stands in for `GameState::arrives_tapped`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_arrives_tapped(_: &mut GameState, id: ObjectId) {
    // SAFETY: as in `acts`.
    unsafe {
        let n = *std::ptr::addr_of!(ACTED);
        if n > 0 {
            if let Some(Act::ArrivesTapped { first, count }) = &mut (*std::ptr::addr_of_mut!(ACTS))[n - 1] {
                if first.0 + u64::from(*count) == id.0 {
                    *count += 1;
                    return;
                }
            }
        }
    }
    record(Act::ArrivesTapped { first: id, count: 1 });
}

/// Stands in for `engine::mill_one`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_mill_one(_: &mut GameState, id: ObjectId, _: &CardRegistry) {
    record(Act::MillCard(id));
}

/// Stands in for `engine::mill_one` for a card that reads the library
/// again after it: as [`record_mill_one`], and the card taken out of its
/// owner's library, which `move_object` does as it moves it (and which
/// `proofs::actions` proves).
#[allow(dead_code)] // named only by `kani::stub`
pub fn take_milled(state: &mut GameState, id: ObjectId, _: &CardRegistry) {
    record(Act::MillCard(id));
    let owner = state.get_object(id).expect("a card in a library").owner;
    let library = if owner == PlayerId(0) { &mut state.players[0].library_order } else { &mut state.players[1].library_order };
    library.retain(|card| *card != id);
}

/// Stands in for `destruction::try_destroy_no_regen`, as [`record_destroy`].
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_destroy_no_regen(_: &mut GameState, id: ObjectId, _: &CardRegistry) -> DestroyResult {
    record(Act::DestroyNoRegen(id));
    any_destroy_result()
}

/// Stands in for `damage::queue_damage`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_queue(_: &mut GameState, source: ObjectId, target: DamageTarget, amount: u32, kind: DamageKind) {
    record(Act::Queue { source, target, amount, kind });
}

/// Stands in for `damage::process_pending_damage`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_deal_queued(_: &mut GameState, _: &CardRegistry) {
    record(Act::DealQueued);
}

/// Stands in for `helpers::tokens_enter_combat_attacking`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_enter_attacking(_: &mut GameState, source: ObjectId, controller: PlayerId, tokens: &[ObjectId], _: &CardRegistry) {
    record(Act::EnterAttacking { source, controller, tokens: tokens.to_vec() });
}

/// Stands in for `GameState::player_loses`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_loses(_: &mut GameState, player: PlayerId, reason: crate::events::LossReason) {
    record(Act::Loses(player, reason));
}

/// Stands in for `GameState::end_game`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_game_ends(_: &mut GameState, result: crate::state::GameResult) {
    record(Act::GameEnds(result));
}

/// Stands in for `helpers::present_target_choice`.
#[allow(dead_code, clippy::too_many_arguments)] // named only by `kani::stub`
pub fn record_choose(
    _: &mut GameState, source: ObjectId, player: PlayerId, options: Vec<Target>, effect: PendingEffect, _: &str,
    optional: bool, _: &CardRegistry,
) {
    record(Act::Choose { source, player, options, effect, optional });
}

/// Stands in for `helpers::present_optional_target_choice`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_choose_optional(
    _: &mut GameState, source: ObjectId, player: PlayerId, options: Vec<Target>, effect: PendingEffect, _: &str, _: &CardRegistry,
) {
    record(Act::Choose { source, player, options, effect, optional: true });
}

/// Stands in for `GameState::set_base_pt`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_set_pt(_: &mut GameState, id: ObjectId, power: Option<i32>, toughness: Option<i32>) {
    record(Act::SetPT(id, power, toughness));
}

/// Stands in for `engine::discard_cards`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_discard_n(_: &mut GameState, player: PlayerId, count: usize, source: ObjectId, _: &str, _: &CardRegistry) {
    record(Act::DiscardN { player, count, source });
}

/// Stands in for `helpers::search_library`.
#[allow(dead_code, clippy::too_many_arguments)] // named only by `kani::stub`
pub fn record_search(
    _: &mut GameState, source: ObjectId, player: PlayerId, candidates: Vec<ObjectId>, to: Zone, tapped: bool, optional: bool, _: &str,
) {
    record(Act::Search { source, player, candidates, to, tapped, optional });
}

/// Stands in for `combat::fight`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_fight(_: &mut GameState, a: ObjectId, b: ObjectId, _: &CardRegistry) {
    record(Act::Fight(a, b));
}

/// Stands in for `GameState::change_control`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_control(_: &mut GameState, id: ObjectId, player: PlayerId) {
    record(Act::Control(id, player));
}

/// Stands in for `engine::pay_cost_with_sources`: paid or not, either, as
/// a player may or may not have the mana.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_pay(_: &mut GameState, player: PlayerId, cost: &crate::types::ManaCost, _: &CardRegistry) -> bool {
    let paid = kani::any();
    record(Act::Pay(player, cost.clone(), paid));
    paid
}

/// Stands in for `engine::payment_plan_with_sources`, which changes nothing:
/// a plan or none, either, as a player may or may not have the mana; the
/// plan, when there is one, the pool alone (no source to tap).
#[allow(dead_code)] // named only by `kani::stub`
pub fn any_payment_plan(_: &GameState, _: PlayerId, _: &crate::types::ManaCost, _: &CardRegistry) -> Option<Vec<(ObjectId, usize)>> {
    if kani::any() { Some(Vec::new()) } else { None }
}

/// Stands in for `GameState::create_token_copy`: the id the token would
/// have, as [`record_tokens`].
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_token_copy(state: &mut GameState, of: ObjectId, controller: PlayerId, _: &CardRegistry) -> ObjectId {
    record(Act::TokenCopy(of, controller));
    ObjectId(state.next_object_id)
}

/// Stands in for `GameState::create_token_with_defined_pt`.
#[allow(dead_code, clippy::too_many_arguments)] // named only by `kani::stub`
pub fn record_defined_token(
    state: &mut GameState, defined_by: ObjectId, _: &str, controller: PlayerId, colors: Vec<Color>, types: Vec<CardType>,
    keywords: Vec<Keyword>, subtypes: Vec<String>, _: &CardRegistry,
) -> Vec<ObjectId> {
    assert!(keywords.is_empty(), "a defined token here has no keyword");
    record(Act::DefinedToken { defined_by, controller, colors, types, subtypes });
    new_ids(state, 1)
}

/// Stands in for `GameState::gain_control_while_source_controlled`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_control_while(_: &mut GameState, object: ObjectId, source: ObjectId, _: &CardRegistry) {
    record(Act::ControlWhile { object, source });
}

/// Stands in for `GameState::flip_coin`: won or lost, either.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_flip(_: &mut GameState) -> bool {
    let won = kani::any();
    record(Act::FlipCoin(won));
    won
}

/// Stands in for `helpers::transform_quietly`, as [`record_transform`].
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_transform_quietly(state: &mut GameState, id: ObjectId, registry: &CardRegistry) -> crate::cards::helpers::TransformResult {
    record_transform(state, id, registry)
}

/// Stands in for `helpers::counter_spell`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_counter(_: &mut GameState, spell: ObjectId, _: &CardRegistry) -> bool {
    record(Act::Counter { spell, exile: false });
    true
}

/// Stands in for `helpers::counter_spell_exiling`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_counter_exiling(_: &mut GameState, spell: ObjectId, _: &CardRegistry) -> bool {
    record(Act::Counter { spell, exile: true });
    true
}

/// Stands in for `GameState::add_counters`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_counters(_: &mut GameState, id: ObjectId, kind: CounterType, n: u32) {
    record(Act::Counters(id, kind, n));
}

/// Stands in for `GameState::add_counters` as [`record_counters`] does, and
/// adds them as well, for a card that reads the count back: by
/// `add_counters_quiet`, which is `add_counters` without its log line.
#[allow(dead_code)] // named only by `kani::stub`
pub fn add_counters(state: &mut GameState, id: ObjectId, kind: CounterType, n: u32) {
    record(Act::Counters(id, kind, n));
    let _ = state.add_counters_quiet(id, kind, n);
}

/// Stands in for `GameState::remove_counters`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_remove_counters(_: &mut GameState, id: ObjectId, kind: CounterType, n: u32) {
    record(Act::RemoveCounters(id, kind, n));
}

/// Stands in for `helpers::apply_transform`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_transform(_: &mut GameState, id: ObjectId, _: &CardRegistry) -> crate::cards::helpers::TransformResult {
    record(Act::Transform(id));
    crate::cards::helpers::TransformResult::Transformed
}

/// Whether `id` is a permanent that is untapped, which is when tapping it
/// does anything: what `GameState::tap` returns.
fn would_tap(state: &GameState, id: ObjectId) -> bool {
    state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield && !o.tapped)
}

/// Stands in for `GameState::tap`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_tap(state: &mut GameState, id: ObjectId) -> bool {
    record(Act::Tap(id));
    would_tap(state, id)
}

/// Stands in for `GameState::tap_for`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_tap_for(state: &mut GameState, id: ObjectId, _: &str) -> bool {
    record_tap(state, id)
}

/// Stands in for `GameState::untap`: whether `id` was tapped, which is
/// what it returns.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_untap(state: &mut GameState, id: ObjectId) -> bool {
    record(Act::Untap(id));
    state.get_object(id).is_some_and(|o| o.tapped)
}

/// Stands in for `GameState::untap_for`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_untap_for(state: &mut GameState, id: ObjectId, _: &str) -> bool {
    record_untap(state, id)
}

/// Stands in for `GameState::change_life`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_life(_: &mut GameState, player: PlayerId, delta: i32) {
    record(Act::Life(player, delta));
}

/// Stands in for `GameState::change_life_for`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_life_for(_: &mut GameState, player: PlayerId, delta: i32, _: &str) {
    record(Act::Life(player, delta));
}

/// Stands in for `GameState::discard_card`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_discard(_: &mut GameState, id: ObjectId, _: &CardRegistry) {
    record(Act::Discard(id));
}

/// Stands in for `GameState::discard_card_for`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_discard_for(_: &mut GameState, id: ObjectId, _: &str, _: &CardRegistry) {
    record(Act::Discard(id));
}

/// Stands in for `GameState::choose_at_random`: any `n` of `candidates`
/// (all of them if there are no more than `n`), in any order, since which
/// is chance's to say. Up to two chosen, which is every board here.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_random(_: &mut GameState, candidates: &[ObjectId], n: usize) -> Vec<ObjectId> {
    record(Act::Random(listed(candidates), n));
    let want = n.min(candidates.len());
    assert!(want <= 2, "more chosen at random than a proof here orders");
    // Each one chosen at a position Kani picks, the second not the first's.
    let mut chosen = Vec::with_capacity(2);
    if want >= 1 {
        let first: usize = kani::any();
        kani::assume(first < candidates.len());
        chosen.push(candidates[first]);
        if want == 2 {
            let second: usize = kani::any();
            kani::assume(second < candidates.len() && second != first);
            chosen.push(candidates[second]);
        }
    }
    chosen
}

/// Stands in for `GameState::put_into_library`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_to_library(_: &mut GameState, id: ObjectId, position: LibraryPosition, _: &CardRegistry) {
    record(Act::ToLibrary(id, position));
}

/// Stands in for `GameState::move_object_under_control`; true for the
/// reason [`record_move`] is.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_move_under(_: &mut GameState, id: ObjectId, to: Zone, controller: PlayerId, _: &CardRegistry) -> bool {
    record(Act::MoveUnder(id, to, controller));
    true
}

/// Stands in for `helpers::shuffle_library`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_shuffle(_: &mut GameState, player: PlayerId) {
    record(Act::Shuffle(player));
}

/// Stands in for `GameState::add_regeneration_shield`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_regenerate(_: &mut GameState, id: ObjectId) {
    record(Act::Regenerate(id));
}

/// Stands in for `GameState::add_mana`.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_mana(_: &mut GameState, player: PlayerId, kind: ManaType, n: u32) {
    record(Act::Mana(player, kind, n));
}

/// Stands in for `destruction::sacrifice_by`: whether `id` is a permanent,
/// which is when it can be sacrificed, and what it returns.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_sacrifice(state: &mut GameState, id: ObjectId, _: &str, _: &CardRegistry) -> bool {
    record(Act::Sacrifice(id));
    state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield)
}

/// Stands in for `destruction::try_destroy_by`, as [`record_destroy`].
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_destroy_by(state: &mut GameState, id: ObjectId, _: &str, registry: &CardRegistry) -> DestroyResult {
    record_destroy(state, id, registry)
}

/// Stands in for `destruction::try_destroy_all`, with every outcome for
/// each.
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_destroy_all(_: &mut GameState, ids: &[ObjectId], _: &CardRegistry) -> Vec<(ObjectId, DestroyResult)> {
    let mut outcomes = Vec::with_capacity(ids.len());
    for id in ids {
        outcomes.push((*id, any_destroy_result()));
    }
    record(Act::DestroyAll(outcomes.clone()));
    outcomes
}

/// Stands in for `destruction::try_destroy_all_by`, as
/// [`record_destroy_all`].
#[allow(dead_code)] // named only by `kani::stub`
pub fn record_destroy_all_by(state: &mut GameState, ids: &[ObjectId], _: &str, registry: &CardRegistry) -> Vec<(ObjectId, DestroyResult)> {
    record_destroy_all(state, ids, registry)
}

/// Any outcome of destroying a permanent.
fn any_destroy_result() -> DestroyResult {
    match kani::any::<u8>() % 4 {
        0 => DestroyResult::Died,
        1 => DestroyResult::Indestructible,
        2 => DestroyResult::Regenerated,
        _ => DestroyResult::NotAPermanent,
    }
}

/// Stands in for `GameState::objects_in_zone`: the zone's objects, by the
/// engine's own `in_zone_of`, in id order, on a board whose graveyard
/// piles each list a first run of that graveyard's cards in id order and
/// nothing else, which it asserts. That is every board here, `any_object`
/// putting each graveyard card on its pile as it makes it; and on every
/// such board it is what `objects_in_zone` returns, which hands
/// `mtg_kernel::zones::zone_order` every object in id order: that order is
/// proved with Verus for every board, and
/// `lemma_a_pile_in_order_lists_in_order` there is this case (a pile that
/// is the first of the zone's cards in id order has its entries in their
/// order, and every card it does not list after every card it does).
///
/// Why: CBMC does not get through `zone_order`. A list built at positions
/// the data decides is a write at a position the solver leaves open, into
/// a buffer it models byte by byte, and a proof of the order alone over
/// eight objects ran it out of memory. A list built in one pass in id
/// order it reads without that.
#[allow(dead_code)] // named only by `kani::stub`
pub fn objects_in_zone_in_id_order(state: &GameState, zone: Zone, player: PlayerId) -> Vec<&crate::state::GameObject> {
    let mut found = Vec::with_capacity(state.objects.len());
    for o in state.objects_in_id_order() {
        if crate::state::in_zone_of(o, zone, player) {
            found.push(o);
        }
    }
    if zone == Zone::Graveyard {
        let pile = if player == PlayerId(0) { &state.players[0].graveyard_order } else { &state.players[1].graveyard_order };
        assert!(pile.len() <= found.len(), "a graveyard pile listing more than the graveyard");
        for (i, id) in pile.iter().enumerate() {
            assert!(*id == found[i].id, "a graveyard pile out of id order");
        }
    }
    found
}

/// Stands in for `GameState::log`: the game log is for people, and no rule
/// reads it (the view shows it; `invariants::transition` checks it only
/// grows).
#[allow(dead_code)] // named only by `kani::stub`
pub fn no_log(_: &mut GameState, _: crate::state::LogLevel, _: String) {}

/// Stands in for the global allocator's growing an allocation, which every
/// `Vec` and `String` that outgrows its capacity asks for: a failure, so
/// that a proof that passes is one in which no list outgrew the capacity
/// it was given, and this was never called.
///
/// Why: whether a list is full is a question CBMC cannot answer before it
/// encodes the answer yes. `push` and `extend` grow a list that is full,
/// so at every push it cannot prove lands in room it encodes the growing:
/// a buffer of a size the data decides, and a copy into it of a length the
/// data decides. Collecting a filtered list of three numbers took 59
/// million clauses with that and 0.7 million with this; a proof whose
/// lists do need to grow fails, naming this, rather than passing.
#[allow(dead_code)] // named only by `kani::stub`
pub fn no_growing(
    _: &std::alloc::Global,
    _: std::ptr::NonNull<u8>,
    _: std::alloc::Layout,
    _: std::alloc::Layout,
    _: bool,
) -> Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> {
    panic!("a list grew past the capacity it was given")
}

/// Stands in for `std::fmt::format`: the game log is for people, and no
/// rule reads it.
#[allow(dead_code)] // named only by `kani::stub`
pub fn no_format(_: std::fmt::Arguments<'_>) -> String {
    String::new()
}

/// A card proof: `#[kani::proof]` with the stubs and bounds every card
/// proof runs under (the module doc says why each).
macro_rules! card_proof {
    ($(#[$meta:meta])* fn $name:ident() $body:block) => {
        $crate::proofs::cards::card_proof! { @with unwind(7) tokens(record_tokens) counters(record_counters) mill(record_mill_one) $(#[$meta])* fn $name() $body }
    };
    (unwind($n:literal) $(#[$meta:meta])* fn $name:ident() $body:block) => {
        $crate::proofs::cards::card_proof! { @with unwind($n) tokens(record_tokens) counters(record_counters) mill(record_mill_one) $(#[$meta])* fn $name() $body }
    };
    // For a card that reads the board back after creating tokens: the
    // tokens made, as [`make_tokens`] says.
    (making_tokens $(#[$meta:meta])* fn $name:ident() $body:block) => {
        $crate::proofs::cards::card_proof! { @with unwind(7) tokens(make_tokens) counters(record_counters) mill(record_mill_one) $(#[$meta])* fn $name() $body }
    };
    // For a card that reads counters back after adding them: the counters
    // added, as [`add_counters`] says.
    (adding_counters $(#[$meta:meta])* fn $name:ident() $body:block) => {
        $crate::proofs::cards::card_proof! { @with unwind(7) tokens(record_tokens) counters(add_counters) mill(record_mill_one) $(#[$meta])* fn $name() $body }
    };
    // For a card that reads the library back after milling a card from
    // it: the card taken out of the library, as [`take_milled`] says.
    (taking_milled $(#[$meta:meta])* fn $name:ident() $body:block) => {
        $crate::proofs::cards::card_proof! { @with unwind(7) tokens(record_tokens) counters(record_counters) mill(take_milled) $(#[$meta])* fn $name() $body }
    };
    (@with unwind($n:literal) tokens($tokens:ident) counters($counters:ident) mill($mill:ident) $(#[$meta:meta])* fn $name:ident() $body:block) => {
        $(#[$meta])*
        #[kani::proof]
        #[kani::unwind($n)]
        #[kani::stub(std::fmt::format, $crate::proofs::cards::no_format)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, $crate::proofs::cards::no_growing)]
        #[kani::stub($crate::state::GameState::log, $crate::proofs::cards::no_log)]
        #[kani::stub($crate::state::GameState::objects_in_zone, $crate::proofs::cards::objects_in_zone_in_id_order)]
        #[kani::stub($crate::engine::apply_pending_effect, $crate::proofs::cards::record_effect)]
        #[kani::stub($crate::destruction::try_destroy, $crate::proofs::cards::record_destroy)]
        #[kani::stub($crate::state::GameState::move_object, $crate::proofs::cards::record_move)]
        #[kani::stub($crate::state::GameState::attach, $crate::proofs::cards::record_attach)]
        #[kani::stub($crate::engine::draw_cards, $crate::proofs::cards::record_draw)]
        #[kani::stub($crate::engine::mill_cards, $crate::proofs::cards::record_mill)]
        #[kani::stub($crate::state::GameState::create_tokens_with_subtypes, $crate::proofs::cards::$tokens)]
        #[kani::stub($crate::cards::helpers::counter_spell, $crate::proofs::cards::record_counter)]
        #[kani::stub($crate::cards::helpers::counter_spell_exiling, $crate::proofs::cards::record_counter_exiling)]
        #[kani::stub($crate::state::GameState::add_counters, $crate::proofs::cards::$counters)]
        #[kani::stub($crate::state::GameState::remove_counters, $crate::proofs::cards::record_remove_counters)]
        #[kani::stub($crate::cards::helpers::apply_transform, $crate::proofs::cards::record_transform)]
        #[kani::stub($crate::state::GameState::tap, $crate::proofs::cards::record_tap)]
        #[kani::stub($crate::state::GameState::tap_for, $crate::proofs::cards::record_tap_for)]
        #[kani::stub($crate::state::GameState::untap, $crate::proofs::cards::record_untap)]
        #[kani::stub($crate::state::GameState::untap_for, $crate::proofs::cards::record_untap_for)]
        #[kani::stub($crate::state::GameState::change_life, $crate::proofs::cards::record_life)]
        #[kani::stub($crate::state::GameState::change_life_for, $crate::proofs::cards::record_life_for)]
        #[kani::stub($crate::state::GameState::discard_card, $crate::proofs::cards::record_discard)]
        #[kani::stub($crate::state::GameState::discard_card_for, $crate::proofs::cards::record_discard_for)]
        #[kani::stub($crate::state::GameState::choose_at_random, $crate::proofs::cards::record_random)]
        #[kani::stub($crate::state::GameState::put_into_library, $crate::proofs::cards::record_to_library)]
        #[kani::stub($crate::state::GameState::move_object_under_control, $crate::proofs::cards::record_move_under)]
        #[kani::stub($crate::cards::helpers::shuffle_library, $crate::proofs::cards::record_shuffle)]
        #[kani::stub($crate::state::GameState::add_regeneration_shield, $crate::proofs::cards::record_regenerate)]
        #[kani::stub($crate::state::GameState::add_mana, $crate::proofs::cards::record_mana)]
        #[kani::stub($crate::destruction::sacrifice_by, $crate::proofs::cards::record_sacrifice)]
        #[kani::stub($crate::destruction::try_destroy_by, $crate::proofs::cards::record_destroy_by)]
        #[kani::stub($crate::destruction::try_destroy_all, $crate::proofs::cards::record_destroy_all)]
        #[kani::stub($crate::destruction::try_destroy_all_by, $crate::proofs::cards::record_destroy_all_by)]
        #[kani::stub($crate::state::GameState::arrives_tapped, $crate::proofs::cards::record_arrives_tapped)]
        #[kani::stub($crate::engine::mill_one, $crate::proofs::cards::$mill)]
        #[kani::stub($crate::destruction::try_destroy_no_regen, $crate::proofs::cards::record_destroy_no_regen)]
        #[kani::stub($crate::damage::queue_damage, $crate::proofs::cards::record_queue)]
        #[kani::stub($crate::damage::process_pending_damage, $crate::proofs::cards::record_deal_queued)]
        #[kani::stub($crate::cards::helpers::transform_quietly, $crate::proofs::cards::record_transform_quietly)]
        #[kani::stub($crate::state::GameState::flip_coin, $crate::proofs::cards::record_flip)]
        #[kani::stub($crate::cards::helpers::tokens_enter_combat_attacking, $crate::proofs::cards::record_enter_attacking)]
        #[kani::stub($crate::state::GameState::player_loses, $crate::proofs::cards::record_loses)]
        #[kani::stub($crate::state::GameState::end_game, $crate::proofs::cards::record_game_ends)]
        #[kani::stub($crate::cards::helpers::present_target_choice, $crate::proofs::cards::record_choose)]
        #[kani::stub($crate::cards::helpers::present_optional_target_choice, $crate::proofs::cards::record_choose_optional)]
        #[kani::stub($crate::state::GameState::set_base_pt, $crate::proofs::cards::record_set_pt)]
        #[kani::stub($crate::engine::discard_cards, $crate::proofs::cards::record_discard_n)]
        #[kani::stub($crate::cards::helpers::search_library, $crate::proofs::cards::record_search)]
        #[kani::stub($crate::combat::fight, $crate::proofs::cards::record_fight)]
        #[kani::stub($crate::state::GameState::change_control, $crate::proofs::cards::record_control)]
        #[kani::stub($crate::engine::pay_cost_with_sources, $crate::proofs::cards::record_pay)]
        #[kani::stub($crate::engine::payment_plan_with_sources, $crate::proofs::cards::any_payment_plan)]
        #[kani::stub($crate::state::GameState::create_token_copy, $crate::proofs::cards::record_token_copy)]
        #[kani::stub($crate::state::GameState::create_token_with_defined_pt, $crate::proofs::cards::record_defined_token)]
        #[kani::stub($crate::state::GameState::gain_control_while_source_controlled, $crate::proofs::cards::record_control_while)]
        fn $name() $body
    };
}
pub(crate) use card_proof;

/// A card proof for each [`Form`] an object may take: `NAME::of_a_card`
/// and `NAME::of_a_token`, each running the body with `form` set.
macro_rules! for_each_form {
    ($(#[$meta:meta])* fn $name:ident($form:ident) $body:block) => {
        mod $name {
            #[allow(unused_imports)]
            use super::*;
            use $crate::proofs::cards::{card_proof, Form};

            fn check($form: Form) $body

            card_proof! { $(#[$meta])* fn of_a_card() { check(Form::Card) } }
            card_proof! { $(#[$meta])* fn of_a_token() { check(Form::Token) } }
        }
    };
}
pub(crate) use for_each_form;

/// What a card's resolution could change, read off the state: each
/// object's zone, controller, tapped state, damage, counters of each kind,
/// attachment and face; each player's life, library and graveyard; the
/// stack, the open question, the turn's effects, the tokens to be exiled at
/// end of combat, damage waiting to be dealt and the events raised.
///
/// Flat on purpose: each object a row of numbers, its counters one count
/// per kind, and each player's library and graveyard their length and
/// their first [`ZONE_ROWS`] cards. A snapshot holding lists, of counters
/// or of cards, was a list of lists of lengths Kani had to leave open, and
/// comparing two of them ran the solver out of memory. Every board here
/// has fewer cards than that in each zone, which [`Snapshot::of`] asserts,
/// so the rows are the whole zones.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    objects: Vec<ObjectRow>,
    players: [(i32, ZoneRows, ZoneRows); 2],
    stack: usize,
    asking: bool,
    until_end_of_turn: usize,
    end_of_combat_exiles: usize,
    pending_damage: usize,
    events: usize,
}

/// One object, as a [`Snapshot`] sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ObjectRow {
    id: ObjectId,
    zone: Zone,
    controller: PlayerId,
    tapped: bool,
    damage: u32,
    counters: [u32; COUNTER_KINDS.len()],
    attached_to: Option<ObjectId>,
    attached_to_player: Option<PlayerId>,
    transformed: bool,
}

/// How many cards of a zone a [`Snapshot`] holds.
const ZONE_ROWS: usize = 5;

/// A zone of a player's: how many cards, and the first [`ZONE_ROWS`].
type ZoneRows = (usize, [Option<ObjectId>; ZONE_ROWS]);

fn zone_rows(cards: &[ObjectId]) -> ZoneRows {
    assert!(cards.len() <= ZONE_ROWS, "a board with more cards in a zone than a snapshot holds");
    let mut rows = [None; ZONE_ROWS];
    for (row, card) in rows.iter_mut().zip(cards) {
        *row = Some(*card);
    }
    (cards.len(), rows)
}

const COUNTER_KINDS: [CounterType; 6] = [
    CounterType::PlusOnePlusOne, CounterType::MinusOneMinusOne, CounterType::Loyalty,
    CounterType::Slime, CounterType::Study, CounterType::Hatchling,
];

impl Snapshot {
    /// How many until-end-of-turn effects there were.
    pub fn until_end_of_turn(&self) -> usize {
        self.until_end_of_turn
    }

    /// How many tokens were to be exiled at end of combat.
    pub fn end_of_combat_exiles(&self) -> usize {
        self.end_of_combat_exiles
    }

    /// This snapshot without its count of tokens to exile at end of
    /// combat, for a proof that reads which were added itself.
    pub fn without_end_of_combat_exiles(&self) -> Self {
        Snapshot { end_of_combat_exiles: 0, ..self.clone() }
    }

    /// This snapshot without its count of until-end-of-turn effects, for a
    /// proof that reads which effects were added itself.
    pub fn without_until_end_of_turn(&self) -> Self {
        Snapshot { until_end_of_turn: 0, ..self.clone() }
    }

    /// This snapshot with `id` attached to player `p`, and nothing else
    /// changed.
    pub fn with_attached_player(&self, id: ObjectId, p: PlayerId) -> Self {
        let mut s = self.clone();
        for row in &mut s.objects {
            if row.id == id {
                row.attached_to_player = Some(p);
            }
        }
        s
    }

    pub fn of(state: &GameState) -> Self {
        Snapshot {
            objects: state.objects_in_id_order().into_iter()
                .map(|o| ObjectRow {
                    id: o.id,
                    zone: o.zone,
                    controller: o.controller,
                    tapped: o.tapped,
                    damage: o.damage_marked,
                    counters: COUNTER_KINDS.map(|k| o.counters.get(&k).copied().unwrap_or(0)),
                    attached_to: o.attached_to,
                    attached_to_player: o.attached_to_player,
                    transformed: o.is_transformed,
                })
                .collect(),
            players: [0, 1].map(|i| {
                let p = &state.players[i];
                (p.life, zone_rows(&p.library_order), zone_rows(&p.graveyard_order))
            }),
            stack: state.stack.len(),
            asking: state.awaiting_action.is_some(),
            until_end_of_turn: state.until_end_of_turn.len(),
            end_of_combat_exiles: state.end_of_combat_exiles.len(),
            pending_damage: state.pending_damage.len(),
            events: state.events.len(),
        }
    }
}

/// Any target a spell could hold as it resolves: either of `objects`,
/// either player, or one that has become illegal since it was chosen (CR
/// 608.2b).
pub fn any_target(objects: [ObjectId; 2]) -> Target {
    match kani::any::<u8>() % 5 {
        0 => Target::Object(objects[0]),
        1 => Target::Object(objects[1]),
        2 => Target::Player(PlayerId(0)),
        3 => Target::Player(PlayerId(1)),
        _ => Target::Illegal,
    }
}

/// A card that prints nothing: an object of it has exactly the
/// characteristics a proof gives it, since an object's characteristics are
/// its own and its card's (`GameState::colors_of` and the rest).
pub(crate) struct Blank;

impl CardBehavior for Blank {
    fn card_data(&self) -> CardData {
        CardData { name: "Blank".into(), ..CardData::default() }
    }
}

/// A registry of the blank card and `card`, and a two-player game: the
/// blank card's id and `card`'s.
pub fn board(card: Box<dyn CardBehavior>) -> (CardRegistry, GameState, CardId, CardId) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(card);
    (registry, new_game(), blank, card)
}

/// A registry of the blank card alone, and a two-player game: the blank
/// card's id.
pub fn blank_board() -> (CardRegistry, GameState, CardId) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    (registry, new_game(), blank)
}

/// A two-player game, its lists allocated up front at more than any board
/// here fills them to. A list that is empty, and so unallocated, until a
/// push the solver may or may not take is a list it follows into two
/// buffers from then on, and into more at each such push; allocated
/// unconditionally first, every later push lands in the one buffer.
pub(crate) fn new_game() -> GameState {
    let mut state = GameState::new(2);
    state.objects.reserve(8);
    state.until_end_of_turn.reserve(8);
    state.events.reserve(8);
    state.pending_damage.reserve(4);
    state.end_of_combat_exiles.reserve(2);
    state.stack.reserve(2);
    for p in &mut state.players {
        p.library_order.reserve(6);
        p.graveyard_order.reserve(6);
    }
    state
}

/// The characteristics an object of [`any_object`] may have.
pub struct Kinds {
    pub types: &'static [CardType],
    pub subtypes: &'static [&'static str],
    pub keywords: &'static [Keyword],
}

/// What an object is: a card, or a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// A card of the blank card, on the battlefield or in its owner's
    /// graveyard.
    Card,
    /// A token, on the battlefield (a token elsewhere ceases to exist, CR
    /// 704.5d).
    Token,
}

/// Any card of the blank card's, or any token: controlled by either
/// player, with any set of colours, and of `kinds`' card types and
/// subtypes, and a creature's power and toughness from 0/1 to 5/5.
///
/// Which form is a parameter, not Kani's choice: it decides which card the
/// object is of, and a card Kani leaves open is an index into the registry
/// it leaves open, which (like an open index into the players, below) makes
/// every later read a question of which entry it was. A proof that holds
/// for both forms is proved once for each.
///
/// Keywords are a token's alone. A card's keywords are those its card
/// prints, which for the blank card is none, and those effects grant: the
/// engine reads a card object's own keyword list nowhere, and nothing
/// writes one (`GameState::scan_keywords` says why). A card object with a
/// keyword written on it is a state no game reaches, which a proof here
/// first built, and whose counterexample said so.
pub fn any_object(state: &mut GameState, blank: CardId, kinds: &Kinds, form: Form) -> ObjectId {
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let token = form == Form::Token;
    let zone = if token || kani::any() { Zone::Battlefield } else { Zone::Graveyard };
    // A token is of no card: `CardId(0)`, which no registry hands out.
    let card = if token { CardId(0) } else { blank };
    let id = state.create_object(card, owner, zone, None, None);
    // Into the owner's graveyard by a branch on the owner, not by indexing
    // the players with it: a write through an index Kani leaves open makes
    // every later read of either player's zones a question of which one it
    // was, and the solver ran out of memory.
    if zone == Zone::Graveyard {
        if owner == PlayerId(0) {
            state.players[0].graveyard_order.push(id);
        } else {
            state.players[1].graveyard_order.push(id);
        }
    }
    let o = state.get_object_mut(id).expect("just created");
    o.is_token = token;
    // Each list allocated at its largest and filled entry by entry, so that
    // no push reallocates: a conditional reallocation leaves CBMC tracking
    // every buffer the list might be in, and the solver ran out of memory.
    o.colors = Vec::with_capacity(5);
    for c in Color::ALL {
        if kani::any() {
            o.colors.push(c);
        }
    }
    o.card_types = Vec::with_capacity(kinds.types.len());
    for t in kinds.types {
        if kani::any() {
            o.card_types.push(*t);
        }
    }
    o.subtypes = Vec::with_capacity(kinds.subtypes.len());
    for t in kinds.subtypes {
        if kani::any() {
            o.subtypes.push((*t).to_string());
        }
    }
    o.keywords = Vec::with_capacity(kinds.keywords.len());
    if token {
        for k in kinds.keywords {
            if kani::any() {
                o.keywords.push(*k);
            }
        }
    }
    if o.card_types.contains(&CardType::Creature) {
        let (p, t): (u8, u8) = (kani::any(), kani::any());
        kani::assume(p <= 5 && (1..=5).contains(&t));
        o.power = Some(i32::from(p));
        o.toughness = Some(i32::from(t));
    }
    id
}

/// A card's triggered abilities as the engine reads them to know when it
/// triggers: each one's event, and what it targets (CR 603.1, 603.3d).
///
/// The face is read and then forgotten, not dropped, here and in
/// [`prints_effects`]: dropping a face is dropping the filters and effects
/// it holds, types defined in terms of themselves, and CBMC runs out of
/// memory working through that recursion.
pub fn triggers(card: &dyn CardBehavior) -> Vec<(crate::cards::TriggerKind, Option<crate::cards::TargetRequirement>)> {
    let data = std::mem::ManuallyDrop::new(card.card_data());
    data.triggered_abilities.iter().map(|t| (t.kind.clone(), t.target_requirement.clone())).collect()
}

/// Whether `card` prints exactly `effects`, its static abilities' effects
/// (CR 604.1, 611.3), which the engine reads off its face to apply them.
///
/// `effects` is a list written out where it is called, never one built on
/// the heap, and holds no filter or effect inside another; [`prints_where`]
/// is for those. Comparing two values of the self-referring types (filters,
/// effects, target requirements) that were both read from the heap is what
/// CBMC ran out of time on, in every form: two vectors, two elements, a
/// loop over both. With one side written out it settles the comparison as
/// written, in seconds. The same holds below wherever a proof compares such
/// values: the expected side is built where it is compared, element by
/// element ([`each_is`]), or is a pattern.
pub fn prints_effects(card: &dyn CardBehavior, effects: &[crate::types::ContinuousEffect]) -> bool {
    let data = std::mem::ManuallyDrop::new(card.card_data());
    data.continuous_effects.as_slice() == effects
}

/// Whether `card`'s printed effects are what `shape` says, for effects that
/// hold a filter or another effect (see [`prints_effects`]).
pub fn prints_where(card: &dyn CardBehavior, shape: impl FnOnce(&[crate::types::ContinuousEffect]) -> bool) -> bool {
    let data = std::mem::ManuallyDrop::new(card.card_data());
    shape(&data.continuous_effects)
}

/// Whether `actual` is `n` values, the `i`th of them `expected(i)`, each
/// expected value built where it is compared ([`prints_effects`] says why).
pub fn each_is<T: PartialEq>(actual: &[T], n: usize, expected: impl Fn(usize) -> T) -> bool {
    actual.len() == n && (0..n).all(|i| {
        let e = std::mem::ManuallyDrop::new(expected(i));
        actual[i] == *e
    })
}

/// Whether `f` is the filter "has subtype `subtype`".
pub fn has_subtype(f: &crate::types::CreatureFilter, subtype: &str) -> bool {
    matches!(f, crate::types::CreatureFilter::HasSubtype(s) if s == subtype)
}

/// Whether `f` is the filter "both of" two filters `a` and `b` describe.
pub fn both(f: &crate::types::CreatureFilter, a: impl Fn(&crate::types::CreatureFilter) -> bool, b: impl Fn(&crate::types::CreatureFilter) -> bool) -> bool {
    matches!(f, crate::types::CreatureFilter::And(fs) if fs.len() == 2 && a(&fs[0]) && b(&fs[1]))
}

/// Whether `f` is the filter "either of" two filters `a` and `b` describe.
pub fn either(f: &crate::types::CreatureFilter, a: impl Fn(&crate::types::CreatureFilter) -> bool, b: impl Fn(&crate::types::CreatureFilter) -> bool) -> bool {
    matches!(f, crate::types::CreatureFilter::Or(fs) if fs.len() == 2 && a(&fs[0]) && b(&fs[1]))
}

/// Any of the five colours.
pub fn any_color() -> Color {
    match kani::any::<u8>() % 5 {
        0 => Color::White,
        1 => Color::Blue,
        2 => Color::Black,
        3 => Color::Red,
        _ => Color::Green,
    }
}

/// A card resolving: the game after, the card's spell, the target it
/// held, and the board before. (The registry is kept so that it is not
/// dropped before the proof forgets it.)
pub struct Resolved {
    _registry: CardRegistry,
    pub state: GameState,
    pub spell: ObjectId,
    pub target: Target,
    pub before: Snapshot,
}

impl Resolved {
    /// Nothing changed but by the actions the card asked for, which the
    /// recorders took and did not perform.
    pub fn nothing_else_changed(&self) -> bool {
        Snapshot::of(&self.state) == self.before
    }
}

/// `card`, cast by player 0, resolving on a board of one other card of any
/// characteristics `kinds` allows ([`any_object`]), after `prepare`
/// has set what the card's text reads (a creature having died this turn,
/// the X it was cast with), holding the target `target` picks from that
/// object and the spell.
pub fn resolve(
    card: Box<dyn CardBehavior>,
    kinds: &Kinds,
    prepare: impl FnOnce(&mut GameState, ObjectId),
    target: impl FnOnce(ObjectId, ObjectId) -> Target,
) -> Resolved {
    let (registry, mut state, blank, card) = board(card);
    let other = any_object(&mut state, blank, kinds, Form::Card);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    prepare(&mut state, spell);
    let target = target(other, spell);
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[target.clone()], &registry);
    Resolved { _registry: registry, state, spell, target, before }
}

/// What a spell that destroys its target asks for as it resolves: that
/// target destroyed, if it is still a permanent (CR 701.8a); with its
/// target gone, a card in a graveyard or illegal, nothing (400.7, 608.2b).
pub fn destroys_its_target(r: &Resolved) -> bool {
    match r.target {
        Target::Object(id) if r.state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield) => asked_for(&[Act::Destroy(id)]),
        _ => acted() == 0,
    }
}

/// What a spell that deals `amount` damage to its target asks for as it
/// resolves: that damage, dealt by the spell, to that target, whatever it
/// is (CR 120.1); a target gone illegal is the action's to ignore
/// (`proofs::actions` proves it does).
pub fn deals_damage_to_its_target(r: &Resolved, amount: u32) -> bool {
    asked_for(&[Act::Effect(r.target.clone(), PendingEffect::DealDamage { amount, source_id: r.spell })])
}

/// Characteristics that no damage or destroy spell's resolution reads, for
/// the object it may hold: proved for any of them all the same.
pub const ANY_PERMANENT: Kinds = Kinds {
    types: &[CardType::Creature, CardType::Artifact, CardType::Enchantment],
    subtypes: &["Spirit"],
    keywords: &[Keyword::Hexproof, Keyword::Indestructible],
};
