//! The spells and permanents that read or return cards from a graveyard,
//! or copy a creature.
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

use super::{acted, any_object, asked_each, asked_for, board, card_proof, triggers, Act, Form, Kinds, Snapshot};
use crate::actions::Target;
use crate::cards::{CardBehavior, TargetFilter, TargetRequirement as R, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::{EnteringPermanent, ReplaceableEvent as Ev, Replacement};
use crate::state::{AwaitingAction, ResolutionChoiceKind};
use crate::types::{CardType, CounterType, Zone};

use crate::cards::isd;

const KINDS: Kinds = Kinds { types: &[CardType::Creature, CardType::Land], subtypes: &["Human"], keywords: &[] };

card_proof! {
    /// Cackling Counterpart: "target creature you control"; a token copy of
    /// it for the spell's controller, if it is still on the battlefield;
    /// nothing otherwise.
    fn cackling_counterpart_copies_your_creature() {
        let counterpart = isd::cackling_counterpart::CacklingCounterpart;
        assert!(counterpart.target_requirement() == R::CreatureWithFilter(TargetFilter::YouControl));
        let (registry, mut state, blank, card) = board(Box::new(counterpart));
        let other = any_object(&mut state, blank, &KINDS, Form::Card);
        let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
        let target = if kani::any() { Target::Object(other) } else { Target::Illegal };
        let on = state.get_object(other).expect("on the board").zone == Zone::Battlefield;
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_resolve(&mut state, spell, &[target.clone()], &registry);
        if target == Target::Object(other) && on {
            assert!(asked_for(&[Act::TokenCopy(other, PlayerId(0))]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Creeping Renaissance: its controller is asked to choose a permanent
    /// type of the five (CR 110.4); then every card of that type in their
    /// graveyard is returned to their hand, and no other card. Over the
    /// choices "Creature" and "Land", and two cards, each either player's,
    /// in a graveyard or not, a creature, a land, or neither.
    fn creeping_renaissance_returns_every_card_of_the_chosen_type() {
        let (registry, mut state, blank, card) = board(Box::new(isd::creeping_renaissance::CreepingRenaissance));
        let others = [any_object(&mut state, blank, &KINDS, Form::Card), any_object(&mut state, blank, &KINDS, Form::Card)];
        let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);
        let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player: PlayerId(0), source, choice: ResolutionChoiceKind::ChooseCardType { options, controller: PlayerId(0), .. },
        }) if *source == spell && options.len() == 5 && options[0] == "Creature" && options[1] == "Artifact"
            && options[2] == "Enchantment" && options[3] == "Land" && options[4] == "Planeswalker");
        assert!(asked && acted() == 0);
        state.awaiting_action = None;
        let land: bool = kani::any();
        let (name, kind) = if land { ("Land", CardType::Land) } else { ("Creature", CardType::Creature) };
        registry.get(card).expect("registered").on_card_type_choice(&mut state, spell, name, &registry);
        let returned: Vec<ObjectId> = others.iter().copied()
            .filter(|id| {
                let o = state.get_object(*id).expect("on the board");
                o.zone == Zone::Graveyard && o.owner == PlayerId(0) && o.card_types.contains(&kind)
            })
            .collect();
        assert!(asked_each(returned.len(), |i| Act::Move(returned[i], Zone::Hand)) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state, returned));
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
                let mut more = e.counters.clone();
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

card_proof! {
    /// Murder of Crows: triggers for every other creature's death; its
    /// controller is asked whether to draw; on "yes" they draw, and if a
    /// card was drawn they discard one (without being asked if one is all
    /// they hold, else the one they choose); on "no", or with no card
    /// drawn, nothing more.
    fn murder_of_crows_may_draw_then_discards() {
        assert!(triggers(&isd::murder_of_crows::MurderOfCrows) == [(TriggerKind::AnyCreatureDies, None)]);
        let (registry, mut state, blank, card) = board(Box::new(isd::murder_of_crows::MurderOfCrows));
        let you = PlayerId(u8::from(kani::any::<bool>()));
        let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(me).expect("just created").controller = you;
        let in_library: bool = kani::any();
        if in_library {
            let top = state.create_object(blank, you, Zone::Library, None, None);
            if you == PlayerId(0) {
                state.players[0].library_order.push(top);
            } else {
                state.players[1].library_order.push(top);
            }
        }
        let in_hand: bool = kani::any();
        if in_hand {
            state.create_object(blank, you, Zone::Hand, None, None);
        }
        let behavior = registry.get(card).expect("registered");
        behavior.on_any_creature_dies(&mut state, me, ObjectId(100), PlayerId(0), &[], 1, false, &[], &registry);
        let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::YesNo { source_card, .. },
        }) if *player == you && *source == me && *source_card == me);
        assert!(asked && acted() == 0);
        state.awaiting_action = None;
        let yes: bool = kani::any();
        behavior.on_yes_no_choice(&mut state, me, yes, &registry);
        let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, you).iter().map(|o| o.id).collect();
        if !yes {
            assert!(acted() == 0);
        } else if !in_library {
            assert!(asked_for(&[Act::Draw(you, 1)]));
        } else if hand.len() == 1 {
            assert!(asked_for(&[Act::Draw(you, 1), Act::Discard(hand[0])]));
        } else {
            let chooses = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source, choice: ResolutionChoiceKind::ChooseCardFromHand { player: chooser, cards, discard_immediately: true, remaining: 1, .. },
            }) if *player == you && *source == me && *chooser == you && *cards == hand);
            assert!(chooses && asked_for(&[Act::Draw(you, 1)]));
        }
        std::mem::forget((registry, state, hand));
    }
}
