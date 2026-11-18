//! The spells and permanents that read or return cards from a graveyard,
//! or copy a creature.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Cackling Counterpart, {1}{U}{U} Instant: "Create a token that's a copy
//!   of target creature you control. Flashback {5}{U}{U}"
//! - Creeping Renaissance, {3}{G}{G} Sorcery: "Choose a permanent type.
//!   Return all cards of the chosen type from your graveyard to your hand.
//!   Flashback {5}{G}{G}"
//! - Dearly Departed, 5/5 Flying: "As long as this creature is in your
//!   graveyard, each Human creature you control enters with an additional
//!   +1/+1 counter on it."
//! - Murder of Crows, 4/4 Flying: "Whenever another creature dies, you may
//!   draw a card. If you do, discard a card."

use super::{acted, any_object, board, card_proof, Form, Kinds, Snapshot};
use crate::cards::{CardBehavior, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::{EnteringPermanent, ReplaceableEvent as Ev, Replacement};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{CardType, CounterType, Zone};

use crate::cards::isd;

const KINDS: Kinds = Kinds { types: &[CardType::Creature, CardType::Land], subtypes: &["Human"], keywords: &[] };

/// Creeping Renaissance's board: two cards, each either player's, in a
/// graveyard or not, a creature, a land, or neither; the spell resolved,
/// and whether it asked its controller to choose a permanent type of the
/// five (CR 110.4). The registry, the game after, the spell, the two
/// cards, and the board before.
fn creeping_renaissance_resolves() -> (CardRegistry, GameState, ObjectId, [ObjectId; 2], Snapshot, bool) {
    let (registry, mut state, blank, card) = board(Box::new(isd::creeping_renaissance::CreepingRenaissance));
    let others = [any_object(&mut state, blank, &KINDS, Form::Card), any_object(&mut state, blank, &KINDS, Form::Card)];
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);
    let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        player: PlayerId(0), source, choice: ResolutionChoiceKind::ChooseCardType { options, controller: PlayerId(0), .. },
    }) if *source == spell && options.len() == 5 && options[0] == "Creature" && options[1] == "Artifact"
        && options[2] == "Enchantment" && options[3] == "Land" && options[4] == "Planeswalker");
    (registry, state, spell, others, before, asked)
}

card_proof! {
    /// Creeping Renaissance: its controller is asked to choose a permanent
    /// type of the five, and nothing more is done yet.
    fn creeping_renaissance_asks_for_a_permanent_type() {
        let (registry, state, _, _, _, asked) = creeping_renaissance_resolves();
        assert!(asked && acted() == 0);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Dearly Departed: from its owner's graveyard and nowhere else (CR
    /// 113.6, its replacement's zone), each Human creature entering under
    /// its owner's control enters with one more +1/+1 counter; anything
    /// else entering, and every other event, as it is.
    fn dearly_departed_gives_your_entering_humans_a_counter_from_the_graveyard() {
        let departed = isd::dearly_departed::DearlyDeparted;
        assert!(departed.replacement_zones() == [Zone::Graveyard]);
        let (registry, mut state, blank, card) = board(Box::new(departed));
        let in_graveyard: bool = kani::any();
        let me = state.create_object(card, PlayerId(0), if in_graveyard { Zone::Graveyard } else { Zone::Battlefield }, None, None);
        let entering = state.create_object(blank, PlayerId(0), Zone::Hand, None, None);
        let human: bool = kani::any();
        if human {
            state.get_object_mut(entering).expect("just created").subtypes = vec!["Human".into()];
        }
        let controller = PlayerId(u8::from(kani::any::<bool>()));
        let mut counters = Vec::with_capacity(2);
        if kani::any() {
            counters.push((CounterType::PlusOnePlusOne, 2));
        }
        let e = EnteringPermanent { object: entering, from: Some(Zone::Hand), controller, tapped: kani::any(), counters, copy_of: None };
        let event = if kani::any() { Ev::EntersBattlefield(e.clone()) } else { Ev::CreatesTokens { controller, count: 1 } };
        let replaced = registry.get(card).expect("registered").replace_event(&mut state, me, &event, &registry);
        match &event {
            Ev::EntersBattlefield(_) if in_graveyard && human && controller == PlayerId(0) => {
                // Built at its size: a clone is exactly as long as the
                // counters were, and one grown stands in as a failure
                // (`no_growing`).
                let mut more = Vec::with_capacity(e.counters.len() + 1);
                more.extend_from_slice(&e.counters);
                more.push((CounterType::PlusOnePlusOne, 1));
                assert!(replaced == Some(Replacement::Modified(Ev::EntersBattlefield(EnteringPermanent { counters: more, ..e.clone() }))));
            }
            _ => {
                assert!(replaced.is_none());
            }
        }
        std::mem::forget((registry, state, replaced, event, e));
    }
}

