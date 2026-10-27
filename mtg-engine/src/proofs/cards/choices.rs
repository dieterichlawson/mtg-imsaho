//! The cards whose effect stops to ask a player something and goes on with
//! the answer: what each asks, of whom, and what it does with each answer.
//!
//! - Nevermore, {1}{W}{W} Enchantment: "As this enchantment enters, choose
//!   a nonland card name. Spells with the chosen name can't be cast."
//! - Caravan Vigil, {G} Sorcery: "Search your library for a basic land
//!   card, reveal it, put it into your hand, then shuffle. Morbid — You may
//!   put that card onto the battlefield instead of putting it into your
//!   hand if a creature died this turn."
//! - Frightful Delusion, {2}{U} Instant: "Counter target spell unless its
//!   controller pays {1}. That player discards a card."
//! - Divine Reckoning, {2}{W}{W} Sorcery: "Each player chooses a creature
//!   they control. Destroy the rest. Flashback {5}{W}{W}"
//!
//! An answer comes back through the engine's handler for the question
//! asked (`engine::actions::choices`), which hands it to the card; where
//! the handler does part of what the card says (paying, or not, for
//! "unless its controller pays"), it is run here with the card, and where
//! it only passes the answer on (a chosen target, to the card's
//! `resolve_card_effect`, through `engine::apply_pending_effect`), the
//! card's own hook is called with every answer the handler accepts.

use super::{acted, asked_for, board, card_proof, Act, Snapshot};
use crate::actions::{ResolvedChoice, Target};
use crate::cards::{CardBehavior, CardRegistry};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::{CardType, ContinuousEffect as E, ManaCost, ManaSymbol, Zone};

use crate::cards::isd;

fn any_player() -> PlayerId {
    PlayerId(u8::from(kani::any::<bool>()))
}

/// This snapshot, waiting on an answer or not.
fn asking(before: &Snapshot, asking: bool) -> Snapshot {
    Snapshot { asking, ..before.clone() }
}

card_proof! {
    /// Nevermore: its name is chosen as it enters (CR 614.12), which is no
    /// trigger; its controller is asked to choose among every nonland card
    /// name there is, in order; the name chosen is the one whose spells
    /// can't be cast (the `PreventCastingNamed` effect it then has, which
    /// the engine's casting rules read); an answer past the end of the
    /// names is refused, and the question stands. Nothing else. Over a
    /// registry of a nonland card, a land and Nevermore, and Nevermore
    /// under either player.
    fn nevermore_names_a_nonland_card() {
        let nevermore = isd::nevermore::Nevermore;
        assert!(nevermore.chooses_as_it_enters() && !nevermore.has_etb_handler());
        let (mut registry, mut state, _blank, card) = board(Box::new(nevermore));
        registry.register(Box::new(isd::gavony_township::GavonyTownship));
        let you = any_player();
        let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(me).expect("just created").controller = you;
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_enter_battlefield(&mut state, me, &[], &registry);
        let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::ChooseCardName { options, source_id, .. },
        }) if *player == you && *source == me && *source_id == me
            && options.len() == 2 && options[0] == "Blank" && options[1] == "Nevermore");
        assert!(asked && acted() == 0 && Snapshot::of(&state) == asking(&before, true));
        let index: usize = kani::any();
        kani::assume(index <= 2);
        let _ = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenIndex(index, String::new()), &registry);
        let effects = state.get_object(me).expect("on the board").instance_continuous_effects.as_deref();
        if index < 2 {
            let name = if index == 0 { "Blank" } else { "Nevermore" };
            assert!(matches!(effects, Some([E::PreventCastingNamed { name: n }]) if n == name));
            assert!(Snapshot::of(&state) == before);
        } else {
            assert!(effects.is_none() && Snapshot::of(&state) == asking(&before, true));
        }
        assert!(acted() == 0);
        std::mem::forget((registry, state));
    }
}

/// Caravan Vigil's board: the spell on the stack under either player;
/// player 0's library, of any of a nonland card, two Forests (the same
/// card twice) and a nonbasic land, in that order; player 1's of a
/// Forest; and whether a creature died this turn.
struct Vigil {
    registry: CardRegistry,
    state: GameState,
    card: CardId,
    spell: ObjectId,
    you: PlayerId,
    /// The basic land cards in `you`'s library, one of each card, in
    /// library order: the Forest nearest the top.
    forest: Option<ObjectId>,
}

fn vigil() -> Vigil {
    let (mut registry, mut state, blank, card) = board(Box::new(isd::caravan_vigil::CaravanVigil));
    let forest = registry.register(Box::new(crate::cards::forest::Forest));
    let township = registry.register(Box::new(isd::gavony_township::GavonyTownship));
    let you = any_player();
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.get_object_mut(spell).expect("just created").controller = you;
    let mut first_forest = [None; 2];
    for of in [blank, forest, township, forest] {
        let id = state.create_object(of, PlayerId(0), Zone::Library, None, None);
        if kani::any() {
            state.players[0].library_order.push(id);
            if of == forest && first_forest[0].is_none() {
                first_forest[0] = Some(id);
            }
        } else {
            state.get_object_mut(id).expect("just created").zone = Zone::Hand;
        }
    }
    let theirs = state.create_object(forest, PlayerId(1), Zone::Library, None, None);
    state.players[1].library_order.push(theirs);
    first_forest[1] = Some(theirs);
    state.creature_died_this_turn = kani::any();
    let forest = if you == PlayerId(0) { first_forest[0] } else { first_forest[1] };
    Vigil { registry, state, card, spell, you, forest }
}

card_proof! {
    /// Caravan Vigil, resolving: its controller searches their own library
    /// (CR 701.23a) for a basic land card, and is asked which, offered one
    /// of each card that is one (CR 701.23b: a player need not find a card
    /// that is there, so "none" is an answer too); with none to find,
    /// their library is shuffled and nothing else happens. Nothing else.
    fn caravan_vigil_offers_one_of_each_basic_land_in_your_library() {
        let Vigil { registry, mut state, card, spell, you, forest } = vigil();
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);
        match forest {
            Some(land) => {
                let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                    player, source, choice: ResolutionChoiceKind::ChooseTarget {
                        options, optional: true, effect: PendingEffect::CardEffect { source_id, .. }, ..
                    },
                }) if *player == you && *source == spell && *source_id == spell
                    && options.len() == 1 && options[0] == Target::Object(land));
                assert!(asked && acted() == 0 && Snapshot::of(&state) == asking(&before, true));
            }
            None => {
                assert!(asked_for(&[Act::Shuffle(you)]) && Snapshot::of(&state) == before);
            }
        }
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Caravan Vigil, the land chosen: with no creature dead this turn, it
    /// goes to its controller's hand and their library is shuffled; with
    /// one (morbid), they are asked whether to put it onto the battlefield
    /// instead, and on the answer it goes there or to their hand, and the
    /// library is shuffled. No land chosen, the library is shuffled all
    /// the same, the search having happened (CR 701.23a). Nothing else.
    fn caravan_vigil_puts_the_land_into_your_hand_or_with_morbid_onto_the_battlefield() {
        let Vigil { registry, mut state, card, spell, you, forest } = vigil();
        let Some(land) = forest else {
            std::mem::forget((registry, state));
            return;
        };
        let vigil = registry.get(card).expect("registered");
        let before = Snapshot::of(&state);
        if kani::any() {
            vigil.on_declined_choice(&mut state, spell, &registry);
            assert!(asked_for(&[Act::Shuffle(you)]) && Snapshot::of(&state) == before);
            std::mem::forget((registry, state));
            return;
        }
        vigil.resolve_card_effect(&mut state, spell, "", &Target::Object(land), &registry);
        if !state.creature_died_this_turn {
            assert!(asked_for(&[Act::Move(land, Zone::Hand), Act::Shuffle(you)]) && Snapshot::of(&state) == before);
            std::mem::forget((registry, state));
            return;
        }
        let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::YesNo { source_card, .. },
        }) if *player == you && *source == spell && *source_card == spell);
        assert!(asked && acted() == 0 && Snapshot::of(&state) == asking(&before, true));
        state.awaiting_action = None;
        let yes: bool = kani::any();
        vigil.on_yes_no_choice(&mut state, spell, yes, &registry);
        let to = if yes { Zone::Battlefield } else { Zone::Hand };
        assert!(asked_for(&[Act::Move(land, to), Act::Shuffle(you)]) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Frightful Delusion: its target still a spell on the stack, that
    /// spell's controller is asked whether to pay {1}; then, paying and
    /// able to, the spell is not countered, and otherwise it is; and either
    /// way that player discards a card (the card's ruling: "The player
    /// discards a card even if they pay {1}"), chosen by them when they
    /// have more than one (CR 701.9a), and with none, nothing. Its target
    /// gone, nothing at all (CR 608.2b). Over the target spell under
    /// either player, and each player's hand of up to two cards.
    fn frightful_delusion_counters_unless_paid_and_that_player_discards() {
        let (registry, mut state, blank, card) = board(Box::new(isd::frightful_delusion::FrightfulDelusion));
        let me = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
        let them = any_player();
        let gone: bool = kani::any();
        let spell = state.create_object(blank, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Stack }, None, None);
        if gone {
            state.players[0].graveyard_order.push(spell);
        } else {
            state.get_object_mut(spell).expect("just created").controller = them;
        }
        let mut hands = [[None; 2]; 2];
        for (p, hand) in hands.iter_mut().enumerate() {
            for slot in hand.iter_mut() {
                if kani::any() {
                    *slot = Some(state.create_object(blank, PlayerId(p as u8), Zone::Hand, None, None));
                }
            }
        }
        let target = if kani::any() { Target::Object(spell) } else { Target::Illegal };
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_resolve(&mut state, me, &[target.clone()], &registry);
        if gone || target == Target::Illegal {
            assert!(acted() == 0 && Snapshot::of(&state) == before);
            std::mem::forget((registry, state));
            return;
        }
        let one = ManaCost::new(vec![ManaSymbol::Generic(1)]);
        let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::PayOrNot { spell_id, source_spell_id, cost, .. },
        }) if *player == them && *source == me && *spell_id == spell && *source_spell_id == me && *cost == one);
        assert!(asked && acted() == 0);
        let pay: bool = kani::any();
        let _ = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::PayDecision(pay), &registry);
        let paid = pay && matches!(super::act(0), Some(Act::Pay(_, _, true)));
        let mut expected = Vec::with_capacity(3);
        if pay {
            expected.push(Act::Pay(them, one.clone(), paid));
        }
        if !paid {
            expected.push(Act::Counter { spell, exile: false });
        }
        let hand = if them == PlayerId(0) { hands[0] } else { hands[1] };
        let cards: Vec<ObjectId> = hand.iter().flatten().copied().collect();
        if cards.len() == 1 {
            expected.push(Act::Discard(cards[0]));
        }
        assert!(asked_for(&expected));
        if cards.len() == 2 {
            let asks_to_discard = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source, choice: ResolutionChoiceKind::ChooseCardFromHand {
                    player: from, cards: offered, discard_immediately: true, remaining: 1, ..
                },
            }) if *player == them && *from == them && *source == me && *offered == cards);
            assert!(asks_to_discard && Snapshot::of(&state) == asking(&before, true));
        } else {
            assert!(Snapshot::of(&state) == before);
        }
        std::mem::forget((registry, state, one, expected, cards));
    }
}

/// Divine Reckoning's board: the spell on the stack, either player active,
/// and three permanents, each either player's, each a creature or not.
struct Reckoning {
    registry: CardRegistry,
    state: GameState,
    card: CardId,
    spell: ObjectId,
    permanents: [ObjectId; 3],
}

fn reckoning() -> Reckoning {
    let (registry, mut state, blank, card) = board(Box::new(isd::divine_reckoning::DivineReckoning));
    state.active_player = any_player();
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let mut permanents = [spell; 3];
    for slot in &mut permanents {
        let id = state.create_object(blank, any_player(), Zone::Battlefield, None, None);
        let o = state.get_object_mut(id).expect("just created");
        o.card_types = Vec::with_capacity(1);
        if kani::any() {
            o.card_types.push(CardType::Creature);
        }
        *slot = id;
    }
    Reckoning { registry, state, card, spell, permanents }
}

impl Reckoning {
    fn creatures_of(&self, p: PlayerId) -> Vec<ObjectId> {
        let mut ids = Vec::with_capacity(3);
        for id in self.permanents {
            let o = self.state.get_object(id).expect("on the board");
            if o.controller == p && o.card_types.contains(&CardType::Creature) {
                ids.push(id);
            }
        }
        ids
    }

    /// The player asked, and the creatures offered, if one is.
    fn asked(&self) -> Option<(PlayerId, Vec<ObjectId>)> {
        match &self.state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice {
                player, source, choice: ResolutionChoiceKind::ChooseTarget {
                    options, optional: false, effect: PendingEffect::CardEffect { source_id, .. }, ..
                },
            }) if *source == self.spell && *source_id == self.spell => {
                let mut ids = Vec::with_capacity(options.len());
                for t in options {
                    let Target::Object(id) = t else { return None };
                    ids.push(*id);
                }
                Some((*player, ids))
            }
            _ => None,
        }
    }
}

card_proof! {
    /// Divine Reckoning: each player, the active player first and then in
    /// turn order (CR 101.4), chooses a creature they control: asked when
    /// they control more than one, the one when they control one, none
    /// when none; then every creature not chosen is destroyed, all at once
    /// (CR 701.8a, one event). Over three permanents, each either
    /// player's and each a creature or not, either player active, and
    /// every choice of a player asked.
    fn divine_reckoning_keeps_one_creature_each_and_destroys_the_rest() {
        let mut r = reckoning();
        let before = Snapshot::of(&r.state);
        let spell = r.spell;
        let active = r.state.active_player;
        r.registry.get(r.card).expect("registered").on_resolve(&mut r.state, spell, &[], &r.registry);
        let mut kept = Vec::with_capacity(2);
        for player in [active, PlayerId(1 - active.0)] {
            let theirs = r.creatures_of(player);
            match theirs.len() {
                0 => {}
                1 => kept.push(theirs[0]),
                _ => {
                    let Some((asked, offered)) = r.asked() else { panic!("a player with a choice is asked it") };
                    assert!(asked == player && offered == theirs && acted() == 0);
                    let chosen = theirs[usize::from(kani::any::<bool>()) % theirs.len()];
                    r.state.awaiting_action = None;
                    r.registry.get(r.card).expect("registered").resolve_card_effect(&mut r.state, spell, "", &Target::Object(chosen), &r.registry);
                    kept.push(chosen);
                }
            }
        }
        assert!(r.state.awaiting_action.is_none());
        let mut destroyed = Vec::with_capacity(3);
        for id in r.permanents {
            if r.state.get_object(id).expect("on the board").card_types.contains(&CardType::Creature) && !kept.contains(&id) {
                destroyed.push(id);
            }
        }
        let Some(Act::DestroyAll(outcomes)) = super::act(0) else { panic!("destroys the rest") };
        assert!(acted() == 1 && outcomes.len() == destroyed.len());
        for (i, (id, _)) in outcomes.iter().enumerate() {
            assert!(*id == destroyed[i]);
        }
        assert!(Snapshot::of(&r.state) == before);
        std::mem::forget((r, kept, destroyed, outcomes));
    }
}
