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

/// What [`a_spell_on_top`] finds the engine hands the rest of a spell's
/// resolution: the spell, its card, its targets (the first two, and how
/// many) and its controller.
static mut HANDED: Option<(ObjectId, crate::ids::CardId, [Option<Target>; 2], usize, PlayerId)> = None;

/// Stands in for `stack::resolve_spell_as`: records what it is handed.
#[allow(dead_code)] // named only by `kani::stub`
fn resolve_spell_as(
    _: &mut GameState,
    _: &CardRegistry,
    object_id: ObjectId,
    card_id: crate::ids::CardId,
    targets: Vec<Target>,
    caster: PlayerId,
) {
    assert!(targets.len() <= 2, "no more targets than this proof gives");
    let held = [targets.first().cloned(), targets.get(1).cloned()];
    // SAFETY: as in `answer`.
    let handed = unsafe { &mut *std::ptr::addr_of_mut!(HANDED) };
    assert!(handed.is_none(), "handed once");
    *handed = Some((object_id, card_id, held, targets.len(), caster));
    std::mem::forget(targets);
}

/// A spell on top of the stack resolving: it leaves the stack, and the
/// rest of its resolution (`stack::resolve_spell_as`, the proof below) is
/// handed the spell, its card, the targets it was cast with, in order,
/// and its controller. For a spell either player's with `n` targets, each
/// target either player's. With [`a_spell_resolving`], what a spell
/// resolving does.
fn a_spell_on_top(n: usize) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Resolving));
    let mut state = crate::proofs::cards::new_game();
    let (ids, targets) = targets_of(&mut state, blank, n);
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let spell = state.create_object(card, controller, Zone::Stack, None, None);
    state.get_object_mut(spell).expect("just created").targets = targets;
    state.stack.push(StackEntry::Spell(spell));

    crate::stack::resolve_top_of_stack(&mut state, &registry);

    let mut chosen = [None, None];
    for (c, &id) in chosen.iter_mut().zip(&ids[..n]) {
        *c = Some(Target::Object(id));
    }
    // SAFETY: as in `answer`.
    let handed = unsafe { &*std::ptr::addr_of!(HANDED) };
    assert!(state.stack.is_empty());
    assert!(handed.as_ref().is_some_and(|(o, c, held, m, p)| *o == spell && *c == card && *held == chosen && *m == n && *p == controller));
    std::mem::forget((registry, state));
}

/// The rest of a spell's resolution (CR 608.2b), from its card, its
/// targets and its controller (read off it as [`a_spell_on_top`] shows):
/// with targets, none still legal (none meeting its requirement, able to
/// be targeted by it and meeting the card's), it is countered by the
/// rules, its effect not happening, and leaves the stack; otherwise it
/// resolves, its card's resolution handed
/// each target as it was chosen or, where it may no longer be targeted,
/// as illegal, its other targets kept in place, and it then leaves the
/// stack. A spell with none resolves. For a spell either player's with
/// `n` targets, each target either player's.
///
/// Proved apart from the read: read back off the spell's object, the
/// targets are a list whose length symbolic execution does not see, and
/// with the re-check walking that list the solver ran out of memory
/// reducing the formula, at 12 GB as at 7. Apart, each half proves in
/// about a minute.
fn a_spell_resolving(n: usize) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Resolving));
    let mut state = crate::proofs::cards::new_game();
    let (ids, targets) = targets_of(&mut state, blank, n);
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let spell = state.create_object(card, controller, Zone::Stack, None, None);
    let events = state.events.len();

    crate::stack::resolve_spell_as(&mut state, &registry, spell, card, targets, controller);

    let chosen = &ids[..n];
    let any_legal = chosen.iter().any(|&id| answer(id, Asked::Legal) && answer(id, Asked::Targetable) && answer(id, Asked::Valid));
    let mut handed = [None, None];
    for (h, &id) in handed.iter_mut().zip(chosen) {
        *h = Some(if answer(id, Asked::Targetable) { Target::Object(id) } else { Target::Illegal });
    }
    // SAFETY: as in `answer`.
    let (none, moved) = unsafe { ((*std::ptr::addr_of!(RESOLVED)).is_none(), *std::ptr::addr_of!(MOVED)) };
    assert!(moved == 1);
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
    ($($(#[$stub:meta])* $name:ident: $proof:ident($case:expr)),* $(,)?) => {$(
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
        $(#[$stub])*
        /// The proof above it, for one case.
        fn $name() {
            $proof($case);
        }
    )*};
}

each_resolution! {
    #[kani::stub(crate::stack::resolve_spell_as, resolve_spell_as)]
    a_spell_with_no_target_is_resolved_with_none: a_spell_on_top(0),
    #[kani::stub(crate::stack::resolve_spell_as, resolve_spell_as)]
    a_spell_with_one_target_is_resolved_with_it: a_spell_on_top(1),
    #[kani::stub(crate::stack::resolve_spell_as, resolve_spell_as)]
    a_spell_with_two_targets_is_resolved_with_both_in_order: a_spell_on_top(2),
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

/// What the game held of the resolving ability when its card's
/// resolution was called: the X, the creature sacrificed, its toughness,
/// and who activated it.
static mut PAID: Option<(Option<u32>, Option<ObjectId>, Option<i32>, Option<PlayerId>)> = None;

/// A card whose ability's resolution records what it would read of its
/// costs (Kessig Wolf Run's X, Disciple of Griselbrand's sacrificed
/// creature's toughness) and of its controller (`helpers::
/// ability_controller`, every ability's "you").
struct Paid;

impl CardBehavior for Paid {
    fn card_data(&self) -> CardData {
        CardData { name: "Paid".into(), ..CardData::default() }
    }
    fn resolve_activated_ability(&self, state: &mut GameState, _: ObjectId, _: usize, _: &[Target], _: &CardRegistry) {
        // SAFETY: as in `answer`, for PAID.
        let paid = unsafe { &mut *std::ptr::addr_of_mut!(PAID) };
        assert!(paid.is_none(), "resolved once");
        *paid = Some((state.last_activated_x_value, state.last_activated_sacrifice,
            state.last_activated_sacrifice_toughness, state.resolving_ability_activator));
    }
}

/// An activated ability resolving with what its costs left (CR 608.2h,
/// 602.2a): its card's resolution reads, as the X announced, the creature
/// sacrificed and that creature's toughness, those the ability carries on
/// the stack (`activating`, `an_ability_goes_on_the_stack_with_what_its_
/// costs_left`), whatever another ability activated since left there; and
/// as its controller the player who activated it, whoever controls its
/// source now. With no question left waiting once it has resolved, it is
/// no longer the ability resolving, and has no controller left to read.
/// For an ability of no target, either player's, its source either
/// player's, and any of each.
fn an_ability_resolving_with_its_costs(_: ()) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Paid));
    let mut state = crate::proofs::cards::new_game();
    let activator = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, None, None);
    let other = state.create_object(blank, activator, Zone::Graveyard, Some(1), Some(1));
    let x: Option<u32> = if kani::any() { Some(kani::any()) } else { None };
    let sacrificed = match kani::any::<u8>() % 3 {
        0 => None,
        1 => Some(source),
        _ => Some(other),
    };
    let toughness: Option<i32> = if kani::any() { Some(kani::any()) } else { None };
    // What another ability activated since left.
    (state.last_activated_x_value, state.last_activated_sacrifice, state.last_activated_sacrifice_toughness) =
        (kani::any::<bool>().then_some(7), kani::any::<bool>().then_some(other), kani::any::<bool>().then_some(5));
    state.stack.push(StackEntry::Ability {
        source_id: source, ability_index: 0, behavior_card_id: card, targets: Vec::new(), activator,
        x_value: x, target_requirement: None, sacrificed, sacrificed_toughness: toughness,
        loyalty: false, source_transforms: 0, source_back_face: false,
    });

    crate::stack::resolve_top_of_stack(&mut state, &registry);

    // SAFETY: as in `answer`.
    let paid = unsafe { *std::ptr::addr_of!(PAID) };
    assert!(state.stack.is_empty() && paid == Some((x, sacrificed, toughness, Some(activator))));
    assert!(state.resolving_ability_activator.is_none() && state.resolving_ability.is_none());
    std::mem::forget((registry, state));
}

each_resolution! {
    an_ability_resolves_with_what_its_costs_left: an_ability_resolving_with_its_costs(()),
}

/// The printed card a permanent's proof resolves: which of the seven card
/// types it has (instant, sorcery, creature, enchantment, artifact,
/// planeswalker, land), whether it is legendary, its starting loyalty, and
/// whether its cost has an X. Fixed before it is registered, so that each
/// time its data is built it is the same card.
static mut PRINTED: ([bool; 7], bool, Option<u32>, bool) = ([false; 7], false, None, false);

/// A card of no hooks of its own but its printed data and loyalty, as
/// [`PRINTED`] says: its resolution is the default.
struct Printed;

impl CardBehavior for Printed {
    fn card_data(&self) -> CardData {
        use crate::types::{CardType, ManaCost, ManaSymbol, Supertype};
        const TYPES: [CardType; 7] = [CardType::Instant, CardType::Sorcery, CardType::Creature,
            CardType::Enchantment, CardType::Artifact, CardType::Planeswalker, CardType::Land];
        // SAFETY: as in `answer`, for PRINTED, only read here.
        let (has, legendary, _, x) = unsafe { *std::ptr::addr_of!(PRINTED) };
        let mut card_types = Vec::with_capacity(7);
        for (t, &h) in TYPES.iter().zip(&has) {
            if h {
                card_types.push(*t);
            }
        }
        CardData {
            name: "Printed".into(),
            card_types,
            supertypes: if legendary { vec![Supertype::Legendary] } else { Vec::new() },
            cost: Some(ManaCost::new(if x { vec![ManaSymbol::X] } else { Vec::new() })),
            ..CardData::default()
        }
    }
    fn starting_loyalty(&self) -> Option<u32> {
        // SAFETY: as in `card_data`.
        unsafe { (*std::ptr::addr_of!(PRINTED)).2 }
    }
}

/// Where the spell was moved, and the loyalty counters put on it, with
/// the zone it was in then.
static mut ENTERED: Option<Zone> = None;
static mut LOYALTY: Option<(crate::types::CounterType, u32, Zone)> = None;

/// Stands in for `GameState::move_object`, proved in `moving`: the move
/// recorded, once, and the object put in that zone.
#[allow(dead_code)] // named only by `kani::stub`
fn entered(state: &mut GameState, id: ObjectId, to: Zone, _: &CardRegistry) -> bool {
    // SAFETY: as in `answer`, for ENTERED.
    let e = unsafe { &mut *std::ptr::addr_of_mut!(ENTERED) };
    assert!(e.is_none(), "moved once");
    *e = Some(to);
    state.get_object_mut(id).expect("on the board").zone = to;
    true
}

/// Stands in for `GameState::add_counters_quiet`, proved in
/// `simple_actions`: recorded, once, with the zone the object is in.
#[allow(dead_code)] // named only by `kani::stub`
fn loyalty_counters(state: &mut GameState, id: ObjectId, kind: crate::types::CounterType, n: u32) -> bool {
    // SAFETY: as in `answer`, for LOYALTY.
    let l = unsafe { &mut *std::ptr::addr_of_mut!(LOYALTY) };
    assert!(l.is_none(), "counters put on once");
    let zone = state.get_object(id).expect("on the board").zone;
    *l = Some((kind, n, zone));
    true
}

#[kani::proof]
#[kani::unwind(8)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
#[kani::stub(crate::state::GameState::move_object, entered)]
#[kani::stub(crate::state::GameState::add_counters_quiet, loyalty_counters)]
/// A spell of a card with no resolution of its own resolving (the
/// default `CardBehavior::on_resolve`, CR 608.3, 306.5b, 205.4d): a
/// permanent spell (a land, creature, enchantment, artifact or
/// planeswalker card) is put onto the battlefield, legendary exactly when
/// the card is, and, a planeswalker card's, with its starting loyalty in
/// loyalty counters put on it there, or as many as the X it was cast for
/// when its cost has an X (none if it has no X recorded); an instant or
/// sorcery, or a card of none of these types, does nothing (its effects
/// are its card's own, and where it goes after is `resolve_spell_as`'s).
/// For every set of the seven types, legendary or not, with a starting
/// loyalty or none, an X in its cost or not, and any X recorded.
fn a_spell_with_no_resolution_of_its_own_enters_the_battlefield_if_it_is_a_permanent() {
    let has: [bool; 7] = kani::any();
    let (legendary, x): (bool, bool) = (kani::any(), kani::any());
    let loyalty: Option<u32> = if kani::any() { Some(kani::any()) } else { None };
    // SAFETY: as in `answer`, before the card is registered.
    unsafe { *std::ptr::addr_of_mut!(PRINTED) = (has, legendary, loyalty, x) };
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Printed));
    let mut state = crate::proofs::cards::new_game();
    let spell = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())), Zone::Stack, None, None);
    let x_value: Option<u32> = if kani::any() { Some(kani::any()) } else { None };
    state.get_object_mut(spell).expect("just created").x_value = x_value;

    Printed.on_resolve(&mut state, spell, &[], &registry);

    // SAFETY: as in `answer`.
    let (moved, counters) = unsafe { (*std::ptr::addr_of!(ENTERED), *std::ptr::addr_of!(LOYALTY)) };
    let permanent = has[2..].iter().any(|&h| h);
    let obj = state.get_object(spell).expect("on the board");
    if permanent {
        assert!(moved == Some(Zone::Battlefield) && obj.is_legendary == legendary);
        match loyalty {
            None => assert!(counters.is_none()),
            Some(n) => {
                let total = if x { x_value.unwrap_or(0) } else { n };
                assert!(counters == Some((crate::types::CounterType::Loyalty, total, Zone::Battlefield)));
            }
        }
    } else {
        assert!(moved.is_none() && counters.is_none() && !obj.is_legendary);
    }
    std::mem::forget((registry, state));
}

crate::proofs::cards::card_proof! {
    /// Where a spell goes once it has resolved, or once it is countered
    /// (`move_spell_after_resolve`, `move_countered_spell`, CR 608.2n,
    /// 701.5a, 702.34a): into exile if it was cast with flashback, and into
    /// its owner's graveyard otherwise (the owner's is `move_object`'s,
    /// `moving`). For a spell either player's, cast with flashback or not,
    /// resolved or countered.
    fn a_spell_done_with_goes_to_the_graveyard_or_into_exile_if_cast_with_flashback() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let spell = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Stack, None, None);
        let flashback: bool = kani::any();
        state.get_object_mut(spell).expect("just created").cast_with_flashback = flashback;

        if kani::any() {
            state.move_spell_after_resolve(spell, &registry);
        } else {
            state.move_countered_spell(spell, &registry);
        }

        let to = if flashback { Zone::Exile } else { Zone::Graveyard };
        assert!(crate::proofs::cards::asked_for(&[crate::proofs::cards::Act::Move(spell, to)]));
        std::mem::forget((registry, state));
    }
}

/// What a spell's own resolution does before it is done with, by
/// harness: nothing, puts it onto the battlefield, or asks a question.
static mut ENDING: u8 = 0;

/// A card whose resolution does what [`ENDING`] says.
struct Ending;

impl CardBehavior for Ending {
    fn card_data(&self) -> CardData {
        CardData { name: "Ending".into(), ..CardData::default() }
    }
    fn on_resolve(&self, state: &mut GameState, id: ObjectId, _: &[Target], _: &CardRegistry) {
        // SAFETY: as in `answer`, for ENDING.
        match unsafe { *std::ptr::addr_of!(ENDING) } {
            0 => {}
            1 => state.get_object_mut(id).expect("on the board").zone = Zone::Battlefield,
            _ => state.awaiting_action = Some(crate::state::AwaitingAction::DiscardToHandSize { player: PlayerId(0), discard_count: 1 }),
        }
    }
}

/// A spell's resolution ending (`resolve_spell_as`, CR 608.2n, 608.3): it
/// is the spell resolving while its card's resolution runs, and once that
/// is over with no question waiting it is not, and it is moved off the
/// stack (to the graveyard or exile, as above) if it is still there, and
/// left where it is if its resolution put it somewhere (a permanent onto
/// the battlefield); with a question waiting, its resolution is not over:
/// it is still the spell resolving and stays on the stack, until the
/// answer finishes it. For a spell of no target, either player's, its
/// card's resolution each of the three.
fn a_spell_ending(ending: u8) {
    // SAFETY: as in `answer`, before the card resolves.
    unsafe { *std::ptr::addr_of_mut!(ENDING) = ending };
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Ending));
    let mut state = crate::proofs::cards::new_game();
    let caster = PlayerId(u8::from(kani::any::<bool>()));
    let spell = state.create_object(card, caster, Zone::Stack, None, None);

    crate::stack::resolve_spell_as(&mut state, &registry, spell, card, Vec::new(), caster);

    // SAFETY: as in `answer`.
    let moved = unsafe { *std::ptr::addr_of!(MOVED) };
    let zone = state.get_object(spell).expect("on the board").zone;
    match ending {
        0 => assert!(moved == 1 && state.resolving_spell.is_none() && state.awaiting_action.is_none()),
        1 => assert!(moved == 0 && state.resolving_spell.is_none() && zone == Zone::Battlefield),
        _ => assert!(moved == 0 && state.resolving_spell == Some(spell) && zone == Zone::Stack && state.awaiting_action.is_some()),
    }
    std::mem::forget((registry, state));
}

each_resolution! {
    a_spell_whose_resolution_does_nothing_is_moved_off_the_stack: a_spell_ending(0),
    a_spell_whose_resolution_puts_it_onto_the_battlefield_stays_there: a_spell_ending(1),
    a_spell_whose_resolution_asks_a_question_stays_resolving: a_spell_ending(2),
}
