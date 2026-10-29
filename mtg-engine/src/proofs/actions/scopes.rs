//! Which objects a continuous effect printed on a permanent reaches, for
//! the scopes and filters the pool's static abilities use (CR 611.3a,
//! 613.1): "creatures you control", "creatures your opponents control",
//! "creature tokens you control", "other <subtype> creatures you control",
//! "this creature", and the conditions "as long as you control a Human"
//! and "as long as an opponent controls a Human".
//!
//! Each proof is over a source permanent of a card printing the effect
//! (either player's, on the battlefield or not) and a creature of the
//! blank card's or a token (either player's, a Spirit, a Human or neither,
//! with or without flying): the effect reaches the creature exactly when
//! its source is on the battlefield and the scope's words describe the
//! creature. "Can't attack" stands for every kind of effect: what each kind
//! does once it reaches an object is `proofs::actions::effects`.

use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{case_by_case, for_each_form, Blank, Form};
use crate::state::GameState;
use crate::types::{CardType, ContinuousEffect as E, CreatureFilter as F, EffectCondition as If, EffectScope, Keyword, Zone};

/// A card that prints exactly one continuous effect.
struct Prints(E);

impl CardBehavior for Prints {
    fn card_data(&self) -> CardData {
        CardData { name: "Prints".into(), continuous_effects: vec![self.0.clone()], ..CardData::default() }
    }
}

/// The source, on the battlefield or not and either player's, and a
/// creature of `form` (either player's, on the battlefield, a Spirit or a
/// Human or neither, with flying or not if a token).
struct Board {
    state: GameState,
    registry: CardRegistry,
    source: ObjectId,
    creature: ObjectId,
}

fn board(effect: E, form: Form) -> Board {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let printer = registry.register(Box::new(Prints(effect)));
    let mut state = crate::proofs::cards::new_game();
    let on: bool = kani::any();
    let source = state.create_object(printer, PlayerId(u8::from(kani::any::<bool>())),
        if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    let card = if form == Form::Token { crate::ids::CardId(0) } else { blank };
    let creature = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    let o = state.get_object_mut(creature).expect("just created");
    o.is_token = form == Form::Token;
    o.card_types = vec![CardType::Creature];
    o.subtypes = Vec::with_capacity(1);
    match kani::any::<u8>() % 3 {
        0 => o.subtypes.push("Spirit".into()),
        1 => o.subtypes.push("Human".into()),
        _ => {}
    }
    o.keywords = Vec::with_capacity(1);
    if form == Form::Token && kani::any() {
        o.keywords.push(Keyword::Flying);
    }
    Board { state, registry, source, creature }
}

impl Board {
    fn source_on(&self) -> bool {
        self.state.get_object(self.source).is_some_and(|o| o.zone == Zone::Battlefield)
    }
    fn controller(&self, id: ObjectId) -> PlayerId {
        self.state.get_object(id).expect("on the board").controller
    }
    fn is(&self, subtype: &str) -> bool {
        self.state.get_object(self.creature).expect("on the board").subtypes.iter().any(|s| s == subtype)
    }
    fn reached(&self) -> bool {
        !self.state.can_attack(self.creature, &self.registry)
    }
}

/// One proof for each "creatures <whose>" filter the pool's effects use,
/// and each form of creature: the filter is the proof's, not Kani's to
/// choose, because a filter Kani leaves open is a tree of filters it
/// leaves open, and the solver ran out of memory matching against it.
macro_rules! reaches_what_it_describes {
    ($($name:ident: $filter:expr => |$b:ident, $yours:ident| $described:expr;)*) => {
        $(
            for_each_form! {
                fn $name(form) {
                    let $b = board(E::PreventAttack { scope: EffectScope::Global($filter) }, form);
                    let $yours = $b.controller($b.creature) == $b.controller($b.source);
                    let described = $described;
                    assert!($b.reached() == ($b.source_on() && described));
                    std::mem::forget($b);
                }
            }
        )*
    };
}

// "Creatures you control", "creatures your opponents control", "creature
// tokens you control", "Spirit creatures you control" and "creatures with
// flying your opponents control" reach exactly the creatures those words
// describe, the source's controller being "you" (CR 109.5).
reaches_what_it_describes! {
    creatures_you_control_are_yours: F::ControlledByYou => |b, yours| yours;
    creatures_your_opponents_control_are_theirs: F::ControlledByOpponent => |b, yours| !yours;
    creature_tokens_you_control_are_your_tokens: F::ControlledByYouToken
        => |b, yours| yours && b.state.get_object(b.creature).expect("on the board").is_token;
    spirit_creatures_you_control_are_your_spirits: F::And(vec![F::ControlledByYou, F::HasSubtype("Spirit".into())])
        => |b, yours| yours && b.is("Spirit");
    creatures_with_flying_your_opponents_control_are_their_fliers:
        F::And(vec![F::ControlledByOpponent, F::HasKeyword(Keyword::Flying)])
        => |b, yours| !yours && b.state.get_object(b.creature).expect("on the board").keywords.contains(&Keyword::Flying);
}

/// "Other ... creatures you control" reaches the creatures those words
/// describe but its source (CR 109.4, "other"); "this creature" reaches
/// its source alone. Which scope, a constant in each harness: the two in
/// one ran out of memory.
fn other_skips_its_source_and_this_creature_is_its_source(other: bool) {
    let scope = if other { EffectScope::GlobalOther(F::ControlledByYou) } else { EffectScope::OnSelf };
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let printer = registry.register(Box::new(Prints(E::PreventAttack { scope })));
    let mut state = crate::proofs::cards::new_game();
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(printer, controller, Zone::Battlefield, Some(1), Some(1));
    let creature = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    let yours = state.get_object(creature).expect("on the board").controller == controller;
    assert!(state.can_attack(source, &registry) == other);
    assert!(state.can_attack(creature, &registry) == !(other && yours));
    std::mem::forget((registry, state));
}

case_by_case! { other_skips_its_source_and_this_creature_is_its_source;
    other_creatures_you_control_skips_its_source: true,
    this_creature_is_its_source: false,
}

/// "As long as you control a Human" and "as long as an opponent controls
/// a Human" (CR 611.3a): an effect on its source holds exactly while that
/// is so, the Human being on the battlefield. The effect reaches the
/// creature only if its controller is the source's; whether it holds is
/// the Human's.
macro_rules! holds_exactly_then {
    ($($name:ident: $condition:expr, $opponent:expr;)*) => {
        $(
            for_each_form! {
                fn $name(form) {
                    let b = board(E::when($condition, E::PreventAttack { scope: EffectScope::Global(F::ControlledByYou) }), form);
                    let yours = b.controller(b.creature) == b.controller(b.source);
                    let holds = b.is("Human") && (yours != $opponent);
                    assert!(b.reached() == (b.source_on() && yours && holds));
                    std::mem::forget(b);
                }
            }
        )*
    };
}

holds_exactly_then! {
    as_long_as_you_control_a_human_holds_exactly_then: If::YouControlSubtype("Human".into()), false;
    as_long_as_an_opponent_controls_a_human_holds_exactly_then: If::OpponentControlsSubtype("Human".into()), true;
}

/// "Protection from Zombies" (CR 702.16a): its source has protection
/// from exactly the objects that are Zombies, while it is on the
/// battlefield. The other object a Zombie or not, a constant in each
/// harness: a subtype list of a length the solver chose, compared by
/// bytes, ran out of memory.
fn protection_from_a_subtype_is_from_exactly_that_subtype(zombie: bool) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let printer = registry.register(Box::new(Prints(E::ProtectionFromSubtype { subtype: "Zombie".into(), scope: EffectScope::OnSelf })));
    let mut state = crate::proofs::cards::new_game();
    let on: bool = kani::any();
    let protected = state.create_object(printer, PlayerId(0), if on { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
    let source = state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(1), Some(1));
    if zombie {
        state.get_object_mut(source).expect("just created").subtypes = vec!["Zombie".into()];
    }
    assert!(state.has_protection_from(protected, source, &registry) == (on && zombie));
    std::mem::forget((registry, state));
}

case_by_case! { protection_from_a_subtype_is_from_exactly_that_subtype;
    protection_from_zombies_is_from_a_zombie: true,
    protection_from_zombies_is_not_from_another_creature: false,
}
