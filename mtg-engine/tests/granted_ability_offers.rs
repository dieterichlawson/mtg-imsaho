//! Two copies of one aura on one creature are two abilities and one offer.
//!
//! CR 113.8 is clear that two Skeletal Grimaces on one Walking Corpse grant
//! it two separate "{B}: Regenerate this creature" abilities. Nothing tells
//! them apart, though: an `ActivateAbility` names the permanent, the ability
//! index and the granting card, so both grants built the byte-identical
//! action and `legal.actions` carried it twice. `legal.activatable_abilities`
//! collapsed the pair, so the two halves of `LegalActions` disagreed about
//! how many abilities were on offer, `distinct_offers` ("a menu, not a
//! multiset") failed on 3 of 5 random-vs-random seeds, and the duplicate row
//! made a random seat twice as likely to regenerate as to do anything else
//! (issue #533).
//!
//! The menu collapses them; the game does not. Two Grimaces are still two
//! activations and two shields.

mod common;
use common::*;

use mtg_engine::actions::Action;
use mtg_engine::ids::{CardId, ObjectId};
use mtg_engine::invariants::check_legal;
use mtg_engine::types::*;

/// A Walking Corpse with `n` Skeletal Grimaces on it and enough Swamps to
/// activate, which is the board the fuzzer found.
fn corpse_under_grimaces(n: usize) -> (mtg_engine::state::GameState, mtg_engine::cards::CardRegistry, ObjectId) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..4 {
        named_permanent(&mut state, &reg, "Swamp", P0);
    }
    let corpse = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    for _ in 0..n {
        let aura = named_permanent(&mut state, &reg, "Skeletal Grimace", P0);
        state.get_object_mut(aura).unwrap().attached_to = Some(corpse);
    }
    state.priority_player = Some(P0);
    (state, reg, corpse)
}

/// Every `ActivateAbility` the menu offers for `creature`, granted by `source`.
fn offers_granted_by(
    state: &mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    creature: ObjectId,
    source: CardId,
) -> Vec<Action> {
    mtg_engine::engine::legal_actions(state, reg).actions.iter()
        .filter(|a| matches!(a,
            Action::ActivateAbility { object_id, source_card_id: Some(src), .. }
                if *object_id == creature && *src == source))
        .cloned()
        .collect()
}

fn grimace_id(reg: &mtg_engine::cards::CardRegistry) -> CardId {
    reg.get_id_by_name("Skeletal Grimace").expect("Skeletal Grimace is in the set")
}

/// One aura, one offer — the baseline the collapsed case has to match.
#[test]
fn one_grimace_offers_its_regenerate_once() {
    let (state, reg, corpse) = corpse_under_grimaces(1);
    assert_eq!(offers_granted_by(&state, &reg, corpse, grimace_id(&reg)).len(), 1,
        "a single Skeletal Grimace grants one regenerate offer");
}

/// Two auras granting the same ability to the same creature are one row of
/// the menu, not two identical rows.
#[test]
fn two_grimaces_on_one_creature_offer_regenerate_once() {
    let (state, reg, corpse) = corpse_under_grimaces(2);
    let offers = offers_granted_by(&state, &reg, corpse, grimace_id(&reg));
    assert_eq!(offers.len(), 1,
        "two Grimaces grant two abilities (CR 113.8) but one indistinguishable offer; got {offers:#?}");
}

/// And the invariant that caught it in the fuzzer stays quiet — over the
/// whole menu, not just the rows this test went looking for.
#[test]
fn a_creature_under_two_grimaces_offers_no_action_twice() {
    for n in [2, 3, 4] {
        let (state, reg, _) = corpse_under_grimaces(n);
        let legal = mtg_engine::engine::legal_actions(&state, &reg);
        let violations = check_legal(&state, P0, &legal, &reg);
        assert!(violations.is_empty(),
            "{n} Grimaces on one creature: {violations:?}");
    }
}

/// The summary list and the action list agree about how many abilities are
/// on offer. They disagreed before: `activatable_abilities` collapsed the
/// pair and `actions` did not.
#[test]
fn the_two_halves_of_the_menu_count_the_same_abilities() {
    let (state, reg, corpse) = corpse_under_grimaces(2);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let grimace = grimace_id(&reg);
    let summarised = legal.activatable_abilities.iter()
        .filter(|a| a.object_id == corpse && a.source_card_id == Some(grimace))
        .count();
    assert_eq!(summarised, offers_granted_by(&state, &reg, corpse, grimace).len(),
        "activatable_abilities and actions describe the same menu");
}

/// Two different granting cards are two different offers — the collapse is
/// keyed on the granting card, not on "anything attached".
#[test]
fn two_different_granting_cards_are_two_offers() {
    let (mut state, reg, corpse) = corpse_under_grimaces(1);
    let torch = named_permanent(&mut state, &reg, "Blazing Torch", P0);
    state.get_object_mut(torch).unwrap().attached_to = Some(corpse);

    let granting: std::collections::HashSet<CardId> =
        mtg_engine::engine::legal_actions(&state, &reg).actions.iter()
            .filter_map(|a| match a {
                Action::ActivateAbility { object_id, source_card_id: Some(src), .. }
                    if *object_id == corpse => Some(*src),
                _ => None,
            })
            .collect();
    let torch_id = reg.get_id_by_name("Blazing Torch").expect("Blazing Torch is in the set");
    assert!(granting.contains(&grimace_id(&reg)), "the aura's regenerate is still offered");
    assert!(granting.contains(&torch_id), "and the equipment's ability beside it");
}

/// The rules half, which the collapse must not touch: two Grimaces really do
/// grant two abilities, so the creature can be regenerated twice and carries
/// two shields. Offering the row once is a statement about the menu, not
/// about how many times it may be taken.
#[test]
fn a_creature_under_two_grimaces_can_still_regenerate_twice() {
    let (mut state, reg, corpse) = corpse_under_grimaces(2);
    add_mana(&mut state, P0, &[(ManaType::Black, 2)]);
    let shields = |s: &mtg_engine::state::GameState| {
        s.get_object(corpse).expect("the Corpse is still there").regeneration_shields
    };

    let state = activate_offered(&state, &reg, corpse, None);
    assert_eq!(shields(&state), 1, "the first activation made a shield");

    // The offer is still there once it has been taken — it is not a
    // once-per-turn ability, and collapsing the menu did not remove one.
    assert_eq!(offers_granted_by(&state, &reg, corpse, grimace_id(&reg)).len(), 1,
        "and the row is still on the menu");

    let state = activate_offered(&state, &reg, corpse, None);
    assert_eq!(shields(&state), 2,
        "the second Grimace's ability is a second ability, and a second shield");
}
