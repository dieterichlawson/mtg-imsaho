//! The answers to the two questions a cast asks before it is made
//! (`actions::choices::resolve_choice`): which cards of a graveyard an
//! exile cost exiles (CR 601.2b, 601.2h), and which targets fill an "up to
//! N" slot (601.2c). An answer that may be given is put back into the cast,
//! which is submitted again as a whole (`casting` proves a cast); one that
//! may not cancels the cast, and nothing has been paid.

use crate::actions::{Action, ResolvedChoice, Target};
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::card_proof;
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::Zone;

/// A card with nothing of its own: the spell being cast.
struct Cast;

impl CardBehavior for Cast {
    fn card_data(&self) -> CardData {
        CardData { name: "Cast".into(), ..CardData::default() }
    }
}

/// The cast submitted again: the spell, its first three targets and
/// cards to exile, how many of each, and the exile count it names.
type Resubmitted = (ObjectId, [Option<Target>; 3], usize, [Option<ObjectId>; 3], usize, Option<u32>);
static mut RESUBMITTED: Option<Resubmitted> = None;

/// Stands in for `engine::submit_action_inner`: the action recorded, once,
/// and a game of nothing handed back in place of the cast made.
#[allow(dead_code)] // named only by `kani::stub`
fn resubmitted(_: &GameState, action: &Action, _: &CardRegistry) -> GameState {
    let Action::CastSpell { object_id, targets, exile_ids, exile_count, .. } = action else {
        panic!("a cast resubmitted");
    };
    assert!(targets.len() <= 3 && exile_ids.len() <= 3, "no more than this proof chooses");
    let held = |i: usize| targets.get(i).cloned();
    let exiled = |i: usize| exile_ids.get(i).copied();
    // SAFETY: a proof runs on one thread, and only this function writes RESUBMITTED.
    let r = unsafe { &mut *std::ptr::addr_of_mut!(RESUBMITTED) };
    assert!(r.is_none(), "submitted once");
    *r = Some((*object_id, [held(0), held(1), held(2)], targets.len(), [exiled(0), exiled(1), exiled(2)], exile_ids.len(), *exile_count));
    crate::proofs::cards::new_game()
}

/// A spell of the second player's in their hand, with its cast stashed
/// while it asks, and three cards in their graveyard.
fn stashed(targets: Vec<Target>) -> (CardRegistry, GameState, ObjectId, [ObjectId; 3]) {
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Cast));
    let blank = registry.register(Box::new(crate::proofs::cards::Blank));
    let mut state = crate::proofs::cards::new_game();
    let caster = PlayerId(1);
    let spell = state.create_object(card, caster, Zone::Hand, None, None);
    let cards = [
        state.create_object(blank, caster, Zone::Graveyard, Some(1), Some(1)),
        state.create_object(blank, caster, Zone::Graveyard, Some(1), Some(1)),
        state.create_object(blank, caster, Zone::Graveyard, Some(1), Some(1)),
    ];
    state.pending_spell_cast = Some(crate::state::PendingSpellCast {
        object_id: spell,
        player: caster,
        card_id: card,
        targets,
        sacrifice: None,
        exile_ids: Vec::new(),
        exile_count: None,
        tap_plan: Vec::new(),
        alternative_cost: None,
        non_x_mana_cost: crate::types::ManaCost::free(),
        is_flashback: false,
        cast_from_graveyard: false,
    });
    state.priority_player = Some(caster);
    (registry, state, spell, cards)
}

/// The cards an exile cost exiles, chosen (CR 601.2b, 601.2h): between
/// the fewest and the most it asks for, each one offered, each once, and
/// the cast is submitted again as it was stashed (its targets) with those
/// cards to exile, in the order chosen, and their number; anything else
/// (too few, too many, one not offered, one twice) cancels the cast, its
/// stash gone and nothing submitted. For an exile of one or two of three
/// cards offered, and each reply a harness's: 0 one, 1 two in either
/// order, 2 none, 3 all three, 4 the spell itself, 5 one twice.
fn exiled(reply: u8) {
    let (registry, mut state, spell, cards) = stashed(vec![Target::Player(PlayerId(0))]);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: PlayerId(1),
        source: spell,
        choice: ResolutionChoiceKind::ChooseExileFromGraveyard {
            description: String::new(),
            options: cards.to_vec(),
            min: 1,
            max: 2,
            source_id: spell,
        },
    });
    let chosen = match reply {
        0 => vec![cards[1]],
        1 => vec![cards[2], cards[0]],
        2 => Vec::new(),
        3 => cards.to_vec(),
        4 => vec![spell],
        _ => vec![cards[0], cards[0]],
    };
    let expected = [chosen.first().copied(), chosen.get(1).copied(), None];
    let n = chosen.len();

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenExileSet(chosen), &registry);

    // SAFETY: as in `resubmitted`.
    let r = unsafe { &*std::ptr::addr_of!(RESUBMITTED) };
    if reply <= 1 {
        assert!(matches!(applied, crate::engine::Applied::Replace(_)));
        let count = u32::try_from(n).ok();
        assert!(*r == Some((spell, [Some(Target::Player(PlayerId(0))), None, None], 1, expected, n, count)));
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(r.is_none() && state.pending_spell_cast.is_none() && state.awaiting_action.is_none());
    }
    std::mem::forget((registry, state));
}

/// The targets of an "up to N" slot chosen (CR 601.2c, 115.1): between
/// the fewest and the most it allows, each one offered, each once, and the
/// cast is submitted again with the targets fixed before the slot first,
/// then those chosen, in the order chosen; anything else (too many, one
/// not offered, one twice) cancels the cast, its stash gone and nothing
/// submitted. For a slot of none to two of three creatures offered, behind
/// one target fixed already, and each reply a harness's: 0 none, 1 one, 2
/// two in either order, 3 all three, 4 one not offered, 5 one twice.
fn targeted(reply: u8) {
    let fixed = Target::Player(PlayerId(0));
    let (registry, mut state, spell, cards) = stashed(Vec::new());
    let offered = [Target::Object(cards[0]), Target::Object(cards[1]), Target::Object(cards[2])];
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: PlayerId(1),
        source: spell,
        choice: ResolutionChoiceKind::ChooseTargetSet {
            description: String::new(),
            options: offered.to_vec(),
            min: 0,
            max: 2,
            source_id: spell,
            fixed: vec![fixed.clone()],
            by_count: Vec::new(),
        },
    });
    let chosen = match reply {
        0 => Vec::new(),
        1 => vec![offered[1].clone()],
        2 => vec![offered[2].clone(), offered[0].clone()],
        3 => offered.to_vec(),
        4 => vec![Target::Object(spell)],
        _ => vec![offered[0].clone(), offered[0].clone()],
    };
    let expected = [Some(fixed.clone()), chosen.first().cloned(), chosen.get(1).cloned()];
    let n = chosen.len();

    let applied = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenTargetSet(chosen), &registry);

    // SAFETY: as in `resubmitted`.
    let r = unsafe { &*std::ptr::addr_of!(RESUBMITTED) };
    if reply <= 2 {
        assert!(matches!(applied, crate::engine::Applied::Replace(_)));
        assert!(matches!(r, Some((s, held, k, [None, None, None], 0, None)) if *s == spell && *k == 1 + n && *held == expected));
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(r.is_none() && state.pending_spell_cast.is_none());
    }
    std::mem::forget((registry, state));
}

/// The proofs of the cast's questions answered, one per reply.
macro_rules! each_cast_reply {
    ($($name:ident: $proof:ident($reply:expr);)*) => {$(
        card_proof! {
            #[kani::stub(crate::engine::submit_action_inner, resubmitted)]
            /// The proof above it for the reply of its name.
            fn $name() {
                $proof($reply);
            }
        }
    )*};
}

each_cast_reply! {
    one_card_offered_is_exiled_for_the_cost: exiled(0);
    two_cards_offered_are_exiled_for_the_cost_in_the_order_chosen: exiled(1);
    no_card_for_an_exile_of_one_or_two_cancels_the_cast: exiled(2);
    three_cards_for_an_exile_of_one_or_two_cancel_the_cast: exiled(3);
    a_card_not_offered_for_the_exile_cancels_the_cast: exiled(4);
    a_card_chosen_twice_for_the_exile_cancels_the_cast: exiled(5);
    no_target_for_an_up_to_slot_is_cast_with_the_fixed_ones: targeted(0);
    one_target_offered_is_cast_after_the_fixed_ones: targeted(1);
    two_targets_offered_are_cast_in_the_order_chosen: targeted(2);
    three_targets_for_an_up_to_two_slot_cancel_the_cast: targeted(3);
    a_target_not_offered_cancels_the_cast: targeted(4);
    a_target_chosen_twice_cancels_the_cast: targeted(5);
}
