//! Transforming a permanent, which a card proof's recorder
//! (`Act::Transform`) stands in for: `helpers::apply_transform` (CR
//! 701.28), on a board of a double-faced card and the blank card.

use crate::cards::{helpers, helpers::TransformResult, CardBehavior, CardData};
use crate::ids::PlayerId;
use crate::proofs::cards::{board, Snapshot};
use crate::state::StackEntry;
use crate::types::Zone;

/// A double-faced card that prints nothing but its faces' names.
struct TwoFaced;

impl CardBehavior for TwoFaced {
    fn card_data(&self) -> CardData {
        CardData { name: "Front".into(), ..CardData::default() }
    }

    fn back_face_data(&self) -> Option<CardData> {
        Some(CardData { name: "Back".into(), ..CardData::default() })
    }
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// "Transform it" (CR 701.28a): a permanent represented by a
/// double-faced card is turned over, so its other face is up and named
/// so, and it has transformed once more; and the answer says it did.
/// Anything else is left as it was, and the answer says why: a card not on
/// the battlefield is no permanent (400.7); a token, even one copying a
/// double-faced card, is no double-faced card and cannot transform
/// (701.28c); nor can a permanent whose card has one face, or one that is
/// a copy by way of a card with one (Evil Twin copying a werewolf); nor
/// one that has transformed since its own ability resolving now was put on
/// the stack (701.28f). For a card of either face up, either card, either
/// player's.
fn transforming_turns_over_exactly_a_double_faced_permanent() {
    let (registry, mut state, blank, two) = board(Box::new(TwoFaced));
    let double: bool = kani::any();
    let on: bool = kani::any();
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let id = state.create_object(if double { two } else { blank }, owner, if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    let (token, back_up, copy, since): (bool, bool, bool, bool) = (kani::any(), kani::any(), kani::any(), kani::any());
    let transforms = u32::from(kani::any::<u8>() % 3);
    let o = state.get_object_mut(id).expect("just created");
    o.is_token = token;
    o.is_transformed = back_up;
    o.name = if back_up { "Back".into() } else { "Front".into() };
    o.transforms = transforms;
    if copy {
        o.copy_grantor = Some(blank);
    }
    if since {
        state.resolving_ability = Some(StackEntry::Ability {
            source_id: id, ability_index: 0, behavior_card_id: two, targets: Vec::new(), activator: owner,
            x_value: None, target_requirement: None, sacrificed: None, sacrificed_toughness: None, loyalty: false,
            source_transforms: Some(transforms.wrapping_sub(1)), source_name: None,
        });
    }
    let before = Snapshot::of(&state);

    let result = helpers::apply_transform(&mut state, id, &registry);

    let expected = if !on {
        TransformResult::NotAPermanent
    } else if token {
        TransformResult::Token
    } else if !double || copy {
        TransformResult::SingleFaced
    } else if since {
        TransformResult::AlreadyTransformed
    } else {
        TransformResult::Transformed
    };
    assert!(result == expected);
    let o = state.get_object(id).expect("on the board");
    if result == TransformResult::Transformed {
        assert!(Snapshot::of(&state) == before.with_transformed(id, !back_up));
        assert!(o.transforms == transforms + 1 && o.name == if back_up { "Front" } else { "Back" });
    } else {
        assert!(Snapshot::of(&state) == before);
        assert!(o.transforms == transforms && o.name == if back_up { "Back" } else { "Front" });
    }
    kani::cover!(result == TransformResult::Transformed && back_up, "back to its front");
    std::mem::forget((registry, state));
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
/// CR 701.28f for a triggered ability: "If an activated or triggered
/// ability of a permanent that isn't a delayed triggered ability of that
/// permanent tries to transform it, the permanent transforms only if it
/// hasn't transformed or converted since the ability was put onto the
/// stack." A double-faced permanent, either face up, with a trigger
/// resolving that is its own or another permanent's, delayed or not, put
/// on the stack when the permanent had transformed as often as now, or
/// less often, or with no count kept (an entry from before the count was
/// kept, which is not checked): it is turned over, and has transformed
/// once more, unless the trigger is its own, not delayed, and was put on
/// the stack before its last transform, when it is left as it was and
/// the answer says why.
fn a_permanents_own_trigger_does_not_transform_it_again() {
    use crate::triggers::{collect::Collector, TriggerEvent};
    let (registry, mut state, blank, two) = board(Box::new(TwoFaced));
    let owner = PlayerId(u8::from(kani::any::<bool>()));
    let id = state.create_object(two, owner, Zone::Battlefield, None, None);
    let other = state.create_object(blank, owner, Zone::Battlefield, None, None);
    let back_up: bool = kani::any();
    let transforms = u32::from(kani::any::<u8>() % 3);
    let o = state.get_object_mut(id).expect("just created");
    o.is_transformed = back_up;
    o.name = if back_up { "Back".into() } else { "Front".into() };
    o.transforms = transforms;
    let (own, delayed): (bool, bool) = (kani::any(), kani::any());
    let stamp = match kani::any::<u8>() % 3 {
        0 => None,
        1 => Some(transforms),
        _ => Some(transforms.wrapping_add(1)),
    };
    let event = if delayed { TriggerEvent::DelayedTokenExile { target_id: id } } else { TriggerEvent::Upkeep };
    // Built where every trigger is (`tests/trigger_source_independence.rs`).
    let mut c = Collector::new(owner);
    c.emit(if own { id } else { other }, two, owner, String::new(), event);
    let mut trigger = c.ap.pop().expect("just emitted");
    std::mem::forget(c);
    trigger.source.source_transforms = stamp;
    state.resolving_ability = Some(StackEntry::Trigger(trigger));
    let before = Snapshot::of(&state);

    let result = helpers::apply_transform(&mut state, id, &registry);

    let held = own && !delayed && stamp.is_some_and(|s| s != transforms);
    assert!(result == if held { TransformResult::AlreadyTransformed } else { TransformResult::Transformed });
    let o = state.get_object(id).expect("on the board");
    if held {
        assert!(Snapshot::of(&state) == before);
        assert!(o.transforms == transforms && o.name == if back_up { "Back" } else { "Front" });
    } else {
        assert!(Snapshot::of(&state) == before.with_transformed(id, !back_up));
        assert!(o.transforms == transforms + 1 && o.name == if back_up { "Front" } else { "Back" });
    }
    kani::cover!(held, "its own trigger, stacked before a transform");
    kani::cover!(own && delayed && stamp.is_some_and(|s| s != transforms) && !held, "a delayed trigger of its own");
    std::mem::forget((registry, state));
}
