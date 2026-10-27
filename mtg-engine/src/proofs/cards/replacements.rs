//! The permanents whose abilities are replacement effects (CR 614): what
//! each does to every event the engine offers it, and that it leaves every
//! other event alone.
//!
//! - Diregraf Ghoul, 2/2: "This creature enters tapped."
//! - Grimgrin, Corpse-Born, 5/5: "Grimgrin enters tapped and ..."
//! - Festerhide Boar, 3/3 Trample: "Morbid — This creature enters with two
//!   +1/+1 counters on it if a creature died this turn."
//! - Somberwald Spider, 2/4 Reach: "Morbid — This creature enters with two
//!   +1/+1 counters on it if a creature died this turn."
//! - Parallel Lives, Enchantment: "If an effect would create one or more
//!   tokens under your control, it creates twice that many of those tokens
//!   instead."
//! - Laboratory Maniac, 2/2: "If you would draw a card while your library
//!   has no cards in it, you win the game instead."
//! - Essence of the Wild, 6/6: "Creatures you control enter as a copy of
//!   this creature."
//! - Mikaeus, the Lunarch, 0/0: "Mikaeus enters with X +1/+1 counters on
//!   it."
//! - Evil Twin, 0/0: "You may have this creature enter as a copy of any
//!   creature on the battlefield, except it has '{U}{B}, {T}: Destroy
//!   target creature with the same name as this creature.'"
//!
//! Each proof offers the card's permanent, controlled by either player,
//! every event the engine asks a replacement effect about: a permanent
//! entering (itself or another, from a hand or as a token, under either
//! player, tapped or not, with counters or not, as a copy or not), tokens
//! being created under either player, either player drawing from an empty
//! library, and damage being dealt. Which permanents' effects the engine
//! asks, and in what order it applies them, is `proofs::actions`'.

use super::{acted, asked_for, blank_board, card_proof, Act};
use crate::cards::{CardBehavior, CardRegistry};
use crate::events::{DamageTarget, LossReason};
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::{EnteringPermanent, ReplaceableEvent as Ev, Replacement};
use crate::state::{EnterAsCopyChoice, GameResult, GameState};
use crate::types::{CardType, CounterType, Zone};

use crate::cards::isd;

/// The board: the card's permanent, on the battlefield under either player,
/// and another object, a card in a hand that is a creature or not; and
/// the event offered.
struct Offer {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    other: ObjectId,
    you: PlayerId,
    event: Ev,
}

fn offer(card: Box<dyn CardBehavior>) -> Offer {
    let (mut registry, mut state, blank) = blank_board();
    let card = registry.register(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let me = state.create_object(card, you, Zone::Battlefield, None, None);
    let other = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Hand, None, None);
    let o = state.get_object_mut(other).expect("just created");
    o.card_types = Vec::with_capacity(1);
    if kani::any() {
        o.card_types.push(CardType::Creature);
    }
    state.creature_died_this_turn = kani::any();
    let event = any_event(me, other);
    Offer { registry, state, me, other, you, event }
}

fn any_player() -> PlayerId {
    PlayerId(u8::from(kani::any::<bool>()))
}

/// Any event a replacement effect is offered, about `me` or `other`.
fn any_event(me: ObjectId, other: ObjectId) -> Ev {
    match kani::any::<u8>() % 4 {
        0 => {
            let object = if kani::any() { me } else { other };
            let mut counters = Vec::with_capacity(1);
            if kani::any() {
                counters.push((CounterType::PlusOnePlusOne, 1));
            }
            Ev::EntersBattlefield(EnteringPermanent {
                object,
                from: if kani::any() { Some(Zone::Hand) } else { None },
                controller: any_player(),
                tapped: kani::any(),
                counters,
                copy_of: if kani::any() { None } else if kani::any() { Some(me) } else { Some(other) },
            })
        }
        1 => {
            let count: u32 = kani::any();
            kani::assume(count <= 13);
            Ev::CreatesTokens { controller: any_player(), count }
        }
        2 => Ev::DrawsFromEmptyLibrary { player: any_player() },
        _ => Ev::DealsDamage {
            source: other,
            target: if kani::any() { DamageTarget::Object(me) } else { DamageTarget::Player(any_player()) },
            amount: 1,
            combat: kani::any(),
        },
    }
}

impl Offer {
    fn replace(&mut self) -> Option<Replacement> {
        let card = self.state.get_object(self.me).expect("on the board").card_id;
        self.registry.get(card).expect("registered").replace_event(&mut self.state, self.me, &self.event.clone(), &self.registry)
    }

    /// The event, if it is this permanent's own entering.
    fn own_entering(&self) -> Option<EnteringPermanent> {
        match &self.event {
            Ev::EntersBattlefield(e) if e.object == self.me => Some(e.clone()),
            _ => None,
        }
    }
}

/// `card` replaces its own entering by `entering(e)` when that is `Some`,
/// and leaves every other event, and its own entering when it is `None`,
/// as it is; and does nothing else.
fn replaces_its_own_entering(card: Box<dyn CardBehavior>, entering: impl FnOnce(&GameState, &EnteringPermanent) -> Option<EnteringPermanent>) {
    let mut o = offer(card);
    let expected = o.own_entering().and_then(|e| entering(&o.state, &e));
    let replaced = o.replace();
    match expected {
        Some(e) => {
            assert!(replaced == Some(Replacement::Modified(Ev::EntersBattlefield(e))));
        }
        None => {
            assert!(replaced.is_none());
        }
    }
    assert!(acted() == 0);
    std::mem::forget((o, replaced));
}

card_proof! {
    /// Diregraf Ghoul: entering untapped, it enters tapped instead (CR
    /// 614.1c); entering tapped already, there is nothing to replace.
    fn diregraf_ghoul_enters_tapped() {
        replaces_its_own_entering(Box::new(isd::diregraf_ghoul::DiregrafGhoul),
            |_, e| (!e.tapped).then(|| EnteringPermanent { tapped: true, ..e.clone() }));
    }
}

card_proof! {
    /// Grimgrin, Corpse-Born: enters tapped, as Diregraf Ghoul does.
    fn grimgrin_enters_tapped() {
        replaces_its_own_entering(Box::new(isd::grimgrin_corpse_born::GrimgrinCorpseBorn),
            |_, e| (!e.tapped).then(|| EnteringPermanent { tapped: true, ..e.clone() }));
    }
}

/// Morbid's "enters with two +1/+1 counters on it if a creature died this
/// turn" (CR 614.1c, 700.4): two more counters as it enters exactly when a
/// creature has died this turn.
fn morbid_counters(state: &GameState, e: &EnteringPermanent) -> Option<EnteringPermanent> {
    state.creature_died_this_turn.then(|| {
        let mut counters = e.counters.clone();
        counters.push((CounterType::PlusOnePlusOne, 2));
        EnteringPermanent { counters, ..e.clone() }
    })
}

card_proof! {
    /// Festerhide Boar: morbid's two +1/+1 counters.
    fn festerhide_boar_enters_with_two_counters_after_a_death() {
        replaces_its_own_entering(Box::new(isd::festerhide_boar::FesterhideBoar), morbid_counters);
    }
}

card_proof! {
    /// Somberwald Spider: morbid's two +1/+1 counters.
    fn somberwald_spider_enters_with_two_counters_after_a_death() {
        replaces_its_own_entering(Box::new(isd::somberwald_spider::SomberwaldSpider), morbid_counters);
    }
}

card_proof! {
    /// Mikaeus, the Lunarch: X +1/+1 counters more as it enters (CR
    /// 614.1c), X being the value it was cast with (CR 107.3m), and none
    /// cast without one; X of 0 recorded all the same, so that the log can
    /// say why a 0/0 Mikaeus died. Over X from 0 to 3, and none.
    fn mikaeus_the_lunarch_enters_with_x_counters() {
        let mut o = offer(Box::new(isd::mikaeus_the_lunarch::MikaeusTheLunarch));
        let x: Option<u32> = kani::any();
        kani::assume(x.is_none_or(|x| x <= 3));
        o.state.get_object_mut(o.me).expect("on the board").x_value = x;
        let expected = o.own_entering().map(|e| {
            let mut counters = Vec::with_capacity(e.counters.len() + 1);
            counters.extend_from_slice(&e.counters);
            counters.push((CounterType::PlusOnePlusOne, x.unwrap_or(0)));
            EnteringPermanent { counters, ..e }
        });
        let replaced = o.replace();
        match expected {
            Some(e) => {
                assert!(replaced == Some(Replacement::Modified(Ev::EntersBattlefield(e))));
            }
            None => {
                assert!(replaced.is_none());
            }
        }
        assert!(acted() == 0);
        std::mem::forget((o, replaced));
    }
}

card_proof! {
    /// Evil Twin: its controller is asked as it enters what to enter as
    /// (CR 614.12, the engine asking, as the card says it does); entering
    /// with the answer a creature to copy, it enters as a copy of that
    /// creature (CR 706.2), its copy carrying the card's ability (CR
    /// 706.9a: an exception the copy effect makes, the card says it does);
    /// declined, or not yet asked, as itself; and a copy effect already
    /// applied to its entering (Essence of the Wild's, CR 616.1) is left as
    /// it is. Every other event as it is.
    fn evil_twin_enters_as_a_copy_of_the_chosen_creature() {
        let twin = isd::evil_twin::EvilTwin;
        assert!(twin.chooses_copy_as_it_enters() && twin.grants_abilities_to_copies());
        let mut o = offer(Box::new(twin));
        let choice = match kani::any::<u8>() % 3 {
            0 => EnterAsCopyChoice::Unasked,
            1 => EnterAsCopyChoice::Declined,
            _ => EnterAsCopyChoice::Copy(o.other),
        };
        o.state.get_object_mut(o.me).expect("on the board").entering_copy_choice = choice;
        let expected = o.own_entering().and_then(|e| match choice {
            EnterAsCopyChoice::Copy(source) if e.copy_of.is_none() => Some(EnteringPermanent { copy_of: Some(source), ..e }),
            _ => None,
        });
        let replaced = o.replace();
        match expected {
            Some(e) => {
                assert!(replaced == Some(Replacement::Modified(Ev::EntersBattlefield(e))));
            }
            None => {
                assert!(replaced.is_none());
            }
        }
        assert!(acted() == 0);
        std::mem::forget((o, replaced));
    }
}

card_proof! {
    /// Parallel Lives: tokens created under its controller's control, twice
    /// as many; under anyone else's, and every other event, as it is.
    fn parallel_lives_doubles_your_tokens() {
        let mut o = offer(Box::new(isd::parallel_lives::ParallelLives));
        let expected = match o.event {
            Ev::CreatesTokens { controller, count } if controller == o.you => Some(Replacement::Modified(Ev::CreatesTokens { controller, count: 2 * count })),
            _ => None,
        };
        let replaced = o.replace();
        assert!(replaced == expected && acted() == 0);
        std::mem::forget((o, replaced));
    }
}

card_proof! {
    /// Laboratory Maniac: its controller drawing from an empty library
    /// instead wins the game (CR 104.2b): the draw replaced, so it is not
    /// the draw from an empty library that loses (704.5b), and the
    /// opponent loses as the effect says they do, and the game ends with
    /// its controller the winner. Anyone else's draw, and every other
    /// event, as it is.
    fn laboratory_maniac_wins_instead_of_drawing_from_an_empty_library() {
        let mut o = offer(Box::new(isd::laboratory_maniac::LaboratoryManiac));
        for p in 0..2 {
            o.state.players[p].has_drawn_from_empty = true;
        }
        let replaced = o.replace();
        match o.event {
            Ev::DrawsFromEmptyLibrary { player } if player == o.you => {
                assert!(replaced == Some(Replacement::Replaced));
                let opponent = PlayerId(1 - player.0);
                assert!(asked_for(&[
                    Act::Loses(opponent, LossReason::OpponentWon { source: o.me }),
                    Act::GameEnds(GameResult::Winner(player)),
                ]));
                assert!(!o.state.players[usize::from(player.0)].has_drawn_from_empty);
                assert!(o.state.players[usize::from(opponent.0)].has_drawn_from_empty);
            }
            _ => {
                assert!(replaced.is_none() && acted() == 0);
                assert!(o.state.players[0].has_drawn_from_empty && o.state.players[1].has_drawn_from_empty);
            }
        }
        std::mem::forget((o, replaced));
    }
}

card_proof! {
    /// Essence of the Wild: another creature entering under its
    /// controller's control enters as a copy of it (CR 614.1c, 706.9);
    /// itself, a noncreature, a creature under the other player, and every
    /// other event, as it is. A creature already entering as a copy of
    /// something, another copy effect having been applied first, is left as
    /// it is (CR 616.1: the order is the engine's, proved there).
    fn essence_of_the_wild_makes_your_creatures_enter_as_copies_of_it() {
        let mut o = offer(Box::new(isd::essence_of_the_wild::EssenceOfTheWild));
        let other_is_creature = o.state.get_object(o.other).expect("on the board").card_types.contains(&CardType::Creature);
        let expected = match &o.event {
            Ev::EntersBattlefield(e) if e.object == o.other && other_is_creature && e.controller == o.you && e.copy_of.is_none() => {
                Some(Replacement::Modified(Ev::EntersBattlefield(EnteringPermanent { copy_of: Some(o.me), ..e.clone() })))
            }
            _ => None,
        };
        let replaced = o.replace();
        assert!(replaced == expected && acted() == 0);
        std::mem::forget((o, replaced));
    }
}

card_proof! {
    /// Unbreathing Horde: entering, under whichever player it enters under
    /// (CR 614.12), it enters with one more +1/+1 counter for each other
    /// Zombie that player controls and each Zombie card in that player's
    /// graveyard, itself among them if it is entering from there; any
    /// other event as it is. (Its damage prevention is the damage
    /// pipeline's, proved in `proofs::actions`.) Over the Horde in its
    /// owner's hand or graveyard, entering under either player, and a card
    /// and a token of any characteristics, each a Zombie or not.
    fn unbreathing_horde_enters_with_a_counter_for_each_of_your_zombies() {
        let (registry, mut state, blank, card) = super::board(Box::new(isd::unbreathing_horde::UnbreathingHorde));
        let from = if kani::any() { Zone::Hand } else { Zone::Graveyard };
        let me = state.create_object(card, PlayerId(0), from, None, None);
        if from == Zone::Graveyard {
            state.players[0].graveyard_order.push(me);
        }
        const ZOMBIES: super::Kinds = super::Kinds { types: &[CardType::Creature], subtypes: &["Zombie"], keywords: &[] };
        let others = [super::any_object(&mut state, blank, &ZOMBIES, super::Form::Card), super::any_object(&mut state, blank, &ZOMBIES, super::Form::Token)];
        let under = PlayerId(u8::from(kani::any::<bool>()));
        let e = EnteringPermanent { object: me, from: Some(from), controller: under, tapped: false, counters: Vec::with_capacity(1), copy_of: None };
        let mut n = u32::from(from == Zone::Graveyard && under == PlayerId(0));
        for id in others {
            let o = state.get_object(id).expect("on the board");
            let zombie = o.subtypes.iter().any(|s| s == "Zombie");
            if zombie && ((o.zone == Zone::Battlefield && o.controller == under) || (o.zone == Zone::Graveyard && o.owner == under && !o.is_token)) {
                n += 1;
            }
        }
        let replaced = registry.get(card).expect("registered").replace_event(&mut state, me, &Ev::EntersBattlefield(e.clone()), &registry);
        let mut counters = Vec::with_capacity(1);
        counters.push((CounterType::PlusOnePlusOne, n));
        assert!(replaced == Some(Replacement::Modified(Ev::EntersBattlefield(EnteringPermanent { counters, ..e.clone() }))));
        let other = registry.get(card).expect("registered").replace_event(&mut state, me, &Ev::CreatesTokens { controller: under, count: 1 }, &registry);
        assert!(other.is_none() && acted() == 0);
        std::mem::forget((registry, state, replaced, e));
    }
}

card_proof! {
    /// Undead Alchemist: combat damage a Zombie its controller controls
    /// would deal to a player is instead that many cards milled by that
    /// player (CR 614.1a), while the Alchemist is on the battlefield; any
    /// other damage, and every other event, as it is. It offers the
    /// replacement exactly when it applies.
    fn undead_alchemist_turns_your_zombies_combat_damage_into_milling() {
        let (registry, mut state, blank, card) = super::board(Box::new(isd::undead_alchemist::UndeadAlchemist));
        let you = PlayerId(u8::from(kani::any::<bool>()));
        let on: bool = kani::any();
        let me = state.create_object(card, PlayerId(0), if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
        if on {
            state.get_object_mut(me).expect("just created").controller = you;
        }
        const ZOMBIES: super::Kinds = super::Kinds { types: &[CardType::Creature], subtypes: &["Zombie"], keywords: &[] };
        let source = super::any_object(&mut state, blank, &ZOMBIES, super::Form::Token);
        let player = PlayerId(u8::from(kani::any::<bool>()));
        let amount: u32 = kani::any();
        kani::assume((1..=5).contains(&amount));
        let combat: bool = kani::any();
        let to_player: bool = kani::any();
        let event = Ev::DealsDamage {
            source,
            target: if to_player { DamageTarget::Player(player) } else { DamageTarget::Object(me) },
            amount,
            combat,
        };
        let s = state.get_object(source).expect("on the board");
        let applies = on && combat && to_player && s.controller == you && s.subtypes.iter().any(|t| t == "Zombie");
        let alchemist = registry.get(card).expect("registered");
        assert!(alchemist.replacement_offer(&state, me, &event, &registry).is_some() == applies);
        let replaced = alchemist.replace_event(&mut state, me, &event, &registry);
        if applies {
            assert!(replaced == Some(Replacement::Replaced) && asked_for(&[Act::Mill(player, amount as usize)]));
        } else {
            assert!(replaced.is_none() && acted() == 0);
        }
        std::mem::forget((registry, state, replaced, event));
    }
}

card_proof! {
    /// Undead Alchemist: "Whenever a creature card is put into an
    /// opponent's graveyard from their library" (which milling the engine
    /// tells it about, `proofs::actions`), that card exiled if it is still
    /// in a graveyard, and a 2/2 black Zombie creature token for its
    /// controller either way (the card's ruling).
    fn undead_alchemist_exiles_milled_creatures_for_zombies() {
        assert!(super::triggers(&isd::undead_alchemist::UndeadAlchemist) == [(crate::cards::TriggerKind::CreatureCardMilled, None)]);
        let (registry, mut state, blank, card) = super::board(Box::new(isd::undead_alchemist::UndeadAlchemist));
        let you = PlayerId(u8::from(kani::any::<bool>()));
        let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
        state.get_object_mut(me).expect("just created").controller = you;
        let still_there: bool = kani::any();
        let milled = state.create_object(blank, PlayerId(1 - you.0), if still_there { Zone::Graveyard } else { Zone::Exile }, None, None);
        let before = super::Snapshot::of(&state);
        registry.get(card).expect("registered").on_creature_card_milled(&mut state, me, milled, PlayerId(1 - you.0), &registry);
        let zombie = Act::Tokens {
            n: 1, controller: you, power: 2, toughness: 2, colors: vec![crate::types::Color::Black],
            types: vec![CardType::Creature], keywords: vec![], subtypes: vec!["Zombie".into()],
        };
        if still_there {
            assert!(asked_for(&[Act::Move(milled, Zone::Exile), zombie]));
        } else {
            assert!(asked_for(&[zombie]));
        }
        assert!(super::Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}
