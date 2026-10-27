//! `engine::valid_targets_for_req`: which targets a requirement offers.
//!
//! Each proof is of the function's own logic: the questions it asks about
//! each object (is it a creature, does it match the requirement's filter,
//! can this player target it with this spell, does the card itself accept
//! it) are answered by [`decided`], so the proof holds for every answer,
//! and each answering function has its own proof.

use crate::actions::Target;
use crate::cards::{CardBehavior, CardRegistry, TargetFilter, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::board;
use crate::proofs::decide::{about_player, decided, Question};
use crate::state::{GameObject, GameState};
use crate::types::Zone;

#[allow(dead_code)] // named only by `kani::stub`
fn is_creature(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    decided(Question::IsCreature, id)
}

#[allow(dead_code)] // named only by `kani::stub`
fn matches_filter(_: &GameState, obj: &GameObject, _: &TargetFilter, _: PlayerId, _: Option<ObjectId>, _: &CardRegistry) -> bool {
    decided(Question::MatchesFilter, obj.id)
}

#[allow(dead_code)] // named only by `kani::stub`
fn has_card_type(_: &GameState, id: ObjectId, t: crate::types::CardType, _: &CardRegistry) -> bool {
    assert!(t == crate::types::CardType::Planeswalker, "targeting asks this only of planeswalkers");
    decided(Question::IsPlaneswalker, id)
}

#[allow(dead_code)] // named only by `kani::stub`
fn can_target_player(_: &GameState, p: PlayerId, _: PlayerId, _: &CardRegistry) -> bool {
    decided(Question::CanTargetPlayer, about_player(p))
}

#[allow(dead_code)] // named only by `kani::stub`
fn can_be_targeted(_: &GameState, id: ObjectId, _: PlayerId, _: Option<ObjectId>, _: &CardRegistry) -> bool {
    decided(Question::CanBeTargeted, id)
}

/// A card that accepts, as `is_valid_target`, whatever Kani decides.
struct Accepting;

impl CardBehavior for Accepting {
    fn card_data(&self) -> crate::cards::CardData {
        crate::cards::CardData { name: "Accepting".into(), ..Default::default() }
    }

    fn is_valid_target(&self, _: &GameState, _: PlayerId, target: &Target, _: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => decided(Question::IsValidTarget, *id),
            Target::Player(p) => decided(Question::IsValidTarget, about_player(*p)),
            Target::Illegal => false,
        }
    }
}

/// Two objects, one each player's, each on the battlefield or not.
fn two_objects(state: &mut GameState, blank: crate::ids::CardId) -> [ObjectId; 2] {
    [PlayerId(0), PlayerId(1)].map(|p| {
        let zone = if kani::any() { Zone::Battlefield } else { Zone::Graveyard };
        state.create_object(blank, p, zone, None, None)
    })
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::is_creature, is_creature)]
#[kani::stub(crate::engine::matches_target_filter, matches_filter)]
#[kani::stub(crate::engine::can_be_targeted_by, can_be_targeted)]
/// "Target <filter> creature" offers each object on the battlefield, in id
/// order, that is a creature, matches the filter, can be targeted by this
/// player with this spell, and that the card accepts, and nothing else (CR
/// 115.1, 601.2c): for two objects, each on the battlefield or not, and
/// every answer to each of those questions.
fn a_filtered_creature_requirement_offers_the_creatures_that_pass() {
    let (registry, mut state, blank, card) = board(Box::new(Accepting));
    let objects = two_objects(&mut state, blank);
    let caster = PlayerId(0);
    let spell = state.create_object(card, caster, Zone::Stack, None, None);

    let offered = crate::engine::valid_targets_for_req(&state, caster, spell,
        &TargetRequirement::CreatureWithFilter(TargetFilter::Nonblack), &Accepting, &registry);

    let mut expected = Vec::new();
    for id in objects {
        let on = state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield);
        if on && decided(Question::IsCreature, id) && decided(Question::MatchesFilter, id)
            && decided(Question::CanBeTargeted, id) && decided(Question::IsValidTarget, id)
        {
            expected.push(Target::Object(id));
        }
    }
    assert!(offered == expected);
    kani::cover!(offered.len() == 2, "both offered");
    std::mem::forget((registry, state, offered, expected));
}

/// Whether `id` is on the battlefield.
fn on_battlefield(state: &GameState, id: ObjectId) -> bool {
    state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield)
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::engine::matches_target_filter, matches_filter)]
#[kani::stub(crate::engine::can_be_targeted_by, can_be_targeted)]
/// "Target <filter> permanent" offers each object on the battlefield, in id
/// order, that matches the filter, can be targeted by this player with
/// this spell, and that the card accepts, and nothing else (CR 115.1,
/// 110.1): for two objects, each on the battlefield or not, and every
/// answer to each of those questions.
fn a_filtered_permanent_requirement_offers_the_permanents_that_pass() {
    let (registry, mut state, blank, card) = board(Box::new(Accepting));
    let objects = two_objects(&mut state, blank);
    let caster = PlayerId(0);
    let spell = state.create_object(card, caster, Zone::Stack, None, None);

    let offered = crate::engine::valid_targets_for_req(&state, caster, spell,
        &TargetRequirement::PermanentWithFilter(TargetFilter::Noncreature), &Accepting, &registry);

    let mut expected = Vec::new();
    for id in objects {
        if on_battlefield(&state, id) && decided(Question::MatchesFilter, id)
            && decided(Question::CanBeTargeted, id) && decided(Question::IsValidTarget, id)
        {
            expected.push(Target::Object(id));
        }
    }
    assert!(offered == expected);
    kani::cover!(offered.len() == 2, "both offered");
    std::mem::forget((registry, state, offered, expected));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::is_creature, is_creature)]
#[kani::stub(crate::state::GameState::has_card_type, has_card_type)]
#[kani::stub(crate::engine::can_be_targeted_by, can_be_targeted)]
#[kani::stub(crate::engine::can_target_player, can_target_player)]
/// "Any target" offers each object on the battlefield, in id order, that
/// is a creature or a planeswalker, can be targeted by this player with
/// this spell and that the card accepts; then each player this player may
/// target and the card accepts, in turn order; and nothing else (CR
/// 115.4): for two objects, each on the battlefield or not, and every
/// answer to each of those questions.
fn any_target_offers_creatures_planeswalkers_and_players_that_pass() {
    let (registry, mut state, blank, card) = board(Box::new(Accepting));
    let objects = two_objects(&mut state, blank);
    let caster = PlayerId(0);
    let spell = state.create_object(card, caster, Zone::Stack, None, None);

    let offered = crate::engine::valid_targets_for_req(&state, caster, spell,
        &TargetRequirement::AnyTarget, &Accepting, &registry);

    let mut expected = Vec::new();
    for id in objects {
        if on_battlefield(&state, id)
            && (decided(Question::IsCreature, id) || decided(Question::IsPlaneswalker, id))
            && decided(Question::CanBeTargeted, id) && decided(Question::IsValidTarget, id)
        {
            expected.push(Target::Object(id));
        }
    }
    for p in [PlayerId(0), PlayerId(1)] {
        if decided(Question::CanTargetPlayer, about_player(p)) && decided(Question::IsValidTarget, about_player(p)) {
            expected.push(Target::Player(p));
        }
    }
    assert!(offered == expected);
    kani::cover!(offered.len() == 4, "everything offered");
    std::mem::forget((registry, state, offered, expected));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::has_card_type, has_card_type)]
#[kani::stub(crate::engine::can_be_targeted_by, can_be_targeted)]
#[kani::stub(crate::engine::can_target_player, can_target_player)]
/// "Target player or planeswalker" offers each player this player may
/// target and the card accepts, in turn order, then each planeswalker on
/// the battlefield, in id order, that can be targeted by this player with
/// this spell and the card accepts; and nothing else (CR 115.4): for two
/// objects, each on the battlefield or not, and every answer.
fn player_or_planeswalker_offers_the_players_and_planeswalkers_that_pass() {
    let (registry, mut state, blank, card) = board(Box::new(Accepting));
    let objects = two_objects(&mut state, blank);
    let caster = PlayerId(0);
    let spell = state.create_object(card, caster, Zone::Stack, None, None);

    let offered = crate::engine::valid_targets_for_req(&state, caster, spell,
        &TargetRequirement::PlayerOrPlaneswalker, &Accepting, &registry);

    let mut expected = Vec::new();
    for p in [PlayerId(0), PlayerId(1)] {
        if decided(Question::CanTargetPlayer, about_player(p)) && decided(Question::IsValidTarget, about_player(p)) {
            expected.push(Target::Player(p));
        }
    }
    for id in objects {
        if on_battlefield(&state, id) && decided(Question::IsPlaneswalker, id)
            && decided(Question::CanBeTargeted, id) && decided(Question::IsValidTarget, id)
        {
            expected.push(Target::Object(id));
        }
    }
    assert!(offered == expected);
    std::mem::forget((registry, state, offered, expected));
}
