//! The Curses: Auras that enchant a player (CR 303.4, 702.5a), and what
//! each one does to the player it enchants.
//!
//! - Curse of Death's Hold: "Creatures enchanted player controls get
//!   -1/-1."
//! - Curse of the Nightly Hunt: "Creatures enchanted player controls attack
//!   each combat if able."
//! - Curse of the Bloody Tome: "At the beginning of enchanted player's
//!   upkeep, that player mills two cards."
//! - Curse of the Pierced Heart: "At the beginning of enchanted player's
//!   upkeep, this Aura deals 1 damage to that player or a planeswalker that
//!   player controls."
//! - Curse of Oblivion: "At the beginning of enchanted player's upkeep,
//!   that player exiles two cards from their graveyard."
//! - Curse of Stalked Prey: "Whenever a creature deals combat damage to
//!   enchanted player, put a +1/+1 counter on that creature."
//!
//! Each is "Enchant player": its requirement is "target player" (CR
//! 303.4a, 115.1), and it resolves by entering attached to that player
//! (303.4f). Each upkeep trigger is its enchanted player's (503.1a), which
//! its trigger scope says, and resolves for the player it enchants, or
//! enchanted last if it has since left the battlefield (113.7a).

use super::{acted, asked_each, asked_for, board, card_proof, case_by_case, prints_where, triggers, Act, Snapshot};
use crate::actions::Target;
use crate::cards::{CardBehavior, TriggerKind, TriggerScope, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::{CardType, ContinuousEffect as E, CounterType, CreatureFilter, EffectScope, Zone};

/// `card` is "Enchant player", and resolving holding a player it enters
/// attached to that player (asking to move onto the battlefield and
/// becoming that player's); holding a target gone illegal it asks for
/// nothing. Nothing else changes.
fn enchants_a_player(card: Box<dyn CardBehavior>) {
    assert!(card.target_requirement() == TargetRequirement::PlayerOnly);
    let (registry, mut state, _, card) = board(card);
    let curse = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())), Zone::Stack, None, None);
    let target = match kani::any::<u8>() % 3 {
        0 => Target::Player(PlayerId(0)),
        1 => Target::Player(PlayerId(1)),
        _ => Target::Illegal,
    };
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, curse, &[target.clone()], &registry);
    match target {
        Target::Player(p) => {
            assert!(asked_for(&[Act::Move(curse, Zone::Battlefield)]));
            assert!(state.get_object(curse).expect("the curse").attached_to_player == Some(p));
            assert!(Snapshot::of(&state) == before.with_attached_player(curse, p));
        }
        _ => {
            assert!(acted() == 0);
            assert!(Snapshot::of(&state) == before);
        }
    }
    std::mem::forget((registry, state));
}

/// A Curse of `card` on the battlefield enchanting either player, or one
/// that has left it, having enchanted that player: the board, the Curse,
/// and the player it enchants.
fn cursing(card: Box<dyn CardBehavior>) -> (crate::cards::CardRegistry, GameState, ObjectId, PlayerId, crate::ids::CardId) {
    cursing_as(card, PlayerId(u8::from(kani::any::<bool>())), PlayerId(u8::from(kani::any::<bool>())), kani::any())
}

/// [`cursing`]'s board, the Curse `owner`'s, enchanting `cursed`, and on
/// the battlefield if `on`.
fn cursing_as(card: Box<dyn CardBehavior>, cursed: PlayerId, owner: PlayerId, on: bool)
    -> (crate::cards::CardRegistry, GameState, ObjectId, PlayerId, crate::ids::CardId)
{
    let (registry, mut state, blank, card) = board(card);
    let curse = state.create_object(card, owner, if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    let o = state.get_object_mut(curse).expect("just created");
    if on {
        o.attached_to_player = Some(cursed);
    } else {
        o.last_attached_to_player = Some(cursed);
    }
    (registry, state, curse, cursed, blank)
}

macro_rules! resolves {
    ($name:ident, $card:expr) => {
        card_proof! {
            fn $name() {
                enchants_a_player(Box::new($card));
            }
        }
    };
}

use crate::cards::isd as isd;

resolves!(curse_of_deaths_hold_enchants_a_player, isd::curse_of_deaths_hold::CurseOfDeathsHold);
resolves!(curse_of_the_nightly_hunt_enchants_a_player, isd::curse_of_the_nightly_hunt::CurseOfTheNightlyHunt);
resolves!(curse_of_the_bloody_tome_enchants_a_player, isd::curse_of_the_bloody_tome::CurseOfTheBloodyTome);
resolves!(curse_of_the_pierced_heart_enchants_a_player, isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart);
resolves!(curse_of_oblivion_enchants_a_player, isd::curse_of_oblivion::CurseOfOblivion);
resolves!(curse_of_stalked_prey_enchants_a_player, isd::curse_of_stalked_prey::CurseOfStalkedPrey);

card_proof! {
    /// Curse of Death's Hold prints "-1/-1 to each creature enchanted player
    /// controls", and Curse of the Nightly Hunt "each creature enchanted
    /// player controls attacks each combat if able", read by the engine as
    /// `proofs::actions::effects` proves for a player-wide scope.
    fn the_static_curses_print_what_their_text_says() {
        // As patterns (`prints_where`): each effect holds a filter, and two
        // filters read from the heap compared whole are what a proof cannot
        // settle (`prints_effects` says why).
        assert!(prints_where(&isd::curse_of_deaths_hold::CurseOfDeathsHold, |e| matches!(e,
            [E::ModifyPT { power: -1, toughness: -1, scope: EffectScope::Global(CreatureFilter::ControlledByAttachedPlayer) }])));
        assert!(prints_where(&isd::curse_of_the_nightly_hunt::CurseOfTheNightlyHunt, |e| matches!(e,
            [E::ForceAttack { scope: EffectScope::Global(CreatureFilter::ControlledByAttachedPlayer) }])));
    }
}

card_proof! {
    /// The upkeep Curses trigger in their enchanted player's upkeep, and no
    /// one else's (CR 503.1a).
    fn the_upkeep_curses_trigger_in_the_enchanted_players_upkeep() {
        for curse in [
            &isd::curse_of_the_bloody_tome::CurseOfTheBloodyTome as &dyn CardBehavior,
            &isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart,
            &isd::curse_of_oblivion::CurseOfOblivion,
        ] {
            assert!(curse.step_trigger_scope(&TriggerKind::Upkeep, false) == TriggerScope::AttachedPlayer);
            assert!(triggers(curse).iter().any(|(kind, _)| *kind == TriggerKind::Upkeep));
        }
    }
}

card_proof! {
    /// Curse of the Bloody Tome's upkeep trigger: the player it enchants
    /// mills two cards (CR 701.17a), whether or not the Curse is still on
    /// the battlefield (113.7a); nothing else changes.
    fn curse_of_the_bloody_tome_mills_its_player_two() {
        let (registry, mut state, curse, cursed, _) = cursing(Box::new(isd::curse_of_the_bloody_tome::CurseOfTheBloodyTome));
        let before = Snapshot::of(&state);
        isd::curse_of_the_bloody_tome::CurseOfTheBloodyTome.on_upkeep(&mut state, curse, &[], &registry);
        assert!(asked_for(&[Act::Mill(cursed, 2)]));
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Curse of the Pierced Heart's upkeep trigger: 1 damage dealt by the
    /// Curse to the player it enchants, or to a planeswalker that player
    /// controls, its controller choosing (CR 120.1, 110.2): with no such
    /// planeswalker, the damage to the player, asked for at once; with one,
    /// the choice of exactly the player and it, for that damage. Nothing
    /// else changes but the question.
    fn curse_of_the_pierced_heart_deals_one_damage_to_its_player_or_their_planeswalker() {
        let (registry, mut state, curse, cursed, blank) = cursing(Box::new(isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart));
        // Owned by either player and controlled by either, so that one the
        // cursed player controls but does not own counts, and one they own
        // but do not control does not.
        let walker = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())),
            if kani::any() { Zone::Battlefield } else { Zone::Exile }, None, None);
        let walker_controller = PlayerId(u8::from(kani::any::<bool>()));
        let is_walker: bool = kani::any();
        let w = state.get_object_mut(walker).expect("just created");
        w.controller = walker_controller;
        if is_walker {
            w.card_types = vec![CardType::Planeswalker];
        }
        let theirs = is_walker && walker_controller == cursed
            && state.get_object(walker).is_some_and(|o| o.zone == Zone::Battlefield);
        let controller = state.get_object(curse).expect("the curse").controller;
        let before = Snapshot::of(&state);

        isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart.on_upkeep(&mut state, curse, &[], &registry);

        let damage = PendingEffect::DealDamage { amount: 1, source_id: curse };
        if theirs {
            assert!(acted() == 0);
            let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. })
                = &state.awaiting_action else { panic!("a question") };
            assert!(*player == controller && !*optional && *effect == damage);
            assert!(*options == [Target::Player(cursed), Target::Object(walker)]);
            crate::proofs::cards::clear_prompt(&mut state);
        } else {
            assert!(asked_for(&[Act::Effect(Target::Player(cursed), damage)]));
        }
        assert!(Snapshot::of(&state) == before);
        kani::cover!(theirs, "a planeswalker to choose");
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Curse of Stalked Prey triggers exactly on combat damage to the
    /// player it enchants (CR 510.2), and its trigger puts one +1/+1
    /// counter on the creature that dealt it (122.1a).
    fn curse_of_stalked_prey_counts_the_creatures_that_hit_its_player() {
        let (registry, mut state, curse, cursed, blank) = cursing(Box::new(isd::curse_of_stalked_prey::CurseOfStalkedPrey));
        let attacker = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(2), Some(2));
        let damaged = PlayerId(u8::from(kani::any::<bool>()));
        let card = isd::curse_of_stalked_prey::CurseOfStalkedPrey;
        assert!(card.should_trigger_on_damage_to_player(&state, curse, attacker, damaged, &registry) == (damaged == cursed));
        let before = Snapshot::of(&state);
        card.on_any_combat_damage_to_player(&mut state, curse, attacker, damaged, 2, &registry);
        assert!(asked_for(&[Act::Counters(attacker, CounterType::PlusOnePlusOne, 1)]));
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

/// Curse of Oblivion's upkeep trigger: the player it enchants (as it
/// left, if it has, CR 113.7a) exiles two cards from their graveyard
/// (no token, CR 109.1): with two or fewer there, all of them; with
/// more, the two they choose, asked as one question of exactly two of
/// them (each then exiled as the card's effect runs once per card
/// chosen); with none, nothing. Nothing else. Over `in_graveyard` of
/// three cards of the cursed player's in their graveyard and the rest
/// in their hand, the Curse itself among that graveyard's cards if it
/// is there, and a token there; the player cursed, and the Curse on the
/// battlefield (`place` 0), in their graveyard (1) or in the other
/// player's (2). These are constants in each harness: the graveyard's
/// cards are a list of every length at once if the solver chooses which
/// are there, and its owner an index into the players, read for the
/// graveyard's order, if it chooses whose.
fn curse_of_oblivion((in_graveyard, cursed, place): (u8, u8, u8)) {
    let owner = PlayerId(if place == 2 { 1 - cursed } else { cursed });
    let (registry, mut state, curse, cursed, blank) =
        cursing_as(Box::new(isd::curse_of_oblivion::CurseOfOblivion), PlayerId(cursed), owner, place == 0);
    let mut cards = Vec::with_capacity(4);
    let c = state.get_object(curse).expect("the curse");
    if c.zone == Zone::Graveyard && c.owner == cursed {
        cards.push(curse);
    }
    for j in 0..3 {
        let there = j < in_graveyard;
        let id = state.create_object(blank, cursed, if there { Zone::Graveyard } else { Zone::Hand }, None, None);
        if there {
            cards.push(id);
        }
    }
    let token = state.create_object(crate::ids::CardId(0), cursed, Zone::Graveyard, None, None);
    state.get_object_mut(token).expect("just created").is_token = true;
    let before = Snapshot::of(&state);
    let oblivion = isd::curse_of_oblivion::CurseOfOblivion;
    oblivion.on_upkeep(&mut state, curse, &[], &registry);
    if cards.len() <= 2 {
        assert!(asked_each(cards.len(), |i| Act::Move(cards[i], Zone::Exile)) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state, cards));
        return;
    }
    let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        player, source, choice: ResolutionChoiceKind::ChooseObjectSet {
            options, min: 2, max: 2, effect: PendingEffect::CardEffect { source_id, .. }, ..
        },
    }) if *player == cursed && *source == curse && *source_id == curse && *options == cards);
    assert!(asked && acted() == 0);
    let chosen = cards[usize::from(kani::any::<u8>() % 4) % cards.len()];
    oblivion.resolve_card_effect(&mut state, curse, "", &Target::Object(chosen), &registry);
    assert!(asked_for(&[Act::Move(chosen, Zone::Exile)]));
    std::mem::forget((registry, state, cards));
}

// The proof above, a harness for each number of cards in the graveyard,
// each player cursed and each place of the Curse: a case of it is a couple
// of gigabytes of symbolic execution, and the cases of one harness add up.
case_by_case! { curse_of_oblivion;
    curse_of_oblivion_with_no_other_card_in_the_graveyard_cursing_the_first_player_on_the_battlefield: (0, 0, 0),
    curse_of_oblivion_with_no_other_card_in_the_graveyard_cursing_the_first_player_in_their_graveyard: (0, 0, 1),
    curse_of_oblivion_with_no_other_card_in_the_graveyard_cursing_the_first_player_in_the_others_graveyard: (0, 0, 2),
    curse_of_oblivion_with_no_other_card_in_the_graveyard_cursing_the_second_player_on_the_battlefield: (0, 1, 0),
    curse_of_oblivion_with_no_other_card_in_the_graveyard_cursing_the_second_player_in_their_graveyard: (0, 1, 1),
    curse_of_oblivion_with_no_other_card_in_the_graveyard_cursing_the_second_player_in_the_others_graveyard: (0, 1, 2),
    curse_of_oblivion_with_one_other_card_in_the_graveyard_cursing_the_first_player_on_the_battlefield: (1, 0, 0),
    curse_of_oblivion_with_one_other_card_in_the_graveyard_cursing_the_first_player_in_their_graveyard: (1, 0, 1),
    curse_of_oblivion_with_one_other_card_in_the_graveyard_cursing_the_first_player_in_the_others_graveyard: (1, 0, 2),
    curse_of_oblivion_with_one_other_card_in_the_graveyard_cursing_the_second_player_on_the_battlefield: (1, 1, 0),
    curse_of_oblivion_with_one_other_card_in_the_graveyard_cursing_the_second_player_in_their_graveyard: (1, 1, 1),
    curse_of_oblivion_with_one_other_card_in_the_graveyard_cursing_the_second_player_in_the_others_graveyard: (1, 1, 2),
    curse_of_oblivion_with_two_other_cards_in_the_graveyard_cursing_the_first_player_on_the_battlefield: (2, 0, 0),
    curse_of_oblivion_with_two_other_cards_in_the_graveyard_cursing_the_first_player_in_their_graveyard: (2, 0, 1),
    curse_of_oblivion_with_two_other_cards_in_the_graveyard_cursing_the_first_player_in_the_others_graveyard: (2, 0, 2),
    curse_of_oblivion_with_two_other_cards_in_the_graveyard_cursing_the_second_player_on_the_battlefield: (2, 1, 0),
    curse_of_oblivion_with_two_other_cards_in_the_graveyard_cursing_the_second_player_in_their_graveyard: (2, 1, 1),
    curse_of_oblivion_with_two_other_cards_in_the_graveyard_cursing_the_second_player_in_the_others_graveyard: (2, 1, 2),
    curse_of_oblivion_with_three_other_cards_in_the_graveyard_cursing_the_first_player_on_the_battlefield: (3, 0, 0),
    curse_of_oblivion_with_three_other_cards_in_the_graveyard_cursing_the_first_player_in_their_graveyard: (3, 0, 1),
    curse_of_oblivion_with_three_other_cards_in_the_graveyard_cursing_the_first_player_in_the_others_graveyard: (3, 0, 2),
    curse_of_oblivion_with_three_other_cards_in_the_graveyard_cursing_the_second_player_on_the_battlefield: (3, 1, 0),
    curse_of_oblivion_with_three_other_cards_in_the_graveyard_cursing_the_second_player_in_their_graveyard: (3, 1, 1),
    curse_of_oblivion_with_three_other_cards_in_the_graveyard_cursing_the_second_player_in_the_others_graveyard: (3, 1, 2),
}
