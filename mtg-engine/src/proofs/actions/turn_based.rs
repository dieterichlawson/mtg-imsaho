//! The turn-based actions of each step (`engine::perform_turn_based_actions`,
//! CR 502-514), what the rest of the cleanup step does once a hand is down
//! to size (`engine::finish_cleanup`), and the discard that answers the
//! cleanup's question (`actions::simple::discard_cards`). Which step comes
//! next is `mtg_kernel::turn::next_step`, proved in Verus; untapping,
//! drawing, discarding and the parts of the cleanup each stand in as
//! recorded, each its own proof.

use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_each, asked_for, card_proof, Act, Blank};
use crate::state::{AwaitingAction, CombatState, GameState};
use crate::types::{Step, Zone};

/// Whether each object (1 to 3) is kept from untapping: any answer.
static mut LOCKED: [Option<bool>; 3] = [None; 3];

fn index(id: ObjectId) -> usize {
    let i = usize::try_from(id.0).expect("an id of the board");
    assert!((1..=3).contains(&i), "an object this proof does not make");
    i - 1
}

/// Stands in for `GameState::untaps_normally`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn untaps_normally(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    // SAFETY: a proof runs on one thread, and only this function names LOCKED.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(LOCKED) };
    !*answers[index(id)].get_or_insert_with(kani::any::<bool>)
}

/// A game in `step` with either player active.
fn game(step: Step) -> (CardRegistry, crate::ids::CardId, GameState, PlayerId) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let active = PlayerId(u8::from(kani::any::<bool>()));
    state.active_player = active;
    state.step = step;
    state.priority_player = None;
    (registry, blank, state, active)
}

card_proof! {
    #[kani::stub(crate::state::GameState::untaps_normally, untaps_normally)]
    /// The untap step (CR 502.2-502.4, 302.6, 305.2): each permanent the
    /// active player controls that is tapped and untaps normally is
    /// untapped, in the board's order, and nothing else; each creature
    /// they control is no longer new to their control; they may play a
    /// land again; and nobody receives priority. For three permanents,
    /// each either player's, on the battlefield or in a graveyard, tapped
    /// or not, new to its controller or not, and kept from untapping or
    /// not (any answer).
    fn the_untap_step_untaps_the_active_players_permanents_but_those_kept_tapped() {
        let (registry, blank, mut state, active) = game(Step::Untap);
        let (mut on, mut mine, mut tapped, mut sick) = ([false; 3], [false; 3], [false; 3], [false; 3]);
        for i in 0..3 {
            on[i] = kani::any();
            mine[i] = kani::any();
            let controller = if mine[i] { active } else { state.opponent(active) };
            let zone = if on[i] { Zone::Battlefield } else { Zone::Graveyard };
            let id = state.create_object(blank, controller, zone, Some(1), Some(1));
            (tapped[i], sick[i]) = (kani::any(), kani::any());
            let o = state.get_object_mut(id).expect("just created");
            (o.tapped, o.summoning_sick) = (tapped[i], sick[i]);
        }
        let other = state.opponent(active);
        state.get_player_mut(active).land_plays_remaining = 0;
        state.get_player_mut(other).land_plays_remaining = 0;

        crate::engine::perform_turn_based_actions(&mut state, &registry);

        let mut untapped = [ObjectId(0); 3];
        let mut n = 0;
        for i in 0..3 {
            let id = ObjectId(i as u64 + 1);
            if on[i] && mine[i] && tapped[i] && untaps_normally(&state, id, &registry) {
                untapped[n] = id;
                n += 1;
            }
            let o = state.get_object(id).expect("on the board");
            assert!(o.summoning_sick == (sick[i] && !(on[i] && mine[i])));
        }
        assert!(asked_each(n, |k| Act::Untap(untapped[k])));
        assert!(state.get_player(active).land_plays_remaining == 1 && state.get_player(other).land_plays_remaining == 0);
        assert!(state.priority_player.is_none() && state.awaiting_action.is_none());
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// The draw step (CR 504.1-504.2): the active player draws a card, and
    /// then receives priority.
    fn the_draw_step_draws_the_active_player_a_card() {
        let (registry, _, mut state, active) = game(Step::Draw);
        crate::engine::perform_turn_based_actions(&mut state, &registry);
        assert!(asked_for(&[Act::Draw(active, 1)]));
        assert!(state.priority_player == Some(active) && state.awaiting_action.is_none());
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// The declare attackers step (CR 508.1, 508.2): the active player is
    /// asked to declare attackers, and holds priority after; nothing else
    /// is done.
    fn the_declare_attackers_step_asks_the_active_player_to_declare() {
        let (registry, _, mut state, active) = game(Step::DeclareAttackers);
        crate::engine::perform_turn_based_actions(&mut state, &registry);
        assert!(matches!(state.awaiting_action, Some(AwaitingAction::DeclareAttackers)));
        assert!(state.priority_player == Some(active) && acted() == 0);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// The declare blockers step (CR 509.1, 508.8): with a creature
    /// attacking, the defending player is asked to declare blockers, and
    /// holds priority for it; with none (the step reached with every
    /// attacker gone), nothing is asked and the active player receives
    /// priority.
    fn the_declare_blockers_step_asks_the_defending_player_when_a_creature_attacks() {
        let (registry, blank, mut state, active) = game(Step::DeclareBlockers);
        let defending = state.opponent(active);
        let attacking: bool = kani::any();
        let mut combat = CombatState::default();
        if attacking {
            let id = state.create_object(blank, active, Zone::Battlefield, Some(1), Some(1));
            combat.attackers.insert(id, defending);
        }
        state.combat = Some(combat);

        crate::engine::perform_turn_based_actions(&mut state, &registry);

        if attacking {
            assert!(matches!(state.awaiting_action, Some(AwaitingAction::DeclareBlockers { defending_player }) if defending_player == defending));
            assert!(state.priority_player == Some(defending));
        } else {
            assert!(state.awaiting_action.is_none() && state.priority_player == Some(active));
        }
        assert!(acted() == 0);
        std::mem::forget((registry, state));
    }
}

/// The combat damage steps taken, and whether each was of first strikers
/// only.
static mut DAMAGE_STEPS: [bool; 2] = [false; 2];
static mut DAMAGE_STEPS_N: usize = 0;
static mut FIRST_STRIKE: Option<bool> = None;

/// Stands in for `combat::combat_damage_step`: recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn damage_step(_: &mut GameState, _: &CardRegistry, first_strike_only: bool) {
    // SAFETY: as in `untaps_normally`, for DAMAGE_STEPS and DAMAGE_STEPS_N.
    let (steps, n) = unsafe { (&mut *std::ptr::addr_of_mut!(DAMAGE_STEPS), &mut *std::ptr::addr_of_mut!(DAMAGE_STEPS_N)) };
    steps[*n] = first_strike_only;
    *n += 1;
}

/// Stands in for `combat::any_first_strike_in_combat`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn any_first_strike(_: &GameState, _: &CardRegistry) -> bool {
    // SAFETY: as in `untaps_normally`, for FIRST_STRIKE.
    *unsafe { &mut *std::ptr::addr_of_mut!(FIRST_STRIKE) }.get_or_insert_with(kani::any::<bool>)
}

card_proof! {
    #[kani::stub(crate::combat::combat_damage_step, damage_step)]
    #[kani::stub(crate::combat::any_first_strike_in_combat, any_first_strike)]
    /// A combat damage step (CR 510.1-510.5): with a creature in combat, a
    /// second step deals the regular damage; a first with a first or
    /// double striker in combat deals only theirs, and a second step is
    /// then due; a first with none deals all of it, and no second is due.
    /// With none in combat, no damage is dealt and no second step is due.
    /// Either way the active player receives priority after. For a second
    /// step due or not, a creature in combat or not, and a first striker
    /// among them or not (any answer).
    fn a_combat_damage_step_deals_first_strike_damage_then_the_rest() {
        let (registry, blank, mut state, active) = game(Step::CombatDamage);
        let second: bool = kani::any();
        state.combat_damage_step_pending = second;
        let fighting: bool = kani::any();
        let mut combat = CombatState::default();
        if fighting {
            let id = state.create_object(blank, active, Zone::Battlefield, Some(1), Some(1));
            combat.attackers.insert(id, state.opponent(active));
        }
        state.combat = Some(combat);

        crate::engine::perform_turn_based_actions(&mut state, &registry);

        // SAFETY: as in `untaps_normally`.
        let (steps, n) = unsafe { (*std::ptr::addr_of!(DAMAGE_STEPS), *std::ptr::addr_of!(DAMAGE_STEPS_N)) };
        let first_strikers = fighting && !second && any_first_strike(&state, &registry);
        if fighting {
            assert!(n == 1 && steps[0] == first_strikers);
        } else {
            assert!(n == 0);
        }
        assert!(state.combat_damage_step_pending == first_strikers);
        assert!(state.priority_player == Some(active) && state.awaiting_action.is_none());
        std::mem::forget((registry, state));
    }
}

/// Whether combat was ended, and the rest of the cleanup step taken.
static mut ENDED: usize = 0;

#[allow(dead_code)] // named only by `kani::stub`
fn combat_ended(_: &mut GameState, _: &CardRegistry) {
    // SAFETY: as in `untaps_normally`, for ENDED.
    unsafe { *std::ptr::addr_of_mut!(ENDED) += 1 };
}

card_proof! {
    #[kani::stub(crate::combat::end_combat, combat_ended)]
    /// The end of combat step (CR 511.1-511.3): the active player receives
    /// priority, and combat goes on through the step: the creatures leave
    /// it as the step ends (`advance_step`'s proof), not as it begins.
    fn the_end_of_combat_step_gives_priority_with_the_creatures_still_in_combat() {
        let (registry, _, mut state, active) = game(Step::EndCombat);
        crate::engine::perform_turn_based_actions(&mut state, &registry);
        // SAFETY: as in `untaps_normally`.
        assert!(unsafe { *std::ptr::addr_of!(ENDED) } == 0);
        assert!(state.priority_player == Some(active) && state.awaiting_action.is_none() && acted() == 0);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// The steps with no turn-based action of their own (CR 503, 505,
    /// 507, 513): the active player receives priority, and nothing else is
    /// done. For each of them.
    fn a_step_with_no_turn_based_action_gives_the_active_player_priority() {
        let step = match kani::any::<u8>() % 5 {
            0 => Step::Upkeep,
            1 => Step::PrecombatMain,
            2 => Step::BeginCombat,
            3 => Step::PostcombatMain,
            _ => Step::EndStep,
        };
        let (registry, _, mut state, active) = game(step);
        crate::engine::perform_turn_based_actions(&mut state, &registry);
        assert!(state.priority_player == Some(active) && state.awaiting_action.is_none() && acted() == 0);
        std::mem::forget((registry, state));
    }
}

#[allow(dead_code)] // named only by `kani::stub`
fn cleanup_finished(_: &mut GameState, _: &CardRegistry) {
    // SAFETY: as in `untaps_normally`, for ENDED.
    unsafe { *std::ptr::addr_of_mut!(ENDED) += 1 };
}

// Eight cards made, and the hand counted, unwound past them.
card_proof! {
    unwind(10)
    #[kani::stub(crate::engine::finish_cleanup, cleanup_finished)]
    /// The cleanup step begins with the hand-size discard (CR 514.1): with
    /// more than seven cards in the active player's hand, they are asked to
    /// discard down to seven, with nobody holding priority, and the rest
    /// of the step waits on the answer; with seven or fewer, the rest of
    /// the step is taken at once. For eight cards, each in the active
    /// player's hand or not.
    fn the_cleanup_step_asks_for_the_discard_before_anything_else() {
        let (registry, blank, mut state, active) = game(Step::Cleanup);
        let mut in_hand = 0;
        for _ in 0..8 {
            let held: bool = kani::any();
            state.create_object(blank, active, if held { Zone::Hand } else { Zone::Graveyard }, None, None);
            in_hand += usize::from(held);
        }

        crate::engine::perform_turn_based_actions(&mut state, &registry);

        // SAFETY: as in `untaps_normally`.
        let finished = unsafe { *std::ptr::addr_of!(ENDED) };
        if in_hand > 7 {
            assert!(matches!(state.awaiting_action, Some(AwaitingAction::DiscardToHandSize { player, discard_count })
                if player == active && discard_count == in_hand - 7));
            assert!(finished == 0 && state.priority_player.is_none());
        } else {
            assert!(finished == 1 && state.awaiting_action.is_none());
        }
        assert!(acted() == 0);
        std::mem::forget((registry, state));
    }
}

/// The parts of the rest of the cleanup step, in the order taken: 1 the
/// turn's effects ended, 2 the mana pools emptied, 3 state-based actions
/// checked.
static mut PARTS: [u8; 3] = [0; 3];
static mut PARTS_N: usize = 0;
static mut SBA: Option<bool> = None;

fn part(n: u8) {
    // SAFETY: as in `untaps_normally`, for PARTS and PARTS_N.
    let (parts, k) = unsafe { (&mut *std::ptr::addr_of_mut!(PARTS), &mut *std::ptr::addr_of_mut!(PARTS_N)) };
    parts[*k] = n;
    *k += 1;
}

#[allow(dead_code)] // each named only by `kani::stub`
fn effects_ended(_: &mut GameState) {
    part(1);
}
#[allow(dead_code)]
fn pools_emptied(_: &mut GameState) {
    part(2);
}
#[allow(dead_code)]
fn sba_checked(_: &mut GameState, _: &CardRegistry) -> bool {
    part(3);
    // SAFETY: as in `untaps_normally`, for SBA.
    *unsafe { &mut *std::ptr::addr_of_mut!(SBA) }.get_or_insert_with(kani::any::<bool>)
}

card_proof! {
    #[kani::stub(crate::engine::end_until_end_of_turn_effects, effects_ended)]
    #[kani::stub(crate::engine::empty_mana_pools, pools_emptied)]
    #[kani::stub(crate::sba::check_state_based_actions, sba_checked)]
    /// The rest of the cleanup step (CR 514.2-514.3a): the damage marked
    /// on each permanent is removed, with the record of what dealt it and
    /// whether any had deathtouch, and its regeneration shields; the
    /// turn's "until end of turn" effects end (`cleanup`'s proofs show
    /// what that does); the mana pools empty; and then state-based actions
    /// are checked: if any was performed, the active player receives
    /// priority and another cleanup step follows, and otherwise nobody
    /// receives priority. Nothing off the battlefield is touched. For three
    /// objects, each on the battlefield or in a graveyard, with damage
    /// marked or not, from a deathtouch source or not, a record of what
    /// dealt it or not and a regeneration shield or not, and whether a
    /// state-based action is performed any answer.
    fn the_rest_of_the_cleanup_step_removes_damage_ends_the_turns_effects_then_checks() {
        let (registry, blank, mut state, active) = game(Step::Cleanup);
        let mut was = [(false, 0u32, false, false, 0u32); 3];
        for w in &mut was {
            let on: bool = kani::any();
            let id = state.create_object(blank, active, if on { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
            let (damage, deathtouch, dealt, shields): (bool, bool, bool, bool) = (kani::any(), kani::any(), kani::any(), kani::any());
            let o = state.get_object_mut(id).expect("just created");
            o.damage_marked = u32::from(damage);
            o.dealt_deathtouch_damage = deathtouch;
            if dealt {
                o.damaged_by.push(ObjectId(9));
            }
            o.regeneration_shields = u32::from(shields);
            *w = (on, u32::from(damage), deathtouch, dealt, u32::from(shields));
        }

        crate::engine::finish_cleanup(&mut state, &registry);

        for (i, &(on, damage, deathtouch, dealt, shields)) in was.iter().enumerate() {
            let o = state.get_object(ObjectId(i as u64 + 1)).expect("on the board");
            if on {
                assert!(o.damage_marked == 0 && !o.dealt_deathtouch_damage && o.damaged_by.is_empty() && o.regeneration_shields == 0);
            } else {
                assert!(o.damage_marked == damage && o.dealt_deathtouch_damage == deathtouch
                    && o.damaged_by.is_empty() != dealt && o.regeneration_shields == shields);
            }
        }
        // SAFETY: as in `untaps_normally`.
        let (parts, n, sba) = unsafe { (*std::ptr::addr_of!(PARTS), *std::ptr::addr_of!(PARTS_N), *std::ptr::addr_of!(SBA)) };
        assert!(n == 3 && parts == [1, 2, 3]);
        if sba == Some(true) {
            assert!(state.priority_player == Some(active) && state.cleanup_repeat);
        } else {
            assert!(state.priority_player.is_none() && !state.cleanup_repeat);
        }
        std::mem::forget((registry, state));
    }
}

card_proof! {
    #[kani::stub(crate::engine::finish_cleanup, cleanup_finished)]
    /// Discarding (`discard_cards`, CR 514.1, 701.8): each card named is
    /// discarded, in order, and the question answered; as the answer to
    /// the cleanup step's discard, the rest of the step is then taken, and
    /// otherwise it is not. For two cards and the cleanup's question
    /// waiting or none.
    fn a_discard_to_hand_size_goes_on_to_the_rest_of_the_cleanup_step() {
        let (registry, blank, mut state, active) = game(Step::Cleanup);
        let cards = [
            state.create_object(blank, active, Zone::Hand, None, None),
            state.create_object(blank, active, Zone::Hand, None, None),
        ];
        let cleanup: bool = kani::any();
        if cleanup {
            state.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: active, discard_count: 2 });
        }

        let applied = crate::engine::actions::simple::discard_cards(&mut state, &cards, &registry);

        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(asked_for(&[Act::Discard(cards[0]), Act::Discard(cards[1])]) && state.awaiting_action.is_none());
        // SAFETY: as in `untaps_normally`.
        assert!(unsafe { *std::ptr::addr_of!(ENDED) } == usize::from(cleanup));
        std::mem::forget((registry, state));
    }
}

/// What stepping on did, in order: 1 the mana pools emptied, 2 combat
/// ended, 3 the new step's turn-based actions.
static mut STEPPED: [u8; 3] = [0; 3];
static mut STEPPED_N: usize = 0;

fn stepped(n: u8) {
    // SAFETY: as in `untaps_normally`, for STEPPED and STEPPED_N.
    let (s, k) = unsafe { (&mut *std::ptr::addr_of_mut!(STEPPED), &mut *std::ptr::addr_of_mut!(STEPPED_N)) };
    s[*k] = n;
    *k += 1;
}

#[allow(dead_code)] // each named only by `kani::stub`
fn emptied(_: &mut GameState) {
    stepped(1);
}
#[allow(dead_code)]
fn combat_over(_: &mut GameState, _: &CardRegistry) {
    stepped(2);
}
#[allow(dead_code)]
fn step_actions(_: &mut GameState, _: &CardRegistry) {
    stepped(3);
}

// `Step::index` walks the twelve steps, unwound past them.
card_proof! {
    unwind(13)
    #[kani::stub(crate::engine::empty_mana_pools, emptied)]
    #[kani::stub(crate::combat::end_combat, combat_over)]
    #[kani::stub(crate::engine::perform_turn_based_actions, step_actions)]
    /// Going on to the next step (`advance_step`, CR 500.1-500.4, 511.3,
    /// 106.4): the mana pools empty; leaving the end of combat step, combat
    /// ends (511.3); the next step is the one the turn kernel gives
    /// (`mtg_kernel::turn::next_step`, proved against 500.1's order and the
    /// facts that change it: a second combat damage step due, a cleanup
    /// step that gave priority, no attackers declared, the first turn's
    /// draw), the question of a repeated cleanup step taken with it; past
    /// the cleanup step, the next player's turn begins, numbered one on,
    /// no longer the game's first, with no creature having died in it and
    /// no ability activated in it yet; the step's beginning is announced,
    /// after the turn's; the passes in a row start again; and the new
    /// step's turn-based actions are taken, last. For every step, either
    /// player active, a second combat damage step due or not, a cleanup
    /// step to repeat or not, attackers declared or none or no combat, and
    /// the game's first turn or not.
    fn the_game_steps_on_as_the_turn_kernel_says() {
        let (registry, blank, mut state, active) = game(Step::from_index(kani::any::<u8>() % 12));
        let from = state.step;
        let (pending, repeat, first): (bool, bool, bool) = (kani::any(), kani::any(), kani::any());
        state.combat_damage_step_pending = pending;
        state.cleanup_repeat = repeat;
        state.is_first_turn = first;
        state.turn_number = 5;
        state.creature_died_this_turn = true;
        state.consecutive_passes = 3;
        let attackers = match kani::any::<u8>() % 3 {
            0 => None,
            n => Some(n == 2),
        };
        if let Some(declared) = attackers {
            state.combat = Some(CombatState { any_attackers_declared: declared, ..CombatState::default() });
        }
        let id = state.create_object(blank, active, Zone::Battlefield, None, None);
        state.get_object_mut(id).expect("just created").abilities_activated_this_turn.insert(0);
        let events = state.events.len();
        let facts = mtg_kernel::turn::StepFacts {
            second_damage_step_due: pending,
            cleanup_repeats: from == Step::Cleanup && repeat,
            attackers_declared: attackers == Some(true),
            skips_first_draw: first,
        };
        let next = mtg_kernel::turn::next_step(from.index(), facts);

        crate::engine::advance_step(&mut state, &registry);

        // SAFETY: as in `untaps_normally`.
        let (parts, n) = unsafe { (*std::ptr::addr_of!(STEPPED), *std::ptr::addr_of!(STEPPED_N)) };
        if from == Step::EndCombat {
            assert!(n == 3 && parts == [1, 2, 3]);
        } else {
            assert!(n == 2 && parts[..2] == [1, 3]);
        }
        assert!(state.cleanup_repeat == (repeat && from != Step::Cleanup));
        let announced = &state.events[events..];
        match next {
            Some(k) => {
                assert!(state.step == Step::from_index(k) && state.active_player == active && state.turn_number == 5);
                assert!(state.is_first_turn == first && state.creature_died_this_turn);
                assert!(matches!(announced, [crate::events::GameEvent::StepStarted { step }] if *step == state.step));
            }
            None => {
                let next_player = state.opponent(active);
                assert!(state.step == Step::Untap && state.active_player == next_player && state.turn_number == 6);
                assert!(!state.is_first_turn && !state.creature_died_this_turn);
                assert!(state.get_object(id).expect("on the board").abilities_activated_this_turn.is_empty());
                assert!(matches!(announced, [
                    crate::events::GameEvent::TurnStarted { player, turn: 6 },
                    crate::events::GameEvent::StepStarted { step: Step::Untap },
                ] if *player == next_player));
            }
        }
        assert!(state.consecutive_passes == 0);
        std::mem::forget((registry, state));
    }
}

// `Step::index` walks the twelve steps, unwound past them.
card_proof! {
    unwind(13)
    #[kani::stub(crate::engine::empty_mana_pools, emptied)]
    #[kani::stub(crate::combat::end_combat, combat_over)]
    #[kani::stub(crate::engine::perform_turn_based_actions, step_actions)]
    /// The spells each player cast in a turn are, once it ends, those they
    /// cast last turn, and none are cast yet in the next: what a Werewolf's
    /// "if no spells were cast last turn" and "if a player cast two or more
    /// spells last turn" read (each spell counted as it is cast, `casting`).
    /// A cleanup step repeated (CR 514.3a) is the same turn, the counts as
    /// they were. For the first player having cast any number this turn,
    /// the second any number or none counted, and the second any number
    /// counted the turn before.
    fn the_spells_cast_in_a_turn_are_those_cast_last_turn_once_it_ends() {
        let (registry, _, mut state, _) = game(Step::Cleanup);
        let repeat: bool = kani::any();
        state.cleanup_repeat = repeat;
        let (first, second, before): (u32, u32, u32) = (kani::any(), kani::any(), kani::any());
        let second_cast: bool = kani::any();
        state.num_spells_cast_this_turn.insert(PlayerId(0), first);
        if second_cast {
            state.num_spells_cast_this_turn.insert(PlayerId(1), second);
        }
        state.num_spells_cast_last_turn.insert(PlayerId(1), before);

        crate::engine::advance_step(&mut state, &registry);

        let count = |map: &std::collections::BTreeMap<PlayerId, u32>, p: u8| map.get(&PlayerId(p)).copied();
        let (now, last) = (&state.num_spells_cast_this_turn, &state.num_spells_cast_last_turn);
        let counted = 1 + usize::from(second_cast);
        if repeat {
            assert!(state.step == Step::Cleanup);
            assert!(now.len() == counted && count(now, 0) == Some(first) && count(now, 1) == second_cast.then_some(second));
            assert!(last.len() == 1 && count(last, 1) == Some(before));
        } else {
            assert!(state.step == Step::Untap);
            assert!(now.is_empty());
            assert!(last.len() == counted && count(last, 0) == Some(first) && count(last, 1) == second_cast.then_some(second));
        }
        std::mem::forget((registry, state));
    }
}
