//! CR 704.5m/n: an attachment whose HOST IS STILL THERE but can no longer
//! legally hold it.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::invariants::check_settled;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::{GameState, TemporaryEffect};
use mtg_engine::types::*;

/// "Enchant creature" — `TargetRequirement::Creature`, no filter.
const AURA: &str = "Holy Strength";
/// An Equipment, so the 704.5n half is the same three states.
const EQUIPMENT: &str = "Butcher's Cleaver";

/// The Aura and the Equipment are OWNED by P1 and sit on P0's side of the
/// board, so "its owner's graveyard" is a different pile from the host
/// controller's and the assertion can tell them apart.
const ATTACHMENT_OWNER: PlayerId = P1;

/// Put `name` onto the battlefield attached to `host` (or to nothing).
fn attachment_on(
    state: &mut GameState,
    reg: &CardRegistry,
    name: &str,
    host: Option<ObjectId>,
) -> ObjectId {
    let id = named_permanent(state, reg, name, ATTACHMENT_OWNER);
    state.get_object_mut(id).unwrap().attached_to = host;
    id
}

/// Give `host` protection from the card type `ty` until end of turn — the
/// shape of grant the set does not yet print, and the one that makes an
/// attachment on a live host illegal (CR 702.16c).
fn grant_protection_from(state: &mut GameState, host: ObjectId, ty: CardType) {
    state.until_end_of_turn.push(TemporaryEffect::GrantProtection {
        target: host,
        filter: CreatureFilter::HasCardType(ty),
    });
}

/// A board the checker is otherwise happy with, so the only violation it
/// reports is the one the test built. `game_at_step` leaves the turn counter
/// at 1 with the first-turn flag off, which is not a state a game can reach.
fn settled_board() -> GameState {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    state
}

/// What `check_settled` says about this state, for the failure message: the
/// point of each test is that the two disagree.
fn oracle(state: &GameState, reg: &CardRegistry) -> String {
    let v = check_settled(state, reg);
    if v.is_empty() { "(no violations)".into() } else { v.join("; ") }
}

#[track_caller]
fn assert_in_owners_graveyard(state: &GameState, reg: &CardRegistry, id: ObjectId, what: &str) {
    let zone = state.get_object(id).unwrap().zone;
    assert_eq!(zone, Zone::Graveyard,
        "{what}: CR 704.5m puts it into its owner's graveyard, but it is in {zone:?}. \
         check_settled says: {}", oracle(state, reg));
    assert!(state.get_player(ATTACHMENT_OWNER).graveyard_order.contains(&id),
        "{what}: it must be in its OWNER's graveyard pile (CR 704.5m), \
         p{}'s pile is {:?}", ATTACHMENT_OWNER.0,
        state.get_player(ATTACHMENT_OWNER).graveyard_order);
}

#[track_caller]
fn assert_unattached_on_battlefield(
    state: &GameState, reg: &CardRegistry, id: ObjectId, what: &str,
) {
    let obj = state.get_object(id).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield,
        "{what}: CR 704.5n leaves an Equipment on the battlefield, it is in {:?}. \
         check_settled says: {}", obj.zone, oracle(state, reg));
    assert_eq!(obj.attached_to, None,
        "{what}: CR 704.5n unattaches it, it is still on #{}. check_settled says: {}",
        obj.attached_to.map_or(0, |h| h.0), oracle(state, reg));
}

// -------------------------------------------------------------------------
// The Aura: CR 704.5m — owner's graveyard, all three states
// -------------------------------------------------------------------------

/// CR 704.5m. "Enchant creature" is an ability of the Aura (CR 702.5a), so a
/// land is an illegal object for it: the Aura is put into its owner's
/// graveyard even though the land is alive and well.
#[test]
fn aura_on_a_noncreature_host_goes_to_its_owners_graveyard() {
    let reg = registry();
    let mut state = settled_board();

    let host = named_permanent(&mut state, &reg, "Forest", P0);
    let aura = attachment_on(&mut state, &reg, AURA, Some(host));

    assert!(!state.is_creature(host, &reg), "the host must be a non-creature for this test");
    assert_eq!(state.get_object(host).unwrap().zone, Zone::Battlefield,
        "the host is ALIVE — this is the case the sweep never asks about");

    check_state_based_actions(&mut state, &reg);

    assert_in_owners_graveyard(&state, &reg, aura,
        "an \"enchant creature\" Aura attached to a land");
}

/// CR 702.16c: an Aura can't enchant a permanent with protection from it, and
/// CR 704.5m sweeps one that already is. The grant is built by hand because
/// no card in the set prints protection a noncreature permanent can match.
#[test]
fn aura_on_a_host_with_protection_from_it_goes_to_its_owners_graveyard() {
    let reg = registry();
    let mut state = settled_board();

    let host = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let aura = attachment_on(&mut state, &reg, AURA, Some(host));
    grant_protection_from(&mut state, host, CardType::Enchantment);

    assert!(state.has_protection_from(host, aura, &reg),
        "the grant has to actually take, or this test proves nothing");

    check_state_based_actions(&mut state, &reg);

    assert_in_owners_graveyard(&state, &reg, aura,
        "an Aura on a creature with protection from enchantments");
}

/// CR 704.5m's other clause: "or is not attached to an object or player".
/// This is the case `sba.rs` names in its comment and excludes in its filter
/// (`o.attached_to.is_some()`), and `invariants::check_settled` reports as
/// "Aura … on the battlefield unattached".
#[test]
fn aura_attached_to_nothing_goes_to_its_owners_graveyard() {
    let reg = registry();
    let mut state = settled_board();

    let aura = attachment_on(&mut state, &reg, AURA, None);
    let obj = state.get_object(aura).unwrap();
    assert_eq!((obj.attached_to, obj.attached_to_player), (None, None),
        "attached to neither an object nor a player");

    check_state_based_actions(&mut state, &reg);

    assert_in_owners_graveyard(&state, &reg, aura,
        "an Aura on the battlefield attached to nothing");
}

// -------------------------------------------------------------------------
// The Equipment: CR 704.5n — unattached, and it stays
// -------------------------------------------------------------------------

/// CR 704.5n: an Equipment attached to an illegal permanent becomes
/// unattached and remains on the battlefield. A land is illegal — "equipped
/// creature" is the whole point — and `invariants::check_settled` already
/// reports this state as "Equipment … attached to non-creature".
#[test]
fn equipment_on_a_noncreature_host_is_unattached_and_stays() {
    let reg = registry();
    let mut state = settled_board();

    let host = named_permanent(&mut state, &reg, "Forest", P0);
    let equip = attachment_on(&mut state, &reg, EQUIPMENT, Some(host));
    assert!(!state.is_creature(host, &reg), "the host must be a non-creature for this test");

    check_state_based_actions(&mut state, &reg);

    assert_unattached_on_battlefield(&state, &reg, equip,
        "an Equipment attached to a land");
}

/// CR 702.16d/704.5n: an Equipment on a permanent with protection from it
/// comes off and stays on the battlefield.
#[test]
fn equipment_on_a_host_with_protection_from_it_is_unattached_and_stays() {
    let reg = registry();
    let mut state = settled_board();

    let host = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let equip = attachment_on(&mut state, &reg, EQUIPMENT, Some(host));
    grant_protection_from(&mut state, host, CardType::Artifact);

    assert!(state.has_protection_from(host, equip, &reg),
        "the grant has to actually take, or this test proves nothing");

    check_state_based_actions(&mut state, &reg);

    assert_unattached_on_battlefield(&state, &reg, equip,
        "an Equipment on a creature with protection from artifacts");
}

/// The control, and the asymmetry the Aura tests are measured against: an
/// Equipment attached to nothing is a perfectly legal permanent (CR 301.5c).
/// It is not swept, not moved, and not touched.
#[test]
fn equipment_attached_to_nothing_is_left_alone() {
    let reg = registry();
    let mut state = settled_board();

    let equip = attachment_on(&mut state, &reg, EQUIPMENT, None);

    check_state_based_actions(&mut state, &reg);

    assert_unattached_on_battlefield(&state, &reg, equip,
        "an unattached Equipment");
    assert!(state.get_player(ATTACHMENT_OWNER).graveyard_order.is_empty(),
        "an unattached Equipment is legal and must not be swept (CR 704.5n)");
}

// -------------------------------------------------------------------------
// The property the two halves of this now share
// -------------------------------------------------------------------------

/// The engine and its own oracle answer one question, not two.
///
/// Each case above pins what happens to one attachment; this pins the
/// relationship, which is what actually went wrong (#552): `sba.rs` and
/// `invariants::check_settled` were two independent readings of CR 704.5m/n,
/// and the fuzzer's only oracle reported violations on boards the engine had
/// declared settled. Whatever cases the predicate grows, a board the sweep
/// has finished with is a board the checker is silent about.
///
/// Written over the states rather than over the wording, so a message may be
/// rephrased freely and a case may be added to `attachment::illegality`
/// without touching this test — it will simply be covered.
#[test]
fn a_board_the_sweep_has_settled_is_a_board_the_oracle_is_silent_about() {
    let reg = registry();

    // Every illegal attachment state the predicate can name, built the way a
    // future card would reach it, plus the legal ones for contrast.
    let boards: Vec<(&str, fn(&mut GameState, &CardRegistry))> = vec![
        ("an \"enchant creature\" Aura on a land", |s, r| {
            let host = named_permanent(s, r, "Forest", P0);
            attachment_on(s, r, AURA, Some(host));
        }),
        ("an Aura on a host with protection from it", |s, r| {
            let host = named_permanent(s, r, "Grizzly Bears", P0);
            attachment_on(s, r, AURA, Some(host));
            grant_protection_from(s, host, CardType::Enchantment);
        }),
        ("an Aura attached to nothing", |s, r| {
            attachment_on(s, r, AURA, None);
        }),
        ("an Aura whose host has left the battlefield", |s, r| {
            let host = named_permanent(s, r, "Grizzly Bears", P0);
            attachment_on(s, r, AURA, Some(host));
            // Exiled rather than killed: a creature in the graveyard that
            // did not die is its own violation (CR 700.4), and this board is
            // asking about the Aura.
            s.move_object(host, Zone::Exile, r);
        }),
        ("an Equipment on a land", |s, r| {
            let host = named_permanent(s, r, "Forest", P0);
            attachment_on(s, r, EQUIPMENT, Some(host));
        }),
        ("an Equipment on a host with protection from it", |s, r| {
            let host = named_permanent(s, r, "Grizzly Bears", P0);
            attachment_on(s, r, EQUIPMENT, Some(host));
            grant_protection_from(s, host, CardType::Artifact);
        }),
        ("an Equipment attached to a player", |s, r| {
            let id = named_permanent(s, r, EQUIPMENT, ATTACHMENT_OWNER);
            s.get_object_mut(id).unwrap().attached_to_player = Some(P0);
        }),
        ("an Equipment attached to nothing", |s, r| {
            attachment_on(s, r, EQUIPMENT, None);
        }),
        ("an Aura legally on a creature", |s, r| {
            let host = named_permanent(s, r, "Grizzly Bears", P0);
            attachment_on(s, r, AURA, Some(host));
        }),
        ("an Equipment legally on a creature", |s, r| {
            let host = named_permanent(s, r, "Grizzly Bears", P0);
            attachment_on(s, r, EQUIPMENT, Some(host));
        }),
        ("a Curse legally on a player", |s, r| {
            let id = named_permanent(s, r, "Curse of the Pierced Heart", ATTACHMENT_OWNER);
            s.get_object_mut(id).unwrap().attached_to_player = Some(P0);
        }),
    ];

    for (what, build) in boards {
        let mut state = settled_board();
        build(&mut state, &reg);

        // To a fixed point, the way the engine runs them (CR 704.3).
        for _ in 0..16 {
            if !check_state_based_actions(&mut state, &reg) {
                break;
            }
        }

        assert!(!check_state_based_actions(&mut state, &reg),
            "{what}: the sweep never reaches a fixed point");

        // The engine's trigger collector runs between the sweep and the next
        // decision point; nothing here is a trigger, so standing in for it is
        // just marking the events read (CR 603.3). Without this the checker
        // reports the unscanned events and says nothing about attachments,
        // which is not the question this test is asking.
        state.trigger_event_index = state.events.len();

        assert_eq!(check_settled(&state, &reg), Vec::<String>::new(),
            "{what}: the sweep says this board is settled and the oracle says it is not");
    }
}
