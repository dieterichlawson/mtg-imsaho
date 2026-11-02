//! A spell or ability resolving off the stack (`stack::resolve_top_of_stack`,
//! CR 608.2): what the engine hands a card's resolution, which is where a
//! card proof starts. A card proof runs `on_resolve` (or the ability's
//! hook) with the targets it is handed, a target gone illegal among them;
//! these show that the engine calls it exactly when the rules say the
//! spell or ability resolves, with exactly those targets.
//!
//! The questions the re-check asks of a target (CR 608.2b) are any answer
//! here, the same each time they are asked of the same target: whether
//! it still meets the requirement it was chosen under (`is_target_legal`),
//! whether it may still be targeted (`can_be_targeted_by`: hexproof,
//! protection) and whether it still meets what the card itself asks
//! (`is_valid_target`). Each is its own function's proof, or the card's.

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::Blank;
use crate::state::{GameState, StackEntry};
use crate::types::Zone;

/// What is asked of a target.
#[derive(Clone, Copy)]
enum Asked {
    /// It meets the requirement it was chosen under.
    Legal,
    /// It may be targeted by this source.
    Targetable,
    /// It meets what the card itself asks of it.
    Valid,
}

/// The answers, by target (the board's objects 1 and 2) and question.
static mut ANSWERS: [[Option<bool>; 3]; 2] = [[None; 3]; 2];

fn answer(id: ObjectId, asked: Asked) -> bool {
    let i = match id.0 {
        1 => 0,
        2 => 1,
        _ => panic!("a target this proof does not make"),
    };
    // SAFETY: a proof runs on one thread, and only this function names ANSWERS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ANSWERS) };
    *answers[i][asked as usize].get_or_insert_with(kani::any::<bool>)
}

/// Which hook resolved, with which targets, and whether the spell was
/// moved off the stack after (CR 608.2n).
static mut RESOLVED: Option<(Hook, [Option<Target>; 2], usize)> = None;
static mut MOVED: usize = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hook {
    Spell,
    Ability,
    Loyalty,
}

fn resolved(hook: Hook, targets: &[Target]) {
    assert!(targets.len() <= 2, "no more targets than this proof gives");
    let held = [targets.first().cloned(), targets.get(1).cloned()];
    // SAFETY: as in `answer`.
    let r = unsafe { &mut *std::ptr::addr_of_mut!(RESOLVED) };
    assert!(r.is_none(), "resolved once");
    *r = Some((hook, held, targets.len()));
}

/// A card whose spell targets a creature, and whose resolution, and its
/// abilities', is recorded.
struct Resolving;

impl CardBehavior for Resolving {
    fn card_data(&self) -> CardData {
        CardData { name: "Resolving".into(), ..CardData::default() }
    }
    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }
    fn is_valid_target(&self, _: &GameState, _: PlayerId, target: &Target, _: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => answer(*id, Asked::Valid),
            _ => false,
        }
    }
    fn on_resolve(&self, _: &mut GameState, _: ObjectId, targets: &[Target], _: &CardRegistry) {
        resolved(Hook::Spell, targets);
    }
    fn resolve_activated_ability(&self, _: &mut GameState, _: ObjectId, _: usize, targets: &[Target], _: &CardRegistry) {
        resolved(Hook::Ability, targets);
    }
    fn on_loyalty_ability(&self, _: &mut GameState, _: ObjectId, _: usize, targets: &[Target], _: &CardRegistry) {
        resolved(Hook::Loyalty, targets);
    }
}

/// Stands in for `stack::is_target_legal`: any answer for an object, none
/// for a target already illegal.
#[allow(dead_code)] // named only by `kani::stub`
fn is_target_legal(_: &GameState, target: &Target, _: &TargetRequirement, _: PlayerId, _: Option<ObjectId>, _: &CardRegistry) -> bool {
    match target {
        Target::Object(id) => answer(*id, Asked::Legal),
        _ => false,
    }
}

/// Stands in for `engine::can_be_targeted_by`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn can_be_targeted_by(_: &GameState, id: ObjectId, _: PlayerId, _: Option<ObjectId>, _: &CardRegistry) -> bool {
    answer(id, Asked::Targetable)
}

/// Stands in for `GameState::move_spell_after_resolve`: counted.
#[allow(dead_code)] // named only by `kani::stub`
fn move_spell_after_resolve(_: &mut GameState, _: ObjectId, _: &CardRegistry) {
    // SAFETY: as in `answer`.
    unsafe { *std::ptr::addr_of_mut!(MOVED) += 1 };
}

/// `n` creatures of the blank card, either player's, on the battlefield,
/// each a target, and none else: their ids, and the list of them the spell
/// or ability goes on the stack with.
///
/// What a proof expects is held in arrays of the ids, not in lists of
/// targets: each list of targets cloned, walked and compared was, to the
/// solver, pointers turned into numbers to check their alignment, each
/// relating the pointer to every object it might point into, and with the
/// harness's lists beside the engine's the solver ran out of memory doing
/// so, at 12 GB as at 7.
fn targets_of(state: &mut GameState, blank: crate::ids::CardId, n: usize) -> ([ObjectId; 2], Vec<Target>) {
    let mut ids = [ObjectId(0); 2];
    let mut targets = Vec::with_capacity(2);
    for (i, id) in ids.iter_mut().enumerate().take(n) {
        *id = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
        assert!(*id == ObjectId(i as u64 + 1));
        targets.push(Target::Object(*id));
    }
    (ids, targets)
}

/// Whether the hook that resolved is `hook`, handed the `n` targets of
/// `expected`.
fn resolved_with(hook: Hook, expected: &[Option<Target>; 2], n: usize) -> bool {
    // SAFETY: as in `answer`.
    let r = unsafe { &*std::ptr::addr_of!(RESOLVED) };
    r.as_ref().is_some_and(|(h, held, m)| *h == hook && *m == n && held == expected)
}

/// A spell resolving (CR 608.2b): with targets, none still legal (none
/// meeting its requirement and the card's), it is countered by the rules,
/// its effect not happening, and leaves the stack; otherwise it resolves,
/// its card's resolution handed each target as it was chosen or, where it
/// may no longer be targeted, as illegal, its other targets kept in place,
/// and it then leaves the stack. A spell with none resolves. For a spell
/// either player's with `n` targets, each target either player's.
///
/// Not yet proved: the solver runs out of memory reducing the formula, at
/// 12 GB as at 7. A spell's targets are read back off its object, a list
/// whose length symbolic execution cannot see; with the engine handed an
/// empty list in its place the case of no target passes in 50 seconds,
/// and a copy of the list made element by element does not help.
fn a_spell_resolving(n: usize) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Resolving));
    let mut state = crate::proofs::cards::new_game();
    let (ids, targets) = targets_of(&mut state, blank, n);
    let spell = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())), Zone::Stack, None, None);
    state.get_object_mut(spell).expect("just created").targets = targets;
    state.stack.push(StackEntry::Spell(spell));
    let events = state.events.len();

    crate::stack::resolve_top_of_stack(&mut state, &registry);

    let chosen = &ids[..n];
    let any_legal = chosen.iter().any(|&id| answer(id, Asked::Legal) && answer(id, Asked::Valid));
    let mut handed = [None, None];
    for (h, &id) in handed.iter_mut().zip(chosen) {
        *h = Some(if answer(id, Asked::Targetable) { Target::Object(id) } else { Target::Illegal });
    }
    // SAFETY: as in `answer`.
    let (none, moved) = unsafe { ((*std::ptr::addr_of!(RESOLVED)).is_none(), *std::ptr::addr_of!(MOVED)) };
    assert!(state.stack.is_empty() && moved == 1);
    if n == 0 || any_legal {
        assert!(resolved_with(Hook::Spell, &handed, n));
        assert!(matches!(&state.events[events..], [GameEvent::SpellResolved { object }] if *object == spell));
    } else {
        assert!(none && state.events.len() == events);
    }
    std::mem::forget((registry, state));
}

/// The proofs of resolving, the re-check's questions and the spell's move
/// answered as above.
macro_rules! each_resolution {
    ($($name:ident: $proof:ident($case:expr)),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::stack::is_target_legal, is_target_legal)]
        #[kani::stub(crate::engine::can_be_targeted_by, can_be_targeted_by)]
        #[kani::stub(crate::state::GameState::move_spell_after_resolve, move_spell_after_resolve)]
        /// The proof above it, for one case.
        fn $name() {
            $proof($case);
        }
    )*};
}

each_resolution! {
    a_spell_with_no_target_resolves: a_spell_resolving(0),
    a_spell_with_one_target_resolves_unless_it_is_illegal: a_spell_resolving(1),
    a_spell_with_two_targets_resolves_unless_both_are_illegal: a_spell_resolving(2),
}

/// An activated ability resolving (CR 608.2b, 602.2a): with targets, none
/// still legal (each may still be targeted by its source, meets the
/// requirement the ability declared, if `required`, and what the card
/// asks), it does nothing; otherwise its card's resolution (a loyalty
/// ability's own, if `loyalty`, 606.5) is handed each target as chosen or,
/// where it is no longer legal, as illegal. Either way it leaves the
/// stack. For an ability either player activated with `n` targets.
fn an_ability_resolving((n, required, loyalty): (usize, bool, bool)) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Resolving));
    let mut state = crate::proofs::cards::new_game();
    let (ids, targets) = targets_of(&mut state, blank, n);
    let activator = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(card, activator, Zone::Battlefield, None, None);
    state.stack.push(StackEntry::Ability {
        source_id: source, ability_index: 0, behavior_card_id: card, targets, activator,
        x_value: None, target_requirement: required.then_some(TargetRequirement::Creature), sacrificed: None,
        sacrificed_toughness: None, loyalty, source_transforms: 0, source_back_face: false,
    });

    crate::stack::resolve_top_of_stack(&mut state, &registry);

    let mut handed = [None, None];
    let mut any_legal = false;
    for (h, &id) in handed.iter_mut().zip(&ids[..n]) {
        let legal = answer(id, Asked::Targetable) && (!required || answer(id, Asked::Legal)) && answer(id, Asked::Valid);
        any_legal |= legal;
        *h = Some(if legal { Target::Object(id) } else { Target::Illegal });
    }
    // SAFETY: as in `answer`.
    let none = unsafe { (*std::ptr::addr_of!(RESOLVED)).is_none() };
    assert!(state.stack.is_empty());
    if n > 0 && !any_legal {
        assert!(none);
    } else {
        assert!(resolved_with(if loyalty { Hook::Loyalty } else { Hook::Ability }, &handed, n));
    }
    std::mem::forget((registry, state));
}

each_resolution! {
    an_ability_with_no_target_resolves: an_ability_resolving((0, false, false)),
    an_ability_with_one_target_resolves_unless_it_is_illegal: an_ability_resolving((1, true, false)),
    an_ability_with_two_targets_resolves_unless_both_are_illegal: an_ability_resolving((2, true, false)),
    an_ability_declaring_no_requirement_rechecks_the_rest: an_ability_resolving((2, false, false)),
    a_loyalty_ability_with_a_target_resolves_unless_it_is_illegal: an_ability_resolving((1, true, true)),
}
