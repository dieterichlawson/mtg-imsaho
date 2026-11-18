//! The check of the targets a submitted cast or activation names
//! (`engine::targeting::targets_are_legal`, CR 601.2c, 602.2b, 115.1,
//! 702.16b): what `casting` and `activating` stand in for as any answer.
//! The questions it asks of each target are any answer here, the same each
//! time they are asked of the same target: whether it meets the
//! requirement (`stack::is_target_legal`, with hexproof), whether the
//! source may target it at all (`can_be_targeted_by`: protection, and
//! hexproof again), and whether it meets what the card itself asks
//! (`is_valid_target`). Each is its own proof.

use crate::proofs::held::Held;
use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::card_proof;
use crate::state::GameState;

/// The answers, by target (objects 1 to 3) and question.
static mut ANSWERS: Held<[[Option<bool>; 3]; 3]> = Held::new([[None; 3]; 3], (&raw const ANSWERS).cast());

fn answer(id: ObjectId, k: usize) -> bool {
    let i = usize::try_from(id.0).expect("an id of the board");
    assert!((1..=3).contains(&i), "a target this proof does not name");
    // SAFETY: a proof runs on one thread, and only this function names ANSWERS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ANSWERS.value) };
    *answers[i - 1][k].get_or_insert_with(kani::any::<bool>)
}

#[allow(dead_code)] // each named only by `kani::stub`
fn meets(_: &GameState, target: &Target, _: &TargetRequirement, _: PlayerId, _: Option<ObjectId>, _: &CardRegistry) -> bool {
    match target {
        Target::Object(id) => answer(*id, 0),
        _ => false,
    }
}
#[allow(dead_code)]
fn targetable(_: &GameState, id: ObjectId, _: PlayerId, _: Option<ObjectId>, _: &CardRegistry) -> bool {
    answer(id, 1)
}

/// A card whose own restriction on a target is any answer.
struct Restricting;

impl CardBehavior for Restricting {
    fn card_data(&self) -> CardData {
        CardData { name: "Restricting".into(), ..CardData::default() }
    }
    fn is_valid_target(&self, _: &GameState, _: PlayerId, target: &Target, _: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => answer(*id, 2),
            _ => false,
        }
    }
}

/// The targets named for `req`, the first `n` of objects 1 to 3, checked:
/// whether the check said they are legal, and whether target `i` passes
/// all three questions.
fn checked(req: &TargetRequirement) -> (usize, bool, [bool; 3]) {
    let registry = CardRegistry::new();
    let state = crate::proofs::cards::new_game();
    let n = usize::from(kani::any::<u8>() % 4);
    let named = [Target::Object(ObjectId(1)), Target::Object(ObjectId(2)), Target::Object(ObjectId(3))];

    let legal = crate::engine::targeting::targets_are_legal(&state, req, &named[..n], PlayerId(0), ObjectId(4), &Restricting, &registry);

    let mut each = [false; 3];
    for (i, e) in each.iter_mut().enumerate().take(n) {
        let id = ObjectId(i as u64 + 1);
        *e = answer(id, 0) && answer(id, 1) && answer(id, 2);
    }
    std::mem::forget((registry, state));
    (n, legal, each)
}

macro_rules! each_requirement {
    ($($(#[$doc:meta])* $name:ident: $req:expr, $holds:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::stack::is_target_legal, meets)]
            #[kani::stub(crate::engine::targeting::can_be_targeted_by, targetable)]
            $(#[$doc])*
            fn $name() {
                let req = $req;
                let (n, legal, each) = checked(&req);
                let holds: fn(usize, [bool; 3]) -> bool = $holds;
                assert!(legal == holds(n, each));
                std::mem::forget(req);
            }
        }
    )*};
}

each_requirement! {
    /// A spell or ability that targets nothing names nothing (CR 601.2c):
    /// for none to three targets named.
    nothing_is_named_for_a_requirement_of_no_target: TargetRequirement::None, |n, _| n == 0;
    /// One that names one target, here a creature, names exactly one, and
    /// it meets the requirement, may be targeted by the source (protection,
    /// hexproof: CR 702.16b, 702.11b) and meets what the card asks (CR
    /// 601.2c, 115.1): for none to three named, each answer any.
    exactly_one_legal_target_is_named_for_a_requirement_of_one: TargetRequirement::Creature, |n, each| n == 1 && each[0];
    /// An "up to two" requirement names at most two, each meeting the
    /// requirement, targetable by the source and meeting what the card
    /// asks (defect 22: the source's was not asked): for none to three
    /// named, each answer any.
    up_to_two_legal_targets_are_named_for_a_requirement_of_up_to_two:
        TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature)),
        |n, each| n <= 2 && each[..n].iter().all(|&e| e);
}

/// The modes asked of (each by its address) and the targets they were
/// asked with, in order, and what each said.
static mut MODES_ASKED: Held<[Option<(usize, usize, usize, bool)>; 2]> = Held::new([None; 2], (&raw const MODES_ASKED).cast());

/// Stands in for `targeting::legal_under_mode`: any answer, recorded.
#[allow(dead_code)] // named only by `kani::stub`
fn under_mode(_: &GameState, mode: &TargetRequirement, targets: &[Target], _: PlayerId, _: ObjectId, _: &dyn CardBehavior, _: &CardRegistry) -> bool {
    let legal: bool = kani::any();
    // SAFETY: a proof runs on one thread, and only this function and the
    // harness name MODES_ASKED.
    let asked = unsafe { &mut *std::ptr::addr_of_mut!(MODES_ASKED.value) };
    let slot = asked.iter_mut().find(|a| a.is_none()).expect("no more modes asked than the requirement has");
    *slot = Some((std::ptr::from_ref(mode) as usize, targets.as_ptr() as usize, targets.len(), legal));
    legal
}

card_proof! {
    #[kani::stub(crate::engine::targeting::legal_under_mode, under_mode)]
    /// A modal requirement's targets are legal exactly when they are under
    /// one of its modes (CR 700.2a, 601.2c): each mode asked in turn, with
    /// the targets as named, until one says they are, and none after. Here
    /// two modes, as Ghoulcaller's Chant prints, for none to three targets
    /// named. What a mode says is the proofs above, of each kind of
    /// requirement a mode is: followed through each mode, this proof ran
    /// out of memory in symbolic execution, at 12 GB after 6.5 million
    /// steps.
    fn the_targets_of_a_modal_requirement_are_legal_under_one_of_its_modes() {
        let req = TargetRequirement::ModalChoice(vec![
            TargetRequirement::Creature,
            TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature)),
        ]);
        let registry = CardRegistry::new();
        let state = crate::proofs::cards::new_game();
        let n = usize::from(kani::any::<u8>() % 4);
        let named = [Target::Object(ObjectId(1)), Target::Object(ObjectId(2)), Target::Object(ObjectId(3))];

        let legal = crate::engine::targeting::targets_are_legal(&state, &req, &named[..n], PlayerId(0), ObjectId(4), &Restricting, &registry);

        let TargetRequirement::ModalChoice(modes) = &req else { unreachable!() };
        // SAFETY: as in `under_mode`.
        let asked = unsafe { *std::ptr::addr_of!(MODES_ASKED.value) };
        let of = |i: usize, (mode, at, len, _): (usize, usize, usize, bool)| {
            mode == std::ptr::from_ref(&modes[i]) as usize && at == named.as_ptr() as usize && len == n
        };
        match asked {
            [Some(first), None] => {
                assert!(of(0, first) && first.3 && legal);
            }
            [Some(first), Some(second)] => {
                assert!(of(0, first) && !first.3 && of(1, second) && legal == second.3);
            }
            _ => panic!("the first mode is asked"),
        }
        std::mem::forget((registry, state, req));
    }
}
