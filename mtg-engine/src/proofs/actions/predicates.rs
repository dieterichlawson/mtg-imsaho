//! The engine's answers to the questions targeting asks about an object,
//! on a board whose cards print nothing, so that an object's
//! characteristics are its own (`proofs::cards::any_object`): whether it is
//! a creature (CR 205.1b, 208.3), its colours (105.2, 202.2), whether it
//! matches a requirement's filter, and whether a player may target it with
//! a spell (115.1, 702.11b, 702.16b).

use crate::cards::TargetFilter;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{any_color, any_object, blank_board, card_proof, for_each_form, Kinds};
use crate::proofs::decide::{decided, Question};
use crate::state::GameState;
use crate::types::{CardType, Color, Keyword, Zone};

const KINDS: Kinds = Kinds {
    types: &[CardType::Creature, CardType::Artifact, CardType::Planeswalker],
    subtypes: &["Spirit", "Vampire"],
    keywords: &[Keyword::Hexproof, Keyword::Flying],
};

/// For the filters that name enchantments and the subtypes of the cards
/// that use them.
const FILTERED: Kinds = Kinds {
    types: &[CardType::Creature, CardType::Artifact, CardType::Enchantment],
    subtypes: &["Spirit", "Vampire", "Zombie"],
    keywords: &[],
};

/// Whether `filter` matches `id`, as the engine reads it for a spell cast
/// by player 0.
fn matches(state: &GameState, id: ObjectId, filter: &TargetFilter, registry: &crate::cards::CardRegistry) -> bool {
    let o = state.get_object(id).expect("on the board");
    crate::engine::matches_target_filter(state, o, filter, PlayerId(0), None, registry)
}

/// The card types an object of a blank board has: its own, and creature
/// if it has power and toughness (CR 205.1b, 208.3).
fn has_type(o: &crate::state::GameObject, t: CardType) -> bool {
    o.card_types.contains(&t) || (t == CardType::Creature && o.power.is_some())
}

for_each_form! {
    /// An object has a card type exactly when it is one of its own (CR
    /// 205.1b), for each of creature, artifact, enchantment and land.
    fn an_objects_card_types_are_its_own(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &FILTERED, form);
        let t = match kani::any::<u8>() % 4 {
            0 => CardType::Creature,
            1 => CardType::Artifact,
            2 => CardType::Enchantment,
            _ => CardType::Land,
        };
        let o = state.get_object(id).expect("on the board");
        assert!(state.has_card_type(id, t, &registry) == has_type(o, t));
        std::mem::forget((registry, state));
    }
}

for_each_form! {
    /// "Artifact" (CR 301) and "artifact or enchantment" (303) match
    /// exactly an object with one of those types, and "noncreature"
    /// (205.4b) one that is not a creature.
    fn the_card_type_filters_match_their_types(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &FILTERED, form);
        let o = state.get_object(id).expect("on the board").clone();
        assert!(matches(&state, id, &TargetFilter::HasCardType(vec![CardType::Artifact]), &registry)
            == has_type(&o, CardType::Artifact));
        assert!(matches(&state, id, &TargetFilter::HasCardType(vec![CardType::Artifact, CardType::Enchantment]), &registry)
            == (has_type(&o, CardType::Artifact) || has_type(&o, CardType::Enchantment)));
        assert!(matches(&state, id, &TargetFilter::Noncreature, &registry) == !has_type(&o, CardType::Creature));
        std::mem::forget((registry, state, o));
    }
}

for_each_form! {
    /// "Spirit or enchantment" matches exactly an object with the subtype
    /// Spirit or the type enchantment, and "non-Vampire, non-Werewolf,
    /// non-Zombie" one with none of those subtypes (CR 205.3, 205.4b).
    fn the_subtype_filters_match_their_subtypes(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &FILTERED, form);
        // What the object is, read off it rather than off a copy of it:
        // copying an object copies its subtype strings, and the copy cost
        // more than every comparison here.
        let o = state.get_object(id).expect("on the board");
        let sub = |s: &str| o.subtypes.iter().any(|t| t == s);
        let (spirit, enchantment) = (sub("Spirit"), has_type(o, CardType::Enchantment));
        let tribal = sub("Vampire") || sub("Werewolf") || sub("Zombie");
        let spirit_or_enchantment = TargetFilter::SubtypeOrCardType {
            subtypes: vec!["Spirit".into()],
            card_types: vec![CardType::Enchantment],
        };
        assert!(matches(&state, id, &spirit_or_enchantment, &registry) == (spirit || enchantment));
        let not = TargetFilter::NotSubtypes(vec!["Vampire".into(), "Werewolf".into(), "Zombie".into()]);
        assert!(matches(&state, id, &not, &registry) == !tribal);
        std::mem::forget((spirit_or_enchantment, not));
        std::mem::forget((registry, state));
    }
}

for_each_form! {
    /// "Power 4 or greater" (CR 208.1) matches exactly a creature whose
    /// power is 4 or more, for powers 0 to 5.
    fn power_four_or_greater_matches_power_four_or_greater(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &FILTERED, form);
        let o = state.get_object(id).expect("on the board").clone();
        assert!(matches(&state, id, &TargetFilter::PowerAtLeast(4), &registry) == o.power.is_some_and(|p| p >= 4));
        kani::cover!(o.power == Some(4), "power 4");
        std::mem::forget((registry, state, o));
    }
}

for_each_form! {
    /// "Attacking" (CR 506.4) matches exactly a creature declared as an
    /// attacker this combat, whether or not there is a combat.
    fn attacking_matches_the_attackers(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &FILTERED, form);
        let attacking: bool = kani::any();
        if kani::any() {
            let mut combat = crate::state::CombatState::new();
            if attacking {
                combat.attackers.insert(id, PlayerId(1));
            }
            state.combat = Some(combat);
        } else {
            kani::assume(!attacking);
        }
        assert!(matches(&state, id, &TargetFilter::Attacking, &registry) == attacking);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// A player may target a player exactly when that player is still in
    /// the game (CR 104.3a, 800.4a) and, if it is an opponent, has no
    /// hexproof (702.11b, which nothing on a blank board grants).
    fn a_player_can_be_targeted_unless_gone() {
        let (registry, mut state, _) = blank_board();
        state.players[1].lost = kani::any();
        let target = PlayerId(u8::from(kani::any::<bool>()));
        let caster = PlayerId(u8::from(kani::any::<bool>()));
        let lost = target == PlayerId(1) && state.players[1].lost;
        assert!(crate::engine::can_target_player(&state, target, caster, &registry) == !lost);
        std::mem::forget((registry, state));
    }
}

for_each_form! {
    /// An object is a creature exactly when one of its card types is
    /// creature (CR 205.1b), and has power and toughness exactly then.
    fn an_object_is_a_creature_when_its_types_say_so(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &KINDS, form);
        let o = state.get_object(id).expect("on the board");
        assert!(state.is_creature(id, &registry) == o.card_types.contains(&CardType::Creature));
        std::mem::forget((registry, state));
    }
}

for_each_form! {
    /// An object's colours are its own (CR 105.2): it has a colour exactly
    /// when that colour is one of them, for any colour.
    fn an_objects_colours_are_its_own(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &KINDS, form);
        let c = any_color();
        let colors = state.colors_of(id, &registry);
        let o = state.get_object(id).expect("on the board");
        assert!(colors.contains(&c) == o.colors.contains(&c));
        std::mem::forget((registry, state, colors));
    }
}

for_each_form! {
    /// "Nonblack" (CR 105.2) is exactly not having black among its colours.
    fn nonblack_is_not_black(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &KINDS, form);
        let o = state.get_object(id).expect("on the board");
        let matches = crate::engine::matches_target_filter(&state, o, &TargetFilter::Nonblack, PlayerId(0), None, &registry);
        assert!(matches == !o.colors.contains(&Color::Black));
        std::mem::forget((registry, state));
    }
}

#[allow(dead_code)] // named only by `kani::stub`
fn has_keyword(_: &GameState, id: ObjectId, keyword: Keyword, _: &crate::cards::CardRegistry) -> bool {
    assert!(keyword == Keyword::Hexproof, "targeting asks only about hexproof");
    decided(Question::HasHexproof, id)
}

#[allow(dead_code)] // named only by `kani::stub`
fn has_protection_from(_: &GameState, target: ObjectId, _: ObjectId, _: &crate::cards::CardRegistry) -> bool {
    decided(Question::HasProtection, target)
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::has_keyword, has_keyword)]
#[kani::stub(crate::state::GameState::has_protection_from, has_protection_from)]
/// A player may target a permanent with a spell exactly when it does not
/// have hexproof while an opponent of that player controls it (CR
/// 702.11b), and does not have protection from the spell (702.16b): for
/// either player, a permanent either player controls, and every answer to
/// whether it has hexproof and whether it has protection from the spell.
fn a_permanent_can_be_targeted_unless_hexproof_or_protection_says_not() {
    let (registry, mut state, blank) = blank_board();
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let id = state.create_object(blank, controller, Zone::Battlefield, None, None);
    let spell = state.create_object(blank, PlayerId(0), Zone::Stack, None, None);
    let caster = PlayerId(u8::from(kani::any::<bool>()));

    let can = crate::engine::can_be_targeted_by(&state, id, caster, Some(spell), &registry);

    let hexproof = decided(Question::HasHexproof, id);
    let protection = decided(Question::HasProtection, id);
    assert!(can == (!(hexproof && controller != caster) && !protection));
    kani::cover!(hexproof && controller == caster && can, "its controller targets past its hexproof");
    std::mem::forget((registry, state));
}

for_each_form! {
    /// On a board whose cards print nothing, an object has a keyword
    /// exactly when it is a token with that keyword (a card has those its
    /// card prints, CR 702.1, and the blank card prints none): hexproof and
    /// flying, for a card or a token, on the battlefield or not.
    fn an_objects_keywords_are_its_tokens_or_its_cards(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &KINDS, form);
        let keyword = if kani::any() { Keyword::Hexproof } else { Keyword::Flying };
        let o = state.get_object(id).expect("on the board");
        let has = o.is_token && o.zone == Zone::Battlefield && o.keywords.contains(&keyword);
        assert!(state.has_keyword(id, keyword, &registry) == has);
        kani::cover!(has, "a token with it");
        std::mem::forget((registry, state));
    }
}

for_each_form! {
    /// On a board whose cards print nothing and with no protection granted
    /// this turn, nothing has protection from anything (CR 702.16).
    fn nothing_has_protection_on_a_board_that_grants_none(form) {
        let (registry, mut state, blank) = blank_board();
        let id = any_object(&mut state, blank, &KINDS, form);
        let spell = state.create_object(blank, PlayerId(0), Zone::Stack, None, None);
        assert!(!state.has_protection_from(id, spell, &registry));
        std::mem::forget((registry, state));
    }
}
