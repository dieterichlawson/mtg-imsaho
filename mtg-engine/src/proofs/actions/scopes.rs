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

/// A card that prints exactly one continuous effect, the one its function
/// builds. Built where it is printed, not held and cloned: the effect a
/// card held came back from its clone with a scope symbolic execution
/// could not tell from any other, and every filter it might have been was
/// matched (protection from Zombies, "Spirit creatures you control", out
/// of memory).
struct Prints(fn() -> E);

impl CardBehavior for Prints {
    fn card_data(&self) -> CardData {
        CardData { name: "Prints".into(), continuous_effects: vec![(self.0)()], ..CardData::default() }
    }
}

/// The source, on the battlefield or not and either player's, and a
/// creature of `form` (either player's, on the battlefield, of one of two
/// subtypes or neither, with flying or not if a token).
struct Board {
    state: GameState,
    registry: CardRegistry,
    source: ObjectId,
    creature: ObjectId,
}

/// The board, its creature a Spirit, a Human or neither.
fn board(effect: fn() -> E, form: Form) -> Board {
    board_of(effect, form, ["Spirit", "Human"])
}

/// The board, its creature of either of `kinds` or neither.
fn board_of(effect: fn() -> E, form: Form, kinds: [&str; 2]) -> Board {
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
        0 => o.subtypes.push(kinds[0].into()),
        1 => o.subtypes.push(kinds[1].into()),
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
                    let $b = board(|| E::PreventAttack { scope: EffectScope::Global($filter) }, form);
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

/// One proof for each "<subtype> creatures you control" filter the pool's
/// effects use besides Spirits, over a creature of that subtype, of
/// another one, or of neither.
macro_rules! reaches_its_subtypes {
    ($($name:ident: $filter:expr, $kinds:expr => |$b:ident| $described:expr;)*) => {
        $(
            for_each_form! {
                fn $name(form) {
                    let $b = board_of(|| E::PreventAttack { scope: EffectScope::Global($filter) }, form, $kinds);
                    let yours = $b.controller($b.creature) == $b.controller($b.source);
                    let described = $described;
                    assert!($b.reached() == ($b.source_on() && yours && described));
                    std::mem::forget($b);
                }
            }
        )*
    };
}

// "Vampire creatures you control" (Lord of Lineage's), "Werewolf
// creatures you control" (Full Moon's Rise's, Terror of Kruin Pass's),
// "Human creatures you control" (Mayor of Avabruck's) and "Werewolf and
// Wolf creatures you control" (Howlpack Alpha's) reach exactly the
// creatures of theirs those words describe.
reaches_its_subtypes! {
    vampire_creatures_you_control_are_your_vampires:
        F::And(vec![F::ControlledByYou, F::HasSubtype("Vampire".into())]), ["Vampire", "Human"] => |b| b.is("Vampire");
    werewolf_creatures_you_control_are_your_werewolves:
        F::And(vec![F::ControlledByYou, F::HasSubtype("Werewolf".into())]), ["Werewolf", "Wolf"] => |b| b.is("Werewolf");
    human_creatures_you_control_are_your_humans:
        F::And(vec![F::ControlledByYou, F::HasSubtype("Human".into())]), ["Human", "Spirit"] => |b| b.is("Human");
    werewolf_and_wolf_creatures_you_control_are_your_werewolves_and_wolves:
        F::And(vec![F::ControlledByYou, F::Or(vec![F::HasSubtype("Werewolf".into()), F::HasSubtype("Wolf".into())])]), ["Werewolf", "Wolf"]
        => |b| b.is("Werewolf") || b.is("Wolf");
}

/// "Other ... creatures you control" reaches the creatures those words
/// describe but its source (CR 109.4, "other"); "this creature" reaches
/// its source alone. Which scope, a constant in each harness: the two in
/// one ran out of memory.
fn other_skips_its_source_and_this_creature_is_its_source(other: bool) {
    let printed: fn() -> E = if other {
        || E::PreventAttack { scope: EffectScope::GlobalOther(F::ControlledByYou) }
    } else {
        || E::PreventAttack { scope: EffectScope::OnSelf }
    };
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let printer = registry.register(Box::new(Prints(printed)));
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
                    let b = board(|| E::when($condition, E::PreventAttack { scope: EffectScope::Global(F::ControlledByYou) }), form);
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
    let printer = registry.register(Box::new(Prints(|| E::ProtectionFromSubtype { subtype: "Zombie".into(), scope: EffectScope::OnSelf })));
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

for_each_form! {
    /// "Creatures enchanted player controls" (CR 303.4e, a Curse's
    /// "enchanted player": Curse of Death's Hold, Curse of the Nightly
    /// Hunt): an effect so scoped reaches exactly the creatures that the
    /// player its source is attached to controls, while its source is on
    /// the battlefield. For a source attached to either player.
    fn creatures_enchanted_player_controls_are_that_players(form) {
        let mut b = board(|| E::PreventAttack { scope: EffectScope::Global(F::ControlledByAttachedPlayer) }, form);
        let cursed = PlayerId(u8::from(kani::any::<bool>()));
        b.state.get_object_mut(b.source).expect("on the board").attached_to_player = Some(cursed);
        assert!(b.reached() == (b.source_on() && b.controller(b.creature) == cursed));
        std::mem::forget(b);
    }
}

for_each_form! {
    /// "Attacking creatures you control" (CR 506.4, Instigator Gang's):
    /// an effect so scoped reaches exactly the creatures its source's
    /// controller controls that are attacking this combat, while its
    /// source is on the battlefield. With a combat in which the creature
    /// attacks or does not, or with no combat.
    fn attacking_creatures_you_control_are_your_attackers(form) {
        let mut b = board(|| E::PreventAttack { scope: EffectScope::Global(F::And(vec![F::ControlledByYou, F::Attacking])) }, form);
        let attacking: bool = kani::any();
        if kani::any() {
            let mut combat = crate::state::CombatState::new();
            if attacking {
                combat.attackers.insert(b.creature, PlayerId(1));
            }
            b.state.combat = Some(combat);
        } else {
            kani::assume(!attacking);
        }
        let yours = b.controller(b.creature) == b.controller(b.source);
        assert!(b.reached() == (b.source_on() && yours && attacking));
        std::mem::forget(b);
    }
}
